# open-bevy-signaling on Cloudflare Workers

This crate deploys the same `open-bevy-signaling` protocol as the native
Axum/Tokio server. It is an additional deployment target, not a forked API.

## Architecture

- The Worker entry point owns CORS, health/config endpoints, and routing.
- One `RoomDirectory` Durable Object owns public room discovery.
- Every `(game_id, game_protocol, room_code)` has its own `GameRoom` Durable
  Object, so games and incompatible protocol revisions are isolated by object.
- `GameRoom` uses hibernatable WebSockets and serialized attachments for peer
  identity, role, invalid-message limits, and host reconnect handling.
- Room descriptors and the host/join tickets are stored in SQLite-backed
  Durable Object storage. No game packets or match state pass through it.

The public Worker service name is `open-bevy-signaling` and its default URL is:

```text
https://open-bevy-signaling.xiongchenyu6.workers.dev
```

## Configuration

Non-secret values live in `wrangler.toml`:

| Variable | Purpose |
| --- | --- |
| `PUBLIC_BASE_URL` | HTTPS origin used to produce the public WSS URL |
| `ALLOWED_ORIGINS` | Comma-separated browser origins; native clients may omit Origin |
| `ROOM_TTL_MS` | Lifetime of a room that never receives a host |
| `HOST_RECONNECT_GRACE_MS` | Admission pause before a disconnected host forfeits the room |
| `TURN_CREDENTIAL_TTL_SECONDS` | Lifetime of issued TURN credentials |

Secrets are set with Wrangler and are never committed:

```sh
# Managed TURN backend. Both values are required together in production.
npx wrangler secret put CLOUDFLARE_TURN_KEY_ID
npx wrangler secret put CLOUDFLARE_TURN_API_TOKEN
```

When both secrets exist, `/v1/config` requests short-lived Cloudflare Realtime
TURN credentials. Without either secret it advertises STUN only for local
development; production verification rejects that configuration. A partial
secret configuration is rejected instead of silently using another provider.

## Build and local development

Prerequisites:

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.5 --locked
npx wrangler login
```

Build and test:

```sh
cargo fmt --check
cargo test --lib
cargo check --target wasm32-unknown-unknown
bash scripts/build.sh
```

Run Durable Objects locally:

```sh
npx wrangler dev --local --port 8787
curl -fsS http://127.0.0.1:8787/readyz
curl -fsS http://127.0.0.1:8787/v1/config
```

Or run the repeatable local backend gate. It starts Wrangler, creates a real
room, connects two stock Matchbox/WebRTC clients, and exchanges both reliable
and unreliable payloads:

```sh
bash scripts/verify_local.sh
```

Local secrets can be placed in an untracked `.dev.vars` file using the same
names as the production secrets.

## Deploy and verify

```sh
npx wrangler deploy
curl -fsS https://open-bevy-signaling.xiongchenyu6.workers.dev/readyz
curl -fsS https://open-bevy-signaling.xiongchenyu6.workers.dev/v1/config
```

Run the shared production protocol/WebRTC check against either backend:

```sh
OPEN_BEVY_SIGNALING_URL=https://open-bevy-signaling.xiongchenyu6.workers.dev \
OPEN_BEVY_REQUIRE_TURN=1 \
../../../scripts/verify_signaling_backend.sh
```

`OPEN_BEVY_REQUIRE_TURN=1` checks credential issuance. The browser multiplayer
gate additionally forces relay-only ICE and checks the selected WebRTC
candidate pair.

## CI deployment

`.github/workflows/signaling-worker.yml` always builds and tests this target.
Automatic deployment from `main` is enabled only when repository variable
`CLOUDFLARE_WORKER_DEPLOY_ENABLED` is `true`. Configure:

```sh
gh variable set CLOUDFLARE_ACCOUNT_ID --body 2764ae0fd9a5cb92c9ac67708620e54c
gh variable set CLOUDFLARE_WORKER_DEPLOY_ENABLED --body true
gh secret set CLOUDFLARE_API_TOKEN
```

The API token needs Workers Scripts write and Durable Objects write access for
the account. The workflow caches Cargo artifacts, `worker-build`, and its
downloaded wasm-bindgen/esbuild tools.

## Rollback

Deploy a known-good Git revision with the same `wrangler.toml` Durable Object
class names and migration history:

```sh
git checkout <known-good-revision>
cd services/open-bevy-signaling/cloudflare-worker
npx wrangler deploy
```

Do not remove or rename `GameRoom`/`RoomDirectory` without adding an explicit
Durable Object migration. Existing rooms are short lived, but migration history
is part of the deployed service contract.
