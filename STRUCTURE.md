# Bevy Open RTS Structure

## Runtime Entry

- `src/main.rs` calls `bevy_open_rts::run_game_app()`.
- `run_game_app()` builds `build_game_app(GameAppMode::Interactive)`, which registers the Godot-style front menu, options/credits screens, setup menu, and the shared match scene from `src/lib.rs`.

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
- AI drones have an active scouting controller: idle AI `Drone` units pick living enemy units, move to their positions, avoid repeating the previous target when possible, and retarget after a short 0.5-1.0s delay.
- AI defense profiles follow the godot difficulty targets: Beginner/Easy do not inherit Normal advanced-defense construction, Normal targets one standard defense layer plus 2 Tesla fence segments where the faction supports them, and Hard scales standard defenses to 2 plus 4 Tesla fence segments.
- Easy AI is tuned as a build-up opponent: it trains a small defensive force but does not launch active attack waves, giving default human starts enough time to build a Barracks and form an army. Normal/Hard keep active offense.
- The minimal `GodotSkirmish` opening remains one `CommandCenter` plus two `Worker` economy units, but each faction gets a distinct starter combat/scout unit (`ScoutRover`, `RocketInfantry`, `ShieldTrooper`) so the default `cargo run` start is not visually identical across races.
- Godot render-part mapping is audited separately from gameplay generation. `assets/data/godot_model_map.model_map.ron` is a Bevy-loadable baseline asset generated from Godot `*.tscn` scenes, and `scripts/audit_model_mapping.py` compares it against `src/generated_registry.rs` without regenerating or overwriting the hand-expanded registry.
- `Worker` and `ScoutRover` intentionally share Godot's `rover.glb`; their one-to-one fidelity comes from separate Godot transforms (`Worker` scale 2.0, `ScoutRover` scale 1.65). Harvest sparkle/cargo motion is a runtime behavior/effect layer, not a different static model file.

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
- The old `real-*-proof` CLI surfaces were removed; do not use them as completion gates.

## Verification

- Use `cargo fmt`, `cargo check`, targeted `cargo test`, full `cargo test`, `cargo build`, and a short `cargo run` smoke.
- `timeout` exit code `124` is acceptable for the interactive smoke once the window opens and no runtime warnings indicate a game bug.
