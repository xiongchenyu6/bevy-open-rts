# Bevy Open RTS Structure

## Workspace

The repository is a Cargo workspace whose default member remains the root
`bevy-open-rts` game, so plain `cargo run` keeps its existing behavior.

- `crates/open-bevy-protocol` owns game-independent room/API contracts and
  identifier validation. It has no Bevy dependency. Its `session_protocol`
  versions the universal service API, while each room's `game_protocol` is an
  opaque, non-zero game-owned namespace component.
- `crates/open-bevy-net` owns the native/wasm HTTP room client and the two-channel
  Matchbox WebRTC transport. It has no Bevy dependency; games poll transport
  events and run its message-loop future on their engine task pool. It does not
  contain a default game ID; each consuming game supplies its own ID and game
  protocol.
- `services/open-bevy-signaling` owns the reusable Open Bevy room service and a
  Matchbox-compatible WebSocket endpoint. It has no dependency on the RTS game.
- `deploy/signaling` contains the production container, Caddy, and Coturn
  example. TURN credentials are generated with the Coturn REST HMAC contract.
- `docs/ONLINE_ARCHITECTURE.md` is the online wire/deployment contract;
  `PLAN.md` tracks the remaining client and gameplay replication milestones.

## Source Modules

`src/lib.rs` is the crate root: app composition/plugin registration, the match
setup/flow (maps, factions, spawn, fog/visibility) and shared core types. The
domains live in modules, each re-exported into the crate root
(`pub(crate) use module::*;`), so items are referenced unqualified:

- `src/nav.rs` — A* nav grid + PlannedPath, unit-vs-unit separation.
- `src/maps.rs` — skirmish map catalog: SkirmishMapDef + per-map data tables,
  MapBounds, random-map selection, terrain-wall specs + spawning.
- `src/support_powers.rs` — support powers (F1–F9): kinds/cooldowns, activation
  + targeting, HUD panel/buttons/tooltips, AI power usage.
- `src/match_tests.rs` — headless match-flow integration tests (cfg(test)).
- `src/spawn.rs` — entity spawning: team startup, unit/structure spawn entry
  points, model assembly, placement wrappers, paradrop spawning.
- `src/selection.rs` — click/box select, control groups, Tab subgroups,
  idle-worker cycling, pointer→terrain helpers.
- `src/command_card.rs` — command-card action grid: labels/icons/hotkeys/
  tooltips/availability, shortcuts, action execution, queue buttons.
- `src/fog.rs` — fog of war: visibility state, shroud overlay, fog memory.
- `src/match_screens.rs` — match briefing card + match-end overlay.
- `src/placement_ghost.rs` — structure placement preview: a translucent green/
  red ghost of the actual building follows the cursor before you commit.
- `src/combat_vfx.rs` — shot pulses, impact bursts, wreckage, structure
  destruction, veterancy promotion effects.
- `src/camera.rs` — RtsCamera state, bevy_rts_camera bridge, settings, bookmarks.
- `src/audio.rs` — unit/announcer voices, sfx kinds, AudioFeedback queue + playback.
- `src/menu.rs` — front menu, options, credits, skirmish lobby + widgets.
- `src/hud.rs` — in-match HUD: resource bar, minimap, battle log, objectives,
  selection panel, command card + queue, support strip, HudHitZones, RTS cursor.
- `src/online.rs` — RTS online lobby/session protocol, stable network entity IDs,
  reliable player and production commands, host validation, authoritative world
  and build-queue snapshots, client reconciliation/interpolation, reconnect
  grace/forfeit handling, and synchronized rematch-lobby lifecycle.
- `src/economy.rs` — Economies/income/power, harvesting + dropoff, resource nodes,
  supply crates.
- `src/ai.rs` — difficulty tiers, AI director (economy/training/waves/support),
  in-match AI systems (repair/scouting/garrisons/capture/saboteur), profiles,
  target scoring, support-power defs.
- `src/production.rs` — build queue/training, construction, manual placement.
- `src/orders.rs` — unit orders + order queue, rally points, targeting modes,
  per-order-type update systems.
- `src/combat.rs` — combat/chase/movement systems, weapons/health, crushing,
  mines, wreckage, veterancy, auras/shields/EMP, combat VFX.
- `src/terrain.rs` — real elevation: per-map plateau/ramp specs sampled into a
  TerrainHeightField (bilinear heights, cliff step legality, cursor ray-march,
  generated ground mesh, spawn settling).
- `src/save.rs` — RON save/load (quicksave Ctrl+S / quickload Ctrl+L), replay
  timeline keyframes + PageUp/PageDown jumps.
- `src/campaign.rs` — mission table (victory conditions, timed triggers),
  campaign menu state, mission victory/trigger systems.
- `src/capture_api.rs` — the `pub fn capture_*` harness surface + offscreen
  render-target plumbing (used by `src/bin/capture.rs` and tests).
- `src/generated_registry.rs` — entity registry (hand-extended; do NOT regenerate
  via scripts/generate_registry.py — it would delete the extensions).

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
- Ground movement now combines a rebuilt `NavGrid` A* path with lightweight unit separation. Units plan around static buildings/resources instead of relying on only straight-line steering, and nearby mobile units apply a small boids-style push so squads do not collapse into one stacked blob.
- Group move, attack-move, patrol, follow, and minimap commands use the Godot `UnitMovementUtils` formation contract: terrain and air units keep independent AABB-centered relative formations, overly loose groups condense without overlapping destination discs, and the full formation shifts inside map bounds instead of sending edge units off-map.
- Shift right-click queues waypoints for selected units when they already have an active order or queued orders. Plain right-click still replaces the current order.
- Rally structures support both normal move rally points and attack-move rally points. Using attack-move from a selected production/rally structure sets an attack rally; newly produced armed mobile units spawn with `AttackMoveOrder`, while unarmed workers still use a normal move rally.
- The command panel exposes an idle Worker action (`I` / `Alt+I`) when the player has idle construction Workers. It cycles to the next idle Worker, selects it, and focuses the camera without selecting the whole worker group.
- World overlays keep selected rings, resource/supply rings, active command markers, and the current placement footprint. Unselected structures and construction-range anchors no longer draw permanent rings.
- Support powers render in the Godot-style top-right strip but only after their required tech structure is completed. Locked powers are hidden, unlocked powers keep their hotkeys/tooltips, cooldown countdowns stay visible, and HUD hit testing shrinks to the visible buttons so empty right-top space does not block camera/world input.
- Runtime player state is data-driven from `ActiveTeams` / `MatchSetupSettings`: economies, team relations, AI timers, support cooldowns, HUD counts, and match-end checks grow by player index instead of assuming three hard-coded teams.
- `Team` is runtime identity only (`Player(index)` / `Neutral`). Playable faction identity comes from `PlayerFactions`, so 人族/魔族/混沌族 rules follow the configured player slot instead of being tied to Player0/1/2.
- Lobby team buttons remain an 8-row setup UI concern, but runtime team IDs are stored and derived as unbounded `usize` values. The battle core no longer clamps alliances to three teams or to the current lobby button count.
- Runtime spawning is not capped to the map's authored spawn-point count. Players beyond the map rows receive clamped virtual fallback base positions instead of being skipped.
- AI/runtime fallback helpers are not capped to the lobby slot count: active AI iteration, opponent helpers, late-slot resources, cooldowns, fallback home positions, virtual spawn positions, runtime team relation IDs, and battle AI participation are verified beyond eight players.
- Runtime entities receive stable `NetworkEntityId` components. Dynamic units
  and structures use the reset-per-match spawn sequence; authored resources and
  supply crates use disjoint map-index namespaces. Local Bevy `Entity` handles
  are never serialized.
- Online matches run the build, economy, AI, combat, death, and victory systems
  only on the host. Clients reconcile units, structures, resources, crates,
  economies, and match state by stable ID and interpolate short transform
  corrections.
- RTS wire protocol v4 sends one compressed full keyframe per second and 10 Hz
  deltas against that keyframe. Deltas contain changed/new entities, explicit
  removals, current global match state, and sequenced transient effects. They
  never chain from another unreliable delta, so one dropped packet cannot
  corrupt later state. `open-bevy-net` owns the reusable versioned LZ4 envelope,
  enforces a 64 KiB wire limit and a 4 MiB pre-allocation decode limit, and is
  shared unchanged by native and wasm clients. Tests prove a compressible
  2,048-entity keyframe whose raw postcard form exceeds the channel budget.
- Unit orders and structure rally points cross the reliable channel as bounded,
  sequenced commands that contain only stable network IDs. The host rejects
  replayed commands, invalid targets, unsupported orders, out-of-bounds points,
  and attempts to control another player's entities; host-local input follows
  the same validation path.
- Training, production cancellation, and structure placement also cross the
  reliable channel with stable producer/Worker IDs. The host validates player
  ownership, faction production relationships, technology, queue capacity,
  resources, placement bounds/collisions, and construction capability before
  mutating the match. Exact charged costs are retained for authoritative
  refunds, and build queues are mirrored in world snapshots so client HUDs show
  host-owned queue state and progress. Remote human teams no longer inherit AI
  automatic construction.
- RTS wire protocol v4 sends stop/hold/guard/scatter/deploy, structure
  sell/repair/cancel, and support-power targeting over the reliable command
  channel. The host resolves stable IDs, rejects foreign/dead/unsupported
  entities, enforces one structure mutation per host tick, and validates support
  faction doctrine, technology, power, cooldown, and map bounds. Support
  cooldown and initial-charge arrays are mirrored in snapshots; clients never
  advance them independently. Snapshots also carry active anchor teams and the
  authoritative finished flag, so an eliminated host cannot end a battle while
  hostile remote teams still fight and every client derives victory/defeat from
  its own alliance perspective. A reconnecting client remains in the running
  match and catches up from the next host snapshot. Protocol v4 gives remote
  humans a 30-second reconnect grace, turns expiry into a host-authoritative
  forfeit, rejects late/forfeited identities, and synchronizes return to a reset
  rematch lobby. Host-sequenced shot, impact, support-warning, structure-death,
  and promotion effects are repeated while alive and deduplicated by clients.
  The next online boundary is full multi-client product verification.
- `bevy_fluent::FluentPlugin` is registered in the shared game scene so future `.ftl` localization bundles can load through Bevy assets. The existing `Locale` / `t()` path remains the active text source until screens are migrated incrementally.
- AI drones have an active scouting controller: idle AI `Drone` units pick living enemy units, move to their positions, avoid repeating the previous target when possible, and retarget after a short 0.5-1.0s delay.
- AI defense profiles follow the godot difficulty targets: Beginner/Easy do not inherit Normal advanced-defense construction, Normal targets one standard defense layer plus 2 Tesla fence segments where the faction supports them, and Hard scales standard defenses to 2 plus 4 Tesla fence segments.
- Easy AI is tuned as a build-up opponent: it trains a small defensive force but does not launch active attack waves, giving default human starts enough time to build a Barracks and form an army. Normal/Hard keep active offense.
- The minimal `GodotSkirmish` opening remains one `CommandCenter` plus two `Worker` economy units, but each faction gets a distinct starter combat/scout unit (`ScoutRover`, `RocketInfantry`, `ShieldTrooper`) so the default `cargo run` start is not visually identical across races.
- Godot render-part mapping is audited separately from gameplay generation. `assets/data/godot_model_map.model_map.ron` is a Bevy-loadable baseline asset generated from Godot `*.tscn` scenes, and `scripts/audit_model_mapping.py` compares it against `src/generated_registry.rs` without regenerating or overwriting the hand-expanded registry. The baseline is reference data, not permission to keep poor Bevy silhouettes.
- `Worker` now uses a distinct field-engineer model composition (`astronautB` plus equipment) instead of sharing `ScoutRover`'s `rover.glb`. Critical unit silhouettes are protected by tests and by `scripts/audit_model_quality.py`.
- `scripts/audit_model_quality.py --fail-critical --require-screenshots` is the model quality gate: it fails missing models, critical shared silhouettes, duplicate unit model signatures, and missing model-harness coverage. It also writes `docs/model-quality/hunyuan3d-queue.json`, a machine-readable Hunyuan3D replacement queue for multipart/kitbashed units with harness page/cell references and generation prompts.
- `scripts/comfy_hunyuan3d_queue.py` stages reference images and API workflows for remote ComfyUI/Hunyuan3D runs. `CryoSprayer`, `FlakHoverTank`, `FlakRocketTeam`, `FlakRocketTeamMk2`, `FlameAssaultBuggy`, `HammerSiegeTank`, `HeavySiegeWalker`, `LanceBeamTank`, `LongbowMissileCrawler`, `MobileShieldProjector`, `ModularMissileCarrier`, `RailArtilleryWalker`, `RailgunTank`, and `TeslaCrawlerMk2` are completed single-GLB replacements mapped under `assets/models/hunyuan3d/`; the quality gate now leaves no multipart unit replacements in the queue.
- Worker harvesting now has runtime VFX parity for the important Godot cues: collecting emits front sparkles and resource-to-worker pulses, carried ore/crystal draws visible cargo dots on the rover, and dropoff clears the cargo through the existing `ResourceCargo` flow.
- Weapon hits now spawn short-lived `ImpactBurst` overlays that scale by applied damage, target radius, and structure hits so combat feedback is visible beyond the unit health bars.
- Command/select voice acknowledgments use the existing `UnitVoiceEvent` pipeline; the default audio feature enables both `bevy/wav` for UI/SFX and `bevy/vorbis` for the `.ogg` unit and announcer voice lines.
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
