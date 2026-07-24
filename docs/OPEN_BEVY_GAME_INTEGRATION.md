# Open Bevy Game Integration

The signaling deployment is shared infrastructure. Every game owns its own
namespace and packet schema; the server does not know Bevy types or game rules.

## Add The Client

Until the Open Bevy crates are published independently, another repository can
consume the workspace packages directly from Git:

```toml
[dependencies]
open-bevy-net = { git = "https://github.com/xiongchenyu6/bevy-open-rts", package = "open-bevy-net" }
open-bevy-protocol = { git = "https://github.com/xiongchenyu6/bevy-open-rts", package = "open-bevy-protocol" }
```

The crates do not depend on Bevy and compile for native and
`wasm32-unknown-unknown`.

## Claim A Namespace

Choose these values in the game repository, not in the signaling service:

- `game_id`: permanent lowercase slug, for example `open-bevy-arena`;
- `build_id`: exact compatibility build, normally the game version plus commit;
- `game_protocol`: non-zero game packet revision; bump it for incompatible wire
  changes.

Construct one scoped client and retain it in the game's online subsystem:

```rust
use open_bevy_net::OpenBevyGameClient;
use open_bevy_protocol::{BuildId, GameId};

let online = OpenBevyGameClient::new(
    "https://signal.example.com",
    GameId::new("open-bevy-arena")?,
    BuildId::new("0.3.0+git.a1b2c3d")?,
    1,
)?;
```

`online.create_room(...)`, `online.list_rooms()`, and `online.room(...)` always
use that tuple. Joining transport must pass the local build ID to
`TransportConfig::player`; the server rejects incompatible builds before the
WebSocket upgrade.

## Drive WebRTC

1. Fetch `online.service_config()` for the WSS base URL and current ICE/TURN
   credentials.
2. Create or resolve a room through the scoped client.
3. Build `TransportConfig::host` or `TransportConfig::player` and call
   `WebRtcTransport::connect`.
4. Run the returned `MessageLoopFuture` on the engine I/O task pool.
5. Poll `WebRtcTransport::poll` from the game schedule.
6. Put commands/control on reliable channel 0 and replaceable snapshots on
   unreliable channel 1. Serialize only stable game-owned IDs.

The full native onboarding executable is compiler-checked with the SDK:

```sh
OPEN_BEVY_SIGNALING_URL=https://signal.example.com \
OPEN_BEVY_GAME_ID=open-bevy-arena \
OPEN_BEVY_BUILD_ID=0.3.0+git.a1b2c3d \
OPEN_BEVY_GAME_PROTOCOL=1 \
cargo run -p open-bevy-net --example universal_room -- --create
```

The service integration suite additionally creates `open-bevy-arena` and
`open-bevy-builder` rooms under the same game protocol and proves discovery,
lookup, and WebSocket signaling cannot cross their namespaces.

## Release Checklist

- Configure the repository variable `OPEN_BEVY_SIGNALING_URL` for browser
  builds.
- Add the production Pages origin to `OPEN_BEVY_ALLOWED_ORIGINS` on the server.
- Never log signaling query strings; they contain short-lived room tickets.
- Test native/native, browser/browser, and native/browser clients.
- Force `iceTransportPolicy: relay` in a release gate and inspect the selected
  ICE pair, as Bevy Open RTS does, so TURN is proven with actual game traffic.
