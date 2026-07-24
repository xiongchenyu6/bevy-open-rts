//! Minimal native onboarding example for any Open Bevy game.

#[cfg(not(target_arch = "wasm32"))]
use open_bevy_net::OpenBevyGameClient;
#[cfg(not(target_arch = "wasm32"))]
use open_bevy_protocol::{BuildId, GameId, RoomVisibility};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_url = std::env::var("OPEN_BEVY_SIGNALING_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3536".to_string());
    let game_id =
        std::env::var("OPEN_BEVY_GAME_ID").unwrap_or_else(|_| "open-bevy-example".to_string());
    let build_id = std::env::var("OPEN_BEVY_BUILD_ID")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let game_protocol = std::env::var("OPEN_BEVY_GAME_PROTOCOL")
        .ok()
        .map(|value| value.parse::<u16>())
        .transpose()?
        .unwrap_or(1);

    let game = OpenBevyGameClient::new(
        service_url,
        GameId::new(game_id)?,
        BuildId::new(build_id)?,
        game_protocol,
    )?;
    let config = game.service_config().await?;
    println!(
        "service={} websocket={} namespace=({}, {}) build={}",
        config.service,
        config.websocket_base_url,
        game.game_id(),
        game.game_protocol(),
        game.build_id(),
    );

    if std::env::args().any(|argument| argument == "--create") {
        let room = game
            .create_room(
                4,
                RoomVisibility::Unlisted,
                BTreeMap::from([
                    ("mode".to_string(), "sdk-example".to_string()),
                    ("game".to_string(), game.game_id().to_string()),
                ]),
            )
            .await?;
        println!(
            "created room={} signaling={} host_token={} join_token={}",
            room.room.room_code,
            room.signaling_url,
            room.host_token,
            room.join_token.as_deref().unwrap_or("not-required"),
        );
    } else {
        let rooms = game.list_rooms().await?;
        println!("public_rooms={}", rooms.rooms.len());
        for room in rooms.rooms {
            println!(
                "room={} peers={}/{} build={} metadata={:?}",
                room.room_code, room.peer_count, room.max_peers, room.build_id, room.metadata,
            );
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Browser games call the same async API from their engine task pool. This
    // command-line onboarding executable is intentionally native-only.
}
