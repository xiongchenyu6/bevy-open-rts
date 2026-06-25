# Bevy Open RTS Structure

## Runtime Entry

- `src/main.rs` calls `bevy_open_rts::run_game_app()`.
- `run_game_app()` builds `build_game_app(GameAppMode::Interactive)`, which registers the setup menu and the shared match scene from `src/lib.rs`.

## Shared Match Scene

- `SharedMatchScenePlugin` owns the live RTS scene.
- `add_shared_match_scene()` registers match resources, `OnEnter(AppScreen::InMatch)` setup, runtime systems, and `OnExit` cleanup.
- `start_shared_match_scene_with_current_setup()` advances any app with the shared scene plugin into `AppScreen::InMatch`.
- `start_shared_match_scene_with_settings()` is the internal helper for capture/test apps that need to inject a specific `MatchSetupSettings` before entering the same live scene.
- The main menu uses `start_shared_match_from_menu_selection()` so `cargo run` and capture/test proofs enter through the same setup contract.
- Match start camera focus is derived from the selected player's actual base anchor: first the `MobileConstructionVehicle` spawn when present, then the `CommandCenter`. It no longer blends toward nearby resources, so `cargo run` opens over the player's base instead of a zoomed-out map work area.
- Resource nodes are left-click selectable for target confirmation, and selected harvesters use a wider resource-specific right-click snap so clicking the visible ore/crystal model edge still issues `HarvestOrder`.
- World overlays keep selected rings, resource/supply rings, active command markers, and the current placement footprint. Unselected structures and construction-range anchors no longer draw permanent rings.
- Runtime player state is data-driven from `ActiveTeams` / `MatchSetupSettings`: economies, team relations, AI timers, support cooldowns, HUD counts, and match-end checks grow by player index instead of assuming three hard-coded teams.
- `Team` is runtime identity only (`Player(index)` / `Neutral`). Playable faction identity comes from `PlayerFactions`, so 人族/魔族/混沌族 rules follow the configured player slot instead of being tied to Player0/1/2.
- Lobby team ids default to one independent team per spawn slot and cycle across the full lobby slot count, so 4/8-player maps are not folded into three activity teams.

## Capture And Proofs

- `src/bin/capture.rs` exposes proof commands for live match simulation.
- Capture snapshots use `CaptureTeam::Player(index)` and `players: Vec<CaptureTeamStats>` rather than fixed human/demon/chaos fields, so proof output can represent every runtime player row.
- `build_capture_match_app_for_faction()` uses `SharedMatchScenePlugin` plus `start_shared_match_scene_with_settings()`.
- Real menu proofs use `RealMenuMatchStart` plus the actual setup menu buttons before running proof logic. The same path can select faction, match mode, starting resources, and AI difficulty before entering the shared live scene.
- Current proof surfaces:
  - `real-match-proof`: direct production/order proof from a menu-started match.
  - `real-harvest-proof`: mouse select collector, right-click ore, harvest, then train.
  - `real-dual-harvest-proof`: low-resource menu start, mouse-order both Ore and Crystal gathering, and require both player resource totals to increase.
  - `real-supply-crate-proof`: real menu FourCorners Human start, mouse-select a movable unit, right-click a visible resource supply crate, consume it, and require Ore/Crystal rewards.
  - `real-tech-oil-proof`: real menu FourCorners start, satisfy EngineerDrone tech, train EngineerDrone, right-click a neutral TechOilDerrick, capture it, consume the engineer, and require the capture bonus.
  - `real-tech-oil-all-factions-proof`: batch CLI wrapper over the FourCorners tech-oil capture proof for every playable faction.
  - `real-ai-pressure-proof`: menu-started Hard AI match that requires AI production growth, AI attack orders, and real damage to the selected player's army/base.
  - `real-ai-pressure-all-factions-proof`: batch CLI wrapper over the Hard AI pressure proof for every playable faction.
  - `real-ai-vs-ai-proof`: real menu AI-vs-AI spectator mode selection that requires both AI sides to grow armies, at least one side to issue attack orders, and live combat damage in the shared match scene while reporting `mode=ai_vs_ai`.
  - `real-ai-vs-ai-all-factions-proof`: batch CLI wrapper over the AI-vs-AI spectator proof for every playable focus faction.
  - `real-build-proof`: mouse select worker, place and construct Barracks, then train.
  - `real-victory-proof`: mouse select VehicleFactory, train combat vehicles, right-click enemy anchors, and win.
  - `real-default-victory-proof`: default `cargo run` menu start, no setup changes or proof-side resource grant, train combat vehicles from the real command panel, right-click enemy anchors, and win.
  - `real-selected-faction-victory-proof`: real menu faction selection, default resources, no proof-side resource grant, train that faction's vehicle roster from the real command panel, right-click enemy anchors, and win.
  - `real-selected-map-victory-proof`: real menu map selection, default resources, no proof-side resource grant, train combat vehicles from the real command panel, right-click enemy anchors, and win while reporting the loaded map id.
  - `real-all-maps-victory-proof`: batch CLI wrapper over the selected-map real menu proof for every playable faction on every migrated skirmish map.
  - `real-allied-victory-proof`: real menu Allied 2v1 mode selection, default resources, no proof-side resource grant, train combat vehicles from the real command panel, right-click enemy anchors, and win while reporting the loaded match mode id.
  - `real-allied-all-factions-victory-proof`: batch CLI wrapper over the Allied 2v1 real menu proof for every playable faction.
  - `real-economy-victory-proof`: low-resource menu start, mouse harvest required resources, train combat vehicles, right-click enemy anchors, and win without proof-side resource injection.
  - `real-playable-proof`: low-resource menu start, mouse harvest both Ore and Crystal, build Barracks, train a Barracks unit, train combat vehicles, right-click enemy anchors, and win.
  - `real-three-faction-playable-proof`: low-resource real menu start, select three-faction skirmish mode, use the same playable proof flow with a larger attack group, and finish all enemy anchors.
  - `real-lobby-slots-proof`: real menu Big Arena setup that enables every map spawn slot and verifies all eight active player rows reach the live scene with economies, units, structures, command centers, and spawn anchors.
  - `real-playability-suite-proof`: top-level CLI regression gate that runs the real menu dual-harvest, supply-crate, tech-oil, playable, three-faction, all-map, Allied 2v1, AI pressure, and AI-vs-AI proof surfaces across all playable factions.

## Verification

- Use `cargo fmt`, `cargo check`, targeted `cargo test`, full `cargo test`, `cargo build`, and a short `cargo run` smoke.
- `timeout` exit code `124` is acceptable for the interactive smoke once the window opens and no runtime warnings indicate a game bug.
