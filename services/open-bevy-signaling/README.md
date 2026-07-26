# open-bevy-signaling

`open-bevy-signaling` is the game-independent room and Matchbox signaling
service used by Open Bevy games. It has two production deployment targets with
the same HTTP and WebSocket contract:

| Target | Runtime | State | Deployment guide |
| --- | --- | --- | --- |
| Native | Axum/Tokio container | In-memory room registry | [`deploy/signaling`](../../deploy/signaling/README.md) |
| Cloudflare | Rust Worker + Durable Objects | SQLite-backed room objects | [`cloudflare-worker`](cloudflare-worker/README.md) |

Both targets expose `/healthz`, `/readyz`, `/metrics`, `/v1/config`, the room
HTTP API, and the Matchbox-compatible `/v1/signal/...` WebSocket endpoint.
Games select a deployment only through `OPEN_BEVY_SIGNALING_URL`; no game
protocol or client code changes are required.

Both production targets issue short-lived Cloudflare Realtime TURN credentials.
The long-lived TURN key and API token remain server-side and are never returned
to game clients.

## Native development

```sh
cargo run -p open-bevy-signaling -- --bind 127.0.0.1:3536
```

See [`deploy/signaling/README.md`](../../deploy/signaling/README.md) for the
container and Caddy production stack.

## Cloudflare development

```sh
cd services/open-bevy-signaling/cloudflare-worker
cargo test --lib
cargo check --target wasm32-unknown-unknown
bash scripts/build.sh
npx wrangler dev --local
```

See [`cloudflare-worker/README.md`](cloudflare-worker/README.md) for secrets,
deployment, rollback, and verification.
