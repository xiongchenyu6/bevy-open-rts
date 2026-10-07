# Open Bevy Online Plan

## Steam quality work (active)

The full acceptance matrix is in `docs/RELEASE_READINESS.md`. It remains open:
green library tests and the earlier art pass do not establish release readiness.

- [ ] Verify Boot Camp from menu entry through graduation with normal inputs.
  Seven lessons pass in `logs/steam-boot-camp-current.log`, with screenshots in
  `screenshots/steam-quality/boot-camp`. Skip/replay and native profile remain.
- [ ] Verify all six campaign missions, persistent completion and continuation.
- [ ] Verify save continuity, corruption recovery and player-facing feedback.
- [ ] Complete desktop delivery, balance/performance, art/audio and provenance gates.

Current verification: 160 library tests pass; formatting, type/build checks and
12-second native launch pass with no error output. Boot Camp passes with actual
profile writes; current missions 1, 4 and 6 pass normal-input capture and
continuation. Mission 2's former infantry strategy passed, but the current
factory recipe hit an unfinished foundation; missions 3 and 5 still lose.
These failures remain open, not accepted as release-ready behavior.

Compact icon cards and disabled tooltips/queues are verified at 1280x720 and
900x720. Linux development packaging launches without /nix from /tmp on a
read-only install; release builds, Steam Runtime, hardware and Windows remain.
See `docs/ASSET_PROVENANCE.md` for exact source hashes and outstanding families.

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
