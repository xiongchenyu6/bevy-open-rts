#!/usr/bin/env python3
"""Stage or integrate approved original model images, preserving old evidence."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def runtime_hashes():
    names = subprocess.check_output(["git", "ls-files", "-z", "assets"], cwd=ROOT).decode().split("\0")
    return {name: digest(ROOT / name) for name in names if name}


def prepare(model_stage, roster_stage):
    inventory = json.loads((ROOT / "art/portraits/entities.json").read_text())
    model_records = json.loads((model_stage / "manifest.json").read_text())
    pairs = {(row["id"], row["run"]): row for row in model_records}
    expected = {entry["entity_id"] for entry in inventory["entities"]}
    if len(pairs) != 140 or set(pairs) != {(entity, run) for entity in expected for run in (0, 1)}:
        raise ValueError("Model stage must contain exactly 70 paired entities")
    selections = []
    for entry in inventory["entities"]:
        entity = entry["entity_id"]
        row = pairs[entity, 0]
        if row["runtime_path"] != entry["runtime_path"] or row["reference_scope"] != "model-only parity; not whole-scene parity":
            raise ValueError(f"Invalid model-only identity: {entity}")
        source = model_stage / f"{entity}-0.png"
        if digest(source) != digest(model_stage / f"{entity}-1.png"):
            raise ValueError(f"Independent model renders differ: {entity}")
        selections.append({"runtime_path": "assets/" + entry["runtime_path"], "source": str(source.relative_to(ROOT)),
                           "entity_id": entity, "faction": entry["portrait_faction"], "model_assets": entry["model_assets"],
                           "assembly_source": entry["assembly_source"], "reference_scope": row["reference_scope"],
                           "geometry_sha256": hashlib.sha256(json.dumps(row["geometry"], sort_keys=True).encode()).hexdigest(),
                           "excluded_contextual_marker_count": row["excluded_contextual_marker_count"]})
    roster = json.loads((roster_stage / "manifest.json").read_text())
    if {row["run"] for row in roster["runs"]} != {0, 1} or len(roster["runs"]) != 2:
        raise ValueError("Roster needs two distinct runs")
    source = roster_stage / "RosterPreview-0.png"
    if digest(source) != digest(roster_stage / "RosterPreview-1.png"):
        raise ValueError("Independent roster renders differ")
    selections.append({"runtime_path": "assets/ui/icons/RosterPreview.png", "source": str(source.relative_to(ROOT)),
                       "entity_id": None, "members": roster["configuration"]["members"],
                       "reference_scope": "Original mixed-faction asset-only composition; per-member model-only parity"})
    before = runtime_hashes()
    if len(before) != 338 or len(selections) != 71 or len({row["runtime_path"] for row in selections}) != 71:
        raise ValueError("Expected 338 tracked runtime files and 71 unique replacements")
    for row in selections:
        path = ROOT / row["source"]
        row["previous_sha256"] = before[row["runtime_path"]]
        row["png_sha256"] = digest(path)
        rgba = subprocess.check_output(["ffmpeg", "-v", "error", "-threads", "1", "-i", str(path),
                                        "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
        if len(rgba) != 512 * 512 * 4:
            raise ValueError(f"Incorrect RGBA size: {path}")
        row["decoded_rgba_sha256"] = hashlib.sha256(rgba).hexdigest()
    return {"schema": 1, "scope": "71 original runtime image replacements; bounded asset/model/UI evidence, not all-roster paid gameplay",
            "before": before, "selections": selections, "runtime_replaced": False,
            "full_roster_paid_gameplay_verified": False, "final_package_verified": False}


def apply(plan, backup):
    if runtime_hashes() != plan["before"]:
        raise ValueError("Runtime assets changed since the prepared plan")
    ledger_before = json.loads((ROOT / "docs/asset-source-hashes.json").read_text())
    stale = {entry["path"] for entry in ledger_before["assets"]
             if plan["before"][entry["path"]] != entry["sha256"]}
    expected_stale = {"assets/migration/asset_manifest.json",
                      "assets/data/godot_model_map.model_map.ron",
                      "assets/migration/gameplay_registry.json"}
    if stale != expected_stale:
        raise ValueError(f"Unexpected stale-ledger scope: {sorted(stale)}")
    backup.mkdir()
    for row in plan["selections"]:
        old = ROOT / row["runtime_path"]
        saved = backup / row["runtime_path"]
        saved.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(old, saved)
        if digest(saved) != row["previous_sha256"]:
            raise ValueError("Legacy backup mismatch")
    ledger_path = ROOT / "docs/asset-source-hashes.json"
    shutil.copyfile(ledger_path, backup / "asset-source-hashes-before.json")
    for row in plan["selections"]:
        source = ROOT / row["source"]
        if digest(source) != row["png_sha256"]:
            raise ValueError("Candidate source changed")
    for row in plan["selections"]:
        shutil.copyfile(ROOT / row["source"], ROOT / row["runtime_path"])
    after = runtime_hashes()
    selected = {row["runtime_path"] for row in plan["selections"]}
    changed = {name for name in after if after[name] != plan["before"][name]}
    if set(after) != set(plan["before"]) or changed != selected or len(after) != 338:
        raise ValueError("Exact 71-path integration invariant failed")
    for row in plan["selections"]:
        if after[row["runtime_path"]] != row["png_sha256"]:
            raise ValueError("Integrated PNG hash mismatch")
    ledger = json.loads(ledger_path.read_text())
    old_entries = {row["path"]: row for row in ledger["assets"]}
    ledger["retired_native_portrait_images"] = {"scope": "Historical replaced image provenance; not current runtime sources",
                                                "records": [old_entries[row["runtime_path"]] for row in plan["selections"]]}
    replacement_entries = {}
    for row in plan["selections"]:
        replacement_entries[row["runtime_path"]] = {
            "path": row["runtime_path"], "sha256": row["png_sha256"],
            "comparison": "generated_original_model_portrait" if row["entity_id"] else "generated_original_roster",
            "provenance": "art/portraits/runtime-integration.json",
            "decoded_rgba_sha256": row["decoded_rgba_sha256"], "entity_id": row["entity_id"],
            "source_scope": row["reference_scope"], "all_roster_paid_gameplay_verified": False}
    refreshed = []
    for entry in ledger["assets"]:
        path = entry["path"]
        if path in replacement_entries:
            continue
        current = after[path]
        if current != entry["sha256"]:
            refreshed.append(entry.copy())
            entry = {"path": path, "sha256": current, "comparison": "authored_derived_registry_metadata",
                     "provenance": "art/portraits/attachment_transforms.json",
                     "scope": "Current derived source metadata; no upstream byte-equality claim"}
        replacement_entries[path] = entry
    ledger["derived_source_refresh_history"] = {"scope": "Historical metadata hashes before reconciliation", "records": refreshed}
    ledger["assets"] = sorted(replacement_entries.values(), key=lambda row: row["path"])
    summary = defaultdict(Counter)
    for entry in ledger["assets"]:
        summary[entry["path"].split("/")[1]][entry["comparison"]] += 1
    ledger["summary_by_directory"] = {key: dict(value) for key, value in sorted(summary.items())}
    ledger_path.write_text(json.dumps(ledger, indent=2) + "\n")
    mismatches = [entry["path"] for entry in ledger["assets"] if digest(ROOT / entry["path"]) != entry["sha256"]]
    if mismatches:
        raise ValueError(f"Active ledger hash mismatches: {mismatches}")
    inventory_path = ROOT / "art/portraits/entities.json"
    inventory = json.loads(inventory_path.read_text())
    for entry in inventory["entities"]:
        entry["provenance_status"] = "original_native_model_runtime"
        entry["runtime_image_sha256"] = after["assets/" + entry["runtime_path"]]
        entry["runtime_image_provenance"] = "art/portraits/runtime-integration.json"
    inventory["unresolved_count"] = 0
    inventory["original_native_runtime_count"] = 70
    inventory.pop("already_provenance_count", None)
    inventory["scope"] = "Current original native runtime entity portraits; model-only parity and bounded UI review, not all-roster paid gameplay"
    inventory_path.write_text(json.dumps(inventory, indent=2) + "\n")
    plan["runtime_replaced"] = True
    plan["after"] = after
    plan["changed_paths"] = sorted(changed)
    plan["other_267_hashes_unchanged"] = True
    plan["active_ledger_hash_mismatches"] = 0
    plan["legacy_backup"] = str(backup.relative_to(ROOT))
    (backup / "integration-result.json").write_text(json.dumps(plan, indent=2) + "\n")
    (ROOT / "art/portraits/runtime-integration.json").write_text(json.dumps(plan, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model-stage", required=True, type=Path)
    parser.add_argument("--roster-stage", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--backup", type=Path)
    args = parser.parse_args()
    if args.apply:
        plan = json.loads(args.plan.read_text())
        if args.backup is None:
            parser.error("--apply needs a new --backup directory")
        apply(plan, args.backup.resolve())
        print("Integrated exactly 71 paths; other 267 hashes unchanged; active ledger matches")
    else:
        plan = prepare(args.model_stage.resolve(), args.roster_stage.resolve())
        with args.plan.open("x") as stream:
            json.dump(plan, stream, indent=2)
            stream.write("\n")
        print("Prepared 71-path plan; runtime assets unchanged")


if __name__ == "__main__":
    main()
