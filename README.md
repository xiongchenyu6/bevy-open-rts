# Bevy Open RTS

This repository is a Bevy `0.19` porting foundation for `../godot-open-rts`.
It keeps the runtime browser-focused: one playable skirmish scene, selection,
right-click orders, automatic combat, registry-driven production/economy, AI
pressure, and a WebGPU build path.

The architecture follows the useful lesson from Digital Extinction: keep RTS
behavior separated into ECS systems instead of a single scene script. This pass
uses one crate, but the systems are grouped by responsibility so Godot mechanics
can be moved across incrementally. Playable entity data is generated from
`../godot-open-rts/source/match/MatchConstants.gd` and the Godot unit scenes.

## Run Native

All native commands should run inside the Nix dev shell so Bevy can find the
X11/Wayland/Vulkan/audio runtime libraries:

```sh
direnv allow
cargo run
```

Or run the wrapper directly:

```sh
scripts/run_desktop.sh
```

## Build WebGPU

```sh
scripts/build_web.sh
scripts/serve_web.py
```

Then open `http://127.0.0.1:8080`.

The web build requires `wasm-bindgen` on `PATH`. The Nix shell in this repo
includes the wasm target tools. `scripts/build_web.sh` stamps `web/index.html`
with a content hash of the generated JS/WASM artifacts so browsers do not reuse
stale game code after a rebuild.

## Regenerate Migration Data

```sh
rsync -a --exclude='*.import' ../godot-open-rts/assets/ assets/
scripts/generate_registry.py
cargo fmt
```

Generated outputs:

- `src/generated_registry.rs`: static Rust registry used by the Bevy runtime.
- `assets/migration/gameplay_registry.json`: complete gameplay entity/faction
  audit data.
- `assets/migration/asset_manifest.json`: copied Godot asset inventory.
- `assets/migration/migration_report.md`: human-readable migration summary.

## Current Scope

- Uses Bevy `0.19.0`.
- Mirrors the non-`.import` Godot asset tree and generates 75 playable
  unit/structure definitions across the three Godot factions.
- Loads all referenced GLB assets for migrated entities; Godot procedural-only
  render scenes are recreated with Bevy primitive meshes.
- Keeps the browser target on WebGPU through Bevy's `webgpu` feature.
- Leaves the Godot project untouched.

## Controls

- The game opens on the skirmish setup menu. Press 1-4 to choose a map,
  H/D/C to choose the player slot, Z/X/V to cycle each slot's faction,
  9/0 to choose 1v1 or three-faction skirmish, J/K/L to cycle team ids,
  Q/W/E to cycle player controllers, U/I/O to cycle player colors, F1-F4
  to choose AI difficulty, 5-8 to choose starting resources, then Enter
  or click `开始对战`.
- Left click selects a visible unit or structure.
- Right click orders selected mobile units to move or attack.
- Drag the mouse near the viewport edge, or use WASD, to pan the camera.
- Mouse wheel zooms.
- Escape opens the in-match menu for resume, restart, or return to setup.
- The bottom command panel changes with the selected producer. Click commands
  to train/build; number keys 1-9 and 0 activate the first ten slots.
