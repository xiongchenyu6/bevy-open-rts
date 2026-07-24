//! Native and browser WebRTC transport shared by Open Bevy games.
//!
//! This crate deliberately has no Bevy dependency. A game plugin can poll the
//! transport from its own schedule and run the returned Matchbox message-loop
//! future on the engine task pool.

use matchbox_socket::{
    ChannelConfig, Packet, PeerState, RtcIceServerConfig, WebRtcSocket, WebRtcSocketBuilder,
};
pub use matchbox_socket::{MessageLoopFuture, PeerId};
use open_bevy_protocol::{
    BuildId, CreateRoomRequest, CreateRoomResponse, ErrorResponse, GameId, IceServer, PeerRole,
    PlayerName, RoomCode, RoomDescriptor, RoomListResponse, SESSION_PROTOCOL_VERSION,
    ServiceConfigResponse, signaling_path,
};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

pub const RELIABLE_CHANNEL: usize = 0;
pub const SNAPSHOT_CHANNEL: usize = 1;
pub const MAX_RELIABLE_PACKET_BYTES: usize = 256 * 1024;
pub const MAX_SNAPSHOT_PACKET_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("signaling service returned HTTP {status} ({code}): {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("the signaling service returned invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("room service URL must use http:// or https://")]
    InvalidHttpScheme,
    #[error("signaling URL must use ws:// or wss://")]
    InvalidWebSocketScheme,
    #[error("WebRTC channel is unavailable: {0}")]
    Channel(String),
    #[error("WebRTC transport is closed")]
    TransportClosed,
    #[error("{channel} packet is {actual} bytes; maximum is {maximum} bytes")]
    PacketTooLarge {
        channel: &'static str,
        actual: usize,
        maximum: usize,
    },
}

#[derive(Debug, Clone)]
pub struct RoomServiceClient {
    base_url: String,
    request_timeout: Duration,
}

impl RoomServiceClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ClientError> {
        let mut url = Url::parse(&base_url.into())?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ClientError::InvalidHttpScheme);
        }
        let normalized_path = url.path().trim_end_matches('/').to_string();
        url.set_path(&normalized_path);
        Ok(Self {
            base_url: url.to_string().trim_end_matches('/').to_string(),
            request_timeout: Duration::from_secs(10),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn service_config(&self) -> Result<ServiceConfigResponse, ClientError> {
        self.get_json(format!("{}/v1/config", self.base_url)).await
    }

    pub async fn create_room(
        &self,
        request: &CreateRoomRequest,
    ) -> Result<CreateRoomResponse, ClientError> {
        let mut request =
            ehttp::Request::post_json(format!("{}/v1/rooms", self.base_url), request)?;
        request.timeout = Some(self.request_timeout);
        decode_response(
            ehttp::fetch_async(request)
                .await
                .map_err(ClientError::Http)?,
        )
    }

    pub async fn list_rooms(
        &self,
        game_id: &GameId,
        protocol_version: u16,
    ) -> Result<RoomListResponse, ClientError> {
        let mut url = Url::parse(&format!("{}/v1/rooms", self.base_url))?;
        url.query_pairs_mut()
            .append_pair("game_id", game_id.as_str())
            .append_pair("protocol_version", &protocol_version.to_string());
        self.get_json(url).await
    }

    pub async fn room(
        &self,
        game_id: &GameId,
        protocol_version: u16,
        room_code: &RoomCode,
    ) -> Result<RoomDescriptor, ClientError> {
        self.get_json(format!(
            "{}/v1/rooms/{}/{}/{}",
            self.base_url, game_id, protocol_version, room_code
        ))
        .await
    }

    async fn get_json<T>(&self, url: impl ToString) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
    {
        let mut request = ehttp::Request::get(url);
        request.timeout = Some(self.request_timeout);
        decode_response(
            ehttp::fetch_async(request)
                .await
                .map_err(ClientError::Http)?,
        )
    }
}

fn decode_response<T>(response: ehttp::Response) -> Result<T, ClientError>
where
    T: DeserializeOwned,
{
    if response.ok {
        return Ok(response.json()?);
    }
    let error = response.json::<ErrorResponse>().unwrap_or(ErrorResponse {
        code: "http_error".to_string(),
        message: response
            .text()
            .unwrap_or(response.status_text.as_str())
            .to_string(),
    });
    Err(ClientError::Api {
        status: response.status,
        code: error.code,
        message: error.message,
    })
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub signaling_url: String,
    pub player_name: PlayerName,
    pub role: PeerRole,
    pub build_id: BuildId,
    pub ticket: Option<String>,
    pub ice_servers: Vec<IceServer>,
    pub reconnect_attempts: Option<u16>,
}

impl TransportConfig {
    pub fn host(
        room: &CreateRoomResponse,
        player_name: PlayerName,
        ice_servers: Vec<IceServer>,
    ) -> Self {
        Self {
            signaling_url: room.signaling_url.clone(),
            player_name,
            role: PeerRole::Host,
            build_id: room.room.build_id.clone(),
            ticket: Some(room.host_token.clone()),
            ice_servers,
            reconnect_attempts: Some(5),
        }
    }

    pub fn player(
        websocket_base_url: &str,
        room: &RoomDescriptor,
        player_name: PlayerName,
        ticket: Option<String>,
        ice_servers: Vec<IceServer>,
    ) -> Self {
        Self {
            signaling_url: format!(
                "{}{}",
                websocket_base_url.trim_end_matches('/'),
                signaling_path(&room.game_id, room.protocol_version, &room.room_code)
            ),
            player_name,
            role: PeerRole::Player,
            build_id: room.build_id.clone(),
            ticket,
            ice_servers,
            reconnect_attempts: Some(5),
        }
    }

    pub fn spectator(
        websocket_base_url: &str,
        room: &RoomDescriptor,
        player_name: PlayerName,
        ticket: Option<String>,
        ice_servers: Vec<IceServer>,
    ) -> Self {
        let mut config = Self::player(websocket_base_url, room, player_name, ticket, ice_servers);
        config.role = PeerRole::Spectator;
        config
    }

    fn websocket_url(&self) -> Result<Url, ClientError> {
        let mut url = Url::parse(&self.signaling_url)?;
        if !matches!(url.scheme(), "ws" | "wss") {
            return Err(ClientError::InvalidWebSocketScheme);
        }
        let role = match self.role {
            PeerRole::Host => "host",
            PeerRole::Player => "player",
            PeerRole::Spectator => "spectator",
        };
        let mut query = url.query_pairs_mut();
        query
            .append_pair("name", self.player_name.as_str())
            .append_pair("role", role)
            .append_pair("build_id", self.build_id.as_str());
        if let Some(ticket) = &self.ticket {
            query.append_pair("ticket", ticket);
        }
        drop(query);
        Ok(url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    ReliableMessage { peer: PeerId, payload: Vec<u8> },
    SnapshotMessage { peer: PeerId, payload: Vec<u8> },
}

pub struct WebRtcTransport {
    socket: WebRtcSocket,
}

impl WebRtcTransport {
    pub fn connect(config: TransportConfig) -> Result<(Self, MessageLoopFuture), ClientError> {
        let url = config.websocket_url()?;
        let mut builder = WebRtcSocketBuilder::new(url.to_string())
            .add_channel(ChannelConfig::reliable())
            .add_channel(ChannelConfig::unreliable())
            .reconnect_attempts(config.reconnect_attempts);
        if let Some(ice_server) = matchbox_ice_server(&config.ice_servers) {
            builder = builder.ice_server(ice_server);
        }
        let (socket, message_loop) = builder.build();
        Ok((Self { socket }, message_loop))
    }

    pub fn local_id(&mut self) -> Option<PeerId> {
        self.socket.id()
    }

    pub fn connected_peers(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.socket.connected_peers()
    }

    pub fn poll(&mut self) -> Result<Vec<TransportEvent>, ClientError> {
        let mut events = Vec::new();
        for (peer, state) in self
            .socket
            .try_update_peers()
            .map_err(|error| ClientError::Channel(error.to_string()))?
        {
            events.push(match state {
                PeerState::Connected => TransportEvent::PeerConnected(peer),
                PeerState::Disconnected => TransportEvent::PeerDisconnected(peer),
            });
        }
        events.extend(
            self.socket
                .channel_mut(RELIABLE_CHANNEL)
                .receive()
                .into_iter()
                .map(|(peer, payload)| TransportEvent::ReliableMessage {
                    peer,
                    payload: payload.into_vec(),
                }),
        );
        events.extend(
            self.socket
                .channel_mut(SNAPSHOT_CHANNEL)
                .receive()
                .into_iter()
                .map(|(peer, payload)| TransportEvent::SnapshotMessage {
                    peer,
                    payload: payload.into_vec(),
                }),
        );
        Ok(events)
    }

    pub fn send_reliable(&mut self, peer: PeerId, payload: Vec<u8>) -> Result<(), ClientError> {
        validate_packet_size("reliable", payload.len(), MAX_RELIABLE_PACKET_BYTES)?;
        self.send(RELIABLE_CHANNEL, peer, payload)
    }

    pub fn send_snapshot(&mut self, peer: PeerId, payload: Vec<u8>) -> Result<(), ClientError> {
        validate_packet_size("snapshot", payload.len(), MAX_SNAPSHOT_PACKET_BYTES)?;
        self.send(SNAPSHOT_CHANNEL, peer, payload)
    }

    pub fn broadcast_reliable(&mut self, payload: &[u8]) -> Result<(), ClientError> {
        validate_packet_size("reliable", payload.len(), MAX_RELIABLE_PACKET_BYTES)?;
        let peers = self.connected_peers().collect::<Vec<_>>();
        for peer in peers {
            self.send(RELIABLE_CHANNEL, peer, payload.to_vec())?;
        }
        Ok(())
    }

    pub fn broadcast_snapshot(&mut self, payload: &[u8]) -> Result<(), ClientError> {
        validate_packet_size("snapshot", payload.len(), MAX_SNAPSHOT_PACKET_BYTES)?;
        let peers = self.connected_peers().collect::<Vec<_>>();
        for peer in peers {
            self.send(SNAPSHOT_CHANNEL, peer, payload.to_vec())?;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.socket.close();
    }

    fn send(&mut self, channel: usize, peer: PeerId, payload: Vec<u8>) -> Result<(), ClientError> {
        self.socket
            .channel_mut(channel)
            .try_send(Packet::from(payload), peer)
            .map_err(|_| ClientError::TransportClosed)
    }
}

fn validate_packet_size(
    channel: &'static str,
    actual: usize,
    maximum: usize,
) -> Result<(), ClientError> {
    if actual > maximum {
        Err(ClientError::PacketTooLarge {
            channel,
            actual,
            maximum,
        })
    } else {
        Ok(())
    }
}

fn matchbox_ice_server(servers: &[IceServer]) -> Option<RtcIceServerConfig> {
    if let Some(turn) = servers
        .iter()
        .rev()
        .find(|server| server.credential.is_some())
    {
        return Some(RtcIceServerConfig {
            urls: turn.urls.clone(),
            username: turn.username.clone(),
            credential: turn.credential.clone(),
        });
    }
    let urls = servers
        .iter()
        .flat_map(|server| server.urls.iter().cloned())
        .collect::<Vec<_>>();
    (!servers.is_empty()).then_some(RtcIceServerConfig {
        urls,
        username: None,
        credential: None,
    })
}

pub fn default_game_id() -> GameId {
    GameId::new("bevy-open-rts").expect("static game id is valid")
}

pub const fn protocol_version() -> u16 {
    SESSION_PROTOCOL_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_query_values_are_percent_encoded() {
        let config = TransportConfig {
            signaling_url: "wss://signal.example.test/v1/signal/game/1/ABC123".to_string(),
            player_name: PlayerName::new("玩家 One").unwrap(),
            role: PeerRole::Player,
            build_id: BuildId::new("0.1.0+abc").unwrap(),
            ticket: Some("secret&other=value".to_string()),
            ice_servers: Vec::new(),
            reconnect_attempts: Some(1),
        };
        let url = config.websocket_url().unwrap();
        let pairs = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(pairs["name"], "玩家 One");
        assert_eq!(pairs["ticket"], "secret&other=value");
        assert_eq!(pairs["build_id"], "0.1.0+abc");
    }

    #[test]
    fn authenticated_turn_is_preferred_over_public_stun() {
        let selected = matchbox_ice_server(&[
            IceServer {
                urls: vec!["stun:stun.example.test".to_string()],
                username: None,
                credential: None,
            },
            IceServer {
                urls: vec!["turn:turn.example.test".to_string()],
                username: Some("user".to_string()),
                credential: Some("credential".to_string()),
            },
        ])
        .unwrap();
        assert_eq!(selected.urls, ["turn:turn.example.test"]);
        assert_eq!(selected.username.as_deref(), Some("user"));
    }

    #[test]
    fn oversized_snapshots_are_rejected_before_the_data_channel() {
        assert!(matches!(
            validate_packet_size(
                "snapshot",
                MAX_SNAPSHOT_PACKET_BYTES + 1,
                MAX_SNAPSHOT_PACKET_BYTES
            ),
            Err(ClientError::PacketTooLarge { .. })
        ));
    }
}
