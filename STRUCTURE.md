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

## Capture And Proofs

- `src/bin/capture.rs` exposes proof commands for live match simulation.
- `build_capture_match_app_for_faction()` uses `SharedMatchScenePlugin` plus `start_shared_match_scene_with_settings()`.
- Real menu proofs use `build_real_menu_match_app_for_faction_with_ai()`, which drives the actual setup menu buttons before running proof logic.
- Current proof surfaces:
  - `real-match-proof`: direct production/order proof from a menu-started match.
  - `real-harvest-proof`: mouse select collector, right-click ore, harvest, then train.
  - `real-build-proof`: mouse select worker, place and construct Barracks, then train.
  - `real-victory-proof`: mouse select VehicleFactory, train combat vehicles, right-click enemy anchors, and win.
  - `real-economy-victory-proof`: low-resource menu start, mouse harvest required resources, train combat vehicles, right-click enemy anchors, and win without proof-side resource injection.
  - `real-playable-proof`: low-resource menu start, mouse harvest, build Barracks, train a Barracks unit, train combat vehicles, right-click enemy anchors, and win.

## Verification

- Use `cargo fmt`, `cargo check`, targeted `cargo test`, full `cargo test`, `cargo build`, and a short `cargo run` smoke.
- `timeout` exit code `124` is acceptable for the interactive smoke once the window opens and no runtime warnings indicate a game bug.
