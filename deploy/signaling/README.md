# Open Bevy Signaling Native Operations

This stack is the game-independent WebRTC room and signaling edge for the Open
Bevy family. Game state never passes through it; peers exchange game packets
over WebRTC data channels after Matchbox signaling completes. Relay traffic
uses Cloudflare Realtime TURN.

This is the native Axum/Tokio deployment target. The protocol-compatible
Cloudflare Worker target lives at
[`services/open-bevy-signaling/cloudflare-worker`](../../services/open-bevy-signaling/cloudflare-worker/README.md).
Games can switch between them by changing only `OPEN_BEVY_SIGNALING_URL`.

## Prerequisites

- A public DNS name for HTTPS/WSS.
- TCP 80 and 443 open at every firewall/NAT.
- Docker Engine with Compose v2.
- A Cloudflare Realtime TURN key and its API token.

## Configure

```sh
cd deploy/signaling
cp .env.example .env
```

Replace every example value in `.env`. `CLOUDFLARE_TURN_API_TOKEN` is secret and
must never be committed. `ALLOWED_ORIGINS` is a comma-separated list of browser
origins such as `https://xiongchenyu6.github.io`; native clients do not send an
Origin and remain supported. `HOST_RECONNECT_GRACE_SECS` defaults to 30 seconds.
The container uses Cloudflare's public resolvers by default so it can reach the
TURN credential API even on Podman hosts with a broken bridge DNS forwarder.
Override `CONTAINER_DNS_PRIMARY` and `CONTAINER_DNS_SECONDARY` when the host must
use organization-managed resolvers.

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
`127.0.0.1:${SIGNALING_BIND_PORT}`. Start the application without the bundled
Caddy:

```sh
docker compose up -d --build signaling
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
scripts/verify_signaling_backend.sh
```

This creates an unlisted two-peer room and exchanges payloads over both the
reliable command channel and unreliable snapshot channel, including a
compressed Open Bevy snapshot-envelope round trip. With
`OPEN_BEVY_REQUIRE_TURN=1`, it also rejects a deployment that does not issue
authenticated TURN credentials.

Credential issuance is only the control-plane check. To force real browser game
traffic through Cloudflare TURN and inspect the selected ICE pair, run:

```sh
OPEN_BEVY_GAME_URL="https://games.example.com/open-bevy-rts/" \
OPEN_BEVY_SIGNALING_URL="https://${SIGNALING_HOST}" \
OPEN_BEVY_FORCE_RELAY=1 \
npm --prefix scripts/browser-smoke run multiplayer
```

The command completes a host-authoritative match and fails unless both browsers
select a succeeded local `relay` candidate with non-zero sent and received
bytes. The Pages release workflow runs this forced-relay gate automatically.

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
docker compose up -d signaling
docker compose ps
docker compose logs --since=5m signaling
```

For rollback, check out the previous known-good commit and repeat the build/up
commands. Rooms are deliberately in memory; a signaling restart ends active
room discovery but does not persist game data or credentials.

## Operational Checks

- Alert when `/readyz` fails or the container is unhealthy.
- Scrape `/metrics`; watch rejected connections, invalid messages, active
  connections, room count, and relayed signals.
- Monitor Cloudflare TURN credential failures and relay usage.
- Rotate the Cloudflare TURN API token as an operational secret. Client
  credentials are short lived and expire naturally.
- Keep query-string logging disabled at every proxy/CDN layer.
