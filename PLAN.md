# Open Bevy Online Plan

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
- Production deployment includes TURN. A signaling server alone cannot connect
  every NAT/firewall combination.

## Milestones

- [x] Universal signaling platform
  - [x] Versioned protocol crate and validation
  - [x] Room create/discover/join API
  - [x] Matchbox-compatible WebSocket signaling
  - [x] Room isolation, capacity, host ownership, cleanup, metrics
  - [x] Container, TURN REST credentials, CI, integration tests
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
- [ ] Product verification
  - [x] Two native clients
  - [x] Two browser clients
  - [x] Native-to-browser client
  - [ ] Internet path through TURN
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
