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
- The universal signaling API and game packet compatibility are separate
  version domains. `session_protocol` must match the service contract;
  `game_protocol` is any non-zero game-owned value used in room discovery and
  paths. `open-bevy-net` therefore must not hardcode an RTS game ID.
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
- RTS wire protocol v3 is command-complete for current player-facing match
  controls. Stop/hold/guard/scatter/deploy, sell/repair/cancel-construction, and
  all support powers use the reliable host inbox. The host validates stable-ID
  ownership and capabilities, prevents same-tick duplicate structure refunds or
  repair charges, validates support doctrine/tech/power/cooldown/bounds, and
  mirrors support cooldowns in snapshots. Maximum action packets and a
  representative cooldown-bearing snapshot remain below channel limits.
- A reconnect Welcome no longer sends a client back to the lobby during a live
  match; the client keeps `InMatch` state and catches up from the next accepted
  authoritative snapshot. Snapshots carry active anchor teams and a finished
  flag. The host waits until no hostile sides remain, while each client derives
  its own victory or defeat from alliance perspective, so eliminating the host
  does not prematurely stop remaining opponents.
- Remote players receive a 30-second game-level reconnect grace after their data
  channel disconnects. Rejoining with the stable session key cancels the timer;
  expiry permanently forfeits that player for the current match, removes its
  match-scoped entities and pending paradrops, and clears its production queue.
  Unknown late joiners and already-forfeited identities are rejected while the
  match is running.
- Returning to the online lobby is host-authoritative. The host can abort to the
  lobby, while client requests are accepted only after the global result. The
  host broadcasts a reset lobby snapshot, retains connected identities, resets
  readiness, and reclaims disconnected/forfeited player rows. Online restart and
  client-side speed changes are disabled to prevent divergent local state.
- RTS protocol v4 replaces raw full-state broadcasting with a one-second
  compressed keyframe plus independently recoverable 10 Hz deltas. Every delta
  references the latest full keyframe rather than the previous unreliable
  packet, carries changed/new entities and removals, and is ignored until the
  exact keyframe exists. The reusable `open-bevy-net` snapshot envelope applies
  LZ4 only when smaller and validates the declared decoded size before
  allocation. A 2,048-entity test exceeds 64 KiB before compression and fits the
  data channel after encoding.
- Shot pulses, impact bursts, support warnings, structure destruction, and
  veterancy promotions now receive host event IDs, remain present in snapshots
  while alive, and are deduplicated by clients. This gives dropped unreliable
  packets another chance to carry the visual event without replaying it when
  multiple packets arrive.
- The game now has an opt-in two-client product verification path rather than a
  signaling-only smoke test. A run-tagged host and player use the real public
  room discovery, lobby readiness, match transition, reliable unit-order
  validation, authoritative snapshots, and victory flow. Native clients write
  atomic JSON reports; browser clients expose the same report in a hidden DOM
  element for Playwright. A 2026-07-25 native run completed room `B7116009` at
  snapshot tick 14 with host victory and player defeat.
- Client admission cannot rely on a one-shot `Hello`/`Welcome` exchange. A real
  two-process run showed the host admitting the player while the first Welcome
  was not observed by the client. Clients now retain the candidate host peer
  and retry the same stable-session Hello once per second until admission;
  host admission is idempotent through the existing resume-key path.
- Directly launching `target/debug/bevy-open-rts` does not inherit Cargo's
  `CARGO_MANIFEST_DIR`, so Bevy otherwise looks under `target/debug/assets`.
  Native verification sets `BEVY_ASSET_ROOT` explicitly; normal `cargo run`
  behavior is unchanged.
- Product verification now uses the same role-driven browser harness for both
  the two-browser CI gate and native/browser compatibility. A deployed-Pages
  browser player and a locally built native host completed run
  `cross-1784925664-1332` in room `366BE938`: both reported two connected humans,
  snapshot ticks 10/9, the browser's reliable move command was observed on both
  sides, and the authoritative results were browser defeat/native victory.
  Browser startup is retried only before entering a room because the live Pages
  path can transiently expose the unsupported/loading fallback; match failures
  are never retried. Terminal reports are frozen on first publication so peer
  shutdown cannot rewrite the player count after successful completion.
