//! Universal, game-independent WebRTC signaling service.
//!
//! The WebSocket endpoint speaks Matchbox's JSON signaling protocol so games
//! can use `matchbox_socket` directly on both native and wasm targets.

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use matchbox_protocol::{JsonPeerEvent, JsonPeerRequest, PeerId};
use open_bevy_protocol::{
    BuildId, CreateRoomRequest, CreateRoomResponse, ErrorResponse, GameId, IceServer, MAX_PEERS,
    PeerRole, PlayerName, RoomCode, RoomDescriptor, RoomListResponse, RoomVisibility,
    SESSION_PROTOCOL_VERSION, ServiceConfigResponse, signaling_path,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{RwLock, mpsc};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{debug, info, warn};
use uuid::Uuid;

const MAX_SIGNAL_BYTES: usize = 256 * 1024;
const MAX_INVALID_MESSAGES: u8 = 3;
const CLOSE_POLICY_VIOLATION: u16 = 1008;
const CLOSE_SERVER_ERROR: u16 = 1011;
const CLOUDFLARE_TURN_API_BASE: &str = "https://rtc.live.cloudflare.com/v1/turn/keys";

#[derive(Clone)]
pub struct ServerConfig {
    pub public_websocket_base_url: String,
    pub ice_servers: Vec<IceServer>,
    pub cloudflare_turn: Option<CloudflareTurnConfig>,
    pub room_ttl: Duration,
    pub host_reconnect_grace: Duration,
    pub allowed_origins: Vec<String>,
}

#[derive(Clone)]
pub struct CloudflareTurnConfig {
    pub key_id: String,
    pub api_token: String,
    pub credential_ttl: Duration,
}

#[derive(Debug, Deserialize)]
struct CloudflareTurnResponse {
    #[serde(rename = "iceServers")]
    ice_servers: Vec<IceServer>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            public_websocket_base_url: "ws://127.0.0.1:3536".to_string(),
            ice_servers: vec![IceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            }],
            cloudflare_turn: None,
            room_ttl: Duration::from_secs(15 * 60),
            host_reconnect_grace: Duration::from_secs(30),
            allowed_origins: vec!["*".to_string()],
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: ServerConfig,
    rooms: RwLock<HashMap<RoomKey, Room>>,
    metrics: Metrics,
}

#[derive(Default)]
struct Metrics {
    rooms_created: AtomicU64,
    active_connections: AtomicU64,
    connections_accepted: AtomicU64,
    connections_rejected: AtomicU64,
    signals_relayed: AtomicU64,
    invalid_messages: AtomicU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RoomKey {
    game_id: GameId,
    game_protocol: u16,
    room_code: RoomCode,
}

struct Room {
    build_id: BuildId,
    visibility: RoomVisibility,
    max_peers: u16,
    metadata: std::collections::BTreeMap<String, String>,
    host_ticket: Option<String>,
    join_token: Option<String>,
    host_peer: Option<PeerId>,
    peers: HashMap<PeerId, PeerSession>,
    created_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    last_activity_unix_ms: u64,
}

struct PeerSession {
    name: PlayerName,
    role: PeerRole,
    outbound: mpsc::UnboundedSender<OutboundMessage>,
}

enum OutboundMessage {
    Event(JsonPeerEvent),
    Close { code: u16, reason: String },
}

#[derive(Debug, Deserialize)]
struct SignalQuery {
    name: String,
    #[serde(default)]
    role: PeerRole,
    ticket: Option<String>,
    build_id: String,
}

#[derive(Debug, Default, Deserialize)]
struct ListRoomsQuery {
    game_id: Option<String>,
    game_protocol: Option<u16>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Debug)]
struct Registration {
    peer_id: PeerId,
    outbound: mpsc::UnboundedReceiver<OutboundMessage>,
    existing_peers: Vec<mpsc::UnboundedSender<OutboundMessage>>,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "room_not_found",
            message: message.into(),
        }
    }

    fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code,
            message: message.into(),
        }
    }

    fn upstream(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code: "turn_credentials_unavailable",
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                code: self.code.to_string(),
                message: self.message,
            }),
        )
            .into_response()
    }
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                rooms: RwLock::new(HashMap::new()),
                metrics: Metrics::default(),
            }),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.inner.config
    }

    pub async fn remove_expired_rooms(&self) -> usize {
        let now = unix_time_ms();
        let mut rooms = self.inner.rooms.write().await;
        let expired = rooms
            .iter()
            .filter(|(_, room)| room.host_peer.is_none() && room.expires_at_unix_ms <= now)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let mut stranded_peers = Vec::new();
        for key in &expired {
            if let Some(room) = rooms.remove(key) {
                stranded_peers.extend(room.peers.into_values().map(|peer| peer.outbound));
            }
        }
        drop(rooms);

        self.inner
            .metrics
            .active_connections
            .fetch_sub(stranded_peers.len() as u64, Ordering::Relaxed);
        for peer in stranded_peers {
            let _ = peer.send(OutboundMessage::Close {
                code: CLOSE_SERVER_ERROR,
                reason: "room host reconnect window expired".to_string(),
            });
        }
        expired.len()
    }

    async fn create_room(
        &self,
        request: CreateRoomRequest,
    ) -> Result<CreateRoomResponse, ApiError> {
        request
            .validate()
            .map_err(|error| ApiError::bad_request("invalid_room", error.to_string()))?;
        if request.session_protocol != SESSION_PROTOCOL_VERSION {
            return Err(ApiError::bad_request(
                "unsupported_session_protocol",
                format!(
                    "session protocol {} is unsupported; this service accepts session protocol {}",
                    request.session_protocol, SESSION_PROTOCOL_VERSION
                ),
            ));
        }

        let now = unix_time_ms();
        let expires_at = now.saturating_add(self.inner.config.room_ttl.as_millis() as u64);
        let host_ticket = random_secret();
        let join_token = (request.visibility == RoomVisibility::Private).then(random_secret);
        let mut rooms = self.inner.rooms.write().await;

        let (room_code, key) = loop {
            let code = generate_room_code();
            let key = RoomKey {
                game_id: request.game_id.clone(),
                game_protocol: request.game_protocol,
                room_code: code.clone(),
            };
            if !rooms.contains_key(&key) {
                break (code, key);
            }
        };

        let room = Room {
            build_id: request.build_id.clone(),
            visibility: request.visibility,
            max_peers: request.max_peers,
            metadata: request.metadata,
            host_ticket: Some(host_ticket.clone()),
            join_token: join_token.clone(),
            host_peer: None,
            peers: HashMap::new(),
            created_at_unix_ms: now,
            expires_at_unix_ms: expires_at,
            last_activity_unix_ms: now,
        };
        let descriptor = room_descriptor(&key, &room);
        rooms.insert(key, room);
        self.inner
            .metrics
            .rooms_created
            .fetch_add(1, Ordering::Relaxed);

        let signaling_url = format!(
            "{}{}",
            self.inner
                .config
                .public_websocket_base_url
                .trim_end_matches('/'),
            signaling_path(&request.game_id, request.game_protocol, &room_code)
        );

        Ok(CreateRoomResponse {
            room: descriptor,
            signaling_url,
            host_token: host_ticket,
            join_token,
        })
    }

    async fn preflight(&self, key: &RoomKey, query: &ValidatedSignalQuery) -> Result<(), ApiError> {
        let rooms = self.inner.rooms.read().await;
        let room = rooms
            .get(key)
            .ok_or_else(|| ApiError::not_found("the requested room does not exist"))?;
        authorize_room_connection(room, query)
    }

    async fn register_peer(
        &self,
        key: &RoomKey,
        query: &ValidatedSignalQuery,
    ) -> Result<Registration, ApiError> {
        let (outbound_sender, outbound) = mpsc::unbounded_channel();
        let peer_id = PeerId(Uuid::new_v4());
        let mut rooms = self.inner.rooms.write().await;
        let room = rooms
            .get_mut(key)
            .ok_or_else(|| ApiError::not_found("the requested room no longer exists"))?;
        authorize_room_connection(room, query)?;

        let existing_peers = room
            .peers
            .values()
            .map(|peer| peer.outbound.clone())
            .collect();
        if query.role == PeerRole::Host {
            room.host_peer = Some(peer_id);
        }
        room.last_activity_unix_ms = unix_time_ms();
        room.expires_at_unix_ms = room
            .last_activity_unix_ms
            .saturating_add(self.inner.config.room_ttl.as_millis() as u64);
        room.peers.insert(
            peer_id,
            PeerSession {
                name: query.name.clone(),
                role: query.role,
                outbound: outbound_sender,
            },
        );
        self.inner
            .metrics
            .active_connections
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .metrics
            .connections_accepted
            .fetch_add(1, Ordering::Relaxed);

        Ok(Registration {
            peer_id,
            outbound,
            existing_peers,
        })
    }

    async fn relay_signal(
        &self,
        key: &RoomKey,
        sender: PeerId,
        receiver: PeerId,
        data: serde_json::Value,
    ) -> bool {
        let target = {
            let mut rooms = self.inner.rooms.write().await;
            let Some(room) = rooms.get_mut(key) else {
                return false;
            };
            if !room.peers.contains_key(&sender) {
                return false;
            }
            room.last_activity_unix_ms = unix_time_ms();
            room.peers.get(&receiver).map(|peer| peer.outbound.clone())
        };
        let Some(target) = target else {
            return false;
        };
        let delivered = target
            .send(OutboundMessage::Event(JsonPeerEvent::Signal {
                sender,
                data,
            }))
            .is_ok();
        if delivered {
            self.inner
                .metrics
                .signals_relayed
                .fetch_add(1, Ordering::Relaxed);
        }
        delivered
    }

    async fn disconnect_peer(&self, key: &RoomKey, peer_id: PeerId) {
        let (remaining, was_host, peer_label) = {
            let mut rooms = self.inner.rooms.write().await;
            let Some(room) = rooms.get_mut(key) else {
                return;
            };
            let removed = room.peers.remove(&peer_id);
            let Some(removed) = removed else {
                return;
            };
            let was_host = room.host_peer == Some(peer_id);
            let peer_label = format!("{} ({:?})", removed.name, removed.role);
            let remaining = room
                .peers
                .values()
                .map(|peer| peer.outbound.clone())
                .collect::<Vec<_>>();
            let now = unix_time_ms();
            room.last_activity_unix_ms = now;
            if was_host {
                room.host_peer = None;
                room.expires_at_unix_ms =
                    now.saturating_add(self.inner.config.host_reconnect_grace.as_millis() as u64);
            } else {
                room.expires_at_unix_ms =
                    now.saturating_add(self.inner.config.room_ttl.as_millis() as u64);
            }
            (remaining, was_host, peer_label)
        };

        self.inner
            .metrics
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);
        for peer in remaining {
            let _ = peer.send(OutboundMessage::Event(JsonPeerEvent::PeerLeft(peer_id)));
        }
        info!(%peer_id, peer = %peer_label, host = was_host, "peer disconnected");
    }
}

#[derive(Debug)]
struct ValidatedSignalQuery {
    name: PlayerName,
    role: PeerRole,
    ticket: Option<String>,
    build_id: BuildId,
}

impl TryFrom<SignalQuery> for ValidatedSignalQuery {
    type Error = ApiError;

    fn try_from(value: SignalQuery) -> Result<Self, Self::Error> {
        Ok(Self {
            name: PlayerName::new(value.name)
                .map_err(|error| ApiError::bad_request("invalid_player_name", error.to_string()))?,
            role: value.role,
            ticket: value.ticket,
            build_id: BuildId::new(value.build_id)
                .map_err(|error| ApiError::bad_request("invalid_build_id", error.to_string()))?,
        })
    }
}

fn authorize_room_connection(room: &Room, query: &ValidatedSignalQuery) -> Result<(), ApiError> {
    if room.build_id != query.build_id {
        return Err(ApiError::conflict(
            "build_mismatch",
            format!(
                "room requires build {}, client reported {}",
                room.build_id, query.build_id
            ),
        ));
    }
    if room.peers.len() >= usize::from(room.max_peers) {
        return Err(ApiError::conflict("room_full", "the room is full"));
    }

    match query.role {
        PeerRole::Host => {
            if room.host_peer.is_some() {
                return Err(ApiError::conflict(
                    "host_already_connected",
                    "the room host is already connected",
                ));
            }
            if room.host_ticket.as_deref() != query.ticket.as_deref() {
                return Err(ApiError::forbidden(
                    "invalid_host_ticket",
                    "a valid host resume ticket is required",
                ));
            }
        }
        PeerRole::Player | PeerRole::Spectator => {
            if room.host_peer.is_none() {
                return Err(ApiError::conflict(
                    "host_not_connected",
                    "the room host has not connected yet",
                ));
            }
            if room.visibility == RoomVisibility::Private
                && room.join_token.as_deref() != query.ticket.as_deref()
            {
                return Err(ApiError::forbidden(
                    "invalid_join_token",
                    "a valid private-room token is required",
                ));
            }
        }
    }
    Ok(())
}

pub fn build_router(state: AppState) -> Router {
    let cors = cors_layer(&state.inner.config);
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(health))
        .route("/metrics", get(metrics))
        .route("/v1/config", get(service_config))
        .route("/v1/rooms", post(create_room).get(list_rooms))
        .route(
            "/v1/rooms/{game_id}/{game_protocol}/{room_code}",
            get(get_room),
        )
        .route(
            "/v1/signal/{game_id}/{game_protocol}/{room_code}",
            get(signal_websocket),
        )
        .layer(cors)
        .with_state(state)
}

fn cors_layer(config: &ServerConfig) -> CorsLayer {
    let allow_origin = if config.allowed_origins.iter().any(|origin| origin == "*") {
        AllowOrigin::any()
    } else {
        let allowed_origins = config.allowed_origins.clone();
        AllowOrigin::predicate(move |origin, _request| {
            origin
                .to_str()
                .is_ok_and(|origin| allowed_origins.iter().any(|allowed| allowed == origin))
        })
    };
    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_headers(Any)
        .allow_methods(Any)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "open-bevy-signaling",
    })
}

async fn service_config(
    State(state): State<AppState>,
) -> Result<Json<ServiceConfigResponse>, ApiError> {
    Ok(Json(ServiceConfigResponse {
        service: "open-bevy-signaling".to_string(),
        api_version: open_bevy_protocol::API_VERSION.to_string(),
        min_session_protocol: SESSION_PROTOCOL_VERSION,
        max_session_protocol: SESSION_PROTOCOL_VERSION,
        default_max_peers: open_bevy_protocol::DEFAULT_MAX_PEERS,
        max_peers: MAX_PEERS,
        websocket_base_url: state.inner.config.public_websocket_base_url.clone(),
        ice_servers: issued_ice_servers(&state.inner.config).await?,
    }))
}

async fn create_room(
    State(state): State<AppState>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<CreateRoomResponse>), ApiError> {
    state
        .create_room(request)
        .await
        .map(|response| (StatusCode::CREATED, Json(response)))
}

async fn list_rooms(
    State(state): State<AppState>,
    Query(query): Query<ListRoomsQuery>,
) -> Result<Json<RoomListResponse>, ApiError> {
    let game_filter = query
        .game_id
        .map(GameId::new)
        .transpose()
        .map_err(|error| ApiError::bad_request("invalid_game_id", error.to_string()))?;
    let rooms = state.inner.rooms.read().await;
    let mut descriptors = rooms
        .iter()
        .filter(|(key, room)| {
            room.visibility == RoomVisibility::Public
                && game_filter
                    .as_ref()
                    .is_none_or(|game_id| &key.game_id == game_id)
                && query
                    .game_protocol
                    .is_none_or(|version| key.game_protocol == version)
        })
        .map(|(key, room)| room_descriptor(key, room))
        .collect::<Vec<_>>();
    descriptors.sort_by_key(|room| std::cmp::Reverse(room.created_at_unix_ms));
    Ok(Json(RoomListResponse { rooms: descriptors }))
}

async fn get_room(
    State(state): State<AppState>,
    Path((game_id, game_protocol, room_code)): Path<(String, u16, String)>,
) -> Result<Json<RoomDescriptor>, ApiError> {
    let key = parse_room_key(game_id, game_protocol, room_code)?;
    let rooms = state.inner.rooms.read().await;
    let room = rooms
        .get(&key)
        .ok_or_else(|| ApiError::not_found("the requested room does not exist"))?;
    Ok(Json(room_descriptor(&key, room)))
}

async fn signal_websocket(
    State(state): State<AppState>,
    Path((game_id, game_protocol, room_code)): Path<(String, u16, String)>,
    Query(query): Query<SignalQuery>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    validate_origin(&state.inner.config, &headers)?;
    let key = parse_room_key(game_id, game_protocol, room_code)?;
    let query = ValidatedSignalQuery::try_from(query)?;
    if let Err(error) = state.preflight(&key, &query).await {
        state
            .inner
            .metrics
            .connections_rejected
            .fetch_add(1, Ordering::Relaxed);
        return Err(error);
    }

    Ok(websocket
        .max_message_size(MAX_SIGNAL_BYTES)
        .max_frame_size(MAX_SIGNAL_BYTES)
        .on_upgrade(move |socket| run_websocket(state, key, query, socket)))
}

async fn run_websocket(
    state: AppState,
    key: RoomKey,
    query: ValidatedSignalQuery,
    socket: WebSocket,
) {
    let registration = match state.register_peer(&key, &query).await {
        Ok(registration) => registration,
        Err(error) => {
            state
                .inner
                .metrics
                .connections_rejected
                .fetch_add(1, Ordering::Relaxed);
            let (mut sender, _) = socket.split();
            let _ = sender
                .send(Message::Close(Some(CloseFrame {
                    code: CLOSE_POLICY_VIOLATION,
                    reason: format!("{}: {}", error.code, error.message).into(),
                })))
                .await;
            return;
        }
    };

    let Registration {
        peer_id,
        mut outbound,
        existing_peers,
    } = registration;
    let (mut websocket_sender, mut websocket_receiver) = socket.split();
    let (writer_finished_sender, mut writer_finished_receiver) = mpsc::channel(1);

    let writer = tokio::spawn(async move {
        while let Some(message) = outbound.recv().await {
            let result = match message {
                OutboundMessage::Event(event) => {
                    websocket_sender
                        .send(Message::Text(event.to_string().into()))
                        .await
                }
                OutboundMessage::Close { code, reason } => {
                    let result = websocket_sender
                        .send(Message::Close(Some(CloseFrame {
                            code,
                            reason: reason.into(),
                        })))
                        .await;
                    let _ = writer_finished_sender.send(()).await;
                    return result;
                }
            };
            if let Err(error) = result {
                debug!(%peer_id, %error, "websocket writer stopped");
                break;
            }
        }
        let _ = writer_finished_sender.send(()).await;
        Ok(())
    });

    let own_sender = {
        let rooms = state.inner.rooms.read().await;
        rooms
            .get(&key)
            .and_then(|room| room.peers.get(&peer_id))
            .map(|peer| peer.outbound.clone())
    };
    let Some(own_sender) = own_sender else {
        writer.abort();
        return;
    };
    let _ = own_sender.send(OutboundMessage::Event(JsonPeerEvent::IdAssigned(peer_id)));
    for peer in existing_peers {
        let _ = peer.send(OutboundMessage::Event(JsonPeerEvent::NewPeer(peer_id)));
    }
    info!(
        %peer_id,
        game = %key.game_id,
        room = %key.room_code,
        role = ?query.role,
        "peer connected"
    );

    let mut invalid_messages = 0_u8;
    loop {
        tokio::select! {
            _ = writer_finished_receiver.recv() => break,
            incoming = websocket_receiver.next() => {
                let Some(incoming) = incoming else { break };
                let message = match incoming {
                    Ok(message) => message,
                    Err(error) => {
                        debug!(%peer_id, %error, "websocket reader stopped");
                        break;
                    }
                };
                match message {
                    Message::Text(text) => match text.as_str().parse::<JsonPeerRequest>() {
                        Ok(JsonPeerRequest::Signal { receiver, data }) => {
                            if !state.relay_signal(&key, peer_id, receiver, data).await {
                                warn!(%peer_id, %receiver, "dropped signal to unknown room peer");
                            }
                        }
                        Ok(JsonPeerRequest::KeepAlive) => {}
                        Err(error) => {
                            invalid_messages = invalid_messages.saturating_add(1);
                            state.inner.metrics.invalid_messages.fetch_add(1, Ordering::Relaxed);
                            warn!(%peer_id, %error, "invalid signaling message");
                            if invalid_messages >= MAX_INVALID_MESSAGES {
                                let _ = own_sender.send(OutboundMessage::Close {
                                    code: CLOSE_POLICY_VIOLATION,
                                    reason: "too many invalid signaling messages".to_string(),
                                });
                                break;
                            }
                        }
                    },
                    Message::Close(_) => break,
                    Message::Binary(_) => {
                        invalid_messages = invalid_messages.saturating_add(1);
                        state.inner.metrics.invalid_messages.fetch_add(1, Ordering::Relaxed);
                        if invalid_messages >= MAX_INVALID_MESSAGES {
                            let _ = own_sender.send(OutboundMessage::Close {
                                code: CLOSE_POLICY_VIOLATION,
                                reason: "binary signaling messages are unsupported".to_string(),
                            });
                            break;
                        }
                    }
                    Message::Ping(_) | Message::Pong(_) => {}
                }
            }
        }
    }

    state.disconnect_peer(&key, peer_id).await;
    writer.abort();
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let rooms = state.inner.rooms.read().await.len();
    let metrics = &state.inner.metrics;
    let body = format!(
        concat!(
            "# TYPE open_bevy_rooms gauge\n",
            "open_bevy_rooms {rooms}\n",
            "# TYPE open_bevy_active_connections gauge\n",
            "open_bevy_active_connections {active}\n",
            "# TYPE open_bevy_rooms_created_total counter\n",
            "open_bevy_rooms_created_total {created}\n",
            "# TYPE open_bevy_connections_accepted_total counter\n",
            "open_bevy_connections_accepted_total {accepted}\n",
            "# TYPE open_bevy_connections_rejected_total counter\n",
            "open_bevy_connections_rejected_total {rejected}\n",
            "# TYPE open_bevy_signals_relayed_total counter\n",
            "open_bevy_signals_relayed_total {signals}\n",
            "# TYPE open_bevy_invalid_messages_total counter\n",
            "open_bevy_invalid_messages_total {invalid}\n"
        ),
        rooms = rooms,
        active = metrics.active_connections.load(Ordering::Relaxed),
        created = metrics.rooms_created.load(Ordering::Relaxed),
        accepted = metrics.connections_accepted.load(Ordering::Relaxed),
        rejected = metrics.connections_rejected.load(Ordering::Relaxed),
        signals = metrics.signals_relayed.load(Ordering::Relaxed),
        invalid = metrics.invalid_messages.load(Ordering::Relaxed),
    );
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}

fn validate_origin(config: &ServerConfig, headers: &HeaderMap) -> Result<(), ApiError> {
    if config.allowed_origins.iter().any(|origin| origin == "*") {
        return Ok(());
    }
    let Some(origin) = headers.get(axum::http::header::ORIGIN) else {
        // Browsers always attach Origin to WebSocket handshakes. Native game
        // clients generally cannot, and an Origin value is not authentication
        // because a non-browser client could forge it anyway.
        return Ok(());
    };
    let origin = origin
        .to_str()
        .map_err(|_| ApiError::forbidden("origin_not_allowed", "the Origin is invalid"))?;
    if config
        .allowed_origins
        .iter()
        .any(|allowed| allowed == origin)
    {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "origin_not_allowed",
            "the websocket Origin is not allowed",
        ))
    }
}

fn parse_room_key(
    game_id: String,
    game_protocol: u16,
    room_code: String,
) -> Result<RoomKey, ApiError> {
    if game_protocol == 0 {
        return Err(ApiError::bad_request(
            "invalid_game_protocol",
            "game protocol must be greater than zero",
        ));
    }
    Ok(RoomKey {
        game_id: GameId::new(game_id)
            .map_err(|error| ApiError::bad_request("invalid_game_id", error.to_string()))?,
        game_protocol,
        room_code: RoomCode::new(room_code)
            .map_err(|error| ApiError::bad_request("invalid_room_code", error.to_string()))?,
    })
}

fn room_descriptor(key: &RoomKey, room: &Room) -> RoomDescriptor {
    RoomDescriptor {
        game_id: key.game_id.clone(),
        build_id: room.build_id.clone(),
        session_protocol: SESSION_PROTOCOL_VERSION,
        game_protocol: key.game_protocol,
        room_code: key.room_code.clone(),
        visibility: room.visibility,
        max_peers: room.max_peers,
        peer_count: u16::try_from(room.peers.len()).unwrap_or(u16::MAX),
        host_connected: room.host_peer.is_some(),
        created_at_unix_ms: room.created_at_unix_ms,
        expires_at_unix_ms: room.expires_at_unix_ms,
        metadata: room.metadata.clone(),
    }
}

fn generate_room_code() -> RoomCode {
    let encoded = Uuid::new_v4().simple().to_string().to_ascii_uppercase();
    RoomCode::new(&encoded[..8]).expect("UUID hex is a valid room code")
}

fn random_secret() -> String {
    Uuid::new_v4().simple().to_string()
}

async fn issued_ice_servers(config: &ServerConfig) -> Result<Vec<IceServer>, ApiError> {
    let Some(turn) = &config.cloudflare_turn else {
        return Ok(config.ice_servers.clone());
    };
    let ttl = turn.credential_ttl.as_secs().clamp(60, 172_800);
    let endpoint = format!(
        "{CLOUDFLARE_TURN_API_BASE}/{}/credentials/generate-ice-servers",
        turn.key_id
    );
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(&turn.api_token)
        .timeout(Duration::from_secs(10))
        .json(&serde_json::json!({ "ttl": ttl }))
        .send()
        .await
        .map_err(|error| {
            warn!(%error, "Cloudflare TURN credential request failed");
            ApiError::upstream("Cloudflare TURN credential request failed")
        })?;
    if !response.status().is_success() {
        let status = response.status();
        warn!(%status, "Cloudflare TURN credential API rejected the request");
        return Err(ApiError::upstream(format!(
            "Cloudflare TURN credential API returned HTTP {status}"
        )));
    }
    let payload = response
        .json::<CloudflareTurnResponse>()
        .await
        .map_err(|error| {
            warn!(%error, "Cloudflare TURN credential response was invalid");
            ApiError::upstream("Cloudflare TURN credential response was invalid")
        })?;
    validate_cloudflare_ice_servers(payload.ice_servers)
}

fn validate_cloudflare_ice_servers(servers: Vec<IceServer>) -> Result<Vec<IceServer>, ApiError> {
    let has_authenticated_turn = servers.iter().any(|server| {
        server.username.is_some()
            && server.credential.is_some()
            && server
                .urls
                .iter()
                .any(|url| url.starts_with("turn:") || url.starts_with("turns:"))
    });
    if !has_authenticated_turn {
        return Err(ApiError::upstream(
            "Cloudflare TURN credential API returned no authenticated TURN server",
        ));
    }
    Ok(servers)
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn expired_waiting_rooms_are_collected() {
        let state = AppState::new(ServerConfig {
            room_ttl: Duration::ZERO,
            ..ServerConfig::default()
        });
        state
            .create_room(CreateRoomRequest {
                game_id: GameId::new("test-game").unwrap(),
                build_id: BuildId::new("test-build").unwrap(),
                session_protocol: SESSION_PROTOCOL_VERSION,
                game_protocol: 7,
                max_peers: 4,
                visibility: RoomVisibility::Public,
                metadata: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(state.remove_expired_rooms().await, 1);
    }

    #[test]
    fn non_matching_builds_cannot_join() {
        let room = Room {
            build_id: BuildId::new("server-build").unwrap(),
            visibility: RoomVisibility::Public,
            max_peers: 4,
            metadata: Default::default(),
            host_ticket: Some("host".to_string()),
            join_token: None,
            host_peer: Some(PeerId(Uuid::new_v4())),
            peers: HashMap::new(),
            created_at_unix_ms: 0,
            expires_at_unix_ms: 1,
            last_activity_unix_ms: 0,
        };
        let query = ValidatedSignalQuery {
            name: PlayerName::new("Player").unwrap(),
            role: PeerRole::Player,
            ticket: None,
            build_id: BuildId::new("other-build").unwrap(),
        };
        assert_eq!(
            authorize_room_connection(&room, &query).unwrap_err().code,
            "build_mismatch"
        );
    }

    #[test]
    fn cloudflare_credentials_require_an_authenticated_turn_server() {
        let servers = vec![
            IceServer {
                urls: vec!["stun:stun.cloudflare.com:3478".to_string()],
                username: None,
                credential: None,
            },
            IceServer {
                urls: vec!["turns:turn.cloudflare.com:443?transport=tcp".to_string()],
                username: Some("short-lived-user".to_string()),
                credential: Some("short-lived-credential".to_string()),
            },
        ];
        assert_eq!(
            validate_cloudflare_ice_servers(servers.clone()).unwrap(),
            servers
        );

        let error = validate_cloudflare_ice_servers(vec![IceServer {
            urls: vec!["stun:stun.cloudflare.com:3478".to_string()],
            username: None,
            credential: None,
        }])
        .unwrap_err();
        assert_eq!(error.code, "turn_credentials_unavailable");
    }
}
