# Bevy Open RTS Structure

## Runtime Entry

- `src/main.rs` calls `bevy_open_rts::run_game_app()`.
- `run_game_app()` builds `build_game_app(GameAppMode::Interactive)`, which registers the Godot-style front menu, options/credits screens, setup menu, and the shared match scene from `src/lib.rs`.
- Native desktop builds are configured for Wayland through Bevy's `wayland` feature and `scripts/native_runner.sh`; the project does not require X11 runtime libraries.
- `AppScreen::AssetLoading` is the default startup state. `iyes_progress::ProgressPlugin<AppScreen>` tracks startup assets and transitions to `AppScreen::MainMenu` once UI, cursor, icon, model-map, and migrated GLB scene assets are ready.
- `StartupLoadingPolicy` keeps interactive/capture apps on the real preload path while headless tests can skip render asset preloading and exercise logic without waiting on graphics assets.

## Shared Match Scene

- `SharedMatchScenePlugin` owns the live RTS scene.
- `add_shared_match_scene()` registers match resources, `OnEnter(AppScreen::InMatch)` setup, runtime systems, and `OnExit` cleanup.
- `start_shared_match_scene_with_current_setup()` advances any app with the shared scene plugin into `AppScreen::InMatch`.
- `start_shared_match_scene_with_settings()` is the internal helper for capture/test apps that need to inject a specific `MatchSetupSettings` before entering the same live scene.
- The main menu uses `start_shared_match_from_menu_selection()` so `cargo run` and capture/test proofs enter through the same setup contract.
- Match start camera focus is derived from the selected player's actual base anchor: first the `CommandCenter`, then a fallback `Worker` spawn. It no longer blends toward nearby resources, so `cargo run` opens over the player's base instead of a zoomed-out map work area.
- Resource nodes are left-click selectable for target confirmation, and selected Workers use a wider resource-specific right-click snap so clicking the visible ore/crystal model edge still issues `HarvestOrder`.
- Manual structure placement assigns selected construction-capable Workers to the foundation, with a nearest-owned-Worker fallback so a valid player placement enters real construction instead of remaining an idle foundation.
- World overlays keep selected rings, resource/supply rings, active command markers, and the current placement footprint. Unselected structures and construction-range anchors no longer draw permanent rings.
- Runtime player state is data-driven from `ActiveTeams` / `MatchSetupSettings`: economies, team relations, AI timers, support cooldowns, HUD counts, and match-end checks grow by player index instead of assuming three hard-coded teams.
- `Team` is runtime identity only (`Player(index)` / `Neutral`). Playable faction identity comes from `PlayerFactions`, so 人族/魔族/混沌族 rules follow the configured player slot instead of being tied to Player0/1/2.
- Lobby team buttons remain an 8-row setup UI concern, but runtime team IDs are stored and derived as unbounded `usize` values. The battle core no longer clamps alliances to three teams or to the current lobby button count.
- Runtime spawning is not capped to the map's authored spawn-point count. Players beyond the map rows receive clamped virtual fallback base positions instead of being skipped.
- AI/runtime fallback helpers are not capped to the lobby slot count: active AI iteration, opponent helpers, late-slot resources, cooldowns, fallback home positions, virtual spawn positions, runtime team relation IDs, and battle AI participation are verified beyond eight players.
- `bevy_fluent::FluentPlugin` is registered in the shared game scene so future `.ftl` localization bundles can load through Bevy assets. The existing `Locale` / `t()` path remains the active text source until screens are migrated incrementally.
- AI drones have an active scouting controller: idle AI `Drone` units pick living enemy units, move to their positions, avoid repeating the previous target when possible, and retarget after a short 0.5-1.0s delay.
- AI defense profiles follow the godot difficulty targets: Beginner/Easy do not inherit Normal advanced-defense construction, Normal targets one standard defense layer plus 2 Tesla fence segments where the faction supports them, and Hard scales standard defenses to 2 plus 4 Tesla fence segments.
- Easy AI is tuned as a build-up opponent: it trains a small defensive force but does not launch active attack waves, giving default human starts enough time to build a Barracks and form an army. Normal/Hard keep active offense.
- The minimal `GodotSkirmish` opening remains one `CommandCenter` plus two `Worker` economy units, but each faction gets a distinct starter combat/scout unit (`ScoutRover`, `RocketInfantry`, `ShieldTrooper`) so the default `cargo run` start is not visually identical across races.
- Godot render-part mapping is audited separately from gameplay generation. `assets/data/godot_model_map.model_map.ron` is a Bevy-loadable baseline asset generated from Godot `*.tscn` scenes, and `scripts/audit_model_mapping.py` compares it against `src/generated_registry.rs` without regenerating or overwriting the hand-expanded registry. The baseline is reference data, not permission to keep poor Bevy silhouettes.
- `Worker` now uses a distinct field-engineer model composition (`astronautB` plus equipment) instead of sharing `ScoutRover`'s `rover.glb`. Critical unit silhouettes are protected by tests and by `scripts/audit_model_quality.py`.
- `scripts/audit_model_quality.py --fail-critical --require-screenshots` is the model quality gate: it fails missing models, critical shared silhouettes, duplicate unit model signatures, and missing model-harness coverage. It also writes `docs/model-quality/hunyuan3d-queue.json`, a machine-readable Hunyuan3D replacement queue for multipart/kitbashed units with harness page/cell references and generation prompts.
- `scripts/comfy_hunyuan3d_queue.py` stages reference images and API workflows for remote ComfyUI/Hunyuan3D runs. `CryoSprayer` is the first completed single-GLB replacement and is mapped to `assets/models/hunyuan3d/CryoSprayer.glb`; the quality gate now leaves 13 remaining multipart unit replacements in the queue.
- Worker harvesting now has runtime VFX parity for the important Godot cues: collecting emits front sparkles and resource-to-worker pulses, carried ore/crystal draws visible cargo dots on the rover, and dropoff clears the cargo through the existing `ResourceCargo` flow.
- Weapon hits now spawn short-lived `ImpactBurst` overlays that scale by applied damage, target radius, and structure hits so combat feedback is visible beyond the unit health bars.
- The OS cursor is owned by the shared game scene through `bevy_cursor_kit`. `assets/ui/cursors/rts_cursor.cur.ron` maps the dedicated atlas to default, move/patrol/rally, attack/support targeting, and build/resource targeting cursor states.

## Capture And Proofs

- `src/bin/capture.rs` exposes proof commands for live match simulation.
- Capture snapshots use `CaptureTeam::Player(index)` and `players: Vec<CaptureTeamStats>` rather than fixed human/demon/chaos fields, so proof output can represent every runtime player row.
- `build_capture_match_app_for_faction()` uses `SharedMatchScenePlugin` plus `start_shared_match_scene_with_settings()`.
- Current capture commands are the authoritative smoke surfaces:
  - `capture menu [path]`: renders the Godot-style command menu.
  - `capture menu-wide [path]`: renders the command menu at 2048x1224 to verify desktop proportions.
  - `capture menu-return [path]`: enters setup, returns to the command menu, and verifies no setup overlay/camera is left behind.
  - `capture menu-options [path]`: renders the migrated options/settings screen.
  - `capture menu-credits [path]`: renders the migrated credits screen.
  - `capture menu-setup [path]`: renders the real lobby/setup screen.
  - `capture screenshot [path]`: renders one still from the shared live match.
  - `capture frames <dir> [count]`: records a frame sequence from the shared live match.
  - `capture harvest <dir>`: selects a real Worker, right-clicks ore, and requires `HarvestOrder` plus resource growth.
  - `capture play <dir>`: uses the real default-start economy for select, move, CommandCenter training, completed unit spawn, Worker build placement, and completed Worker construction.
  - `capture assault <dir> [max-seconds]`: uses real input to build a Barracks, train Heavy Machinegun Troopers, retarget attack-move orders onto living enemy anchors, and requires `HumanVictory`.
  - `capture factions <dir>`: starts each playable faction and verifies train/build through the human command panel using default resources.
  - `capture match [max-seconds]`: runs a headless AI-vs-AI shared match and requires economy, production, combat, and elimination to resolve.
  - `capture model-harness` uses the harness-owned `MODEL_HARNESS_ENTITY_IDS` and
    `moonshine-kind::Instance<ModelHarnessRoot>` roots so registry coverage and
    harness spawn boundaries are checked by Rust types instead of loose entity
    plumbing.
- Current model-harness output is intentionally ignored under `screenshots/`.
  Regenerate it with `RUST_LOG=warn cargo run --bin capture -- model-harness
  screenshots/model-harness 6` before running the quality gate when changing
  unit visuals.
- The old `real-*-proof` CLI surfaces were removed; do not use them as completion gates.

## Verification

- Use `cargo fmt`, `cargo check`, targeted `cargo test`, full `cargo test`, `cargo build`, and a short `cargo run` smoke.
- `timeout` exit code `124` is acceptable for the interactive smoke once the window opens and no runtime warnings indicate a game bug.
- Dependency direction notes live in `docs/dependency-decisions.md`; currently `iyes_progress 0.17.0` owns startup progress loading, `moonshine-kind 0.5.1` is used for typed harness roots, and `bevy_egui 0.40.1` is reserved for optional debug tooling, not shipped RTS menu/lobby UI.
