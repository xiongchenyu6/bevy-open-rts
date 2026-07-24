# Open Bevy Online Architecture

## Scope

`open-bevy-signaling` is a reusable signaling and room service for every game in
the Open Bevy family. It has no Bevy or Bevy Open RTS dependency. Games identify
themselves with a stable `game_id`, identify incompatible binaries with a
`build_id`, and version their game-level wire protocol independently.

The service is compatible with `matchbox_socket` 0.14 on native and wasm. It
only carries WebRTC session descriptions and ICE candidates; game packets flow
peer-to-peer after the data channels connect.

`open-bevy-net` is the corresponding reusable client. It exposes the room HTTP
API through `ehttp`, builds safely encoded connection URLs, chooses issued TURN
credentials when available, and fixes channel 0 as reliable control/commands
and channel 1 as low-latency unreliable snapshots. Its snapshot codec is an
opaque, versioned envelope usable by any Open Bevy game: payloads are compressed
with LZ4 only when that is smaller, wire packets are capped at 64 KiB, and the
declared decoded size is rejected before allocation above 4 MiB.

## Namespacing

Rooms are isolated by this tuple:

```text
(game_id, game_protocol, room_code)
```

`session_protocol` versions the universal room/signaling API and is validated
by the service. `game_protocol` is owned by each game, may be any non-zero
value, and isolates incompatible game packet formats without requiring a
signaling-server release.

The signaling URL is:

```text
wss://signal.example.com/v1/signal/{game_id}/{game_protocol}/{room_code}
  ?name={player_name}
  &role={host|player|spectator}
  &build_id={build_id}
  &ticket={optional_ticket}
```

The first connection must be the room host and must present the host resume
ticket returned by room creation. Private rooms additionally require their join
token. A build mismatch is rejected before WebSocket upgrade. The host keeps
this ticket for the lifetime of the room so a transient disconnect can resume
the same room during the configured grace window.

Do not log query strings at the reverse proxy: browser WebSocket APIs cannot set
an Authorization header, so short-lived connection credentials travel in the
query. Always expose the endpoint over TLS (`wss://`).

## HTTP API

### Service and health

```text
GET /healthz
GET /readyz
GET /metrics
GET /v1/config
```

`/v1/config` returns supported universal session protocol versions and ICE
servers. When TURN REST is configured, each response contains a newly
generated, time-limited Coturn username and HMAC-SHA1 credential; the static
shared secret is never returned.

### Create a room

```sh
curl -sS http://127.0.0.1:3536/v1/rooms \
  -H 'content-type: application/json' \
  -d '{
    "game_id":"bevy-open-rts",
    "build_id":"0.1.0+git.abcdef0",
    "session_protocol":1,
    "game_protocol":4,
    "max_peers":8,
    "visibility":"public",
    "metadata":{"map":"four-corners","mode":"skirmish"}
  }'
```

The response includes the room descriptor, signaling URL, host resume ticket,
and a join token only for private rooms.

### Discover rooms

```text
GET /v1/rooms?game_id=bevy-open-rts&game_protocol=4
GET /v1/rooms/{game_id}/{game_protocol}/{room_code}
```

Only public rooms appear in discovery. Unlisted/private rooms remain directly
addressable by code. Metadata is deliberately small and opaque to the service.

## Matchbox Signaling Contract

WebSocket text frames use `matchbox_protocol::JsonPeerRequest` and
`JsonPeerEvent` without a proprietary wrapper:

- server sends `IdAssigned` first;
- existing peers receive `NewPeer` when a peer joins and become offerers;
- `Signal` is relayed only when sender and receiver are currently in the same
  room;
- peers receive `PeerLeft` on disconnect;
- host disconnect pauses admission and starts a bounded reconnect grace window;
  existing peers remain connected to signaling, and a host presenting the same
  ticket rejoins with a new signaling peer ID;
- if the grace window expires, the service closes the remaining peer sockets
  and removes the room.

This makes the server usable by stock `matchbox_socket::WebRtcSocketBuilder`.

## RTS Replication Model

Bevy Open RTS uses host authority rather than lockstep:

1. The host owns the canonical simulation.
2. Clients send high-level input commands on a reliable ordered channel.
3. Commands reference stable network entity IDs, never local Bevy `Entity`
   values.
4. The host validates player ownership, visibility, target validity, resources,
   and command rate before applying commands.
5. The host sends periodic snapshots/deltas on an unreliable channel.
6. Clients interpolate remote transforms and reconcile locally predicted
   selections/order feedback.

RTS protocol v4 emits a compressed full keyframe once per second and sends 10
Hz deltas between keyframes. Each delta references the latest full keyframe,
not another delta, and contains changed/new entities plus explicit removals.
This keeps packets small without making later state depend on delivery of an
earlier unreliable packet. A client that missed a keyframe ignores its deltas
and recovers automatically at the next keyframe.

Short-lived shot, impact, support-warning, destruction, and promotion effects
carry monotonically increasing host event IDs. The host repeats active effects
in snapshot packets; clients keep a bounded deduplication history. A lost packet
therefore does not normally hide a warning, while duplicate delivery never
spawns the same effect twice.

The current RTS simulation uses floating-point transforms, wall-clock timers,
and ordinary Bevy `Update` systems, so deterministic peer lockstep would be a
separate simulation rewrite rather than a safe shortcut.

### Match Lifecycle

- A disconnected non-host player keeps its stable game identity for a 30-second
  reconnect grace. Reconnection with the same session key cancels the timer and
  the next authoritative snapshot catches the client up.
- Grace expiry is a host-authoritative forfeit: the host removes that player's
  match entities, pending paradrops, and queued production. New identities and
  forfeited identities cannot join a running match.
- Global match completion waits until no hostile sides remain. Each client maps
  the same finished snapshot to victory or defeat from its own alliance view.
- Returning to the war room is reliable and synchronized. A host may abort a
  match; a client may request return only after the global result. The rematch
  snapshot preserves connected players, clears readiness, and reclaims stale
  player slots.

## Production Network Requirements

- HTTPS/WSS reverse proxy with query-string logging disabled.
- Coturn reachable on TCP/UDP 3478 and a configured UDP relay range.
- `OPEN_BEVY_TURN_SECRET` must match Coturn's `static-auth-secret`.
- The TURN URL must use a public DNS name/IP, not the Compose service name.
- Restrict `OPEN_BEVY_ALLOWED_ORIGINS` to production game origins.
- Apply IP-level connection/request limits at the edge proxy.
- Persist no game state or chat in the signaling service.

The deployment example in `deploy/signaling/` provides the service, Coturn, and
Caddy wiring. Secrets and public addresses are intentionally required inputs.

## Product Verification

The verification harness is disabled during normal play. It is enabled only by
an explicit role and run ID, and it drives the existing lobby and match systems
rather than a separate mock protocol.

Two native clients against the deployed service:

```sh
OPEN_BEVY_SIGNALING_URL=https://signal.example.com \
  scripts/verify_native_online.sh
```

The script builds once, launches isolated host/player processes, and requires
both atomic JSON reports to name the same room and prove a synchronized move
command plus host-victory/player-defeat result.

The WebGPU workflow runs `scripts/browser-smoke/multiplayer_smoke.mjs` against
the freshly built web bundle. It launches two independent browser contexts with
these query parameters:

```text
?online_verify={host|player}
&online_run={unique-run-id}
&online_service={signaling-base-url}
```

Each browser publishes its current report in the hidden
`#open-bevy-online-verification` JSON element. The CI gate fails unless both
clients enter the same room, receive snapshots, observe the player command, and
finish with complementary authoritative results.
