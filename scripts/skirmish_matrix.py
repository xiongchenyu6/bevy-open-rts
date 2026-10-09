#!/usr/bin/env python3
"""Run reproducible, serial real-system skirmishes; timeouts remain failures."""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

MAPS = {"plain_and_simple": 4, "four_corners": 4, "tech_divide": 6,
        "big_arena": 8, "canyon_pass": 2, "crossfire": 4,
        "ring_valley": 4, "highland_bastion": 2}
FACTIONS = ("alliance", "demon", "chaos")
DIFFICULTIES = ("beginner", "easy", "normal", "hard")


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def scenarios(kind, all_slots):
    cases = []
    for map_id, capacity in MAPS.items():
        slots = list(itertools.permutations(range(capacity), 2)) if all_slots else [(0, 1)]
        for starts in slots:
            if kind in ("factions", "all"):
                for factions in itertools.product(FACTIONS, repeat=2):
                    cases.append(dict(map=map_id, factions=list(factions),
                                      difficulties=["normal", "normal"], spawn_slots=list(starts)))
            if kind in ("difficulties", "all"):
                for faction in FACTIONS:
                    for low, high in zip(DIFFICULTIES, DIFFICULTIES[1:]):
                        for difficulties in ((low, high), (high, low)):
                            cases.append(dict(map=map_id, factions=[faction, faction],
                                              difficulties=list(difficulties), spawn_slots=list(starts)))
    return cases


def write_json(path, value):
    pending = path.with_suffix(path.suffix + ".pending")
    with pending.open("w") as stream:
        stream.write(json.dumps(value, indent=2) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
    pending.replace(path)
    if os.name == "posix":
        descriptor = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)


def cached_status(case_dir, case):
    """Keep valid observations; preserve interrupted files before replaying."""
    if not case_dir.exists():
        return None
    try:
        status = json.loads((case_dir / "status.json").read_text())
        if status.get("config") != case:
            raise ValueError("cached configuration differs")
        kind = status.get("classification")
        if kind in ("completed", "simulation_timeout"):
            report = json.loads((case_dir / "result.json").read_text())
            expected_complete = kind == "completed"
            if (report.get("config") != case
                    or report.get("completed") is not expected_complete
                    or report.get("timeout") is not (not expected_complete)
                    or status.get("exit_code") != (0 if expected_complete else 2)):
                raise ValueError("cached result does not match status")
        elif kind not in ("wall_timeout", "process_error"):
            raise ValueError("unknown cached status")
        return status
    except (OSError, ValueError, AttributeError):
        preserved = case_dir.parent / "interrupted"
        preserved.mkdir(exist_ok=True)
        destination = preserved / f"{case_dir.name}-{time.time_ns()}"
        case_dir.rename(destination)
        print(f"Preserved interrupted evidence {destination}; replaying case", flush=True)
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", choices=("factions", "difficulties", "all"), default="factions")
    parser.add_argument("--all-slots", action="store_true")
    parser.add_argument("--seconds", type=int, default=600)
    parser.add_argument("--wall-seconds", type=int, default=180)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/skirmish_audit"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    if not 1 <= args.seconds <= 3600 or args.wall_seconds < 1:
        parser.error("simulation limit must be 1..3600; wall limit must be positive")
    cases = scenarios(args.matrix, args.all_slots)
    for case in cases:
        case["max_seconds"] = args.seconds
    plan = dict(schema=1, matrix=args.matrix, all_slots=args.all_slots,
                wall_seconds=args.wall_seconds, scenarios=cases)
    if args.dry_run:
        print(json.dumps(dict(count=len(cases), plan=plan), indent=2))
        return 0
    if args.output is None:
        parser.error("--output is required unless --dry-run is used")
    root = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    manifest_path = output / "manifest.json"
    plan["source_binary_sha256"] = digest(binary)
    pinned = output / "skirmish_audit"
    if manifest_path.exists():
        if json.loads(manifest_path.read_text()) != plan:
            parser.error("existing matrix uses a different executable or configuration")
        if not pinned.is_file() or digest(pinned) != plan["source_binary_sha256"]:
            parser.error("pinned executable is missing or changed")
    else:
        if any(output.iterdir()):
            parser.error("new output directory must be empty")
        shutil.copyfile(binary, pinned)
        pinned.chmod(0o555)
        if digest(pinned) != plan["source_binary_sha256"]:
            parser.error("copied executable does not match source")
        write_json(manifest_path, plan)
    env = dict(os.environ, BEVY_ASSET_ROOT=str(root), RUST_LOG="warn")
    results = []
    for index, case in enumerate(cases):
        case_dir = output / f"{index:04d}"
        cached = cached_status(case_dir, case)
        if cached is not None:
            results.append(cached)
            continue
        case_dir.mkdir(exist_ok=True)
        status_path = case_dir / "status.json"
        evidence = case_dir / "result.json"
        factions, difficulties, slots = case["factions"], case["difficulties"], case["spawn_slots"]
        command = [str(pinned), case["map"], factions[0], difficulties[0], str(slots[0]),
                   factions[1], difficulties[1], str(slots[1]), str(args.seconds), str(evidence)]
        started = time.monotonic()
        with (case_dir / "runtime.log").open("w") as log:
            try:
                code = subprocess.run(command, cwd=root, env=env, stdout=log,
                                      stderr=subprocess.STDOUT, timeout=args.wall_seconds).returncode
                classification = "process_error"
            except subprocess.TimeoutExpired:
                code, classification = None, "wall_timeout"
            log.flush()
            os.fsync(log.fileno())
        try:
            report = json.loads(evidence.read_text()) if evidence.is_file() else None
        except (OSError, ValueError):
            report = None
        if evidence.is_file():
            with evidence.open("rb") as stream:
                os.fsync(stream.fileno())
        if report is not None and report.get("config") == case:
            if code == 0 and report.get("completed") is True and report.get("timeout") is False:
                classification = "completed"
            elif code == 2 and report.get("completed") is False and report.get("timeout") is True:
                classification = "simulation_timeout"
        status = dict(index=index, config=case, exit_code=code, classification=classification,
                      wall_seconds=time.monotonic() - started,
                      winning_teams=report.get("winning_teams", []) if report else [])
        write_json(status_path, status)
        results.append(status)
        counts = {kind: sum(item["classification"] == kind for item in results)
                  for kind in {item["classification"] for item in results}}
        write_json(output / "summary.json", dict(total=len(cases), observed=len(results), counts=counts))
        print(f"{index + 1}/{len(cases)} {case['map']} {classification}", flush=True)
    failed = any(item["classification"] != "completed" for item in results)
    return 2 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
