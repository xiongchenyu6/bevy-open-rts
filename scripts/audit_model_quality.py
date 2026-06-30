#!/usr/bin/env python3
"""Audit Bevy RTS model quality risks.

This harness is deliberately stricter than the Godot mapping audit. Godot scene
parts are useful reference data, but the Bevy port still needs clear silhouettes:
critical units must not share the same visible model recipe, and large multipart
"kitbash" units need screenshot review or replacement with a generated GLB.
"""

from __future__ import annotations

import argparse
import collections
import json
import re
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
REGISTRY_RS = REPO_ROOT / "src/generated_registry.rs"
DEFAULT_REPORT = REPO_ROOT / "screenshots/model-harness/model-quality-report.md"
DEFAULT_QUEUE = REPO_ROOT / "docs/model-quality/hunyuan3d-queue.json"
DEFAULT_SCREENSHOT_DIR = REPO_ROOT / "screenshots/model-harness"
CRITICAL_DISTINCT_GROUPS = [
    ("Worker", "ScoutRover"),
    ("ScoutRover", "RocketInfantry"),
    ("ScoutRover", "ShieldTrooper"),
    ("RocketInfantry", "ShieldTrooper"),
]
PROCEDURAL_EMPTY_OK = {"LandMine", "TeslaFenceSegment"}

sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit_model_mapping  # noqa: E402


def parse_roles(registry_path: Path) -> dict[str, str]:
    text = registry_path.read_text()
    roles: dict[str, str] = {}
    entity_re = re.compile(
        r"EntityDef\s*\{\s*"
        r'id:\s*"([^"]+)".*?'
        r"role:\s*EntityRole::([A-Za-z]+).*?"
        r"render_parts:\s*(PARTS_[A-Z0-9_]+),",
        re.S,
    )
    for match in entity_re.finditer(text):
        roles[match.group(1)] = match.group(2)
    return roles


def parse_labels(registry_path: Path) -> dict[str, str]:
    text = registry_path.read_text()
    labels: dict[str, str] = {}
    entity_re = re.compile(
        r"EntityDef\s*\{\s*"
        r'id:\s*"([^"]+)".*?'
        r'label:\s*"([^"]+)".*?'
        r"render_parts:\s*(PARTS_[A-Z0-9_]+),",
        re.S,
    )
    for match in entity_re.finditer(text):
        labels[match.group(1)] = match.group(2)
    return labels


def model_signature(parts: list[dict[str, object]]) -> tuple[tuple[str, int], ...]:
    counts = collections.Counter(str(part["model"]) for part in parts)
    return tuple(sorted(counts.items()))


def exact_signature(parts: list[dict[str, object]]) -> tuple[str, ...]:
    return tuple(
        "|".join(
            [
                str(part["model"]),
                ",".join(str(value) for value in part["translation"]),  # type: ignore[index]
                ",".join(str(value) for value in part["rotation"]),  # type: ignore[index]
                ",".join(str(value) for value in part["scale"]),  # type: ignore[index]
            ]
        )
        for part in parts
    )


def signature_label(signature: tuple[tuple[str, int], ...]) -> str:
    if not signature:
        return "<empty>"
    return ", ".join(f"{count}x {model}" for model, count in signature)


def markdown_table(rows: list[list[str]], headers: list[str]) -> list[str]:
    if not rows:
        return ["_None._", ""]
    out = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    out.extend("| " + " | ".join(row) + " |" for row in rows)
    out.append("")
    return out


def rel_path(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT)) if path.is_relative_to(REPO_ROOT) else str(path)


def parse_harness_locations(screenshots_dir: Path) -> dict[str, dict[str, str]]:
    manifest = screenshots_dir / "manifest.md"
    if not manifest.exists():
        return {}

    locations: dict[str, dict[str, str]] = {}
    for line in manifest.read_text().splitlines():
        if not line.startswith("|") or "`" not in line:
            continue
        columns = [column.strip() for column in line.split("|")]
        if len(columns) < 10:
            continue
        entity = columns[4].strip("`")
        if not entity or entity == "Entity":
            continue
        locations[entity] = {
            "page": columns[2],
            "cell": columns[3],
            "screenshot": columns[9],
        }
    return locations


def generation_prompt(entity_id: str, label: str, signature: str) -> str:
    return (
        f"Create a single cohesive low-poly 3D RTS game unit GLB for {label} "
        f"({entity_id}). Style: clean sci-fi base-building RTS, readable from an "
        "isometric camera, compact silhouette, team-color accent panels, no floating "
        "or separated kitbash parts, centered pivot at ground contact, proportions "
        "compatible with Bevy Open RTS / Kenney spacekit units. Current weak source "
        f"parts: {signature}. Replace them with one fused production-ready model."
    )


def model_harness_coverage(
    registry: dict[str, dict[str, object]],
    screenshots_dir: Path,
    per_page: int,
) -> tuple[list[list[str]], list[str], int]:
    entity_ids = sorted(registry.keys())
    expected_pages = (len(entity_ids) + per_page - 1) // per_page
    manifest = screenshots_dir / "manifest.md"
    failures: list[str] = []
    rows: list[list[str]] = []

    if not manifest.exists():
        failures.append("manifest missing")
        rows.append(["Manifest", "FAIL", rel_path(manifest)])
    else:
        text = manifest.read_text()
        missing_entities = [entity_id for entity_id in entity_ids if f"`{entity_id}`" not in text]
        if missing_entities:
            failures.append("manifest missing entities")
            rows.append(
                [
                    "Manifest entities",
                    "FAIL",
                    ", ".join(missing_entities),
                ]
            )
        else:
            rows.append(["Manifest entities", "PASS", f"{len(entity_ids)} entities"])

    missing_pages: list[str] = []
    empty_pages: list[str] = []
    for page in range(expected_pages):
        path = screenshots_dir / f"page_{page:02}.png"
        if not path.exists():
            missing_pages.append(path.name)
        elif path.stat().st_size <= 0:
            empty_pages.append(path.name)

    if missing_pages:
        failures.append("screenshot pages missing")
        rows.append(["Screenshot pages", "FAIL", ", ".join(missing_pages)])
    if empty_pages:
        failures.append("screenshot pages empty")
        rows.append(["Screenshot pages empty", "FAIL", ", ".join(empty_pages)])
    if not missing_pages and not empty_pages:
        rows.append(["Screenshot pages", "PASS", f"{expected_pages} pages"])

    return rows, failures, expected_pages


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--registry", type=Path, default=REGISTRY_RS)
    parser.add_argument("--out", type=Path, default=DEFAULT_REPORT)
    parser.add_argument(
        "--queue-out",
        type=Path,
        default=DEFAULT_QUEUE,
        help="Write a machine-readable Hunyuan3D replacement queue.",
    )
    parser.add_argument("--screenshots-dir", type=Path, default=DEFAULT_SCREENSHOT_DIR)
    parser.add_argument(
        "--per-page",
        type=int,
        default=6,
        help="Model harness screenshot slots per page.",
    )
    parser.add_argument(
        "--require-screenshots",
        action="store_true",
        help="Return non-zero unless model-harness manifest and page screenshots cover every entity.",
    )
    parser.add_argument(
        "--fail-critical",
        action="store_true",
        help=(
            "Return non-zero for critical duplicate units, any duplicate unit "
            "model signatures, or missing model files."
        ),
    )
    parser.add_argument(
        "--max-unit-parts",
        type=int,
        default=3,
        help="Units above this render-part count are listed for screenshot review.",
    )
    args = parser.parse_args()
    if args.per_page <= 0:
        parser.error("--per-page must be positive")

    audit_model_mapping.REGISTRY_RS = args.registry
    registry = audit_model_mapping.parse_registry()
    roles = parse_roles(args.registry)
    labels = parse_labels(args.registry)
    harness_locations = parse_harness_locations(args.screenshots_dir)
    harness_rows, harness_failures, expected_harness_pages = model_harness_coverage(
        registry,
        args.screenshots_dir,
        args.per_page,
    )

    missing_assets: list[tuple[str, str]] = []
    empty_visuals: list[str] = []
    model_groups: dict[tuple[tuple[str, int], ...], list[str]] = collections.defaultdict(list)
    exact_groups: dict[tuple[str, ...], list[str]] = collections.defaultdict(list)
    multipart_units: list[tuple[str, int, str]] = []

    for entity_id, entry in sorted(registry.items()):
        role = roles.get(entity_id, "Unknown")
        parts = entry["parts"]  # type: ignore[assignment]
        if not parts and entity_id not in PROCEDURAL_EMPTY_OK:
            empty_visuals.append(entity_id)
        if role == "Unit":
            model_groups[model_signature(parts)].append(entity_id)
            exact_groups[exact_signature(parts)].append(entity_id)
            if len(parts) > args.max_unit_parts:
                multipart_units.append((entity_id, len(parts), signature_label(model_signature(parts))))
        for part in parts:  # type: ignore[assignment]
            model = str(part["model"])
            if not (REPO_ROOT / "assets" / model).exists():
                missing_assets.append((entity_id, model))

    critical_rows: list[list[str]] = []
    critical_failures: list[str] = []
    for left, right in CRITICAL_DISTINCT_GROUPS:
        left_parts = registry[left]["parts"]  # type: ignore[index]
        right_parts = registry[right]["parts"]  # type: ignore[index]
        left_sig = model_signature(left_parts)
        right_sig = model_signature(right_parts)
        shared_models = sorted(
            {model for model, _ in left_sig} & {model for model, _ in right_sig}
        )
        ok = left_sig != right_sig and not (
            left == "Worker" and right == "ScoutRover" and shared_models
        )
        if not ok:
            critical_failures.append(f"{left} vs {right}")
        critical_rows.append(
            [
                f"{left} / {right}",
                "PASS" if ok else "FAIL",
                signature_label(left_sig),
                signature_label(right_sig),
                ", ".join(shared_models) if shared_models else "-",
            ]
        )

    exact_duplicate_rows = [
        [", ".join(ids), signature_label(model_signature(registry[ids[0]]["parts"]))]  # type: ignore[index]
        for _, ids in sorted(exact_groups.items(), key=lambda item: item[1])
        if len(ids) > 1 and _
    ]
    model_duplicate_rows = [
        [", ".join(ids), signature_label(sig)]
        for sig, ids in sorted(model_groups.items(), key=lambda item: item[1])
        if len(ids) > 1 and sig
    ]

    candidate_reasons: dict[str, set[str]] = collections.defaultdict(set)
    for left, right in CRITICAL_DISTINCT_GROUPS:
        if f"{left} vs {right}" in critical_failures:
            candidate_reasons[left].add(f"critical visual overlap with {right}")
            candidate_reasons[right].add(f"critical visual overlap with {left}")
    for entity_id, count, _signature in multipart_units:
        candidate_reasons[entity_id].add(f"multipart kitbash above {args.max_unit_parts} parts ({count})")
    for ids, _signature in model_duplicate_rows:
        group = ids.split(", ")
        for entity_id in group:
            candidate_reasons[entity_id].add("duplicate unit model signature: " + ", ".join(group))
    for entity_id in empty_visuals:
        candidate_reasons[entity_id].add("empty non-procedural visual")

    queue_records = []
    for entity_id in sorted(candidate_reasons):
        entry = registry[entity_id]
        parts = entry["parts"]  # type: ignore[assignment]
        signature = signature_label(model_signature(parts))
        label = labels.get(entity_id, entity_id)
        queue_records.append(
            {
                "entity": entity_id,
                "label": label,
                "role": roles.get(entity_id, "Unknown"),
                "reasons": sorted(candidate_reasons[entity_id]),
                "part_count": len(parts),
                "model_signature": signature,
                "render_parts": parts,
                "harness": harness_locations.get(entity_id),
                "prompt": generation_prompt(entity_id, label, signature),
                "negative_prompt": (
                    "floating pieces, separated weapons, disconnected missiles, unreadable tiny "
                    "details, realistic high-poly grime, huge base plate, wrong pivot, oversized scale"
                ),
                "target_path": f"assets/models/hunyuan3d/{entity_id}.glb",
            }
        )

    lines = [
        "# Model Quality Audit",
        "",
        f"- Registry: `{args.registry.relative_to(REPO_ROOT)}`",
        f"- Entities checked: {len(registry)}",
        f"- Missing model files: {len(missing_assets)}",
        f"- Empty non-procedural visuals: {len(empty_visuals)}",
        f"- Unit model-signature duplicate groups: {len(model_duplicate_rows)}",
        f"- Unit exact-render duplicate groups: {len(exact_duplicate_rows)}",
        f"- Multipart units above {args.max_unit_parts} parts: {len(multipart_units)}",
        f"- Model harness screenshots dir: `{rel_path(args.screenshots_dir)}`",
        f"- Model harness expected pages: {expected_harness_pages} at {args.per_page} per page",
        f"- Model harness coverage failures: {len(harness_failures)}",
        "",
        "## Critical Distinct Units",
        "",
    ]
    lines.extend(
        markdown_table(
            critical_rows,
            ["Pair", "Status", "Left signature", "Right signature", "Shared models"],
        )
    )
    lines.extend(["## Missing Model Files", ""])
    lines.extend(markdown_table([[entity, model] for entity, model in missing_assets], ["Entity", "Model"]))
    lines.extend(["## Empty Non-Procedural Visuals", ""])
    lines.extend(markdown_table([[entity] for entity in empty_visuals], ["Entity"]))
    lines.extend(["## Exact Duplicate Unit Render Signatures", ""])
    lines.extend(markdown_table(exact_duplicate_rows, ["Units", "Signature"]))
    lines.extend(["## Duplicate Unit Model Signatures", ""])
    lines.extend(markdown_table(model_duplicate_rows, ["Units", "Signature"]))
    lines.extend(["## Multipart Units Needing Screenshot Review", ""])
    lines.extend(
        markdown_table(
            [[entity, str(count), signature] for entity, count, signature in multipart_units],
            ["Unit", "Parts", "Signature"],
        )
    )
    lines.extend(["## Model Harness Screenshot Coverage", ""])
    lines.extend(markdown_table(harness_rows, ["Check", "Status", "Detail"]))
    lines.extend(["## Hunyuan3D Replacement Candidates", ""])
    lines.extend(
        markdown_table(
            [
                [
                    record["entity"],
                    "; ".join(record["reasons"]),
                    (
                        f"{record['harness']['screenshot']} {record['harness']['cell']}"
                        if record["harness"]
                        else "missing harness location"
                    ),
                ]
                for record in queue_records
            ],
            ["Entity", "Reasons", "Harness"],
        )
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(lines) + "\n")
    if args.queue_out:
        args.queue_out.parent.mkdir(parents=True, exist_ok=True)
        args.queue_out.write_text(json.dumps(queue_records, indent=2, ensure_ascii=False) + "\n")
    display = args.out.relative_to(REPO_ROOT) if args.out.is_relative_to(REPO_ROOT) else args.out
    print(f"[model-quality] wrote {display}")
    if args.queue_out:
        queue_display = (
            args.queue_out.relative_to(REPO_ROOT)
            if args.queue_out.is_relative_to(REPO_ROOT)
            else args.queue_out
        )
        print(f"[model-quality] wrote {queue_display}")
    print(
        "[model-quality] "
        f"critical_failures={len(critical_failures)} "
        f"missing_assets={len(missing_assets)} "
        f"model_duplicate_groups={len(model_duplicate_rows)} "
        f"multipart_units={len(multipart_units)} "
        f"harness_failures={len(harness_failures)}"
    )

    if args.fail_critical and (critical_failures or missing_assets or model_duplicate_rows):
        return 1
    if args.require_screenshots and harness_failures:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
