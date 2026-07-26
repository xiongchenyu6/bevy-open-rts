//! Cloudflare Worker deployment adapter for `open-bevy-signaling`.
//!
//! The public HTTP and Matchbox WebSocket contracts are identical to the
//! native Axum service. A singleton directory Durable Object owns discovery,
//! while each game room has an isolated hibernatable Durable Object.

use matchbox_protocol::{JsonPeerEvent, JsonPeerRequest, PeerId};
use open_bevy_protocol::{
    BuildId, CreateRoomRequest, CreateRoomResponse, ErrorResponse, GameId, IceServer, MAX_PEERS,
    PeerRole, PlayerName, RoomCode, RoomDescriptor, RoomListResponse, RoomVisibility,
    SESSION_PROTOCOL_VERSION, ServiceConfigResponse, signaling_path,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;
use worker::{wasm_bindgen::JsValue, *};

const SERVICE_NAME: &str = "open-bevy-signaling";
const DIRECTORY_NAME: &str = "open-bevy-signaling-directory";
const DIRECTORY_KEYS: &str = "room_keys";
const ROOM_STATE_KEY: &str = "room_state";
const MAX_SIGNAL_BYTES: usize = 256 * 1024;
const MAX_INVALID_MESSAGES: u8 = 3;
const CLOSE_POLICY_VIOLATION: u16 = 1008;
const CLOSE_SERVER_ERROR: u16 = 1011;

#[derive(Debug)]
struct ApiFailure {
    status: u16,
    code: &'static str,
    message: String,
}

impl ApiFailure {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: 400,
            code,
            message: message.into(),
        }
    }

    fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: 403,
            code,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: 404,
            code: "room_not_found",
            message: message.into(),
        }
    }

    fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: 409,
            code,
            message: message.into(),
        }
    }

    fn upstream(message: impl Into<String>) -> Self {
        Self {
            status: 502,
            code: "turn_credentials_unavailable",
            message: message.into(),
        }
    }

    fn response(self) -> Result<Response> {
        Ok(Response::from_json(&ErrorResponse {
            code: self.code.to_string(),
            message: self.message,
        })?
        .with_status(self.status))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoomKey {
    game_id: GameId,
    game_protocol: u16,
    room_code: RoomCode,
}

impl RoomKey {
    fn new(game_id: String, game_protocol: u16, room_code: String) -> Result<Self, ApiFailure> {
        if game_protocol == 0 {
            return Err(ApiFailure::bad_request(
                "invalid_game_protocol",
                "game protocol must be greater than zero",
            ));
        }
        Ok(Self {
            game_id: GameId::new(game_id)
                .map_err(|error| ApiFailure::bad_request("invalid_game_id", error.to_string()))?,
            game_protocol,
            room_code: RoomCode::new(room_code)
                .map_err(|error| ApiFailure::bad_request("invalid_room_code", error.to_string()))?,
        })
    }

    fn storage_key(&self) -> String {
        format!(
            "room:{}:{}:{}",
            self.game_id, self.game_protocol, self.room_code
        )
    }

    fn object_name(&self) -> String {
        format!("{}:{}:{}", self.game_id, self.game_protocol, self.room_code)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRoom {
    descriptor: RoomDescriptor,
    host_ticket: String,
    join_token: Option<String>,
    last_activity_unix_ms: u64,
}

impl PersistedRoom {
    fn key(&self) -> RoomKey {
        RoomKey {
            game_id: self.descriptor.game_id.clone(),
            game_protocol: self.descriptor.game_protocol,
            room_code: self.descriptor.room_code.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PeerAttachment {
    peer_id: Uuid,
    name: PlayerName,
    role: PeerRole,
    invalid_messages: u8,
}

#[derive(Debug, Deserialize)]
struct SignalQuery {
    name: String,
    #[serde(default)]
    role: PeerRole,
    ticket: Option<String>,
    build_id: String,
}

#[derive(Debug)]
struct ValidatedSignalQuery {
    name: PlayerName,
    role: PeerRole,
    ticket: Option<String>,
    build_id: BuildId,
}

impl TryFrom<SignalQuery> for ValidatedSignalQuery {
    type Error = ApiFailure;

    fn try_from(value: SignalQuery) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: PlayerName::new(value.name).map_err(|error| {
                ApiFailure::bad_request("invalid_player_name", error.to_string())
            })?,
            role: value.role,
            ticket: value.ticket,
            build_id: BuildId::new(value.build_id)
                .map_err(|error| ApiFailure::bad_request("invalid_build_id", error.to_string()))?,
        })
    }
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
    deployment: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryUpsert {
    descriptor: RoomDescriptor,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryRemove {
    key: RoomKey,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudflareTurnResponse {
    ice_servers: Vec<IceServer>,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _context: Context) -> Result<Response> {
    let origin = req.headers().get("Origin")?;
    if req.method() == Method::Options {
        return with_cors(Response::empty()?, origin.as_deref(), &env);
    }

    let path = req.path();
    let method = req.method();
    let response = match (method, path.as_str()) {
        (Method::Get, "/") => Response::from_json(&HealthResponse {
            status: "ok",
            service: SERVICE_NAME,
            deployment: "cloudflare-worker",
        }),
        (Method::Get, "/healthz" | "/readyz") => Response::from_json(&HealthResponse {
            status: "ok",
            service: SERVICE_NAME,
            deployment: "cloudflare-worker",
        }),
        (Method::Get, "/metrics") => Response::ok(concat!(
            "# TYPE open_bevy_signaling_worker_up gauge\n",
            "open_bevy_signaling_worker_up 1\n"
        )),
        (Method::Get, "/v1/config") => service_config_response(&env).await,
        (Method::Post | Method::Get, "/v1/rooms") => {
            directory_stub(&env)?.fetch_with_request(req).await
        }
        (Method::Get, _) if path.starts_with("/v1/rooms/") => {
            match parse_room_path(&path, "rooms") {
                Ok(key) => room_stub(&env, &key)?.fetch_with_request(req).await,
                Err(error) => error.response(),
            }
        }
        (Method::Get, _) if path.starts_with("/v1/signal/") => {
            if let Err(error) = validate_websocket_origin(origin.as_deref(), &env) {
                error.response()
            } else {
                match parse_room_path(&path, "signal") {
                    Ok(key) => room_stub(&env, &key)?.fetch_with_request(req).await,
                    Err(error) => error.response(),
                }
            }
        }
        _ => ApiFailure {
            status: 404,
            code: "not_found",
            message: "the requested endpoint does not exist".to_string(),
        }
        .response(),
    }?;

    if response.status_code() == 101 {
        Ok(response)
    } else {
        with_cors(response, origin.as_deref(), &env)
    }
}

async fn service_config_response(env: &Env) -> Result<Response> {
    let ice_servers = match issued_ice_servers(env).await {
        Ok(servers) => servers,
        Err(error) => return ApiFailure::upstream(error.to_string()).response(),
    };
    let mut response = Response::from_json(&ServiceConfigResponse {
        service: SERVICE_NAME.to_string(),
        api_version: open_bevy_protocol::API_VERSION.to_string(),
        min_session_protocol: SESSION_PROTOCOL_VERSION,
        max_session_protocol: SESSION_PROTOCOL_VERSION,
        default_max_peers: open_bevy_protocol::DEFAULT_MAX_PEERS,
        max_peers: MAX_PEERS,
        websocket_base_url: websocket_base_url(env)?,
        ice_servers,
    })?;
    response.headers_mut().set("Cache-Control", "no-store")?;
    Ok(response)
}

async fn issued_ice_servers(env: &Env) -> Result<Vec<IceServer>> {
    let cloudflare_key = optional_secret(env, "CLOUDFLARE_TURN_KEY_ID");
    let cloudflare_token = optional_secret(env, "CLOUDFLARE_TURN_API_TOKEN");
    match (cloudflare_key, cloudflare_token) {
        (Some(key), Some(token)) => cloudflare_turn_credentials(env, &key, &token).await,
        (None, None) => Ok(vec![IceServer {
            urls: vec!["stun:stun.cloudflare.com:3478".to_string()],
            username: None,
            credential: None,
        }]),
        _ => Err(Error::RustError(
            "both CLOUDFLARE_TURN_KEY_ID and CLOUDFLARE_TURN_API_TOKEN are required".to_string(),
        )),
    }
}

async fn cloudflare_turn_credentials(env: &Env, key: &str, token: &str) -> Result<Vec<IceServer>> {
    let ttl = env_u64(env, "TURN_CREDENTIAL_TTL_SECONDS", 3600).clamp(60, 172_800);
    let endpoint = format!(
        "https://rtc.live.cloudflare.com/v1/turn/keys/{key}/credentials/generate-ice-servers"
    );
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {token}"))?;
    headers.set("Content-Type", "application/json")?;
    let body = serde_json::to_string(&serde_json::json!({ "ttl": ttl }))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));
    let request = Request::new_with_init(&endpoint, &init)?;
    let mut response = Fetch::Request(request).send().await?;
    if !(200..300).contains(&response.status_code()) {
        return Err(Error::RustError(format!(
            "Cloudflare TURN credential API returned HTTP {}",
            response.status_code()
        )));
    }
    let payload: CloudflareTurnResponse = response.json().await?;
    let has_authenticated_turn = payload.ice_servers.iter().any(|server| {
        server.username.is_some()
            && server.credential.is_some()
            && server
                .urls
                .iter()
                .any(|url| url.starts_with("turn:") || url.starts_with("turns:"))
    });
    if !has_authenticated_turn {
        return Err(Error::RustError(
            "Cloudflare TURN credential API returned no authenticated TURN server".to_string(),
        ));
    }
    Ok(payload.ice_servers)
}

fn with_cors(response: Response, origin: Option<&str>, env: &Env) -> Result<Response> {
    // Responses returned by Durable Object stubs carry immutable Fetch headers.
    // Replacing them with a mutable clone preserves the body/status while
    // allowing the outer Worker to apply its CORS policy.
    let headers = response.headers().clone();
    let mut response = response.with_headers(headers);
    match origin {
        Some(origin) if origin_allowed(origin, env) => response
            .headers_mut()
            .set("Access-Control-Allow-Origin", origin)?,
        None => response
            .headers_mut()
            .set("Access-Control-Allow-Origin", "*")?,
        Some(_) => {}
    }
    response
        .headers_mut()
        .set("Access-Control-Allow-Headers", "Content-Type")?;
    response
        .headers_mut()
        .set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")?;
    response.headers_mut().set("Vary", "Origin")?;
    Ok(response)
}

fn validate_websocket_origin(origin: Option<&str>, env: &Env) -> Result<(), ApiFailure> {
    if origin.is_none_or(|origin| origin_allowed(origin, env)) {
        Ok(())
    } else {
        Err(ApiFailure::forbidden(
            "origin_not_allowed",
            "the websocket Origin is not allowed",
        ))
    }
}

fn origin_allowed(origin: &str, env: &Env) -> bool {
    env_var(env, "ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "*".to_string())
        .split(',')
        .map(str::trim)
        .any(|allowed| allowed == "*" || allowed == origin)
}

fn parse_room_path(path: &str, endpoint: &str) -> Result<RoomKey, ApiFailure> {
    let segments = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() != 5 || segments[0] != "v1" || segments[1] != endpoint {
        return Err(ApiFailure::bad_request(
            "invalid_room_path",
            "room paths must contain game id, protocol, and room code",
        ));
    }
    let protocol = segments[3].parse::<u16>().map_err(|_| {
        ApiFailure::bad_request("invalid_game_protocol", "game protocol must be a u16")
    })?;
    RoomKey::new(segments[2].to_string(), protocol, segments[4].to_string())
}

fn room_stub(env: &Env, key: &RoomKey) -> Result<Stub> {
    env.durable_object("ROOMS")?.get_by_name(&key.object_name())
}

fn directory_stub(env: &Env) -> Result<Stub> {
    env.durable_object("DIRECTORY")?.get_by_name(DIRECTORY_NAME)
}

fn public_base_url(env: &Env) -> Result<String> {
    Ok(env_var(env, "PUBLIC_BASE_URL")?
        .trim_end_matches('/')
        .to_string())
}

fn websocket_base_url(env: &Env) -> Result<String> {
    let public = public_base_url(env)?;
    if let Some(rest) = public.strip_prefix("https://") {
        Ok(format!("wss://{rest}"))
    } else if let Some(rest) = public.strip_prefix("http://") {
        Ok(format!("ws://{rest}"))
    } else {
        Err(Error::RustError(
            "PUBLIC_BASE_URL must use http or https".to_string(),
        ))
    }
}

fn env_var(env: &Env, name: &str) -> Result<String> {
    env.var(name).map(|value| value.to_string())
}

fn optional_secret(env: &Env, name: &str) -> Option<String> {
    env.secret(name).ok().map(|value| value.to_string())
}

fn env_u64(env: &Env, name: &str, default: u64) -> u64 {
    env.var(name)
        .ok()
        .and_then(|value| value.to_string().parse().ok())
        .unwrap_or(default)
}

fn unix_time_ms() -> u64 {
    Date::now().as_millis()
}

fn generate_room_code() -> RoomCode {
    let encoded = Uuid::new_v4().simple().to_string().to_ascii_uppercase();
    RoomCode::new(&encoded[..8]).expect("UUID hex is a valid room code")
}

fn random_secret() -> String {
    Uuid::new_v4().simple().to_string()
}

fn request_with_json<T: Serialize>(url: &str, method: Method, value: &T) -> Result<Request> {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    let body = serde_json::to_string(value)?;
    let mut init = RequestInit::new();
    init.with_method(method)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));
    Request::new_with_init(url, &init)
}

#[durable_object]
pub struct RoomDirectory {
    state: State,
    env: Env,
}

impl DurableObject for RoomDirectory {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/v1/rooms") => self.create_room(&mut req).await,
            (Method::Get, "/v1/rooms") => self.list_rooms(&req).await,
            (Method::Post, "/internal/upsert") => {
                let update: DirectoryUpsert = req.json().await?;
                self.upsert(update.descriptor).await?;
                Response::empty()
            }
            (Method::Post, "/internal/remove") => {
                let remove: DirectoryRemove = req.json().await?;
                self.remove(&remove.key).await?;
                Response::empty()
            }
            _ => Response::error("directory endpoint not found", 404),
        }
    }
}

impl RoomDirectory {
    async fn create_room(&self, req: &mut Request) -> Result<Response> {
        let request = match req.json::<CreateRoomRequest>().await {
            Ok(request) => request,
            Err(error) => {
                return ApiFailure::bad_request("invalid_json", error.to_string()).response();
            }
        };
        if let Err(error) = request.validate() {
            return ApiFailure::bad_request("invalid_room", error.to_string()).response();
        }
        if request.session_protocol != SESSION_PROTOCOL_VERSION {
            return ApiFailure::bad_request(
                "unsupported_session_protocol",
                format!(
                    "session protocol {} is unsupported; this service accepts session protocol {}",
                    request.session_protocol, SESSION_PROTOCOL_VERSION
                ),
            )
            .response();
        }

        let now = unix_time_ms();
        let expires_at = now.saturating_add(env_u64(&self.env, "ROOM_TTL_MS", 900_000));
        let host_ticket = random_secret();
        let join_token = (request.visibility == RoomVisibility::Private).then(random_secret);
        let (key, descriptor) = loop {
            let room_code = generate_room_code();
            let key = RoomKey {
                game_id: request.game_id.clone(),
                game_protocol: request.game_protocol,
                room_code: room_code.clone(),
            };
            if self
                .state
                .storage()
                .get::<RoomDescriptor>(&key.storage_key())
                .await?
                .is_none()
            {
                let descriptor = RoomDescriptor {
                    game_id: request.game_id.clone(),
                    build_id: request.build_id.clone(),
                    session_protocol: SESSION_PROTOCOL_VERSION,
                    game_protocol: request.game_protocol,
                    room_code,
                    visibility: request.visibility,
                    max_peers: request.max_peers,
                    peer_count: 0,
                    host_connected: false,
                    created_at_unix_ms: now,
                    expires_at_unix_ms: expires_at,
                    metadata: request.metadata.clone(),
                };
                break (key, descriptor);
            }
        };

        let persisted = PersistedRoom {
            descriptor: descriptor.clone(),
            host_ticket: host_ticket.clone(),
            join_token: join_token.clone(),
            last_activity_unix_ms: now,
        };
        let stub = room_stub(&self.env, &key)?;
        let init_request = request_with_json(
            "https://room.internal/internal/init",
            Method::Post,
            &persisted,
        )?;
        let initialized = stub.fetch_with_request(init_request).await?;
        if initialized.status_code() != 201 {
            return Err(Error::RustError(format!(
                "room Durable Object initialization returned HTTP {}",
                initialized.status_code()
            )));
        }
        self.upsert(descriptor.clone()).await?;

        let signaling_url = format!(
            "{}{}",
            websocket_base_url(&self.env)?,
            signaling_path(&key.game_id, key.game_protocol, &key.room_code)
        );
        Ok(Response::from_json(&CreateRoomResponse {
            room: descriptor,
            signaling_url,
            host_token: host_ticket,
            join_token,
        })?
        .with_status(201))
    }

    async fn list_rooms(&self, req: &Request) -> Result<Response> {
        let query = match req.query::<ListRoomsQuery>() {
            Ok(query) => query,
            Err(error) => {
                return ApiFailure::bad_request("invalid_query", error.to_string()).response();
            }
        };
        let game_filter = match query.game_id.map(GameId::new).transpose() {
            Ok(game) => game,
            Err(error) => {
                return ApiFailure::bad_request("invalid_game_id", error.to_string()).response();
            }
        };
        let mut keys = self.keys().await?;
        let now = unix_time_ms();
        let mut rooms = Vec::new();
        let mut retained = Vec::with_capacity(keys.len());
        for key in keys.drain(..) {
            let Some(room) = self.state.storage().get::<RoomDescriptor>(&key).await? else {
                continue;
            };
            if !room.host_connected && room.expires_at_unix_ms <= now {
                let _ = self.state.storage().delete(&key).await?;
                continue;
            }
            retained.push(key);
            if room.visibility == RoomVisibility::Public
                && game_filter
                    .as_ref()
                    .is_none_or(|game| game == &room.game_id)
                && query
                    .game_protocol
                    .is_none_or(|protocol| protocol == room.game_protocol)
            {
                rooms.push(room);
            }
        }
        if retained != self.keys().await? {
            self.state.storage().put(DIRECTORY_KEYS, &retained).await?;
        }
        rooms.sort_by_key(|room| std::cmp::Reverse(room.created_at_unix_ms));
        Response::from_json(&RoomListResponse { rooms })
    }

    async fn keys(&self) -> Result<Vec<String>> {
        Ok(self
            .state
            .storage()
            .get(DIRECTORY_KEYS)
            .await?
            .unwrap_or_default())
    }

    async fn upsert(&self, descriptor: RoomDescriptor) -> Result<()> {
        let key = RoomKey {
            game_id: descriptor.game_id.clone(),
            game_protocol: descriptor.game_protocol,
            room_code: descriptor.room_code.clone(),
        }
        .storage_key();
        let mut keys = self.keys().await?;
        if !keys.contains(&key) {
            keys.push(key.clone());
            self.state.storage().put(DIRECTORY_KEYS, &keys).await?;
        }
        self.state.storage().put(&key, descriptor).await
    }

    async fn remove(&self, key: &RoomKey) -> Result<()> {
        let storage_key = key.storage_key();
        let _ = self.state.storage().delete(&storage_key).await?;
        let mut keys = self.keys().await?;
        keys.retain(|candidate| candidate != &storage_key);
        self.state.storage().put(DIRECTORY_KEYS, keys).await
    }
}

#[durable_object]
pub struct GameRoom {
    state: State,
    env: Env,
}

impl DurableObject for GameRoom {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/internal/init") => self.initialize(&mut req).await,
            (Method::Get, path) if path.starts_with("/v1/rooms/") => self.descriptor().await,
            (Method::Get, path) if path.starts_with("/v1/signal/") => self.connect(req).await,
            _ => Response::error("room endpoint not found", 404),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        let Some(mut room) = self.load().await? else {
            return Response::empty();
        };
        self.refresh_descriptor(&mut room);
        if room.descriptor.host_connected {
            self.persist_and_publish(&room).await?;
            self.state.storage().delete_alarm().await?;
            return Response::empty();
        }
        let now = unix_time_ms();
        if room.descriptor.expires_at_unix_ms > now {
            self.state
                .storage()
                .set_alarm(alarm_timestamp(room.descriptor.expires_at_unix_ms))
                .await?;
            return Response::empty();
        }
        for socket in self.state.get_websockets() {
            let _ = socket.close(
                Some(CLOSE_SERVER_ERROR),
                Some("room host reconnect window expired"),
            );
        }
        self.state.storage().delete_all().await?;
        self.remove_from_directory(&room.key()).await?;
        Response::empty()
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        let Some(mut peer) = ws.deserialize_attachment::<PeerAttachment>()? else {
            return ws.close(
                Some(CLOSE_POLICY_VIOLATION),
                Some("missing peer attachment"),
            );
        };
        let text = match message {
            WebSocketIncomingMessage::String(text) if text.len() <= MAX_SIGNAL_BYTES => text,
            WebSocketIncomingMessage::String(_) | WebSocketIncomingMessage::Binary(_) => {
                return self.invalid_message(&ws, &mut peer).await;
            }
        };
        match JsonPeerRequest::from_str(&text) {
            Ok(JsonPeerRequest::KeepAlive) => Ok(()),
            Ok(JsonPeerRequest::Signal { receiver, data }) => {
                let tag = peer_tag(receiver.0);
                if let Some(target) = self.state.get_websockets_with_tag(&tag).into_iter().next() {
                    send_event(
                        &target,
                        JsonPeerEvent::Signal {
                            sender: PeerId(peer.peer_id),
                            data,
                        },
                    )?;
                }
                Ok(())
            }
            Err(_) => self.invalid_message(&ws, &mut peer).await,
        }
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        self.disconnect(&ws).await
    }

    async fn websocket_error(&self, ws: WebSocket, _error: Error) -> Result<()> {
        self.disconnect(&ws).await
    }
}

impl GameRoom {
    async fn initialize(&self, req: &mut Request) -> Result<Response> {
        if self.load().await?.is_some() {
            return ApiFailure::conflict("room_exists", "the room already exists").response();
        }
        let room: PersistedRoom = req.json().await?;
        self.state.storage().put(ROOM_STATE_KEY, &room).await?;
        self.state
            .storage()
            .set_alarm(alarm_timestamp(room.descriptor.expires_at_unix_ms))
            .await?;
        Ok(Response::empty()?.with_status(201))
    }

    async fn descriptor(&self) -> Result<Response> {
        let Some(mut room) = self.load().await? else {
            return ApiFailure::not_found("the requested room does not exist").response();
        };
        self.refresh_descriptor(&mut room);
        if !room.descriptor.host_connected && room.descriptor.expires_at_unix_ms <= unix_time_ms() {
            self.state.storage().delete_all().await?;
            self.remove_from_directory(&room.key()).await?;
            return ApiFailure::not_found("the requested room has expired").response();
        }
        self.persist_and_publish(&room).await?;
        Response::from_json(&room.descriptor)
    }

    async fn connect(&self, req: Request) -> Result<Response> {
        let upgrade = req.headers().get("Upgrade")?.unwrap_or_default();
        if !upgrade.eq_ignore_ascii_case("websocket") {
            return Response::error("Expected Upgrade: websocket", 426);
        }
        let query = match req.query::<SignalQuery>() {
            Ok(query) => match ValidatedSignalQuery::try_from(query) {
                Ok(query) => query,
                Err(error) => return error.response(),
            },
            Err(error) => {
                return ApiFailure::bad_request("invalid_query", error.to_string()).response();
            }
        };
        let Some(mut room) = self.load().await? else {
            return ApiFailure::not_found("the requested room does not exist").response();
        };
        self.refresh_descriptor(&mut room);
        if !room.descriptor.host_connected && room.descriptor.expires_at_unix_ms <= unix_time_ms() {
            return ApiFailure::not_found("the requested room has expired").response();
        }
        if let Err(error) = authorize_room_connection(&room, &query) {
            return error.response();
        }

        let peer_id = Uuid::new_v4();
        let existing = self.connected_peers();
        let pair = WebSocketPair::new()?;
        let client = pair.client;
        let server = pair.server;
        let attachment = PeerAttachment {
            peer_id,
            name: query.name,
            role: query.role,
            invalid_messages: 0,
        };
        server.serialize_attachment(&attachment)?;
        let peer_tag = peer_tag(peer_id);
        let role_tag = role_tag(query.role);
        self.state
            .accept_websocket_with_tags(&server, &[&peer_tag, role_tag]);
        send_event(&server, JsonPeerEvent::IdAssigned(PeerId(peer_id)))?;
        for (socket, _) in &existing {
            let _ = send_event(socket, JsonPeerEvent::NewPeer(PeerId(peer_id)));
        }

        room.last_activity_unix_ms = unix_time_ms();
        room.descriptor.expires_at_unix_ms =
            room.last_activity_unix_ms
                .saturating_add(env_u64(&self.env, "ROOM_TTL_MS", 900_000));
        self.refresh_descriptor(&mut room);
        if room.descriptor.host_connected {
            self.state.storage().delete_alarm().await?;
        }
        self.persist_and_publish(&room).await?;
        Response::from_websocket(client)
    }

    async fn invalid_message(&self, ws: &WebSocket, peer: &mut PeerAttachment) -> Result<()> {
        peer.invalid_messages = peer.invalid_messages.saturating_add(1);
        ws.serialize_attachment(&*peer)?;
        if peer.invalid_messages >= MAX_INVALID_MESSAGES {
            ws.close(
                Some(CLOSE_POLICY_VIOLATION),
                Some("too many invalid signaling messages"),
            )?;
        }
        Ok(())
    }

    async fn disconnect(&self, ws: &WebSocket) -> Result<()> {
        let Some(leaving) = ws.deserialize_attachment::<PeerAttachment>()? else {
            return Ok(());
        };
        let remaining = self
            .connected_peers()
            .into_iter()
            .filter(|(_, peer)| peer.peer_id != leaving.peer_id)
            .collect::<Vec<_>>();
        for (socket, _) in &remaining {
            let _ = send_event(socket, JsonPeerEvent::PeerLeft(PeerId(leaving.peer_id)));
        }

        let Some(mut room) = self.load().await? else {
            return Ok(());
        };
        room.last_activity_unix_ms = unix_time_ms();
        room.descriptor.peer_count = u16::try_from(remaining.len()).unwrap_or(u16::MAX);
        room.descriptor.host_connected = remaining
            .iter()
            .any(|(_, peer)| peer.role == PeerRole::Host);
        if leaving.role == PeerRole::Host && !room.descriptor.host_connected {
            room.descriptor.expires_at_unix_ms = room
                .last_activity_unix_ms
                .saturating_add(env_u64(&self.env, "HOST_RECONNECT_GRACE_MS", 30_000));
            self.state
                .storage()
                .set_alarm(alarm_timestamp(room.descriptor.expires_at_unix_ms))
                .await?;
        }
        self.persist_and_publish(&room).await
    }

    async fn load(&self) -> Result<Option<PersistedRoom>> {
        self.state.storage().get(ROOM_STATE_KEY).await
    }

    fn connected_peers(&self) -> Vec<(WebSocket, PeerAttachment)> {
        self.state
            .get_websockets()
            .into_iter()
            .filter_map(|socket| {
                socket
                    .deserialize_attachment::<PeerAttachment>()
                    .ok()
                    .flatten()
                    .map(|peer| (socket, peer))
            })
            .collect()
    }

    fn refresh_descriptor(&self, room: &mut PersistedRoom) {
        let peers = self.connected_peers();
        room.descriptor.peer_count = u16::try_from(peers.len()).unwrap_or(u16::MAX);
        room.descriptor.host_connected = peers.iter().any(|(_, peer)| peer.role == PeerRole::Host);
    }

    async fn persist_and_publish(&self, room: &PersistedRoom) -> Result<()> {
        self.state.storage().put(ROOM_STATE_KEY, room).await?;
        let request = request_with_json(
            "https://directory.internal/internal/upsert",
            Method::Post,
            &DirectoryUpsert {
                descriptor: room.descriptor.clone(),
            },
        )?;
        let response = directory_stub(&self.env)?
            .fetch_with_request(request)
            .await?;
        if response.status_code() >= 300 {
            return Err(Error::RustError(format!(
                "directory update returned HTTP {}",
                response.status_code()
            )));
        }
        Ok(())
    }

    async fn remove_from_directory(&self, key: &RoomKey) -> Result<()> {
        let request = request_with_json(
            "https://directory.internal/internal/remove",
            Method::Post,
            &DirectoryRemove { key: key.clone() },
        )?;
        let response = directory_stub(&self.env)?
            .fetch_with_request(request)
            .await?;
        if response.status_code() >= 300 {
            return Err(Error::RustError(format!(
                "directory removal returned HTTP {}",
                response.status_code()
            )));
        }
        Ok(())
    }
}

fn authorize_room_connection(
    room: &PersistedRoom,
    query: &ValidatedSignalQuery,
) -> Result<(), ApiFailure> {
    if room.descriptor.build_id != query.build_id {
        return Err(ApiFailure::conflict(
            "build_mismatch",
            format!(
                "room requires build {}, client reported {}",
                room.descriptor.build_id, query.build_id
            ),
        ));
    }
    if room.descriptor.peer_count >= room.descriptor.max_peers {
        return Err(ApiFailure::conflict("room_full", "the room is full"));
    }
    match query.role {
        PeerRole::Host => {
            if room.descriptor.host_connected {
                return Err(ApiFailure::conflict(
                    "host_already_connected",
                    "the room host is already connected",
                ));
            }
            if Some(room.host_ticket.as_str()) != query.ticket.as_deref() {
                return Err(ApiFailure::forbidden(
                    "invalid_host_ticket",
                    "a valid host resume ticket is required",
                ));
            }
        }
        PeerRole::Player | PeerRole::Spectator => {
            if !room.descriptor.host_connected {
                return Err(ApiFailure::conflict(
                    "host_not_connected",
                    "the room host has not connected yet",
                ));
            }
            if room.descriptor.visibility == RoomVisibility::Private
                && room.join_token.as_deref() != query.ticket.as_deref()
            {
                return Err(ApiFailure::forbidden(
                    "invalid_join_token",
                    "a valid private-room token is required",
                ));
            }
        }
    }
    Ok(())
}

fn send_event(socket: &WebSocket, event: JsonPeerEvent) -> Result<()> {
    socket.send_with_str(event.to_string())
}

fn peer_tag(peer_id: Uuid) -> String {
    format!("peer:{peer_id}")
}

fn role_tag(role: PeerRole) -> &'static str {
    match role {
        PeerRole::Host => "role:host",
        PeerRole::Player => "role:player",
        PeerRole::Spectator => "role:spectator",
    }
}

fn alarm_timestamp(unix_time_ms: u64) -> i64 {
    i64::try_from(unix_time_ms).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn route_parser_preserves_game_namespace() {
        let key = parse_room_path("/v1/signal/bevy-open-rts/4/AB12CD34", "signal").unwrap();
        assert_eq!(key.game_id.as_str(), "bevy-open-rts");
        assert_eq!(key.game_protocol, 4);
        assert_eq!(key.room_code.as_str(), "AB12CD34");
        assert_eq!(key.object_name(), "bevy-open-rts:4:AB12CD34");
    }

    #[test]
    fn route_parser_rejects_path_injection_and_zero_protocol() {
        assert!(parse_room_path("/v1/signal/Bad/Game/4/AB12CD34", "signal").is_err());
        assert!(parse_room_path("/v1/signal/bevy-open-rts/0/AB12CD34", "signal").is_err());
        assert!(parse_room_path("/v1/rooms/bevy-open-rts/4/abc", "rooms").is_err());
    }

    #[test]
    fn authorization_matches_native_service_rules() {
        let room = PersistedRoom {
            descriptor: RoomDescriptor {
                game_id: GameId::new("bevy-open-rts").unwrap(),
                build_id: BuildId::new("0.1.0+test").unwrap(),
                session_protocol: SESSION_PROTOCOL_VERSION,
                game_protocol: 4,
                room_code: RoomCode::new("AB12CD34").unwrap(),
                visibility: RoomVisibility::Private,
                max_peers: 4,
                peer_count: 0,
                host_connected: false,
                created_at_unix_ms: 1,
                expires_at_unix_ms: 2,
                metadata: BTreeMap::new(),
            },
            host_ticket: "host-secret".to_string(),
            join_token: Some("join-secret".to_string()),
            last_activity_unix_ms: 1,
        };
        let host = ValidatedSignalQuery {
            name: PlayerName::new("Host").unwrap(),
            role: PeerRole::Host,
            ticket: Some("host-secret".to_string()),
            build_id: BuildId::new("0.1.0+test").unwrap(),
        };
        assert!(authorize_room_connection(&room, &host).is_ok());

        let player = ValidatedSignalQuery {
            name: PlayerName::new("Player").unwrap(),
            role: PeerRole::Player,
            ticket: Some("join-secret".to_string()),
            build_id: BuildId::new("0.1.0+test").unwrap(),
        };
        assert!(authorize_room_connection(&room, &player).is_err());
    }

    #[test]
    fn alarm_timestamp_saturates_for_out_of_range_values() {
        assert_eq!(alarm_timestamp(42), 42);
        assert_eq!(alarm_timestamp(u64::MAX), i64::MAX);
    }
}
