//! Game-independent API contracts shared by Open Bevy games and the signaling
//! service.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, str::FromStr};

pub const API_VERSION: &str = "v1";
pub const SESSION_PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_MAX_PEERS: u16 = 8;
pub const MAX_PEERS: u16 = 64;
pub const MAX_METADATA_ENTRIES: usize = 16;
pub const MAX_METADATA_KEY_BYTES: usize = 32;
pub const MAX_METADATA_VALUE_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("{field} cannot be empty")]
    Empty { field: &'static str },
    #[error("{field} exceeds {max} bytes")]
    TooLong { field: &'static str, max: usize },
    #[error("{field} contains unsupported characters")]
    InvalidCharacters { field: &'static str },
    #[error("max_peers must be between 2 and {MAX_PEERS}")]
    InvalidMaxPeers,
    #[error("metadata has more than {MAX_METADATA_ENTRIES} entries")]
    TooManyMetadataEntries,
    #[error("game_protocol must be greater than zero")]
    InvalidGameProtocol,
}

macro_rules! validated_id {
    ($name:ident, $field:literal, $max:expr, $validator:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                validate_text($field, &value, $max, $validator)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = ValidationError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

validated_id!(GameId, "game_id", 48, is_slug_character);
validated_id!(BuildId, "build_id", 64, is_build_character);
validated_id!(RoomCode, "room_code", 12, is_room_character);
validated_id!(PlayerName, "player_name", 32, is_player_name_character);

fn validate_text(
    field: &'static str,
    value: &str,
    max_bytes: usize,
    valid_character: fn(char) -> bool,
) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Empty { field });
    }
    if value.len() > max_bytes {
        return Err(ValidationError::TooLong {
            field,
            max: max_bytes,
        });
    }
    if !value.chars().all(valid_character) {
        return Err(ValidationError::InvalidCharacters { field });
    }
    Ok(())
}

fn is_slug_character(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_')
}

fn is_build_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
}

fn is_room_character(character: char) -> bool {
    character.is_ascii_uppercase() || character.is_ascii_digit()
}

fn is_player_name_character(character: char) -> bool {
    !character.is_control() && character != '/' && character != '\\'
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomVisibility {
    #[default]
    Public,
    Unlisted,
    Private,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerRole {
    Host,
    #[default]
    Player,
    Spectator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceConfigResponse {
    pub service: String,
    pub api_version: String,
    pub min_session_protocol: u16,
    pub max_session_protocol: u16,
    pub default_max_peers: u16,
    pub max_peers: u16,
    pub websocket_base_url: String,
    pub ice_servers: Vec<IceServer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub game_id: GameId,
    pub build_id: BuildId,
    pub session_protocol: u16,
    pub game_protocol: u16,
    #[serde(default = "default_max_peers")]
    pub max_peers: u16,
    #[serde(default)]
    pub visibility: RoomVisibility,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl CreateRoomRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.game_protocol == 0 {
            return Err(ValidationError::InvalidGameProtocol);
        }
        if !(2..=MAX_PEERS).contains(&self.max_peers) {
            return Err(ValidationError::InvalidMaxPeers);
        }
        validate_metadata(&self.metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room: RoomDescriptor,
    pub signaling_url: String,
    pub host_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomDescriptor {
    pub game_id: GameId,
    pub build_id: BuildId,
    pub session_protocol: u16,
    pub game_protocol: u16,
    pub room_code: RoomCode,
    pub visibility: RoomVisibility,
    pub max_peers: u16,
    pub peer_count: u16,
    pub host_connected: bool,
    pub created_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomListResponse {
    pub rooms: Vec<RoomDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

pub fn signaling_path(game_id: &GameId, game_protocol: u16, room_code: &RoomCode) -> String {
    format!("/{API_VERSION}/signal/{game_id}/{game_protocol}/{room_code}")
}

pub fn validate_metadata(metadata: &BTreeMap<String, String>) -> Result<(), ValidationError> {
    if metadata.len() > MAX_METADATA_ENTRIES {
        return Err(ValidationError::TooManyMetadataEntries);
    }
    for (key, value) in metadata {
        validate_text(
            "metadata key",
            key,
            MAX_METADATA_KEY_BYTES,
            is_metadata_key_character,
        )?;
        if value.len() > MAX_METADATA_VALUE_BYTES {
            return Err(ValidationError::TooLong {
                field: "metadata value",
                max: MAX_METADATA_VALUE_BYTES,
            });
        }
        if value.chars().any(char::is_control) {
            return Err(ValidationError::InvalidCharacters {
                field: "metadata value",
            });
        }
    }
    Ok(())
}

fn is_metadata_key_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
}

const fn default_max_peers() -> u16 {
    DEFAULT_MAX_PEERS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_path_injection_and_ambiguous_room_codes() {
        assert!(GameId::new("bevy-open-rts").is_ok());
        assert!(GameId::new("Bevy/OpenRTS").is_err());
        assert!(RoomCode::new("A7F9K2").is_ok());
        assert!(RoomCode::new("a7f9k2").is_err());
        assert!(PlayerName::new("联机玩家 1").is_ok());
        assert!(PlayerName::new("bad/name").is_err());
    }

    #[test]
    fn create_room_validation_caps_capacity_and_metadata() {
        let request = CreateRoomRequest {
            game_id: GameId::new("bevy-open-rts").unwrap(),
            build_id: BuildId::new("0.1.0+abc123").unwrap(),
            session_protocol: SESSION_PROTOCOL_VERSION,
            game_protocol: 4,
            max_peers: MAX_PEERS + 1,
            visibility: RoomVisibility::Public,
            metadata: BTreeMap::new(),
        };
        assert_eq!(request.validate(), Err(ValidationError::InvalidMaxPeers));

        let mut request = request;
        request.max_peers = 2;
        request.game_protocol = 0;
        assert_eq!(
            request.validate(),
            Err(ValidationError::InvalidGameProtocol)
        );
    }

    #[test]
    fn signaling_urls_use_a_versioned_game_namespace() {
        let path = signaling_path(
            &GameId::new("bevy-open-rts").unwrap(),
            3,
            &RoomCode::new("ABCD12").unwrap(),
        );
        assert_eq!(path, "/v1/signal/bevy-open-rts/3/ABCD12");
    }
}
