#!/usr/bin/env python3
"""Generate Bevy gameplay registry data from the Godot RTS project."""

from __future__ import annotations

import json
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
GODOT_ROOT = REPO_ROOT.parent / "godot-open-rts"
MATCH_CONSTANTS = GODOT_ROOT / "source/match/MatchConstants.gd"
UNITS_DIR = GODOT_ROOT / "source/match/units"
OUT_RS = REPO_ROOT / "src/generated_registry.rs"
OUT_JSON = REPO_ROOT / "assets/migration/gameplay_registry.json"
OUT_ASSET_MANIFEST = REPO_ROOT / "assets/migration/asset_manifest.json"
OUT_REPORT = REPO_ROOT / "assets/migration/migration_report.md"


FACTION_LABELS = {
    "alliance": "苍穹联盟",
    "demon": "炽炎魔军",
    "chaos": "混沌裂隙",
}

FACTION_COLORS = {
    "alliance": [0.18, 0.43, 0.95],
    "demon": [0.85, 0.2, 0.12],
    "chaos": [0.54, 0.25, 0.88],
}

FACTION_EMBLEMS = {
    "alliance": "ui/factions/alliance_emblem.png",
    "demon": "ui/factions/demon_emblem.png",
    "chaos": "ui/factions/chaos_emblem.png",
}

PROCEDURAL_RENDER_IDS = {
    "LandMine": "Godot CylinderMesh/TorusMesh scene; Bevy runtime recreates it with procedural primitive meshes.",
    "TeslaFenceSegment": "Godot BoxMesh/CylinderMesh scene; Bevy runtime recreates it with procedural primitive meshes.",
}

FACTION_STRUCTURE_LIST = {
    "alliance": "ALL_STRUCTURES",
    "demon": "DEMON_STRUCTURES",
    "chaos": "CHAOS_STRUCTURES",
}

FACTION_PRODUCTION_LISTS = {
    "alliance": {
        "CommandCenter": "COMMAND_CENTER_UNITS",
        "Barracks": "ALL_INFANTRY",
        "VehicleFactory": "ALL_VEHICLES",
        "AircraftFactory": "ALL_AIRCRAFT",
    },
    "demon": {
        "CommandCenter": "COMMAND_CENTER_UNITS",
        "Barracks": "DEMON_INFANTRY",
        "VehicleFactory": "DEMON_VEHICLES",
        "AircraftFactory": "DEMON_AIRCRAFT",
    },
    "chaos": {
        "CommandCenter": "COMMAND_CENTER_UNITS",
        "Barracks": "CHAOS_INFANTRY",
        "VehicleFactory": "CHAOS_VEHICLES",
        "AircraftFactory": "CHAOS_AIRCRAFT",
    },
}

STRUCTURE_ORDER_OVERRIDE = [
    "AntiGroundTurret",
    "AntiAirTurret",
    "TeslaFenceSegment",
    "ArcCoilDefenseTower",
    "LanceBeamDefenseTower",
    "PrismDefenseObelisk",
    "RailCannonBunker",
    "RadarUplink",
    "RoboticsBay",
    "TechLab",
    "WeatherControlSpire",
    "CommandCenter",
    "VehicleFactory",
    "AircraftFactory",
    "PowerReactor",
    "AdvancedReactorPlant",
    "Refinery",
    "OrePurifier",
    "Barracks",
    "RepairPad",
]

PRODUCTION_ORDER_OVERRIDES = {
    "CommandCenter": [
        "Worker",
        "EngineerDrone",
    ],
    "Barracks": [
        "LightRifleInfantry",
        "RocketInfantry",
        "FieldMedic",
        "ShieldTrooper",
        "FlakRocketTeam",
        "FlakRocketTeamMk2",
        "HeavyMachinegunTrooper",
        "ShockTrooper",
        "GrenadierTrooper",
        "MortarTeam",
        "CryoSprayer",
        "SniperScout",
        "RailSniperTeam",
        "PhaseSaboteur",
        "SaboteurInfiltrator",
        "PulseRifleCommando",
        "TacticalOfficer",
    ],
    "VehicleFactory": [
        "Tank",
        "ScoutRover",
        "MobileConstructionVehicle",
        "MirageScoutTank",
        "FlameAssaultBuggy",
        "DroneMineLayer",
        "TeslaCrawlerMk2",
        "RocketTrooperRobot",
        "ModularMissileCarrier",
        "LongbowMissileCrawler",
        "JammerVehicle",
        "AntiAirWalker",
        "FlakHoverTank",
        "MobileRepairCrawler",
        "MobileShieldProjector",
        "SiegeArtilleryVehicle",
        "SiegeDrillTank",
        "LanceBeamTank",
        "RailgunTank",
        "HammerSiegeTank",
        "HeavySiegeWalker",
        "RailArtilleryWalker",
    ],
    "AircraftFactory": [
        "Helicopter",
        "InterceptorVTOL",
        "Drone",
        "BomberVTOL",
        "RocketGunship",
        "HeavyBombardmentAirship",
        "SiegeAirship",
    ],
}

REMOVED_ENTITY_IDS = {"OreHarvester", "MobileConstructionVehicle"}

# Per-entity render overrides. NOTE: the registry has since been hand-expanded
# well beyond godot (extra factions/units), so a full regen is destructive —
# fixes are applied surgically in src/generated_registry.rs, not by re-running
# this script. Worker intentionally diverges from godot's rover mesh so it does
# not visually duplicate ScoutRover in the Bevy port.
ENTITY_RENDER_OVERRIDES: dict[str, dict] = {
    "CryoSprayer": {
        "model_assets": ["models/hunyuan3d/CryoSprayer.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/CryoSprayer.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "LongbowMissileCrawler": {
        "model_assets": ["models/hunyuan3d/LongbowMissileCrawler.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/LongbowMissileCrawler.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "FlameAssaultBuggy": {
        "model_assets": ["models/hunyuan3d/FlameAssaultBuggy.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/FlameAssaultBuggy.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "HammerSiegeTank": {
        "model_assets": ["models/hunyuan3d/HammerSiegeTank.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/HammerSiegeTank.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "HeavySiegeWalker": {
        "model_assets": ["models/hunyuan3d/HeavySiegeWalker.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/HeavySiegeWalker.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "RailArtilleryWalker": {
        "model_assets": ["models/hunyuan3d/RailArtilleryWalker.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/RailArtilleryWalker.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "FlakHoverTank": {
        "model_assets": ["models/hunyuan3d/FlakHoverTank.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/FlakHoverTank.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "LanceBeamTank": {
        "model_assets": ["models/hunyuan3d/LanceBeamTank.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/LanceBeamTank.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
    "RailgunTank": {
        "model_assets": ["models/hunyuan3d/RailgunTank.glb"],
        "render_parts": [
            {
                "model": "models/hunyuan3d/RailgunTank.glb",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
            }
        ],
    },
}


def read_balanced(text: str, start: int) -> tuple[str, int]:
    pairs = {"{": "}", "[": "]", "(": ")"}
    stack: list[str] = []
    in_string = False
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in pairs:
            stack.append(pairs[char])
        elif stack and char == stack[-1]:
            stack.pop()
            if not stack:
                return text[start : index + 1], index + 1
    raise ValueError(f"unterminated balanced value at offset {start}")


def extract_const(text: str, name: str) -> str:
    match = re.search(rf"\bconst\s+{re.escape(name)}\s*=\s*", text)
    if not match:
        raise KeyError(name)
    index = match.end()
    while index < len(text) and text[index].isspace():
        index += 1
    if text[index] not in "{[(":
        raise ValueError(f"{name} is not a balanced literal")
    block, _ = read_balanced(text, index)
    return block


def parse_string(text: str, start: int) -> tuple[str, int]:
    assert text[start] == '"'
    value = []
    escaped = False
    for index in range(start + 1, len(text)):
        char = text[index]
        if escaped:
            value.append(char)
            escaped = False
        elif char == "\\":
            escaped = True
        elif char == '"':
            return "".join(value), index + 1
        else:
            value.append(char)
    raise ValueError("unterminated string")


def skip_ws_and_commas(text: str, index: int) -> int:
    while index < len(text) and (text[index].isspace() or text[index] == ","):
        index += 1
    return index


def top_level_dict_entries(block: str) -> dict[str, str]:
    entries: dict[str, str] = {}
    index = 1
    while index < len(block) - 1:
        index = skip_ws_and_commas(block, index)
        if index >= len(block) - 1:
            break
        if block[index] != '"':
            index += 1
            continue
        key, index = parse_string(block, index)
        index = skip_ws_and_commas(block, index)
        if index >= len(block) or block[index] != ":":
            continue
        index += 1
        index = skip_ws_and_commas(block, index)
        value_start = index
        if block[index] in "{[(":
            value, index = read_balanced(block, index)
            entries[key] = value.strip()
        elif block[index] == '"':
            _, index = parse_string(block, index)
            entries[key] = block[value_start:index].strip()
        else:
            while index < len(block) and block[index] not in ",\n":
                index += 1
            entries[key] = block[value_start:index].strip()
    return entries


def scene_path_to_id(path: str) -> str:
    return Path(path).stem


def apply_production_order_override(producer_id: str, products: list[str]) -> list[str]:
    order = PRODUCTION_ORDER_OVERRIDES.get(producer_id)
    if order is None:
        return products
    order_index = {product_id: index for index, product_id in enumerate(order)}
    return sorted(products, key=lambda product_id: order_index.get(product_id, len(order_index)))


def apply_structure_order_override(structures: list[str]) -> list[str]:
    order_index = {
        structure_id: index for index, structure_id in enumerate(STRUCTURE_ORDER_OVERRIDE)
    }
    return sorted(
        structures, key=lambda structure_id: order_index.get(structure_id, len(order_index))
    )


def bevy_asset_path(godot_path: str) -> str:
    if not godot_path.startswith("res://assets/"):
        raise ValueError(godot_path)
    return godot_path.removeprefix("res://assets/")


def label_from_id(entity_id: str) -> str:
    pieces = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", " ", entity_id).split()
    return " ".join(pieces)


def parse_paths(block: str) -> list[str]:
    return re.findall(r'"(res://source/match/units/[^"]+\.tscn)"', block)


def parse_costs(block: str) -> dict[str, dict[str, int]]:
    result = {}
    for path, body in top_level_dict_entries(block).items():
        resource_a = re.search(r'"resource_a"\s*:\s*([0-9]+)', body)
        resource_b = re.search(r'"resource_b"\s*:\s*([0-9]+)', body)
        result[path] = {
            "ore": int(resource_a.group(1)) if resource_a else 0,
            "crystal": int(resource_b.group(1)) if resource_b else 0,
        }
    return result


def parse_number_map(block: str) -> dict[str, float]:
    result = {}
    for path, value in re.findall(
        r'"(res://source/match/units/[^"]+\.tscn)"\s*:\s*([-0-9.]+)', block
    ):
        result[path] = float(value)
    return result


def parse_string_map(block: str) -> dict[str, str]:
    result = {}
    for path, value in re.findall(
        r'"(res://source/match/units/[^"]+\.tscn)"\s*:\s*"([^"]+)"', block
    ):
        result[path] = value
    return result


def parse_constant_floats(text: str) -> dict[str, float]:
    constants: dict[str, float] = {}
    class_stack: list[tuple[int, str]] = []
    for raw_line in text.splitlines():
        class_match = re.match(r"^(\s*)class\s+([A-Za-z_]\w*)\s*:", raw_line)
        if class_match:
            indent = len(class_match.group(1).expandtabs(4))
            while class_stack and indent <= class_stack[-1][0]:
                class_stack.pop()
            class_stack.append((indent, class_match.group(2)))
            continue

        const_match = re.match(r"^\s*const\s+([A-Za-z_]\w*)\s*=\s*([^\n#]+)", raw_line)
        if not const_match or not class_stack:
            continue
        value_token = const_match.group(2).strip().rstrip(",")
        try:
            value = float(value_token)
        except ValueError:
            continue
        class_name = ".".join(name for _, name in class_stack)
        constants[f"{class_name}.{const_match.group(1)}"] = value
    return constants


def parse_property_number(value: str, constants: dict[str, float]) -> float | None:
    token = value.strip()
    if not token:
        return None
    if token in constants:
        return constants[token]
    if "*" not in token and "/" not in token:
        try:
            return float(token)
        except ValueError:
            return None

    terms = token.split("/")
    if len(terms) > 1:
        left = parse_product(terms[0].strip(), constants)
        if left is None:
            return None
        result = left
        for divisor_token in terms[1:]:
            right = parse_product(divisor_token.strip(), constants)
            if right in (None, 0.0):
                return None
            result /= right
        return result
    return parse_product(token, constants)


def parse_product(value: str, constants: dict[str, float]) -> float | None:
    result = 1.0
    has_factor = False
    for factor in value.split("*"):
        factor = factor.strip()
        if not factor:
            continue
        if factor in constants:
            result *= constants[factor]
        else:
            try:
                result *= float(factor)
            except ValueError:
                return None
        has_factor = True
    if not has_factor:
        return None
    return result


def parse_script_constant_floats(text: str) -> dict[str, float]:
    constants: dict[str, float] = {}
    for raw_line in text.splitlines():
        if raw_line.startswith((" ", "\t")):
            continue
        match = re.match(r"^const\s+([A-Za-z_]\w*)\s*=\s*([^\n#]+)", raw_line)
        if not match:
            continue
        value = parse_property_number(match.group(2).strip().rstrip(","), constants)
        if value is not None:
            constants[match.group(1)] = value
    return constants


def parse_properties(block: str, constants: dict[str, float]) -> dict[str, dict[str, object]]:
    result = {}
    for path, body in top_level_dict_entries(block).items():
        props: dict[str, object] = {}
        for key in (
            "sight_range",
            "hp",
            "hp_max",
            "attack_damage",
            "attack_interval",
            "attack_range",
            "splash_radius",
            "splash_damage_multiplier",
            "structure_damage_multiplier",
            "resources_max",
            "mine_damage",
            "trigger_radius",
            "blast_radius",
            "arming_delay",
            "mine_deploy_interval",
            "mine_deploy_radius",
            "mine_spacing",
            "mine_limit",
            "repair_radius",
            "repair_rate",
            "healing_radius",
            "healing_rate",
            "capture_time",
            "infiltration_resource_steal_ratio",
            "infiltration_resource_steal_cap",
            "infiltration_production_veterancy_rank",
            "infiltration_power_sabotage_duration",
            "resource_income_a",
            "resource_income_b",
            "income_interval_s",
            "capture_bonus_a",
            "capture_bonus_b",
            "garrison_capacity",
            "garrison_attack_damage_per_unit",
            "support_shield_radius",
            "support_shield_duration",
            "support_shield_damage_multiplier",
        ):
            match = re.search(rf'"{key}"\s*:\s*([^,\\n}}]+)', body)
            if match:
                value = parse_property_number(match.group(1), constants)
                if value is None:
                    continue
                props[key] = int(value) if value.is_integer() else value
        if "attack_domains" in body:
            props["can_attack_air"] = "Navigation.Domain.AIR" in body
            props["can_attack_ground"] = "Navigation.Domain.TERRAIN" in body
        else:
            # Fallback for earlier data snapshots: derive from movement domain labels.
            if "Navigation.Domain.AIR" in body:
                props["can_attack_air"] = True
            if "Navigation.Domain.TERRAIN" in body:
                props["can_attack_ground"] = True
        result[path] = props
    return result


def parse_scene(scene_file: Path) -> dict[str, object]:
    text = scene_file.read_text()
    ext_resources = {
        resource_id: path
        for path, resource_id in re.findall(
            r'\[ext_resource[^\]]*path="([^"]+)"[^\]]*id="([^"]+)"\]', text
        )
    }
    asset_paths = [
        bevy_asset_path(path)
        for path in ext_resources.values()
        if path.startswith("res://assets/")
        and not path.endswith(".import")
        and Path(path).suffix.lower() in {".glb", ".png", ".ogg", ".wav"}
    ]
    render_parts = extract_render_parts(scene_file)
    model_assets = sorted({part["model"] for part in render_parts})
    media_assets = sorted({asset for asset in asset_paths if not asset.endswith(".glb")})
    movement_block = node_block(text, "Movement")
    targetability_block = node_block(text, "Targetability")
    selection_block = node_block(text, "Selection")

    movement_radius = parse_float(movement_block, "radius")
    selection_radius = parse_float(selection_block, "radius")
    target_radius = parse_float(targetability_block, "radius")
    collision_radius = parse_float(text, "radius")
    speed = parse_float(movement_block, "speed")
    domain = "air" if re.search(r"domain\s*=\s*0\b", movement_block) else "terrain"
    return {
        "model_assets": model_assets,
        "render_parts": render_parts,
        "media_assets": media_assets,
        "radius": target_radius or selection_radius or movement_radius or collision_radius or 0.6,
        "speed": speed or 4.0,
        "domain": domain,
    }


def extract_render_parts(
    scene_file: Path,
    parent_matrix: list[list[float]] | None = None,
    seen: set[Path] | None = None,
) -> list[dict[str, object]]:
    if seen is None:
        seen = set()
    if parent_matrix is None:
        parent_matrix = identity_matrix()
    scene_file = scene_file.resolve()
    if scene_file in seen:
        return []
    seen.add(scene_file)

    text = scene_file.read_text()
    ext_by_id = {
        resource_id: path
        for path, resource_id in re.findall(
            r'\[ext_resource[^\]]*path="([^"]+)"[^\]]*id="([^"]+)"\]', text
        )
    }
    matrices: dict[str, list[list[float]]] = {".": parent_matrix}
    parts: list[dict[str, object]] = []
    for header, body in iter_node_blocks(text):
        name = attr_value(header, "name")
        if not name:
            continue
        parent = attr_value(header, "parent") or "."
        full_path = name if parent == "." else f"{parent}/{name}"
        local = parse_transform_matrix(body)
        world = matmul(matrices.get(parent, parent_matrix), local)
        matrices[full_path] = world

        instance_id = instance_resource_id(header)
        if not instance_id:
            continue
        resource_path = ext_by_id.get(instance_id)
        if not resource_path:
            continue
        if resource_path.startswith("res://assets/") and resource_path.endswith(".glb"):
            translation, scale, rotation = decompose_transform(world)
            parts.append(
                {
                    "model": bevy_asset_path(resource_path),
                    "translation": translation,
                    "scale": scale,
                    "rotation": rotation,
                }
            )
        elif resource_path.startswith("res://source/match/units/structure-geometries/"):
            nested_file = GODOT_ROOT / resource_path.removeprefix("res://")
            if nested_file.exists():
                parts.extend(extract_render_parts(nested_file, world, seen.copy()))
    return parts


def iter_node_blocks(text: str):
    matches = list(re.finditer(r"\[node\s+([^\]]+)\]", text))
    for index, match in enumerate(matches):
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        yield match.group(1), text[start:end]


def attr_value(header: str, name: str) -> str | None:
    match = re.search(rf'\b{name}="([^"]+)"', header)
    return match.group(1) if match else None


def instance_resource_id(header: str) -> str | None:
    match = re.search(r'instance=ExtResource\("([^"]+)"\)', header)
    return match.group(1) if match else None


def identity_matrix() -> list[list[float]]:
    return [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]


def parse_transform_matrix(body: str) -> list[list[float]]:
    match = re.search(r"transform\s*=\s*Transform3D\(([^)]+)\)", body)
    if not match:
        return identity_matrix()
    numbers = [float(value) for value in re.findall(r"[-+]?\d*\.?\d+(?:e[-+]?\d+)?", match.group(1))]
    if len(numbers) != 12:
        return identity_matrix()
    return [
        [numbers[0], numbers[1], numbers[2], numbers[9]],
        [numbers[3], numbers[4], numbers[5], numbers[10]],
        [numbers[6], numbers[7], numbers[8], numbers[11]],
        [0.0, 0.0, 0.0, 1.0],
    ]


def matmul(a: list[list[float]], b: list[list[float]]) -> list[list[float]]:
    return [
        [sum(a[row][k] * b[k][col] for k in range(4)) for col in range(4)]
        for row in range(4)
    ]


def decompose_transform(matrix: list[list[float]]) -> tuple[list[float], list[float], list[float]]:
    translation = [matrix[0][3], matrix[1][3], matrix[2][3]]
    scale = [
        (matrix[0][0] ** 2 + matrix[1][0] ** 2 + matrix[2][0] ** 2) ** 0.5,
        (matrix[0][1] ** 2 + matrix[1][1] ** 2 + matrix[2][1] ** 2) ** 0.5,
        (matrix[0][2] ** 2 + matrix[1][2] ** 2 + matrix[2][2] ** 2) ** 0.5,
    ]
    norm00 = scale[0] if scale[0] else 1.0
    norm01 = scale[1] if scale[1] else 1.0
    norm02 = scale[2] if scale[2] else 1.0
    rot00 = matrix[0][0] / norm00
    rot01 = matrix[0][1] / norm01
    rot02 = matrix[0][2] / norm02
    rot10 = matrix[1][0] / norm00
    rot11 = matrix[1][1] / norm01
    rot12 = matrix[1][2] / norm02
    rot20 = matrix[2][0] / norm00
    rot21 = matrix[2][1] / norm01
    rot22 = matrix[2][2] / norm02

    trace = rot00 + rot11 + rot22
    if trace > 0:
        s = (trace + 1.0) ** 0.5 * 2.0
        qw = 0.25 * s
        qx = (rot21 - rot12) / s
        qy = (rot02 - rot20) / s
        qz = (rot10 - rot01) / s
    elif rot00 > rot11 and rot00 > rot22:
        s = (1.0 + rot00 - rot11 - rot22) ** 0.5 * 2.0
        qw = (rot21 - rot12) / s
        qx = 0.25 * s
        qy = (rot01 + rot10) / s
        qz = (rot02 + rot20) / s
    elif rot11 > rot22:
        s = (1.0 + rot11 - rot00 - rot22) ** 0.5 * 2.0
        qw = (rot02 - rot20) / s
        qx = (rot01 + rot10) / s
        qy = 0.25 * s
        qz = (rot12 + rot21) / s
    else:
        s = (1.0 + rot22 - rot00 - rot11) ** 0.5 * 2.0
        qw = (rot10 - rot01) / s
        qx = (rot02 + rot20) / s
        qy = (rot12 + rot21) / s
        qz = 0.25 * s
    length = (qx * qx + qy * qy + qz * qz + qw * qw) ** 0.5
    if length == 0.0:
        length = 1.0
    rotation = [qx / length, qy / length, qz / length, qw / length]
    return translation, scale, rotation


def node_block(text: str, node_name: str) -> str:
    match = re.search(rf'\[node name="{re.escape(node_name)}"[^\]]*\]', text)
    if not match:
        return ""
    next_node = re.search(r"\n\[node ", text[match.end() :])
    if next_node:
        return text[match.end() : match.end() + next_node.start()]
    return text[match.end() :]


def parse_float(text: str, name: str) -> float | None:
    match = re.search(rf"^{re.escape(name)}\s*=\s*([-0-9.]+)", text, re.MULTILINE)
    return float(match.group(1)) if match else None


def rust_str(value: str) -> str:
    escaped = []
    for char in value:
        if char == "\\":
            escaped.append("\\\\")
        elif char == '"':
            escaped.append('\\"')
        elif char == "\n":
            escaped.append("\\n")
        elif char == "\r":
            escaped.append("\\r")
        elif char == "\t":
            escaped.append("\\t")
        elif ord(char) < 0x20:
            escaped.append(f"\\u{{{ord(char):x}}}")
        else:
            escaped.append(char)
    return f'"{"".join(escaped)}"'


def rust_opt_str(value: str | None) -> str:
    return f"Some({rust_str(value)})" if value else "None"


def rust_bool(value: bool) -> str:
    return "true" if value else "false"


def rust_f32(value: float) -> str:
    formatted = f"{float(value):.4f}".rstrip("0").rstrip(".")
    if formatted in {"-0", "0"}:
        formatted = "0"
    if "." not in formatted:
        formatted += ".0"
    return formatted


def rust_ident(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "_", value).upper()


def build_registry() -> dict[str, object]:
    constants_text = MATCH_CONSTANTS.read_text()
    constant_values = parse_constant_floats(constants_text)
    list_const_names = [
        "COMMAND_CENTER_UNITS",
        "ALL_INFANTRY",
        "DEMON_INFANTRY",
        "CHAOS_INFANTRY",
        "ALL_VEHICLES",
        "DEMON_VEHICLES",
        "CHAOS_VEHICLES",
        "ALL_AIRCRAFT",
        "DEMON_AIRCRAFT",
        "CHAOS_AIRCRAFT",
        "ALL_STRUCTURES",
        "DEMON_STRUCTURES",
        "CHAOS_STRUCTURES",
    ]
    gameplay_list_const_names = [
        "CRUSHER_UNIT_PATHS",
        "CRUSHABLE_UNIT_PATHS",
    ]
    consts = {
        name: extract_const(constants_text, name)
        for name in list_const_names
        + gameplay_list_const_names
        + [
            "PRODUCTION_COSTS",
            "PRODUCTION_TIMES",
            "PRODUCTION_REQUIREMENTS",
            "STRUCTURE_BLUEPRINTS",
            "CONSTRUCTION_REQUIREMENTS",
            "STRUCTURE_NAME_KEYS",
            "CONSTRUCTION_COSTS",
            "POWER_SUPPLY",
            "POWER_DRAIN",
            "INFILTRATION_RESOURCE_TARGETS",
            "INFILTRATION_POWER_SABOTAGE_TARGETS",
            "INFILTRATION_PRODUCTION_VETERANCY_TARGETS",
            "DEFAULT_PROPERTIES",
        ]
    }
    path_lists = {name: parse_paths(block) for name, block in consts.items()}
    production_costs = parse_costs(consts["PRODUCTION_COSTS"])
    construction_costs = parse_costs(consts["CONSTRUCTION_COSTS"])
    production_times = parse_number_map(consts["PRODUCTION_TIMES"])
    production_requirements = {
        path: parse_paths(body)
        for path, body in top_level_dict_entries(consts["PRODUCTION_REQUIREMENTS"]).items()
    }
    construction_requirements = {
        path: parse_paths(body)
        for path, body in top_level_dict_entries(consts["CONSTRUCTION_REQUIREMENTS"]).items()
    }
    structure_blueprints = parse_string_map(consts["STRUCTURE_BLUEPRINTS"])
    structure_name_keys = parse_string_map(consts["STRUCTURE_NAME_KEYS"])
    power_supply = {path: int(value) for path, value in parse_number_map(consts["POWER_SUPPLY"]).items()}
    power_drain = {path: int(value) for path, value in parse_number_map(consts["POWER_DRAIN"]).items()}
    default_properties = parse_properties(consts["DEFAULT_PROPERTIES"], constant_values)
    crusher_paths = set(path_lists["CRUSHER_UNIT_PATHS"])
    crushable_paths = set(path_lists["CRUSHABLE_UNIT_PATHS"])
    infiltration_resource_targets = set(parse_paths(consts["INFILTRATION_RESOURCE_TARGETS"]))
    infiltration_power_sabotage_targets = set(parse_paths(consts["INFILTRATION_POWER_SABOTAGE_TARGETS"]))
    infiltration_production_veterancy_targets = parse_string_map(
        consts["INFILTRATION_PRODUCTION_VETERANCY_TARGETS"]
    )

    all_scene_paths = set()
    for name in list_const_names:
        all_scene_paths.update(path_lists[name])
    all_scene_paths.update(production_costs)
    all_scene_paths.update(construction_costs)
    all_scene_paths.update(production_times)
    all_scene_paths.update(structure_blueprints)
    all_scene_paths.update(structure_name_keys)
    all_scene_paths.update(default_properties)
    all_scene_paths = {
        path
        for path in all_scene_paths
        if "/structure-geometries/" not in path
        and (GODOT_ROOT / path.removeprefix("res://")).exists()
    }

    entities = []
    for scene_path in sorted(all_scene_paths, key=scene_path_to_id):
        entity_id = scene_path_to_id(scene_path)
        if entity_id in REMOVED_ENTITY_IDS:
            continue
        role = "structure" if scene_path in structure_name_keys or scene_path in construction_costs else "unit"
        scene_file = GODOT_ROOT / scene_path.removeprefix("res://")
        scene_data = parse_scene(scene_file) if scene_file.exists() else {
            "model_assets": [],
            "render_parts": [],
            "media_assets": [],
            "radius": 1.0 if role == "structure" else 0.6,
            "speed": 0.0 if role == "structure" else 4.0,
            "domain": "terrain",
        }
        if entity_id in ENTITY_RENDER_OVERRIDES:
            scene_data = {**scene_data, **ENTITY_RENDER_OVERRIDES[entity_id]}
        props = dict(default_properties.get(scene_path, {}))
        script_file = scene_file.with_suffix(".gd")
        if script_file.exists():
            script_constants = parse_script_constant_floats(script_file.read_text())
            if "repair_radius" not in props and "REPAIR_RADIUS" in script_constants:
                props["repair_radius"] = script_constants["REPAIR_RADIUS"]
        hp = float(props.get("hp_max", props.get("hp", 20 if role == "structure" else 6)))
        attack_damage = props.get("attack_damage")
        attack_range = props.get("attack_range")
        attack_interval = props.get("attack_interval")
        can_attack_air = bool(props.get("can_attack_air", True))
        can_attack_ground = bool(props.get("can_attack_ground", True))
        weapon = None
        if attack_damage is not None and attack_range is not None and attack_interval is not None:
            weapon = {
                "range": float(attack_range),
                "damage": float(attack_damage),
                "cooldown": float(attack_interval),
                "splash_radius": float(props.get("splash_radius", 0.0)),
                "splash_damage_multiplier": float(props.get("splash_damage_multiplier", 0.5)),
                "structure_damage_multiplier": float(
                    props.get("structure_damage_multiplier", 1.0)
                ),
                "can_attack_air": can_attack_air,
                "can_attack_ground": can_attack_ground,
            }
        icon = f"ui/icons/{entity_id}.png"
        icon_path = REPO_ROOT / "assets" / icon
        godot_icon_path = GODOT_ROOT / "assets/ui/icons" / f"{entity_id}.png"
        if not icon_path.exists() and not godot_icon_path.exists():
            icon = None
        cost = construction_costs.get(scene_path) if role == "structure" else production_costs.get(scene_path)
        if cost is None:
            cost = {"ore": 0, "crystal": 0}
        resource_capacity = int(props.get("resources_max", 0))
        if entity_id == "Worker":
            resource_capacity = max(resource_capacity, 6)

        entities.append(
            {
                "id": entity_id,
                "scene_path": scene_path,
                "role": role,
                "label": label_from_id(entity_id),
                "name_key": structure_name_keys.get(scene_path),
                "domain": scene_data["domain"],
                "model_assets": scene_data["model_assets"],
                "render_parts": scene_data["render_parts"],
                "media_assets": scene_data["media_assets"],
                "icon": icon,
                "health": hp,
                "radius": float(scene_data["radius"]),
                "sight_range": float(props.get("sight_range", 0.0)),
                "speed": 0.0 if role == "structure" else float(scene_data["speed"]),
                "height": 1.5 if scene_data["domain"] == "air" else 0.05,
                "scale": 0.65 if role == "unit" else 1.0,
                "weapon": weapon,
                "repair_rate": float(props.get("repair_rate", 0.0)),
                "repair_radius": float(props.get("repair_radius", 0.0)),
                "healing_rate": float(props.get("healing_rate", 0.0)),
                "healing_radius": float(props.get("healing_radius", 0.0)),
                "capture_time": float(props.get("capture_time", 0.0)),
                "infiltration_resource_steal_ratio": float(
                    props.get("infiltration_resource_steal_ratio", 0.0)
                ),
                "infiltration_resource_steal_cap": int(
                    props.get("infiltration_resource_steal_cap", 0)
                ),
                "infiltration_production_veterancy_rank": int(
                    props.get("infiltration_production_veterancy_rank", 0)
                ),
                "infiltration_power_sabotage_duration": float(
                    props.get("infiltration_power_sabotage_duration", 0.0)
                ),
                "is_infiltration_resource_target": scene_path in infiltration_resource_targets,
                "is_infiltration_power_sabotage_target": scene_path
                in infiltration_power_sabotage_targets,
                "infiltration_production_veterancy_producer": (
                    scene_path_to_id(infiltration_production_veterancy_targets[scene_path])
                    if scene_path in infiltration_production_veterancy_targets
                    else None
                ),
                "resource_capacity": resource_capacity,
                "mine_damage": float(props.get("mine_damage", 0.0)),
                "mine_trigger_radius": float(props.get("trigger_radius", 0.0)),
                "mine_blast_radius": float(props.get("blast_radius", 0.0)),
                "mine_arming_delay": float(props.get("arming_delay", 0.0)),
                "mine_deploy_interval": float(props.get("mine_deploy_interval", 0.0)),
                "mine_deploy_radius": float(props.get("mine_deploy_radius", 0.0)),
                "mine_spacing": float(props.get("mine_spacing", 0.0)),
                "mine_limit": int(props.get("mine_limit", 0)),
                "resource_income_ore": int(props.get("resource_income_a", 0)),
                "resource_income_crystal": int(props.get("resource_income_b", 0)),
                "income_interval": float(props.get("income_interval_s", 0.0)),
                "capture_bonus_ore": int(props.get("capture_bonus_a", 0)),
                "capture_bonus_crystal": int(props.get("capture_bonus_b", 0)),
                "garrison_capacity": int(props.get("garrison_capacity", 0)),
                "garrison_attack_damage_per_unit": float(
                    props.get("garrison_attack_damage_per_unit", 0.0)
                ),
                "support_shield_radius": float(props.get("support_shield_radius", 0.0)),
                "support_shield_duration": float(props.get("support_shield_duration", 0.0)),
                "support_shield_damage_multiplier": float(
                    props.get("support_shield_damage_multiplier", 1.0)
                ),
                "cost": cost,
                "build_seconds": float(production_times.get(scene_path, 8.0 if role == "structure" else 4.0)),
                "power_delta": int(power_supply.get(scene_path, 0) - power_drain.get(scene_path, 0)),
                "requirements": [
                    scene_path_to_id(path)
                    for path in (
                        construction_requirements.get(scene_path, [])
                        if role == "structure"
                        else production_requirements.get(scene_path, [])
                    )
                ],
                "procedural_render_note": PROCEDURAL_RENDER_IDS.get(entity_id),
                "blueprint_scene": structure_blueprints.get(scene_path),
                "is_resource_producer": entity_id in {"Refinery", "OrePurifier"},
                "is_worker": entity_id == "Worker",
                "can_crush": scene_path in crusher_paths,
                "can_be_crushed": scene_path in crushable_paths,
            }
        )

    entity_ids = {entity["id"] for entity in entities}
    factions = []
    for faction_id, structure_const in FACTION_STRUCTURE_LIST.items():
        structures = [
            scene_path_to_id(path)
            for path in path_lists[structure_const]
            if scene_path_to_id(path) in entity_ids
        ]
        structures = apply_structure_order_override(structures)
        production = []
        for producer_id, list_const in FACTION_PRODUCTION_LISTS[faction_id].items():
            products = [
                scene_path_to_id(path)
                for path in path_lists[list_const]
                if scene_path_to_id(path) in entity_ids
            ]
            products = apply_production_order_override(producer_id, products)
            production.append({"producer": producer_id, "products": products})
        factions.append(
            {
                "id": faction_id,
                "label": FACTION_LABELS[faction_id],
                "emblem": FACTION_EMBLEMS[faction_id],
                "color": FACTION_COLORS[faction_id],
                "structures": structures,
                "production": production,
            }
        )
    return {"entities": entities, "factions": factions}


def write_rust(registry: dict[str, object]) -> None:
    entities: list[dict[str, object]] = registry["entities"]  # type: ignore[assignment]
    factions: list[dict[str, object]] = registry["factions"]  # type: ignore[assignment]
    lines = [
        "// @generated by scripts/generate_registry.py",
        "#![allow(dead_code)]",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum EntityRole {",
        "    Unit,",
        "    Structure,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum MoveDomain {",
        "    Terrain,",
        "    Air,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]",
        "pub struct Cost {",
        "    pub ore: i32,",
        "    pub crystal: i32,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct WeaponDef {",
        "    pub range: f32,",
        "    pub damage: f32,",
        "    pub cooldown: f32,",
        "    pub splash_radius: f32,",
        "    pub splash_damage_multiplier: f32,",
        "    pub structure_damage_multiplier: f32,",
        "    pub can_attack_air: bool,",
        "    pub can_attack_ground: bool,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct RenderPart {",
        "    pub model: &'static str,",
        "    pub translation: [f32; 3],",
        "    pub rotation: [f32; 4],",
        "    pub scale: [f32; 3],",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct EntityDef {",
        "    pub id: &'static str,",
        "    pub scene_path: &'static str,",
        "    pub label: &'static str,",
        "    pub role: EntityRole,",
        "    pub domain: MoveDomain,",
        "    pub model_assets: &'static [&'static str],",
        "    pub render_parts: &'static [RenderPart],",
        "    pub icon: Option<&'static str>,",
        "    pub health: f32,",
        "    pub radius: f32,",
        "    pub sight_range: f32,",
        "    pub speed: f32,",
        "    pub height: f32,",
        "    pub scale: f32,",
        "    pub weapon: Option<WeaponDef>,",
        "    pub repair_rate: f32,",
        "    pub repair_radius: f32,",
        "    pub healing_rate: f32,",
        "    pub healing_radius: f32,",
        "    pub capture_time: f32,",
        "    pub infiltration_resource_steal_ratio: f32,",
        "    pub infiltration_resource_steal_cap: i32,",
        "    pub infiltration_production_veterancy_rank: u8,",
        "    pub infiltration_power_sabotage_duration: f32,",
        "    pub is_infiltration_resource_target: bool,",
        "    pub is_infiltration_power_sabotage_target: bool,",
        "    pub infiltration_production_veterancy_producer: Option<&'static str>,",
        "    pub resource_capacity: i32,",
        "    pub mine_damage: f32,",
        "    pub mine_trigger_radius: f32,",
        "    pub mine_blast_radius: f32,",
        "    pub mine_arming_delay: f32,",
        "    pub mine_deploy_interval: f32,",
        "    pub mine_deploy_radius: f32,",
        "    pub mine_spacing: f32,",
        "    pub mine_limit: usize,",
        "    pub resource_income_ore: i32,",
        "    pub resource_income_crystal: i32,",
        "    pub income_interval: f32,",
        "    pub capture_bonus_ore: i32,",
        "    pub capture_bonus_crystal: i32,",
        "    pub garrison_capacity: usize,",
        "    pub garrison_attack_damage_per_unit: f32,",
        "    pub support_shield_radius: f32,",
        "    pub support_shield_duration: f32,",
        "    pub support_shield_damage_multiplier: f32,",
        "    pub cost: Cost,",
        "    pub build_seconds: f32,",
        "    pub power_delta: i32,",
        "    pub is_resource_producer: bool,",
        "    pub is_worker: bool,",
        "    pub can_crush: bool,",
        "    pub can_be_crushed: bool,",
        "    pub requirements: &'static [&'static str],",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct ProductionDef {",
        "    pub producer: &'static str,",
        "    pub products: &'static [&'static str],",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct FactionDef {",
        "    pub id: &'static str,",
        "    pub label: &'static str,",
        "    pub emblem: &'static str,",
        "    pub color: [f32; 3],",
        "    pub structures: &'static [&'static str],",
        "    pub production: &'static [ProductionDef],",
        "}",
        "",
    ]

    for entity in entities:
        const_name = f"MODELS_{rust_ident(entity['id'])}"
        lines.append(f"const {const_name}: &[&str] = &[")
        for model in entity["model_assets"]:  # type: ignore[index]
            lines.append(f"    {rust_str(model)},")
        lines.append("];")
        parts_const_name = f"PARTS_{rust_ident(entity['id'])}"
        lines.append(f"const {parts_const_name}: &[RenderPart] = &[")
        for part in entity["render_parts"]:  # type: ignore[index]
            translation = ", ".join(rust_f32(component) for component in part["translation"])
            scale = ", ".join(rust_f32(component) for component in part["scale"])
            rotation = ", ".join(rust_f32(component) for component in part["rotation"])
            lines.append(
                "    RenderPart { "
                f"model: {rust_str(part['model'])}, "
                f"translation: [{translation}], "
                f"rotation: [{rotation}], "
                f"scale: [{scale}] "
                "},"
            )
        lines.append("];")
    lines.append("")

    lines.append("pub const ENTITY_DEFS: &[EntityDef] = &[")
    for entity in entities:
        weapon = entity["weapon"]
        if weapon:
            weapon_value = (
                "Some(WeaponDef { "
                f"range: {rust_f32(weapon['range'])}, "
                f"damage: {rust_f32(weapon['damage'])}, "
                f"cooldown: {rust_f32(weapon['cooldown'])}, "
                f"splash_radius: {rust_f32(weapon['splash_radius'])}, "
                f"splash_damage_multiplier: {rust_f32(weapon['splash_damage_multiplier'])}, "
                f"structure_damage_multiplier: {rust_f32(weapon['structure_damage_multiplier'])}, "
                f"can_attack_air: {rust_bool(weapon['can_attack_air'])}, "
                f"can_attack_ground: {rust_bool(weapon['can_attack_ground'])} "
                "})"
            )
        else:
            weapon_value = "None"
        lines.extend(
            [
                "    EntityDef {",
                f"        id: {rust_str(entity['id'])},",
                f"        scene_path: {rust_str(entity['scene_path'])},",
                f"        label: {rust_str(entity['label'])},",
                f"        role: EntityRole::{'Structure' if entity['role'] == 'structure' else 'Unit'},",
                f"        domain: MoveDomain::{'Air' if entity['domain'] == 'air' else 'Terrain'},",
                f"        model_assets: MODELS_{rust_ident(entity['id'])},",
                f"        render_parts: PARTS_{rust_ident(entity['id'])},",
                f"        icon: {rust_opt_str(entity['icon'])},",
                f"        health: {rust_f32(entity['health'])},",
                f"        radius: {rust_f32(entity['radius'])},",
                f"        sight_range: {rust_f32(entity['sight_range'])},",
                f"        speed: {rust_f32(entity['speed'])},",
                f"        height: {rust_f32(entity['height'])},",
                f"        scale: {rust_f32(entity['scale'])},",
                f"        weapon: {weapon_value},",
                f"        repair_rate: {rust_f32(entity['repair_rate'])},",
                f"        repair_radius: {rust_f32(entity['repair_radius'])},",
                f"        healing_rate: {rust_f32(entity['healing_rate'])},",
                f"        healing_radius: {rust_f32(entity['healing_radius'])},",
                f"        capture_time: {rust_f32(entity['capture_time'])},",
                f"        infiltration_resource_steal_ratio: {rust_f32(entity['infiltration_resource_steal_ratio'])},",
                f"        infiltration_resource_steal_cap: {entity['infiltration_resource_steal_cap']},",
                f"        infiltration_production_veterancy_rank: {entity['infiltration_production_veterancy_rank']},",
                f"        infiltration_power_sabotage_duration: {rust_f32(entity['infiltration_power_sabotage_duration'])},",
                f"        is_infiltration_resource_target: {rust_bool(entity['is_infiltration_resource_target'])},",
                f"        is_infiltration_power_sabotage_target: {rust_bool(entity['is_infiltration_power_sabotage_target'])},",
                f"        infiltration_production_veterancy_producer: {rust_opt_str(entity['infiltration_production_veterancy_producer'])},",
                f"        resource_capacity: {entity['resource_capacity']},",
                f"        mine_damage: {rust_f32(entity['mine_damage'])},",
                f"        mine_trigger_radius: {rust_f32(entity['mine_trigger_radius'])},",
                f"        mine_blast_radius: {rust_f32(entity['mine_blast_radius'])},",
                f"        mine_arming_delay: {rust_f32(entity['mine_arming_delay'])},",
                f"        mine_deploy_interval: {rust_f32(entity['mine_deploy_interval'])},",
                f"        mine_deploy_radius: {rust_f32(entity['mine_deploy_radius'])},",
                f"        mine_spacing: {rust_f32(entity['mine_spacing'])},",
                f"        mine_limit: {entity['mine_limit']},",
                f"        resource_income_ore: {entity['resource_income_ore']},",
                f"        resource_income_crystal: {entity['resource_income_crystal']},",
                f"        income_interval: {rust_f32(entity['income_interval'])},",
                f"        capture_bonus_ore: {entity['capture_bonus_ore']},",
                f"        capture_bonus_crystal: {entity['capture_bonus_crystal']},",
                f"        garrison_capacity: {entity['garrison_capacity']},",
                f"        garrison_attack_damage_per_unit: {rust_f32(entity['garrison_attack_damage_per_unit'])},",
                f"        support_shield_radius: {rust_f32(entity['support_shield_radius'])},",
                f"        support_shield_duration: {rust_f32(entity['support_shield_duration'])},",
                f"        support_shield_damage_multiplier: {rust_f32(entity['support_shield_damage_multiplier'])},",
                "        cost: Cost { "
                f"ore: {entity['cost']['ore']}, crystal: {entity['cost']['crystal']} "
                "},",
                f"        build_seconds: {rust_f32(entity['build_seconds'])},",
                f"        power_delta: {entity['power_delta']},",
                f"        is_resource_producer: {rust_bool(entity['is_resource_producer'])},",
                f"        is_worker: {rust_bool(entity['is_worker'])},",
                f"        can_crush: {rust_bool(entity['can_crush'])},",
                f"        can_be_crushed: {rust_bool(entity['can_be_crushed'])},",
            ]
        )
        lines.append("        requirements: &[")
        for req in entity["requirements"]:  # type: ignore[index]
            lines.append(f"            {rust_str(req)},")
        lines.extend(["        ],", "    },"])
    lines.append("];")
    lines.append("")

    for faction in factions:
        prefix = rust_ident(faction["id"])
        lines.append(f"const {prefix}_STRUCTURES: &[&str] = &[")
        for structure in faction["structures"]:
            lines.append(f"    {rust_str(structure)},")
        lines.append("];")
        for production in faction["production"]:
            products_const = f"{prefix}_PRODUCTS_{rust_ident(production['producer'])}"
            lines.append(f"const {products_const}: &[&str] = &[")
            for product in production["products"]:
                lines.append(f"    {rust_str(product)},")
            lines.append("];")
        lines.append(f"const {prefix}_PRODUCTION: &[ProductionDef] = &[")
        for production in faction["production"]:
            products_const = f"{prefix}_PRODUCTS_{rust_ident(production['producer'])}"
            lines.append(
                f"    ProductionDef {{ producer: {rust_str(production['producer'])}, products: {products_const} }},"
            )
        lines.append("];")
    lines.append("")

    lines.append("pub const FACTION_DEFS: &[FactionDef] = &[")
    for faction in factions:
        prefix = rust_ident(faction["id"])
        color = ", ".join(rust_f32(component) for component in faction["color"])
        lines.extend(
            [
                "    FactionDef {",
                f"        id: {rust_str(faction['id'])},",
                f"        label: {rust_str(faction['label'])},",
                f"        emblem: {rust_str(faction['emblem'])},",
                f"        color: [{color}],",
                f"        structures: {prefix}_STRUCTURES,",
                f"        production: {prefix}_PRODUCTION,",
                "    },",
            ]
        )
    lines.append("];")
    lines.append("")
    lines.extend(
        [
            "pub fn entity(id: &str) -> Option<&'static EntityDef> {",
            "    ENTITY_DEFS.iter().find(|entity| entity.id == id)",
            "}",
            "",
            "pub fn faction(id: &str) -> Option<&'static FactionDef> {",
            "    FACTION_DEFS.iter().find(|faction| faction.id == id)",
            "}",
            "",
            "impl FactionDef {",
            "    pub fn production_for(&self, producer: &str) -> Option<&'static [&'static str]> {",
            "        self.production",
            "            .iter()",
            "            .find(|entry| entry.producer == producer)",
            "            .map(|entry| entry.products)",
            "    }",
            "",
            "    pub fn can_produce(&self, producer: &str, product: &str) -> bool {",
            "        self.production_for(producer)",
            "            .is_some_and(|products| products.contains(&product))",
            "    }",
            "",
            "    pub fn can_construct(&self, structure: &str) -> bool {",
            "        self.structures.contains(&structure)",
            "    }",
            "}",
            "",
            "impl EntityDef {",
            "    pub fn primary_model(&self) -> Option<&'static str> {",
            "        self.model_assets.first().copied()",
            "    }",
            "}",
        ]
    )
    OUT_RS.write_text("\n".join(lines) + "\n")


def write_reports(registry: dict[str, object]) -> None:
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n")
    removed_asset_paths = {
        f"ui/icons/{entity_id}.png" for entity_id in REMOVED_ENTITY_IDS
    }
    source_assets = sorted(
        str(path.relative_to(GODOT_ROOT / "assets"))
        for path in (GODOT_ROOT / "assets").rglob("*")
        if path.is_file()
        and path.suffix != ".import"
        and str(path.relative_to(GODOT_ROOT / "assets")) not in removed_asset_paths
    )
    asset_categories: dict[str, int] = {}
    for asset in source_assets:
        category = asset.split("/", 1)[0]
        asset_categories[category] = asset_categories.get(category, 0) + 1
    OUT_ASSET_MANIFEST.write_text(
        json.dumps(
            {
                "source": str(GODOT_ROOT / "assets"),
                "asset_count": len(source_assets),
                "categories": dict(sorted(asset_categories.items())),
                "assets": source_assets,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    entities: list[dict[str, object]] = registry["entities"]  # type: ignore[assignment]
    factions: list[dict[str, object]] = registry["factions"]  # type: ignore[assignment]
    procedural_only = [
        entity["id"]
        for entity in entities
        if not entity["model_assets"] and entity["procedural_render_note"] is not None  # type: ignore[index]
    ]
    missing_models = [
        entity["id"]
        for entity in entities
        if not entity["model_assets"] and entity["procedural_render_note"] is None  # type: ignore[index]
    ]
    unit_count = sum(1 for entity in entities if entity["role"] == "unit")
    structure_count = sum(1 for entity in entities if entity["role"] == "structure")
    model_refs = sorted(
        {
            model
            for entity in entities
            for model in entity["model_assets"]  # type: ignore[index]
        }
    )
    icon_refs = sorted(
        {
            entity["icon"]
            for entity in entities
            if entity["icon"] is not None
        }
    )
    report = [
        "# Gameplay Migration Report",
        "",
        f"- Entities: {len(entities)} ({unit_count} units, {structure_count} structures)",
        f"- Factions: {len(factions)}",
        f"- Mirrored Godot asset files: {len(source_assets)}",
        f"- Referenced GLB models: {len(model_refs)}",
        f"- Matched command icons: {len(icon_refs)}",
        f"- Procedural-only Godot render scenes: {len(procedural_only)}",
        f"- Entity definitions without render data: {len(missing_models)}",
        "",
        "## Factions",
        "",
    ]
    for faction in factions:
        product_count = sum(len(entry["products"]) for entry in faction["production"])  # type: ignore[index]
        report.append(
            f"- {faction['label']}: {len(faction['structures'])} structures, {product_count} production entries"
        )
    if missing_models:
        report.extend(["", "## Missing Models", ""])
        for entity_id in missing_models:
            report.append(f"- {entity_id}")
    if procedural_only:
        report.extend(["", "## Procedural Render Scenes", ""])
        for entity_id in procedural_only:
            report.append(f"- {entity_id}: {PROCEDURAL_RENDER_IDS[entity_id]}")
    OUT_REPORT.write_text("\n".join(report) + "\n")


def main() -> None:
    registry = build_registry()
    write_rust(registry)
    write_reports(registry)


if __name__ == "__main__":
    main()
