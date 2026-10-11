# Open Bevy Online Plan

## Steam quality work (active)

The acceptance matrix is in `docs/RELEASE_READINESS.md`; the full Steam goal
remains open. Current evidence must match the source/artifact being claimed.

- [x] Boot Camp entry, skip, seven lessons, graduation and fresh replay;
  native sandbox persistence/restart verified separately.
- [x] Remaining missions3/5 now win through ordinary input and continue.
- [x] Six missions complete in pinned separate-process sequence; exact growing
  progress prefix and fresh-process all-unlocked menu verified.
- [ ] Same-App campaign/retry audit and revalidation after current AI changes.
- [x] Keyboard and mouse save/recovery in skirmish, including missing/corrupt files.
- [ ] Campaign-runtime save continuity, full UI/browser and audio review.
- [ ] Release delivery, balance/performance, provenance and store-media gates.

The b416 construction checkpoint passed168 library tests, fmt/check/bins build and a12-second
native launch with no error output. Proof is `screenshots/result/15`:450frames,
15seconds/30fps, inspected opening/movement/combat/ending. Unit motion4.24m,
siege113damage and94effectframes are recorded in its log.

An exact alive-worker AA construction stall was reduced to a65-cell routing
pocket caused by treating nearby mobile units as fixed barriers. Preferred
unit-aware routing now falls back to worker-sized static routing; endpoint
occupancy and normal movement collisions remain enforced. Six construction
regressions cover walking/yielding, physical clearance, speed bounds and cost.
The same AA(-22,-13) completes through normal input; Mission3 wins3:03/HQ20,
Mission5 wins4:17/HQ20/no building losses. Pin d569 subsequently completes all
six with persisted prefixes; same-App/restart and new AI behavior remain pending.

Mouse Save/Load and Main Menu return pass1280x720/900x720. Fresh-process first-
wave campaign Load also restores flags, paid queue, resources and fog without
wave duplication. Broader orders/mines/effects/support runtime coverage remains.

Standard Linux fat LTO failed twice with OOM; logs are preserved. The separately
labeled thin-LTO candidate builds with full features. Its exact offline archive
passes no-Nix/read-only AMD startup and real Enter/F10/select/right-move input.
The online-default candidate finished11m22s/exit0 and its exact Linux package
passes AMD read-only/no-Nix input checks. Tagged Windows debug refresh passes
Wine menu, mouse save/load and fresh-process restoration; native Windows remains
unverified. Both packages predate the current AI construction changes.

Six authored audio mismatches now regenerate deterministically and pass actual
Bevy loading/decoding; human listening remains open. Packagers use PLAYER_GUIDE
as README and retain BUILD_NOTES/inventory/licenses. Remaining art/voice provider
records are explicit in ASSET_PROVENANCE. User flake.lock/full font are untouched.

## Objective

Build one reusable WebRTC signaling and session platform for the Open Bevy game
family, then use it to provide real online multiplayer in Bevy Open RTS on both
native desktop and browser builds.

## Architecture Decisions

- The signaling service and wire protocol must not depend on Bevy or RTS code.
- The universal `session_protocol` is service-owned; every room also carries an
  arbitrary non-zero `game_protocol`, so different games and incompatible game
  packet revisions are isolated without changing the signaling server.
- WebRTC peers use Matchbox's native/wasm socket implementation, but the server
  owns room admission, discovery, isolation, observability, and lifecycle.
- RTS networking is host-authoritative. Clients submit validated player
  commands; the host runs the simulation and broadcasts state snapshots.
- Reliable WebRTC data is used for lobby/control/commands. Unreliable ordered-
  independent data is used for frequent world snapshots.
- Runtime entities receive stable network IDs. Bevy `Entity` values never cross
  the network.
- Production deployment uses Cloudflare Managed TURN with short-lived
  credentials. A signaling server alone cannot connect every NAT/firewall
  combination.

## Milestones

- [x] Universal signaling platform
  - [x] Versioned protocol crate and validation
  - [x] Room create/discover/join API
  - [x] Matchbox-compatible WebSocket signaling
  - [x] Room isolation, capacity, host ownership, cleanup, metrics
  - [x] Container, Cloudflare Managed TURN credentials, CI, integration tests
  - [x] Native container and Cloudflare Worker/Durable Objects deployments share
        the same API, room rules, and WebRTC verification harness
- [x] Shared native/wasm client transport
  - [x] Reliable and unreliable channels
  - [x] Connection lifecycle and structured errors
  - [x] Signaling-level host reconnect/resume contract
  - [x] Stable game-session identity across signaling peer changes
- [x] Bevy Open RTS online lobby
  - [x] Online menu, public room discovery/create, room code/token join
  - [x] Exact map slot rows with readiness/faction/team/color synchronization
  - [x] Host-validated settings and synchronized match launch
- [x] Host-authoritative match
  - [x] Stable network entity IDs
  - [x] Host world snapshots, client reconciliation, and interpolation
  - [x] Serializable high-level commands
    - [x] Unit orders and structure rally points
    - [x] Training, construction, and queue management
    - [x] Remaining command-card actions and support powers
  - [x] Command validation and ownership checks
    - [x] Unit/rally ownership, capability, target, bounds, and replay validation
    - [x] Economy, technology, and production validation
    - [x] Support-power validation
  - [x] Snapshot delta/compression for battles beyond the full-snapshot budget
  - [x] Sequenced transient combat/support visual events with client deduplication
  - [x] Match-state reconnect/resume through the next authoritative snapshot
  - [x] Match completion and session lifecycle
    - [x] Global multiplayer victory with per-player result presentation
    - [x] Disconnect grace, reconnect cancellation, and authoritative forfeit
    - [x] Synchronized return-to-lobby flow
- [x] Product verification
  - [x] Two native clients
  - [x] Two browser clients
  - [x] Native-to-browser client
  - [x] Internet path through TURN
  - [x] Deployment and operational runbook

## Verification Gates

- `cargo run` still starts the native RTS game.
- `cargo test -p open-bevy-protocol -p open-bevy-net -p open-bevy-signaling`
  passes.
- `cargo test -p bevy-open-rts online::tests --lib` passes.
- Signaling integration tests prove room isolation, signal relay, capacity, and
  disconnect cleanup using real WebSocket clients.
- Two game clients can create/join a lobby and complete a match while observing
  the same authoritative result.
- The GitHub Pages build connects through the deployed `wss://` endpoint.

## Battlefield Art Revision (verified)

Goal: improve art presentation so the battlefield reads as military machinery
and terrain rather than toys.

- [x] Identify initial issues in existing base captures: smooth ground and white panels.
- [x] Verify layered soil detail in `screenshots/art-revision/base.png`.
- [x] Verify muted metal armor, controlled team accents and lighting across all three factions.
- [x] Refine roof fittings, armor wear, palette, sky fill and soil detail from captures.
- [x] Formatting, build/check, 137 library tests, native launch and faction gameplay checks pass.
- [x] Inspect `screenshots/result/7`: 450 raw frames and a 15-second, 30-fps video.

Evidence: `logs/art-tests-fixed.log`, `logs/art-build-checked.log`,
`logs/art-check-fixed.log`, `logs/art-final-factions.log`, and
`logs/art-proof-verified.log`. Native launch ran on Wayland for 12 seconds
without error output; final capture verifies 4.24m movement and 113 siege damage.

## Compact selection markers

- [x] Separate visual selection size from click/nav radius.
- [x] Tighten worker/infantry markers; replace full circles with four short arcs.
- [x] Remove the duplicate aircraft ground ring and reduce small-unit health-bar width.
- [x] Verify selected worker, movement/harvest input and battle footage (`screenshots/selection-compact`, `screenshots/result/8`).
- [x] Formatting, type/build checks and 2 existing selection tests pass.

## Current Steam acceptance continuation (2026-10-11)

- [x] Verify Wasm16 host-only eight-slot lobby footer at1280×720 with normal wheel input; raw evidence hashes and reverse-scroll frame independently reviewed.
- [x] Complete exact SDK267 normal Mission1 victory → normal exit → fresh campaign-menu persistence in one successful workflow; proof `docs/qa/current267-sdk-mission1-normal-review.json`, prior workflow failure stays recorded.
- [ ] Complete six missions through normal menu/input in one final-package application, including Retry/Next/finale and fresh-process final menu.
- [ ] Verify actual moving guest ECS and active paid jobs, eight real peers, and reconnect.
- [ ] Close all nine gates in `docs/RELEASE_READINESS.md`; bounded checks above do not close those gates.

- [ ] Resolve current267 Big Arena cases2641/2668, preserving original600sim/180wall limits and retained failures.
- [ ] Verify result-tooltip cleanup in full suite, fresh rendered victory/defeat and final package; focused real-App regression passes.

- [ ] Verify persisted player preferences in a new optimized package through normal Options clicks and fresh restart; native disk regressions pass, startup initialization/consolidated checks pending.
- [ ] Repeat isolated audio channel acceptance with per-phase recording boundaries; first semantic-stop run and actual fresh-process volume reset are retained.

- [x] Build/check fullnative279 checkpoint and officialSDK optimized archive with immutable743-file snapshot and integrity0.
- [x] Verify exact2668 afterAI focus and exact2641 on optimizedsame-core SDK under original600/180; retaineddebug failures remain.
- [x] Verify normalSDK267 Mission2 victory, two normal exits and fresh-process Mission3 unlock; separate fromsixsameApp.
- [x] Verify Wasm16 twoHuman motion and activepaidHMG jobs through actualguestECS; 8peers/reconnect remain.
- [ ] Fix actualnormalMission3 construction-contact stall using originalWorker/Drone/terrain and preserved30s failure.

- [x] Exactnative279 privateUbuntu channel-volume/restart acceptance: five normal exits,10closedPCMchecks/all5Optionsdirectreview, no clipping/hostcapture, unchanged archive. Full officialRuntime4/audiohumanreview remain.
- [ ] Verify both navigation fixes through full suite and a new optimizedpackage normalMission3; actualsaved-state REDs retained.
