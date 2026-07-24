use open_bevy_net::{
    OpenBevyGameClient, TransportConfig, TransportEvent, WebRtcTransport, decode_snapshot_payload,
    encode_snapshot_payload, session_protocol_version,
};
use open_bevy_protocol::{
    BuildId, CreateRoomResponse, GameId, IceServer, PlayerName, RoomVisibility,
    ServiceConfigResponse,
};
use open_bevy_signaling::{AppState, ServerConfig, build_router};
use std::{collections::BTreeMap, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle, time::Instant};

const TEST_GAME_PROTOCOL: u16 = 17;

fn test_game_id() -> GameId {
    GameId::new("open-bevy-net-test").expect("static test game id is valid")
}

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
        ice_servers: vec![IceServer {
            // Explicitly empty selects host ICE candidates only, keeping this
            // integration test independent of public STUN infrastructure.
            urls: Vec::new(),
            username: None,
            credential: None,
        }],
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

async fn wait_for_local_id(transport: &mut WebRtcTransport) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if transport.local_id().is_some() {
            return;
        }
        let _ = transport.poll();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for signaling ID assignment");
}

async fn exchange_both_channels(room: &CreateRoomResponse, service_config: ServiceConfigResponse) {
    let (mut host, host_loop) = WebRtcTransport::connect(TransportConfig::host(
        room,
        PlayerName::new("Host").unwrap(),
        service_config.ice_servers.clone(),
    ))
    .unwrap();
    let host_task = tokio::spawn(host_loop);
    wait_for_local_id(&mut host).await;

    let (mut player, player_loop) = WebRtcTransport::connect(TransportConfig::player(
        &service_config.websocket_base_url,
        &room.room,
        PlayerName::new("Player").unwrap(),
        room.room.build_id.clone(),
        room.join_token.clone(),
        service_config.ice_servers,
    ))
    .unwrap();
    let player_task = tokio::spawn(player_loop);
    wait_for_local_id(&mut player).await;

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut host_peer = None;
    let mut player_peer = None;
    loop {
        host_peer = host_peer.or_else(|| {
            host.poll().unwrap().into_iter().find_map(|event| {
                if let TransportEvent::PeerConnected(peer) = event {
                    Some(peer)
                } else {
                    None
                }
            })
        });
        player_peer = player_peer.or_else(|| {
            player.poll().unwrap().into_iter().find_map(|event| {
                if let TransportEvent::PeerConnected(peer) = event {
                    Some(peer)
                } else {
                    None
                }
            })
        });
        if host_peer.is_some() && player_peer.is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "WebRTC handshake timed out");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let host_peer = host_peer.unwrap();
    let player_peer = player_peer.unwrap();

    host.send_reliable(host_peer, b"lobby-ready".to_vec())
        .unwrap();
    let snapshot_body = vec![0x42; 8 * 1024];
    let snapshot_packet = encode_snapshot_payload(&snapshot_body).unwrap();
    assert!(snapshot_packet.len() < snapshot_body.len());
    player.send_snapshot(player_peer, snapshot_packet).unwrap();

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut reliable_received = false;
    let mut snapshot_received = false;
    while Instant::now() < deadline && !(reliable_received && snapshot_received) {
        reliable_received |= player.poll().unwrap().into_iter().any(|event| {
            matches!(
                event,
                TransportEvent::ReliableMessage { payload, .. } if payload == b"lobby-ready"
            )
        });
        snapshot_received |= host.poll().unwrap().into_iter().any(|event| match event {
            TransportEvent::SnapshotMessage { payload, .. } => {
                decode_snapshot_payload(&payload).is_ok_and(|decoded| decoded == snapshot_body)
            }
            _ => false,
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    host.close();
    player.close();
    host_task.abort();
    player_task.abort();
    assert!(
        reliable_received,
        "reliable WebRTC packet was not delivered"
    );
    assert!(
        snapshot_received,
        "snapshot WebRTC packet was not delivered"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_client_creates_room_and_exchanges_both_webrtc_channel_types() {
    let server = spawn_server().await;
    let game = OpenBevyGameClient::new(
        &server.http_base,
        test_game_id(),
        BuildId::new("transport-integration").unwrap(),
        TEST_GAME_PROTOCOL,
    )
    .unwrap();
    let service_config = game.service_config().await.unwrap();
    let room = game
        .create_room(
            2,
            RoomVisibility::Public,
            BTreeMap::from([("mode".to_string(), "transport-test".to_string())]),
        )
        .await
        .unwrap();

    let listed = game.list_rooms().await.unwrap();
    assert_eq!(listed.rooms.len(), 1);
    assert_eq!(listed.rooms[0].room_code, room.room.room_code);

    exchange_both_channels(&room, service_config).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires OPEN_BEVY_SIGNALING_URL and a deployed signaling/TURN service"]
async fn deployed_service_exchanges_reliable_and_snapshot_channels() {
    let service_url = std::env::var("OPEN_BEVY_SIGNALING_URL")
        .expect("OPEN_BEVY_SIGNALING_URL must point at the deployed HTTPS service");
    let game = OpenBevyGameClient::new(
        &service_url,
        test_game_id(),
        BuildId::new("production-transport-smoke").unwrap(),
        TEST_GAME_PROTOCOL,
    )
    .unwrap();
    let service_config = game.service_config().await.unwrap();
    assert_eq!(
        service_config.min_session_protocol,
        session_protocol_version()
    );
    assert_eq!(
        service_config.max_session_protocol,
        session_protocol_version()
    );

    let require_turn = std::env::var("OPEN_BEVY_REQUIRE_TURN").is_ok_and(|value| value != "0");
    if require_turn {
        assert!(
            service_config.ice_servers.iter().any(|server| {
                server.credential.is_some()
                    && server.urls.iter().any(|url| url.starts_with("turn:"))
            }),
            "deployed service did not issue authenticated TURN credentials"
        );
    }

    let room = game
        .create_room(
            2,
            RoomVisibility::Unlisted,
            BTreeMap::from([("mode".to_string(), "production-smoke".to_string())]),
        )
        .await
        .unwrap();
    println!(
        "created production smoke room {} through {}",
        room.room.room_code, service_url
    );
    exchange_both_channels(&room, service_config).await;
}
