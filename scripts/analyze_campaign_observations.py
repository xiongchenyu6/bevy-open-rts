#!/usr/bin/env python3
"""Read archived campaign observations without inferring combat/path causes.

Inputs are never modified. Output contains sampled facts, source locations and
missing fields. Absence from a later snapshot is not labelled a death.
"""
import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

FOCUS = {"Tank", "AntiAirWalker", "LongbowMissileCrawler"}
CLOCK = re.compile(r"\[capture\] campaign bot [^:]+: t=([0-9.]+)")


def read_json(path):
    return json.loads(path.read_text()) if path.exists() else None


def read_jsonl(path):
    if path.exists():
        with path.open() as stream:
            for number, line in enumerate(stream, 1):
                if line.strip():
                    yield number, json.loads(line)


def input_record(path):
    return {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def analyze(evidence, log):
    telemetry_path = evidence / "telemetry.jsonl"
    samples = list(read_jsonl(telemetry_path))
    if not samples:
        raise ValueError("No periodic telemetry samples found")
    rows = [row for _, row in samples]
    unit_fields = set()
    entities = {}
    births = []
    assault_samples = []
    air_samples = []
    ledger = []
    hq_samples = []
    for line, row in samples:
        clock = row["clock"]
        hq_samples.append({"clock": clock, "headquarters": row.get("headquarters")})
        ledger.append({"clock": clock, "resources": row.get("resources"),
                       "low_power": row.get("low_power"), "queued_counts": row.get("queued"),
                       "producer_sites": [s for s in row.get("structures", [])
                                          if s["id"] in {"VehicleFactory", "Barracks", "CommandCenter"}]})
        air = [t for t in row.get("threats", []) if t.get("domain") == "air"]
        if air:
            air_samples.append({"clock": clock, "actors": air,
                                "identity_and_visibility": "not recorded in raw threat entries"})
        for unit in row.get("unit_orders", []):
            unit_fields.update(unit)
            if unit["id"] not in FOCUS:
                continue
            key = str(unit["entity"])
            if key not in entities:
                births.append({"entity": unit["entity"], "type": unit["id"],
                               "first_observed_clock": clock, "telemetry_line": line,
                               "event": "first appearance in these samples; not an exact birth timestamp"})
                entities[key] = {"type": unit["id"], "first_observed_clock": clock}
            entities[key]["last_observed_clock"] = clock
            entities[key]["last_observation"] = unit
            if 1 in unit.get("groups", []):
                assault_samples.append({"clock": clock, "entity": unit["entity"],
                                        "type": unit["id"], "position": unit.get("position"),
                                        "hp": unit.get("hp"), "move_target": unit.get("move_target"),
                                        "moving": unit.get("moving"), "holding": unit.get("holding"),
                                        "telemetry_line": line})
    trace_path = evidence / "construction-trace.jsonl"
    trace_fields, trace_clocks, trace_focus = set(), [], Counter()
    for _, entry in read_jsonl(trace_path):
        snapshot = entry.get("snapshot", entry)
        trace_clocks.append(snapshot["clock"])
        for entity in snapshot.get("entities", []):
            trace_fields.update(entity)
            if entity.get("team") == "Player(0)" and entity.get("unit") in FOCUS:
                trace_focus[entity["unit"]] += 1
    endpoint_path = evidence / "failure-contact.json"
    endpoint = read_json(endpoint_path) or {}
    endpoint_entities = endpoint.get("entities", [])
    endpoint_hqs = [{"entity": e["entity"], "team": e.get("team"),
                     "position": e["position"], "health": e.get("health"),
                     "visibility": "not recorded in this diagnostic snapshot"}
                    for e in endpoint_entities if e.get("structure") == "CommandCenter"]
    endpoint_engineers = [e for e in endpoint_entities if e.get("unit") == "EngineerDrone"]
    lines = log.read_text().splitlines() if log.exists() else []
    clocks = [(i, float(match.group(1))) for i, text in enumerate(lines, 1)
              if (match := CLOCK.search(text))]
    events = []
    for i, text in enumerate(lines, 1):
        if any(marker in text for marker in ["visible-only assault anchor", "visible-only map scouting",
                "visible-air guard intercept", "died during", "empty-ground click", "error:"]):
            before = [t for n, t in clocks if n < i]
            after = [t for n, t in clocks if n > i]
            events.append({"log_line": i, "text": text,
                           "bounding_logged_clock": [before[-1] if before else None,
                                                     after[0] if after else None],
                           "exact_event_clock": "not recorded"})
    source_paths = [telemetry_path, trace_path, endpoint_path,
                    evidence / "process-result.json", evidence / "artifact.json", log]
    fields = {
        "MoveOrder": {"live_mine_state": "units.active[].Move coordinates",
                      "construction_debug": "entities.move_target", "periodic_recorded": "move_target" in unit_fields},
        "AttackMoveOrder": {"live_mine_state": "units.active[].AttackMove coordinates",
                            "construction_debug": "not exposed", "periodic_recorded": False},
        "AttackOrder": {"live_mine_state": "units.active[].Attack target entity key",
                        "construction_debug": "not exposed", "periodic_recorded": False},
        "PlannedPath.goal": {"live_mine_state": "not exposed", "construction_debug": "entities.path_goal",
                             "periodic_recorded": "path_goal" in unit_fields},
        "PlannedPath.next": {"construction_debug": "entities.path_next is usize INDEX, not a coordinate",
                             "next_waypoint_coordinate": "not exposed by either existing API",
                             "periodic_recorded": "path_next" in unit_fields},
        "EMP": {"live_mine_state": "effects not included in this view", "construction_debug": "entities.emp_remaining",
                "periodic_recorded": "emp_remaining" in unit_fields},
        "visible_enemy_HQ": {"live_mine_state": "structures: key/team/id/position/health/actual visible bool",
                             "construction_debug": "no visible bool",
                             "periodic_raw_threat_list": "units only, no visibility bool or entity key"},
    }
    longbows = sum(row.get("types", {}).get("LongbowMissileCrawler", 0) for row in rows)
    trace_end = max(trace_clocks) if trace_clocks else None
    return {
        "scope": "archived observations only; no simulation, commands, input edits or causal verdict",
        "inputs": [input_record(p) for p in source_paths if p.exists()],
        "process": read_json(evidence / "process-result.json"),
        "artifact": read_json(evidence / "artifact.json"),
        "telemetry": {"sample_count": len(rows), "clock_range": [rows[0]["clock"], rows[-1]["clock"]],
                      "own_unit_fields": sorted(unit_fields), "focus_entities": entities,
                      "first_appearances": births, "assault_observations": assault_samples,
                      "longbow_observed": bool(longbows)},
        "hq_observations": hq_samples,
        "economy_and_producer_observations": ledger,
        "raw_air_observations": air_samples,
        "construction_trace": {"sample_count": len(trace_clocks),
                               "clock_range": [min(trace_clocks), trace_end] if trace_clocks else None,
                               "entity_fields": sorted(trace_fields), "focus_observation_counts": dict(trace_focus)},
        "failure_endpoint": {"clock": endpoint.get("clock"), "headquarters": endpoint_hqs,
                             "engineer_count": len(endpoint_engineers),
                             "own_focus_units": [e for e in endpoint_entities
                                                 if e.get("team") == "Player(0)" and e.get("unit") in FOCUS]},
        "recorded_action_events": events,
        "api_coverage": fields,
        "missing_for_causal_route_analysis": [
            "Periodic AttackOrder target keys and AttackMove destinations were not archived.",
            "Periodic PlannedPath goal/index/next coordinate and EMP were not archived.",
            f"Construction snapshots stop at {trace_end}; they do not cover the later assault.",
            "No live_mine_state snapshots with actual enemy-HQ visibility/health were archived.",
            "Visible HQ anchor log events exist, but no exact event timestamp or HQ HP was logged.",
            "Raw air entries lack entity keys; type-level HP samples cannot identify one actor over time.",
            "Paid producer queue head timers, charged costs and blocked-spawn status were not archived.",
        ],
        "limits": ["Missing entity from a later sample is not labelled a death.",
                   "Queue counts alone do not establish production blockage.",
                   "Moving flags and positions alone do not distinguish chase, path obstruction or combat.",
                   "Endpoint HQ presence/absence is not assigned an unrecorded capture/damage mechanism.",
                   "Raw enemy diagnostics are not used to issue actions or infer visibility."],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = analyze(args.evidence, args.log)
    text = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        with args.output.open("x") as stream:
            stream.write(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
