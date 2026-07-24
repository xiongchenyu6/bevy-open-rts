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
and channel 1 as low-latency unreliable snapshots.

## Namespacing

Rooms are isolated by this tuple:

```text
(game_id, protocol_version, room_code)
```

The signaling URL is:

```text
wss://signal.example.com/v1/signal/{game_id}/{protocol_version}/{room_code}
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

`/v1/config` returns supported protocol versions and ICE servers. When TURN REST
is configured, each response contains a newly generated, time-limited Coturn
username and HMAC-SHA1 credential; the static shared secret is never returned.

### Create a room

```sh
curl -sS http://127.0.0.1:3536/v1/rooms \
  -H 'content-type: application/json' \
  -d '{
    "game_id":"bevy-open-rts",
    "build_id":"0.1.0+git.abcdef0",
    "protocol_version":1,
    "max_peers":8,
    "visibility":"public",
    "metadata":{"map":"four-corners","mode":"skirmish"}
  }'
```

The response includes the room descriptor, signaling URL, host resume ticket,
and a join token only for private rooms.

### Discover rooms

```text
GET /v1/rooms?game_id=bevy-open-rts&protocol_version=1
GET /v1/rooms/{game_id}/{protocol_version}/{room_code}
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

The current RTS simulation uses floating-point transforms, wall-clock timers,
and ordinary Bevy `Update` systems, so deterministic peer lockstep would be a
separate simulation rewrite rather than a safe shortcut.

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
