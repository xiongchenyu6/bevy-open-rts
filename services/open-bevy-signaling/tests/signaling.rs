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
    tungstenite::{
        Error as WebSocketError, Message,
        client::IntoClientRequest,
        http::{HeaderValue, StatusCode, header::ORIGIN},
    },
};

type ClientSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestServer {
    http_base: String,
    state: AppState,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_server() -> TestServer {
    spawn_server_with_grace(Duration::from_secs(30)).await
}

async fn spawn_server_with_grace(host_reconnect_grace: Duration) -> TestServer {
    spawn_server_with_options(host_reconnect_grace, vec!["*".to_string()]).await
}

async fn spawn_server_with_options(
    host_reconnect_grace: Duration,
    allowed_origins: Vec<String>,
) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = AppState::new(ServerConfig {
        public_websocket_base_url: format!("ws://{address}"),
        room_ttl: Duration::from_secs(60),
        host_reconnect_grace,
        allowed_origins,
        ..ServerConfig::default()
    });
    let server_state = state.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, build_router(server_state))
            .await
            .unwrap();
    });
    TestServer {
        http_base: format!("http://{address}"),
        state,
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
    connect_async(player_url(room, name))
        .await
        .map(|connection| connection.0)
}

fn player_url(room: &CreateRoomResponse, name: &str) -> String {
    let ticket = room
        .join_token
        .as_ref()
        .map(|token| format!("&ticket={token}"))
        .unwrap_or_default();
    format!(
        "{}?name={name}&role=player&build_id=integration-build{ticket}",
        room.signaling_url
    )
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
async fn production_origin_policy_allows_native_and_approved_browser_clients() {
    const GAME_ORIGIN: &str = "https://games.example.test";
    let server =
        spawn_server_with_options(Duration::from_secs(30), vec![GAME_ORIGIN.to_string()]).await;
    let room = create_room(&server, RoomVisibility::Public, 4).await;

    // Native Matchbox clients do not send Origin. They must remain usable when
    // browser origins are restricted in production.
    let mut host = connect_host(&room, "NativeHost").await;
    let _host_id = assigned_id(&mut host).await;

    let mut browser_request = player_url(&room, "BrowserPlayer")
        .into_client_request()
        .unwrap();
    browser_request
        .headers_mut()
        .insert(ORIGIN, HeaderValue::from_static(GAME_ORIGIN));
    let mut browser = connect_async(browser_request).await.unwrap().0;
    let _browser_id = assigned_id(&mut browser).await;

    let mut rejected_request = player_url(&room, "ForeignBrowser")
        .into_client_request()
        .unwrap();
    rejected_request.headers_mut().insert(
        ORIGIN,
        HeaderValue::from_static("https://foreign.example.test"),
    );
    let error = connect_async(rejected_request).await.unwrap_err();
    assert!(matches!(
        error,
        WebSocketError::Http(response) if response.status() == StatusCode::FORBIDDEN
    ));

    let client = reqwest::Client::new();
    let approved = client
        .get(format!("{}/v1/config", server.http_base))
        .header("origin", GAME_ORIGIN)
        .send()
        .await
        .unwrap();
    assert_eq!(
        approved
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some(GAME_ORIGIN)
    );
    let rejected = client
        .get(format!("{}/v1/config", server.http_base))
        .header("origin", "https://foreign.example.test")
        .send()
        .await
        .unwrap();
    assert!(
        rejected
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
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
async fn host_can_resume_with_the_same_ticket_without_dropping_players() {
    let server = spawn_server().await;
    let room = create_room(&server, RoomVisibility::Public, 4).await;
    let mut host = connect_host(&room, "Host").await;
    let first_host_id = assigned_id(&mut host).await;
    let mut player = connect_player(&room, "Player").await.unwrap();
    let player_id = assigned_id(&mut player).await;
    assert_eq!(
        receive_event(&mut host).await,
        JsonPeerEvent::NewPeer(player_id)
    );

    host.close(None).await.unwrap();
    assert_eq!(
        receive_event(&mut player).await,
        JsonPeerEvent::PeerLeft(first_host_id)
    );

    let mut resumed_host = connect_host(&room, "Host").await;
    let resumed_host_id = assigned_id(&mut resumed_host).await;
    assert_ne!(resumed_host_id, first_host_id);
    assert_eq!(
        receive_event(&mut player).await,
        JsonPeerEvent::NewPeer(resumed_host_id)
    );

    let offer = json!({"Sdp": {"type": "offer", "sdp": "resumed-host"}});
    resumed_host
        .send(Message::Text(
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
            sender: resumed_host_id,
            data: offer,
        }
    );
}

#[tokio::test]
async fn host_reconnect_timeout_closes_stranded_players_and_removes_room() {
    let server = spawn_server_with_grace(Duration::from_millis(20)).await;
    let room = create_room(&server, RoomVisibility::Public, 4).await;
    let mut host = connect_host(&room, "Host").await;
    let host_id = assigned_id(&mut host).await;
    let mut player = connect_player(&room, "Player").await.unwrap();
    let player_id = assigned_id(&mut player).await;
    assert_eq!(
        receive_event(&mut host).await,
        JsonPeerEvent::NewPeer(player_id)
    );

    host.close(None).await.unwrap();
    assert_eq!(
        receive_event(&mut player).await,
        JsonPeerEvent::PeerLeft(host_id)
    );
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(server.state.remove_expired_rooms().await, 1);

    let close = timeout(Duration::from_secs(2), player.next())
        .await
        .expect("timed out waiting for reconnect-expiry close")
        .expect("player stream ended before close frame")
        .expect("player websocket read failed");
    assert!(matches!(close, Message::Close(_)));

    let response = reqwest::get(format!(
        "{}/v1/rooms/integration-game/{SESSION_PROTOCOL_VERSION}/{}",
        server.http_base, room.room.room_code
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
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
