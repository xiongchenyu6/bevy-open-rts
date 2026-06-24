# Gameplay Migration Report

- Entities: 75 (50 units, 25 structures)
- Factions: 3
- Mirrored Godot asset files: 528
- Referenced GLB models: 61
- Matched command icons: 72
- Procedural-only Godot render scenes: 2
- Entity definitions without render data: 0

## Factions

- 人族: 20 structures, 49 production entries
- 魔族: 17 structures, 24 production entries
- 混沌族: 17 structures, 32 production entries

## Procedural Render Scenes

- LandMine: Godot CylinderMesh/TorusMesh scene; Bevy runtime recreates it with procedural primitive meshes.
- TeslaFenceSegment: Godot BoxMesh/CylinderMesh scene; Bevy runtime recreates it with procedural primitive meshes.
