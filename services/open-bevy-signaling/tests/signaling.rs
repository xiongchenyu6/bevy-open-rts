use futures_util::{SinkExt, StreamExt};
use matchbox_protocol::{JsonPeerEvent, JsonPeerRequest, PeerId};
use open_bevy_protocol::{
    BuildId, CreateRoomRequest, CreateRoomResponse, GameId, RoomVisibility,
    SESSION_PROTOCOL_VERSION,
};
use open_bevy_signaling::{AppState, ServerConfig, build_router};
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Error as WebSocketError, Message},
};

type ClientSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestServer {
    http_base: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_server() -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = AppState::new(ServerConfig {
        public_websocket_base_url: format!("ws://{address}"),
        room_ttl: Duration::from_secs(60),
        ..ServerConfig::default()
    });
    let task = tokio::spawn(async move {
        axum::serve(listener, build_router(state)).await.unwrap();
    });
    TestServer {
        http_base: format!("http://{address}"),
        task,
    }
}

async fn create_room(
    server: &TestServer,
    visibility: RoomVisibility,
    max_peers: u16,
) -> CreateRoomResponse {
    reqwest::Client::new()
        .post(format!("{}/v1/rooms", server.http_base))
        .json(&CreateRoomRequest {
            game_id: GameId::new("integration-game").unwrap(),
            build_id: BuildId::new("integration-build").unwrap(),
            protocol_version: SESSION_PROTOCOL_VERSION,
            max_peers,
            visibility,
            metadata: BTreeMap::from([
                ("map".to_string(), "four-corners".to_string()),
                ("mode".to_string(), "skirmish".to_string()),
            ]),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn connect_host(room: &CreateRoomResponse, name: &str) -> ClientSocket {
    let url = format!(
        "{}?name={name}&role=host&ticket={}&build_id=integration-build",
        room.signaling_url, room.host_token
    );
    connect_async(url).await.unwrap().0
}

async fn connect_player(
    room: &CreateRoomResponse,
    name: &str,
) -> Result<ClientSocket, WebSocketError> {
    let ticket = room
        .join_token
        .as_ref()
        .map(|token| format!("&ticket={token}"))
        .unwrap_or_default();
    let url = format!(
        "{}?name={name}&role=player&build_id=integration-build{ticket}",
        room.signaling_url
    );
    connect_async(url).await.map(|connection| connection.0)
}

async fn receive_event(socket: &mut ClientSocket) -> JsonPeerEvent {
    let message = timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("timed out waiting for signaling event")
        .expect("websocket ended before event")
        .expect("websocket read failed");
    let Message::Text(text) = message else {
        panic!("expected text event, received {message:?}");
    };
    text.as_str().parse().expect("invalid Matchbox event JSON")
}

async fn assigned_id(socket: &mut ClientSocket) -> PeerId {
    match receive_event(socket).await {
        JsonPeerEvent::IdAssigned(peer_id) => peer_id,
        event => panic!("expected IdAssigned, received {event:?}"),
    }
}

#[tokio::test]
async fn relays_matchbox_signals_and_reports_disconnects() {
    let server = spawn_server().await;
    let room = create_room(&server, RoomVisibility::Public, 4).await;
    let mut host = connect_host(&room, "Host").await;
    let host_id = assigned_id(&mut host).await;

    let mut player = connect_player(&room, "Player").await.unwrap();
    let player_id = assigned_id(&mut player).await;
    assert_eq!(
        receive_event(&mut host).await,
        JsonPeerEvent::NewPeer(player_id)
    );

    let offer = json!({"Sdp": {"type": "offer", "sdp": "integration-test"}});
    host.send(Message::Text(
        JsonPeerRequest::Signal {
            receiver: player_id,
            data: offer.clone(),
        }
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    assert_eq!(
        receive_event(&mut player).await,
        JsonPeerEvent::Signal {
            sender: host_id,
            data: offer,
        }
    );

    player.close(None).await.unwrap();
    assert_eq!(
        receive_event(&mut host).await,
        JsonPeerEvent::PeerLeft(player_id)
    );
}

#[tokio::test]
async fn isolates_rooms_and_never_announces_cross_room_peers() {
    let server = spawn_server().await;
    let first_room = create_room(&server, RoomVisibility::Public, 4).await;
    let second_room = create_room(&server, RoomVisibility::Public, 4).await;
    let mut first_host = connect_host(&first_room, "FirstHost").await;
    let _ = assigned_id(&mut first_host).await;
    let mut second_host = connect_host(&second_room, "SecondHost").await;
    let _ = assigned_id(&mut second_host).await;

    assert!(
        timeout(Duration::from_millis(150), first_host.next())
            .await
            .is_err(),
        "a peer in another room leaked into the first room"
    );
}

#[tokio::test]
async fn rejects_over_capacity_and_invalid_private_room_tokens_before_upgrade() {
    let server = spawn_server().await;
    let full_room = create_room(&server, RoomVisibility::Public, 2).await;
    let mut host = connect_host(&full_room, "Host").await;
    let _ = assigned_id(&mut host).await;
    let mut first_player = connect_player(&full_room, "PlayerOne").await.unwrap();
    let _ = assigned_id(&mut first_player).await;
    let error = connect_player(&full_room, "PlayerTwo")
        .await
        .expect_err("third peer should be rejected");
    let WebSocketError::Http(response) = error else {
        panic!("expected HTTP upgrade rejection");
    };
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);

    let private_room = create_room(&server, RoomVisibility::Private, 4).await;
    let mut private_host = connect_host(&private_room, "PrivateHost").await;
    let _ = assigned_id(&mut private_host).await;
    let bad_url = format!(
        "{}?name=Intruder&role=player&build_id=integration-build&ticket=wrong",
        private_room.signaling_url
    );
    let error = connect_async(bad_url)
        .await
        .expect_err("wrong private-room token should be rejected");
    let WebSocketError::Http(response) = error else {
        panic!("expected HTTP upgrade rejection");
    };
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn discovery_only_lists_public_rooms_and_preserves_metadata() {
    let server = spawn_server().await;
    let public_room = create_room(&server, RoomVisibility::Public, 8).await;
    let _private_room = create_room(&server, RoomVisibility::Private, 8).await;

    let response = reqwest::get(format!(
        "{}/v1/rooms?game_id=integration-game&protocol_version={SESSION_PROTOCOL_VERSION}",
        server.http_base
    ))
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json::<open_bevy_protocol::RoomListResponse>()
    .await
    .unwrap();

    assert_eq!(response.rooms.len(), 1);
    assert_eq!(response.rooms[0].room_code, public_room.room.room_code);
    assert_eq!(response.rooms[0].metadata["map"], "four-corners");
}
