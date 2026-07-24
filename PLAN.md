# Open Bevy Online Plan

## Objective

Build one reusable WebRTC signaling and session platform for the Open Bevy game
family, then use it to provide real online multiplayer in Bevy Open RTS on both
native desktop and browser builds.

## Architecture Decisions

- The signaling service and wire protocol must not depend on Bevy or RTS code.
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
- [ ] Shared native/wasm client transport
  - [ ] Reliable and unreliable channels
  - [ ] Connection lifecycle and structured errors
  - [ ] Reconnect/session-resume contract
- [ ] Bevy Open RTS online lobby
  - [ ] Online menu, create room, room code join
  - [ ] Player readiness, slot/faction/team/color synchronization
  - [ ] Host-controlled match launch
- [ ] Host-authoritative match
  - [ ] Stable network entity IDs
  - [ ] Serializable high-level commands
  - [ ] Command validation and ownership checks
  - [ ] Snapshot/delta replication and interpolation
  - [ ] Victory, disconnect, reconnect, and return-to-lobby flow
- [ ] Product verification
  - [ ] Two native clients
  - [ ] Two browser clients
  - [ ] Native-to-browser client
  - [ ] Internet path through TURN
  - [ ] Deployment and operational runbook

## Verification Gates

- `cargo run` still starts the native RTS game.
- `cargo test -p open-bevy-protocol -p open-bevy-signaling` passes.
- Signaling integration tests prove room isolation, signal relay, capacity, and
  disconnect cleanup using real WebSocket clients.
- Two game clients can create/join a lobby and complete a match while observing
  the same authoritative result.
- The GitHub Pages build connects through the deployed `wss://` endpoint.
