#!/usr/bin/env python3
"""Audit Bevy render-part mappings against the Godot unit scenes.

This is intentionally non-destructive. `scripts/generate_registry.py` can rebuild
the registry, but the current port has hand-expanded gameplay data, so this
harness only compares the existing Bevy registry to the Godot scene files and can
write a small Bevy-loadable baseline asset.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = Path(__file__).resolve().parent
GODOT_ROOT = REPO_ROOT.parent / "godot-open-rts"
REGISTRY_RS = REPO_ROOT / "src/generated_registry.rs"
DEFAULT_OUT = REPO_ROOT / "assets/data/godot_model_map.model_map.ron"
EPSILON = 0.0005

sys.path.insert(0, str(SCRIPTS_DIR))
import generate_registry as godot_registry  # noqa: E402


def parse_floats(raw: str) -> list[float]:
    return [float(value) for value in re.findall(r"[-+]?\d*\.?\d+(?:e[-+]?\d+)?", raw)]


def normalize_number(value: float) -> float:
    rounded = round(value, 4)
    return 0.0 if rounded == -0.0 else rounded


def normalize_part(part: dict[str, object]) -> dict[str, object]:
    return {
        "model": str(part["model"]),
        "translation": [normalize_number(float(v)) for v in part["translation"]],  # type: ignore[index]
        "rotation": [normalize_number(float(v)) for v in part["rotation"]],  # type: ignore[index]
        "scale": [normalize_number(float(v)) for v in part["scale"]],  # type: ignore[index]
    }


def normalize_parts(parts: list[dict[str, object]]) -> list[dict[str, object]]:
    return [normalize_part(part) for part in parts]


def parse_registry() -> dict[str, dict[str, object]]:
    text = REGISTRY_RS.read_text()
    const_bodies = {
        name: body
        for name, body in re.findall(
            r"const\s+(PARTS_[A-Z0-9_]+):\s*&\[RenderPart\]\s*=\s*&\[(.*?)\];",
            text,
            re.S,
        )
    }

    parsed_parts: dict[str, list[dict[str, object]]] = {}
    part_re = re.compile(
        r"RenderPart\s*\{\s*"
        r'model:\s*"([^"]+)",\s*'
        r"translation:\s*\[([^\]]*)\],\s*"
        r"rotation:\s*\[([^\]]*)\],\s*"
        r"scale:\s*\[([^\]]*)\],\s*"
        r"\}",
        re.S,
    )
    for const_name, body in const_bodies.items():
        parts = []
        for match in part_re.finditer(body):
            parts.append(
                normalize_part(
                    {
                        "model": match.group(1),
                        "translation": parse_floats(match.group(2)),
                        "rotation": parse_floats(match.group(3)),
                        "scale": parse_floats(match.group(4)),
                    }
                )
            )
        parsed_parts[const_name] = parts

    entities: dict[str, dict[str, object]] = {}
    entity_re = re.compile(
        r"EntityDef\s*\{\s*"
        r'id:\s*"([^"]+)".*?'
        r'scene_path:\s*"([^"]+)".*?'
        r"render_parts:\s*(PARTS_[A-Z0-9_]+),",
        re.S,
    )
    for match in entity_re.finditer(text):
        entity_id, scene_path, parts_const = match.groups()
        entities[entity_id] = {
            "scene_path": scene_path,
            "parts": parsed_parts.get(parts_const, []),
        }
    return entities


def godot_scene_file(scene_path: str) -> Path:
    return GODOT_ROOT / scene_path.removeprefix("res://")


def godot_parts(scene_path: str) -> list[dict[str, object]]:
    scene_file = godot_scene_file(scene_path)
    if not scene_file.exists():
        return []
    return normalize_parts(godot_registry.extract_render_parts(scene_file))


def nearly_equal(left: float, right: float) -> bool:
    return abs(left - right) <= EPSILON


def parts_equal(left: list[dict[str, object]], right: list[dict[str, object]]) -> bool:
    if len(left) != len(right):
        return False
    for left_part, right_part in zip(left, right):
        if left_part["model"] != right_part["model"]:
            return False
        for field in ("translation", "rotation", "scale"):
            left_values = left_part[field]
            right_values = right_part[field]
            if len(left_values) != len(right_values):  # type: ignore[arg-type]
                return False
            for a, b in zip(left_values, right_values):  # type: ignore[arg-type]
                if not nearly_equal(float(a), float(b)):
                    return False
    return True


def ron_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def ron_float(value: float) -> str:
    text = f"{normalize_number(value):.4f}".rstrip("0").rstrip(".")
    if text in {"", "-0"}:
        text = "0"
    if "." not in text:
        text += ".0"
    return text


def ron_array(values: list[float]) -> str:
    return "[" + ", ".join(ron_float(value) for value in values) + "]"


def write_model_map(path: Path, expected: dict[str, dict[str, object]]) -> None:
    lines = [
        "(",
        '    source: "res://source/match/units",',
        '    generated_by: "scripts/audit_model_mapping.py",',
        "    entities: [",
    ]
    for entity_id in sorted(expected):
        entry = expected[entity_id]
        lines.extend(
            [
                "        (",
                f"            id: {ron_string(entity_id)},",
                f"            scene_path: {ron_string(str(entry['scene_path']))},",
                "            parts: [",
            ]
        )
        for part in entry["parts"]:  # type: ignore[index]
            lines.extend(
                [
                    "                (",
                    f"                    model: {ron_string(str(part['model']))},",
                    f"                    translation: {ron_array(part['translation'])},",
                    f"                    rotation: {ron_array(part['rotation'])},",
                    f"                    scale: {ron_array(part['scale'])},",
                    "                ),",
                ]
            )
        lines.extend(["            ],", "        ),"])
    lines.extend(["    ],", ")", ""])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines))


def describe_parts(parts: list[dict[str, object]]) -> str:
    if not parts:
        return "<procedural/empty>"
    return "; ".join(
        f"{part['model']} t={part['translation']} s={part['scale']}" for part in parts
    )


def main() -> int:
    global GODOT_ROOT, REGISTRY_RS

    parser = argparse.ArgumentParser()
    parser.add_argument("--godot-root", type=Path, default=GODOT_ROOT)
    parser.add_argument("--registry", type=Path, default=REGISTRY_RS)
    parser.add_argument("--write", type=Path, nargs="?", const=DEFAULT_OUT)
    args = parser.parse_args()

    GODOT_ROOT = args.godot_root
    REGISTRY_RS = args.registry
    godot_registry.GODOT_ROOT = GODOT_ROOT

    registry = parse_registry()
    expected: dict[str, dict[str, object]] = {}
    mismatches: list[str] = []
    missing_scenes: list[str] = []

    for entity_id, actual_entry in sorted(registry.items()):
        scene_path = str(actual_entry["scene_path"])
        scene_file = godot_scene_file(scene_path)
        if not scene_file.exists():
            missing_scenes.append(entity_id)
            continue
        expected_parts = godot_parts(scene_path)
        actual_parts = actual_entry["parts"]  # type: ignore[assignment]
        expected[entity_id] = {
            "scene_path": scene_path,
            "parts": expected_parts,
        }
        if not parts_equal(actual_parts, expected_parts):
            mismatches.append(
                "\n".join(
                    [
                        f"{entity_id}",
                        f"  Godot: {describe_parts(expected_parts)}",
                        f"  Bevy : {describe_parts(actual_parts)}",
                    ]
                )
            )

    if args.write:
        write_model_map(args.write, expected)
        try:
            display_path = args.write.relative_to(REPO_ROOT)
        except ValueError:
            display_path = args.write
        print(f"[model-audit] wrote {display_path}")

    print(
        f"[model-audit] checked={len(expected)} missing_scenes={len(missing_scenes)} mismatches={len(mismatches)}"
    )
    for entity_id in ("Worker", "ScoutRover"):
        entry = expected.get(entity_id)
        if entry:
            print(f"[model-audit] {entity_id}: {describe_parts(entry['parts'])}")
    if missing_scenes:
        print("[model-audit] missing Godot scenes:")
        for entity_id in missing_scenes:
            print(f"  - {entity_id}: {registry[entity_id]['scene_path']}")
    if mismatches:
        print("[model-audit] mismatches:")
        for mismatch in mismatches:
            print(mismatch)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
