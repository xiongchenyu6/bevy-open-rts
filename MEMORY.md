# Open Bevy Online Notes

- Bevy Open RTS uses Bevy 0.19 and runs its simulation in normal `Update`
  systems with floating-point transforms and timers. It is not deterministic
  enough for peer lockstep without a much larger simulation rewrite.
- Existing orders contain local Bevy `Entity` handles. Online commands need
  stable network IDs and host-side resolution/validation.
- `matchbox_socket` 0.14 supports native and wasm WebRTC with multiple reliable
  and unreliable channels. `bevy_matchbox` currently targets an older Bevy, so
  the project integrates `matchbox_socket` directly.
- Matchbox's standard wire format is `JsonPeerRequest`/`JsonPeerEvent` from
  `matchbox_protocol`. The custom Open Bevy server retains this format so the
  stock socket client remains compatible.
- The signaling container was built and run locally on 2026-07-24. Docker
  reported it healthy; `/healthz`, `/metrics`, and `/v1/config` were exercised,
  including dynamically issued Coturn REST credentials.
- Browser WebSocket clients cannot attach an Authorization header. Connection
  tickets therefore use query parameters; production proxies must use WSS and
  must not persist request URIs.
- `open-bevy-net` uses `ehttp` for one HTTP API on native/wasm without requiring
  a Tokio runtime in Bevy. Its Matchbox message-loop future is executor-neutral
  and can run on Bevy's task pool.
- A real native WebRTC integration test starts the custom server, creates a room
  through `RoomServiceClient`, connects two stock Matchbox sockets with host ICE
  candidates, and verifies both reliable and unreliable payload delivery.
- The signaling service keeps the host ticket valid and preserves connected
  players for a configurable 30-second host reconnect window. A resumed host
  receives a new Matchbox peer ID, so game-level identity must use its own
  stable session key rather than treating `PeerId` as player identity.
- `src/online.rs` now provides the first real game integration slice: public
  room discovery/create, code/token join, reconnect-aware player identity,
  exact map-derived slot rows, host-validated lobby options, readiness, and a
  synchronized transition into the existing match setup.
- Running online matches now have stable `NetworkEntityId` components and a
  first host-authoritative replication path. The host alone advances economy,
  build, AI, combat, death, and victory systems, then broadcasts 10 Hz world
  snapshots over the unreliable channel. Clients reconcile replicated entities,
  economies, and match state by stable ID and interpolate transform corrections.
- Snapshot receivers only accept the negotiated host, current RTS protocol, and
  monotonically newer ticks. A postcard roundtrip test keeps a representative
  eight-player/512-entity snapshot under the 64 KiB channel limit.
- Unit orders and structure rally points now use bounded, sequenced reliable
  commands containing stable network IDs. Host-local input uses the same inbox
  as remote input, and the host validates protocol, replay sequence, ownership,
  capabilities, target relations, liveness, and map bounds before mutating the
  authoritative simulation. A maximum 256-unit order batch fits the 256 KiB
  reliable-channel budget.
- Training, production cancellation, and structure placement now use bounded,
  sequenced reliable commands with stable producer and Worker IDs. The host
  validates ownership, faction production relationships, technology, queue
  capacity, resources, placement legality, and construction capability. Exact
  charged costs drive cancellation refunds, and authoritative build queues are
  mirrored in snapshots for client HUD progress.
- Remote human teams must never be inferred as AI merely because they are not
  the locally controlled team. Automatic construction now checks the configured
  slot controller explicitly, so only real AI slots progress foundations
  without an assigned Worker.
- RTS wire protocol v2 is command-complete for current player-facing match
  controls. Stop/hold/guard/scatter/deploy, sell/repair/cancel-construction, and
  all support powers use the reliable host inbox. The host validates stable-ID
  ownership and capabilities, prevents same-tick duplicate structure refunds or
  repair charges, validates support doctrine/tech/power/cooldown/bounds, and
  mirrors support cooldowns in snapshots. Maximum action packets and a
  representative cooldown-bearing snapshot remain below channel limits.
- The running match still needs in-match reconnect/catch-up, a global
  multiplayer victory/disconnect/return-to-lobby contract, and visual event
  replication for transient support warnings/impacts. Large battles also need
  delta/compressed snapshots rather than relying only on full state.
