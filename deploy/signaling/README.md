# Open Bevy Signaling Operations

This stack is the game-independent WebRTC room, signaling, and TURN edge for
the Open Bevy family. Game state never passes through it; peers exchange game
packets over WebRTC data channels after Matchbox signaling completes.

## Prerequisites

- A public DNS name for HTTPS/WSS and TURN.
- TCP 80, 443, and 3478; UDP 3478 and 49160-49200 open at every firewall/NAT.
- Docker Engine with Compose v2.
- The public IP assigned to the host, not a private interface address.

## Configure

```sh
cd deploy/signaling
cp .env.example .env
openssl rand -hex 32
```

Replace every example value in `.env`. `TURN_SECRET` must be the generated
secret and must never be committed. `ALLOWED_ORIGINS` is a comma-separated list
of browser origins such as `https://xiongchenyu6.github.io`; native clients do
not send an Origin and remain supported. `HOST_RECONNECT_GRACE_SECS` defaults to
30 seconds.

## Start With The Bundled Caddy

Use this mode when ports 80/443 are free on the host:

```sh
docker compose up -d --build
docker compose ps
curl "https://${SIGNALING_HOST}/healthz"
```

The bundled Caddy intentionally has no access log because WebSocket host/join
tickets are query parameters in browser handshakes.

## Start Behind An Existing TLS Proxy

The signaling service publishes plain HTTP only on
`127.0.0.1:${SIGNALING_BIND_PORT}`. Start the application and TURN without the
bundled Caddy:

```sh
docker compose up -d --build signaling turn
```

Add an equivalent route to the host proxy:

```caddyfile
signal.example.com {
    # Do not enable request access logs on this host.
    reverse_proxy 127.0.0.1:3536
}
```

Reload the proxy only after `curl http://127.0.0.1:3536/readyz` succeeds.

## Verify

The config response must advertise the public WSS URL and short-lived,
authenticated TURN credentials:

```sh
curl -fsS "https://${SIGNALING_HOST}/healthz"
curl -fsS "https://${SIGNALING_HOST}/readyz"
curl -fsS "https://${SIGNALING_HOST}/v1/config"
curl -fsS "https://${SIGNALING_HOST}/metrics"
```

From a native development machine, run the real room + WebRTC data-channel
smoke against production:

```sh
OPEN_BEVY_SIGNALING_URL="https://${SIGNALING_HOST}" \
OPEN_BEVY_REQUIRE_TURN=1 \
cargo test -p open-bevy-net --test transport \
  deployed_service_exchanges_reliable_and_snapshot_channels \
  -- --ignored --nocapture
```

This creates an unlisted two-peer room and exchanges payloads over both the
reliable command channel and unreliable snapshot channel, including a
compressed Open Bevy snapshot-envelope round trip. With
`OPEN_BEVY_REQUIRE_TURN=1`, it also rejects a deployment that does not issue
TURN REST credentials.

## Publish A Game Client

The endpoint is public configuration, not a secret. Set it once on each Open
Bevy game repository before building browser artifacts:

```sh
gh variable set OPEN_BEVY_SIGNALING_URL \
  --body "https://${SIGNALING_HOST}"
```

The wasm workflow compiles this value into the lobby. Native builds can either
use the same compile-time value or set it while building locally.

## Update And Roll Back

```sh
git pull --ff-only
docker compose build signaling
docker compose up -d signaling turn
docker compose ps
docker compose logs --since=5m signaling turn
```

For rollback, check out the previous known-good commit and repeat the build/up
commands. Rooms are deliberately in memory; a signaling restart ends active
room discovery but does not persist game data or credentials.

## Operational Checks

- Alert when `/readyz` fails or the container is unhealthy.
- Scrape `/metrics`; watch rejected connections, invalid messages, active
  connections, room count, and relayed signals.
- Monitor Coturn allocation/authentication failures and relay bandwidth.
- Rotate `TURN_SECRET` in both services together; existing one-hour credentials
  expire naturally.
- Keep query-string logging disabled at every proxy/CDN layer.
