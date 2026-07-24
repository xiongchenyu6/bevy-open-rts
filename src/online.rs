//! Online lobby and RTS-specific session protocol.
//!
//! Room discovery and WebRTC transport stay in the game-independent
//! `open-bevy-*` crates. This module owns only Bevy Open RTS lobby semantics
//! and conversion into the shared match scene.

use bevy::{
    ecs::system::SystemParam,
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    tasks::IoTaskPool,
};
use open_bevy_net::{
    ClientError, MAX_SNAPSHOT_PACKET_BYTES, MessageLoopFuture, PeerId, RoomServiceClient,
    TransportConfig, TransportEvent, WebRtcTransport, default_game_id, protocol_version,
};
use open_bevy_protocol::{
    BuildId, CreateRoomRequest, CreateRoomResponse, PlayerName, RoomCode, RoomDescriptor,
    RoomListResponse, RoomVisibility, ServiceConfigResponse,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    future::Future,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::*;

const RTS_ONLINE_PROTOCOL: u16 = 3;
const ONLINE_DEFAULT_SERVICE_URL: &str = "http://127.0.0.1:3536";
const ONLINE_MAX_STATUS_BYTES: usize = 180;
const ONLINE_PLAYER_RECONNECT_GRACE_SECONDS: f32 = 30.0;
const ONLINE_SNAPSHOT_HZ: f32 = 10.0;
const ONLINE_SNAPSHOT_INTERVAL_SECONDS: f32 = 1.0 / ONLINE_SNAPSHOT_HZ;
const ONLINE_SNAPSHOT_SNAP_DISTANCE: f32 = 8.0;
const ONLINE_MAX_UNIT_ORDERS_PER_COMMAND: usize = 256;
const ONLINE_MAX_UNIT_ACTIONS_PER_COMMAND: usize = 256;
const ONLINE_MAX_RALLY_STRUCTURES_PER_COMMAND: usize = 64;
const ONLINE_MAX_PRODUCERS_PER_COMMAND: usize = 64;
const ONLINE_MAX_STRUCTURE_ACTIONS_PER_COMMAND: usize = 64;
pub(crate) const ONLINE_MAX_CONSTRUCTORS_PER_COMMAND: usize = 64;
const ONLINE_MAX_ENTITY_ID_BYTES: usize = 64;

const NETWORK_RESOURCE_NAMESPACE: u64 = 1 << 62;
const NETWORK_SUPPLY_CRATE_NAMESPACE: u64 = 2 << 62;

/// Stable identity shared by every peer in an online match.
///
/// Bevy [`Entity`] values are local allocator handles and must never cross the
/// network. Dynamic gameplay entities use the deterministic spawn counter;
/// map-owned resources and crates live in separate namespaces keyed by their
/// map order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NetworkEntityId(pub(crate) u64);

impl NetworkEntityId {
    pub(crate) const fn dynamic(spawn_id: u32) -> Self {
        Self(spawn_id as u64)
    }

    pub(crate) const fn map_resource(index: usize) -> Self {
        Self(NETWORK_RESOURCE_NAMESPACE | index as u64)
    }

    pub(crate) const fn supply_crate(index: usize) -> Self {
        Self(NETWORK_SUPPLY_CRATE_NAMESPACE | index as u64)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OnlinePhase {
    #[default]
    Home,
    Connecting,
    Lobby,
    InMatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnlineTextField {
    PlayerName,
    ServiceUrl,
    RoomCode,
    JoinToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineFaction {
    Alliance,
    Demon,
    Chaos,
}

impl OnlineFaction {
    fn from_game(faction: SkirmishFaction) -> Self {
        match faction {
            SkirmishFaction::Alliance => Self::Alliance,
            SkirmishFaction::Demon => Self::Demon,
            SkirmishFaction::Chaos => Self::Chaos,
        }
    }

    fn to_game(self) -> SkirmishFaction {
        match self {
            Self::Alliance => SkirmishFaction::Alliance,
            Self::Demon => SkirmishFaction::Demon,
            Self::Chaos => SkirmishFaction::Chaos,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Alliance => Self::Demon,
            Self::Demon => Self::Chaos,
            Self::Chaos => Self::Alliance,
        }
    }

    fn label(self) -> &'static str {
        self.to_game().label()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineAiDifficulty {
    Beginner,
    Easy,
    Normal,
    Hard,
}

impl OnlineAiDifficulty {
    fn to_game(self) -> AiDifficulty {
        match self {
            Self::Beginner => AiDifficulty::Beginner,
            Self::Easy => AiDifficulty::Easy,
            Self::Normal => AiDifficulty::Normal,
            Self::Hard => AiDifficulty::Hard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineSlotOccupant {
    Open,
    Closed,
    Human {
        player_id: u64,
        name: String,
        ready: bool,
        connected: bool,
    },
    Ai(OnlineAiDifficulty),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct OnlineLobbySlot {
    occupant: OnlineSlotOccupant,
    faction: OnlineFaction,
    team_id: usize,
    color_slot: usize,
    spawn_slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct OnlineLobbySnapshot {
    revision: u64,
    room_code: String,
    map_id: String,
    starting_resources_index: usize,
    victory_condition_index: usize,
    host_player_id: u64,
    slots: Vec<OnlineLobbySlot>,
}

impl OnlineLobbySnapshot {
    fn new(room_code: String, host_name: String) -> Self {
        let map = &SKIRMISH_MAPS[0];
        let mut slots = (0..map.players)
            .map(|slot| OnlineLobbySlot {
                occupant: if slot == 1 {
                    OnlineSlotOccupant::Open
                } else {
                    OnlineSlotOccupant::Closed
                },
                faction: OnlineFaction::from_game(DEFAULT_LOBBY_FACTIONS[slot]),
                team_id: slot,
                color_slot: slot,
                spawn_slot: slot,
            })
            .collect::<Vec<_>>();
        slots[0].occupant = OnlineSlotOccupant::Human {
            player_id: 1,
            name: host_name,
            ready: true,
            connected: true,
        };
        Self {
            revision: 1,
            room_code,
            map_id: map.id.to_string(),
            starting_resources_index: DEFAULT_STARTING_RESOURCE_INDEX,
            victory_condition_index: 0,
            host_player_id: 1,
            slots,
        }
    }

    fn map(&self) -> &'static SkirmishMapDef {
        SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == self.map_id)
            .unwrap_or(&SKIRMISH_MAPS[0])
    }

    fn human_slot(&self, player_id: u64) -> Option<usize> {
        self.slots.iter().position(|slot| {
            matches!(
                slot.occupant,
                OnlineSlotOccupant::Human {
                    player_id: id,
                    ..
                } if id == player_id
            )
        })
    }

    fn connected_humans_ready(&self) -> bool {
        let humans = self
            .slots
            .iter()
            .filter_map(|slot| match &slot.occupant {
                OnlineSlotOccupant::Human {
                    ready, connected, ..
                } => Some(*ready && *connected),
                _ => None,
            })
            .collect::<Vec<_>>();
        humans.len() >= 2 && humans.into_iter().all(|ready| ready)
    }

    fn active_slot_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| {
                matches!(
                    slot.occupant,
                    OnlineSlotOccupant::Human { .. } | OnlineSlotOccupant::Ai(_)
                )
            })
            .count()
    }

    fn can_start(&self) -> bool {
        self.connected_humans_ready() && self.active_slot_count() >= 2
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineLobbyCommand {
    Ready(bool),
    Faction {
        slot: usize,
        faction: OnlineFaction,
    },
    Team {
        slot: usize,
        team_id: usize,
    },
    Color {
        slot: usize,
        color_slot: usize,
    },
    Map {
        map_id: String,
    },
    StartingResources(usize),
    VictoryCondition(usize),
    SlotOccupant {
        slot: usize,
        occupant: OnlineSlotOccupant,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct OnlineMatchSlot {
    source_slot: usize,
    occupant: OnlineSlotOccupant,
    faction: OnlineFaction,
    team_id: usize,
    color_slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct OnlineMatchConfig {
    map_id: String,
    starting_resources_index: usize,
    victory_condition_index: usize,
    slots: Vec<OnlineMatchSlot>,
}

impl From<&OnlineLobbySnapshot> for OnlineMatchConfig {
    fn from(snapshot: &OnlineLobbySnapshot) -> Self {
        Self {
            map_id: snapshot.map_id.clone(),
            starting_resources_index: snapshot.starting_resources_index,
            victory_condition_index: snapshot.victory_condition_index,
            slots: snapshot
                .slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| {
                    matches!(
                        slot.occupant,
                        OnlineSlotOccupant::Human { .. } | OnlineSlotOccupant::Ai(_)
                    )
                })
                .map(|(source_slot, slot)| OnlineMatchSlot {
                    source_slot,
                    occupant: slot.occupant.clone(),
                    faction: slot.faction,
                    team_id: slot.team_id,
                    color_slot: slot.color_slot,
                })
                .collect(),
        }
    }
}

impl OnlineMatchConfig {
    fn runtime_team_for_player(&self, player_id: u64) -> Option<Team> {
        self.slots
            .iter()
            .position(|slot| {
                matches!(
                    slot.occupant,
                    OnlineSlotOccupant::Human {
                        player_id: occupant_id,
                        ..
                    } if occupant_id == player_id
                )
            })
            .map(Team::Player)
    }

    fn runtime_faction_for_player(&self, player_id: u64) -> SkirmishFaction {
        self.slots
            .iter()
            .find(|slot| {
                matches!(
                    slot.occupant,
                    OnlineSlotOccupant::Human {
                        player_id: occupant_id,
                        ..
                    } if occupant_id == player_id
                )
            })
            .map_or(SkirmishFaction::Alliance, |slot| slot.faction.to_game())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OnlineHarvestTarget {
    Resource,
    Dropoff,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum OnlineUnitOrderKind {
    Move {
        destination: [f32; 3],
    },
    Attack {
        target: u64,
    },
    Capture {
        target: u64,
    },
    Garrison {
        target: u64,
    },
    Harvest {
        target: u64,
        destination: OnlineHarvestTarget,
    },
    Repair {
        target: u64,
    },
    Construct {
        target: u64,
    },
    Follow {
        target: u64,
        offset: [f32; 3],
        allow_enemy: bool,
    },
    AttackMove {
        destination: [f32; 3],
    },
    Patrol {
        origin: [f32; 3],
        destination: [f32; 3],
    },
}

impl OnlineUnitOrderKind {
    pub(crate) fn from_local(
        order: &UnitQueuedOrder,
        mut network_id_for: impl FnMut(Entity) -> Option<u64>,
    ) -> Option<Self> {
        Some(match order {
            UnitQueuedOrder::Move(destination) => Self::Move {
                destination: destination.to_array(),
            },
            UnitQueuedOrder::Attack(target) => Self::Attack {
                target: network_id_for(*target)?,
            },
            UnitQueuedOrder::Capture(target) => Self::Capture {
                target: network_id_for(*target)?,
            },
            UnitQueuedOrder::Garrison(target) => Self::Garrison {
                target: network_id_for(*target)?,
            },
            UnitQueuedOrder::Harvest { target, state } => Self::Harvest {
                target: network_id_for(*target)?,
                destination: match state {
                    HarvestState::MovingToResource | HarvestState::Collecting => {
                        OnlineHarvestTarget::Resource
                    }
                    HarvestState::MovingToDropoff => OnlineHarvestTarget::Dropoff,
                },
            },
            UnitQueuedOrder::Repair(target) => Self::Repair {
                target: network_id_for(*target)?,
            },
            UnitQueuedOrder::Construct(target) => Self::Construct {
                target: network_id_for(*target)?,
            },
            UnitQueuedOrder::Follow { target, offset } => Self::Follow {
                target: network_id_for(*target)?,
                offset: offset.to_array(),
                allow_enemy: false,
            },
            UnitQueuedOrder::AttackMove(destination) => Self::AttackMove {
                destination: destination.to_array(),
            },
            UnitQueuedOrder::Patrol {
                origin,
                destination,
            } => Self::Patrol {
                origin: origin.to_array(),
                destination: destination.to_array(),
            },
            UnitQueuedOrder::ForceFollow { target, offset } => Self::Follow {
                target: network_id_for(*target)?,
                offset: offset.to_array(),
                allow_enemy: true,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OnlineUnitOrderCommand {
    pub(crate) unit_id: u64,
    pub(crate) order: OnlineUnitOrderKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OnlineRallyMode {
    Move,
    AttackMove,
}

impl OnlineRallyMode {
    fn to_game(self) -> RallyMode {
        match self {
            Self::Move => RallyMode::Move,
            Self::AttackMove => RallyMode::AttackMove,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OnlineUnitAction {
    Stop,
    ToggleHoldPosition,
    GuardArea,
    Scatter,
    ToggleDeployMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OnlineStructureAction {
    Sell,
    Repair,
    CancelConstruction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OnlineSupportPower {
    RadarSweep,
    OrbitalStrike,
    EmpPulse,
    ChronoRelay,
    ShieldOverdrive,
    NaniteRepairSwarm,
    WeatherStorm,
    StrategicMissile,
    Paradrop,
}

impl OnlineSupportPower {
    pub(crate) fn from_game(power: SupportPowerKind) -> Self {
        match power {
            SupportPowerKind::RadarSweep => Self::RadarSweep,
            SupportPowerKind::OrbitalStrike => Self::OrbitalStrike,
            SupportPowerKind::EmpPulse => Self::EmpPulse,
            SupportPowerKind::ChronoRelay => Self::ChronoRelay,
            SupportPowerKind::ShieldOverdrive => Self::ShieldOverdrive,
            SupportPowerKind::NaniteRepairSwarm => Self::NaniteRepairSwarm,
            SupportPowerKind::WeatherStorm => Self::WeatherStorm,
            SupportPowerKind::StrategicMissile => Self::StrategicMissile,
            SupportPowerKind::Paradrop => Self::Paradrop,
        }
    }

    fn to_game(self) -> SupportPowerKind {
        match self {
            Self::RadarSweep => SupportPowerKind::RadarSweep,
            Self::OrbitalStrike => SupportPowerKind::OrbitalStrike,
            Self::EmpPulse => SupportPowerKind::EmpPulse,
            Self::ChronoRelay => SupportPowerKind::ChronoRelay,
            Self::ShieldOverdrive => SupportPowerKind::ShieldOverdrive,
            Self::NaniteRepairSwarm => SupportPowerKind::NaniteRepairSwarm,
            Self::WeatherStorm => SupportPowerKind::WeatherStorm,
            Self::StrategicMissile => SupportPowerKind::StrategicMissile,
            Self::Paradrop => SupportPowerKind::Paradrop,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum OnlinePlayerCommand {
    UnitOrders {
        orders: Vec<OnlineUnitOrderCommand>,
        queue: bool,
    },
    UnitAction {
        units: Vec<u64>,
        action: OnlineUnitAction,
    },
    SetRallyPoints {
        structures: Vec<u64>,
        target: [f32; 3],
        target_entity: Option<u64>,
        mode: OnlineRallyMode,
    },
    TrainUnits {
        producers: Vec<u64>,
        unit_id: String,
        batch_to_limit: bool,
    },
    CancelProduction {
        producers: Vec<u64>,
        product_id: String,
        local_index: Option<u8>,
    },
    PlaceStructure {
        constructors: Vec<u64>,
        structure_id: String,
        position: [f32; 3],
        rotation_y_radians: f32,
    },
    StructureAction {
        structures: Vec<u64>,
        action: OnlineStructureAction,
    },
    UseSupportPower {
        power: OnlineSupportPower,
        target: [f32; 3],
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlinePlayerCommandEnvelope {
    protocol: u16,
    sequence: u64,
    command: OnlinePlayerCommand,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum OnlineReliableMessage {
    Hello {
        protocol: u16,
        session_key: String,
        player_name: String,
    },
    Welcome {
        player_id: u64,
        assigned_slot: usize,
        snapshot: OnlineLobbySnapshot,
        match_config: Option<OnlineMatchConfig>,
    },
    LobbyCommand(OnlineLobbyCommand),
    LobbySnapshot(OnlineLobbySnapshot),
    StartMatch(OnlineMatchConfig),
    ReturnToLobbyRequest,
    ReturnToLobby(OnlineLobbySnapshot),
    SessionClosed {
        reason: String,
    },
    PlayerCommand(OnlinePlayerCommandEnvelope),
    Rejected {
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineEntityTeam {
    Player(usize),
    Neutral,
}

impl OnlineEntityTeam {
    fn from_game(team: Team) -> Self {
        match team {
            Team::Player(index) => Self::Player(index),
            Team::Neutral => Self::Neutral,
        }
    }

    fn to_game(self) -> Team {
        match self {
            Self::Player(index) => Team::Player(index),
            Self::Neutral => Team::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineResourceKind {
    Ore,
    Crystal,
}

impl OnlineResourceKind {
    fn from_game(kind: ResourceKind) -> Self {
        match kind {
            ResourceKind::Ore => Self::Ore,
            ResourceKind::Crystal => Self::Crystal,
        }
    }

    fn to_game(self) -> ResourceKind {
        match self {
            Self::Ore => ResourceKind::Ore,
            Self::Crystal => ResourceKind::Crystal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum OnlineSupplyCrateEffect {
    Resources,
    Repair,
    Veterancy,
}

impl OnlineSupplyCrateEffect {
    fn from_game(effect: SupplyCrateEffect) -> Self {
        match effect {
            SupplyCrateEffect::Resources => Self::Resources,
            SupplyCrateEffect::Repair => Self::Repair,
            SupplyCrateEffect::Veterancy => Self::Veterancy,
        }
    }

    fn to_game(self) -> SupplyCrateEffect {
        match self {
            Self::Resources => SupplyCrateEffect::Resources,
            Self::Repair => SupplyCrateEffect::Repair,
            Self::Veterancy => SupplyCrateEffect::Veterancy,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum OnlineEntityKind {
    Unit {
        id: String,
    },
    Structure {
        id: String,
    },
    Resource {
        kind: OnlineResourceKind,
        amount: i32,
    },
    SupplyCrate {
        effect: OnlineSupplyCrateEffect,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlineEntitySnapshot {
    id: u64,
    kind: OnlineEntityKind,
    team: OnlineEntityTeam,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    health: Option<[f32; 2]>,
    visual_faction: Option<OnlineFaction>,
    cargo: Option<[i32; 3]>,
    construction: Option<[f32; 2]>,
    veterancy: Option<(u8, u32)>,
}

impl OnlineEntitySnapshot {
    fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_array(self.rotation),
            scale: Vec3::from_array(self.scale),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlineEconomySnapshot {
    ore: i32,
    crystal: i32,
    power_used: i32,
    power_capacity: i32,
    power_sabotage_remaining: f32,
    production_veterancy_ranks: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum OnlineBuildActionSnapshot {
    Train(String),
    Build(String),
}

impl OnlineBuildActionSnapshot {
    fn from_game(action: BuildAction) -> Option<Self> {
        match action {
            BuildAction::Train(id) => Some(Self::Train(id.to_string())),
            BuildAction::Build(id) => Some(Self::Build(id.to_string())),
            _ => None,
        }
    }

    fn to_game(&self) -> Option<BuildAction> {
        match self {
            Self::Train(id) => registry::entity(id).map(|def| BuildAction::Train(def.id)),
            Self::Build(id) => registry::entity(id).map(|def| BuildAction::Build(def.id)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlineBuildJobSnapshot {
    team: OnlineEntityTeam,
    action: OnlineBuildActionSnapshot,
    producer_entity: u64,
    producer_id: String,
    timer: f32,
    origin: [f32; 3],
    cost: [i32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlineMatchStateSnapshot {
    start_time_sec: f32,
    remaining_teams: u32,
    remaining_anchors: u32,
    active_anchor_teams: Vec<bool>,
    finished: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OnlineWorldSnapshot {
    protocol: u16,
    tick: u64,
    entities: Vec<OnlineEntitySnapshot>,
    economies: Vec<OnlineEconomySnapshot>,
    build_queue: Vec<OnlineBuildJobSnapshot>,
    support_cooldowns: Vec<[f32; SupportPowerKind::ALL.len()]>,
    support_initial_charge_started: Vec<[bool; SupportPowerKind::ALL.len()]>,
    match_state: OnlineMatchStateSnapshot,
}

#[derive(Resource, Default)]
struct OnlineMatchReplication {
    next_tick: u64,
    last_applied_tick: u64,
    send_accumulator: f32,
    pending_snapshot: Option<OnlineWorldSnapshot>,
}

#[derive(Resource, Default)]
pub(crate) struct OnlineCommandOutbox {
    next_sequence: u64,
    pending: VecDeque<OnlinePlayerCommandEnvelope>,
}

impl OnlineCommandOutbox {
    pub(crate) fn submit(&mut self, command: OnlinePlayerCommand) {
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.pending.push_back(OnlinePlayerCommandEnvelope {
            protocol: RTS_ONLINE_PROTOCOL,
            sequence: self.next_sequence,
            command,
        });
    }
}

#[derive(Clone, Debug)]
struct AuthorizedOnlinePlayerCommand {
    player_id: u64,
    command: OnlinePlayerCommand,
}

#[derive(Resource, Default)]
struct OnlineCommandInbox {
    last_sequence_by_player: HashMap<u64, u64>,
    pending: VecDeque<AuthorizedOnlinePlayerCommand>,
}

fn enqueue_online_player_command(
    inbox: &mut OnlineCommandInbox,
    player_id: u64,
    envelope: OnlinePlayerCommandEnvelope,
) -> bool {
    if envelope.protocol != RTS_ONLINE_PROTOCOL || envelope.sequence == 0 {
        return false;
    }
    let previous = inbox
        .last_sequence_by_player
        .get(&player_id)
        .copied()
        .unwrap_or_default();
    if envelope.sequence <= previous {
        return false;
    }
    inbox
        .last_sequence_by_player
        .insert(player_id, envelope.sequence);
    inbox.pending.push_back(AuthorizedOnlinePlayerCommand {
        player_id,
        command: envelope.command,
    });
    true
}

#[derive(Component, Clone, Copy)]
struct NetworkInterpolation {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

type OnlineSnapshotSource<'a> = (
    &'a NetworkEntityId,
    &'a Transform,
    &'a Team,
    Option<&'a Unit>,
    Option<&'a Structure>,
    Option<&'a ResourceNode>,
    Option<&'a SupplyCrate>,
    Option<&'a Health>,
    Option<&'a VisualFaction>,
    Option<&'a ResourceCargo>,
    Option<&'a UnderConstruction>,
    Option<&'a Veterancy>,
);

type OnlineSnapshotTarget<'a> = (
    Entity,
    &'a NetworkEntityId,
    &'a mut Transform,
    &'a Team,
    Option<&'a mut Health>,
    Option<&'a mut ResourceNode>,
    Option<&'a mut ResourceCargo>,
    Option<&'a mut UnderConstruction>,
    Option<&'a mut Veterancy>,
);

type OnlineCommandActor<'a> = (
    Entity,
    &'a NetworkEntityId,
    &'a Team,
    &'a Unit,
    &'a Health,
    &'a Transform,
    CommandOrderStateItem<'a>,
);

type OnlineCommandTarget<'a> = (
    &'a Team,
    &'a Transform,
    Option<&'a Unit>,
    Option<&'a Structure>,
    Option<&'a ResourceNode>,
    Option<&'a Health>,
    Option<&'a UnderConstruction>,
    Option<&'a Garrison>,
);

type OnlineSupportTarget<'a> = (
    Entity,
    &'a Team,
    &'a Transform,
    &'a Health,
    Option<&'a Unit>,
    Option<&'a Structure>,
);

#[derive(SystemParam)]
struct OnlineSnapshotBroadcastParams<'w, 's> {
    time: Res<'w, Time>,
    session: ResMut<'w, OnlineSession>,
    transport: ResMut<'w, OnlineTransport>,
    replication: ResMut<'w, OnlineMatchReplication>,
    entities: Query<'w, 's, OnlineSnapshotSource<'static>>,
    network_entities: Query<'w, 's, (Entity, &'static NetworkEntityId)>,
    economies: Res<'w, Economies>,
    build_queue: Res<'w, BuildQueue>,
    support_cooldowns: Res<'w, SupportCooldowns>,
    match_state: Res<'w, MatchState>,
}

#[derive(SystemParam)]
struct OnlineSnapshotApplyParams<'w, 's> {
    commands: Commands<'w, 's>,
    asset_server: Res<'w, AssetServer>,
    session: Res<'w, OnlineSession>,
    replication: ResMut<'w, OnlineMatchReplication>,
    next_id: ResMut<'w, NextSpawnId>,
    visible_player: Res<'w, VisiblePlayer>,
    relations: Res<'w, TeamRelations>,
    economies: ResMut<'w, Economies>,
    build_queue: ResMut<'w, BuildQueue>,
    support_cooldowns: ResMut<'w, SupportCooldowns>,
    match_state: ResMut<'w, MatchState>,
    match_flow: ResMut<'w, MatchFlow>,
    audio_feedback: ResMut<'w, AudioFeedback>,
    network_entities: Query<'w, 's, (Entity, &'static NetworkEntityId)>,
    entities: Query<'w, 's, OnlineSnapshotTarget<'static>>,
}

#[derive(SystemParam)]
pub(crate) struct OnlineOrderCommandParams<'w, 's> {
    pub(crate) session: Option<Res<'w, OnlineSession>>,
    pub(crate) outbox: Option<ResMut<'w, OnlineCommandOutbox>>,
    pub(crate) network_ids: Query<'w, 's, &'static NetworkEntityId>,
    pub(crate) selected_rally_points: Query<
        'w,
        's,
        (&'static NetworkEntityId, &'static Team),
        (
            With<Selected>,
            With<Structure>,
            With<RallyPoint>,
            Without<Unit>,
        ),
    >,
}

#[derive(SystemParam)]
pub(crate) struct OnlineGameplayCommandParams<'w, 's> {
    session: Option<Res<'w, OnlineSession>>,
    outbox: Option<ResMut<'w, OnlineCommandOutbox>>,
    network_ids: Query<'w, 's, &'static NetworkEntityId>,
}

impl OnlineGameplayCommandParams<'_, '_> {
    pub(crate) fn is_active(&self) -> bool {
        online_match_uses_command_transport(self.session.as_deref())
    }

    pub(crate) fn network_id_for(&self, entity: Entity) -> Option<u64> {
        self.network_ids.get(entity).ok().map(|id| id.0)
    }

    pub(crate) fn submit(&mut self, command: OnlinePlayerCommand) -> bool {
        let Some(outbox) = self.outbox.as_deref_mut() else {
            return false;
        };
        outbox.submit(command);
        true
    }
}

#[derive(SystemParam)]
struct OnlineCommandApplyParams<'w, 's> {
    commands: Commands<'w, 's>,
    session: Res<'w, OnlineSession>,
    inbox: ResMut<'w, OnlineCommandInbox>,
    asset_server: Option<Res<'w, AssetServer>>,
    terrain: Option<Res<'w, TerrainHeightField>>,
    map_bounds: Res<'w, MapBounds>,
    relations: Res<'w, TeamRelations>,
    next_id: Option<ResMut<'w, NextSpawnId>>,
    economies: Option<ResMut<'w, Economies>>,
    build_queue: Option<ResMut<'w, BuildQueue>>,
    support_cooldowns: Option<ResMut<'w, SupportCooldowns>>,
    battle_log: Option<ResMut<'w, BattleLog>>,
    audio_feedback: Option<ResMut<'w, AudioFeedback>>,
    network_entities: Query<'w, 's, (Entity, &'static NetworkEntityId)>,
    actors: Query<'w, 's, OnlineCommandActor<'static>>,
    holds: Query<'w, 's, Option<&'static HoldPosition>, With<Unit>>,
    targets: Query<'w, 's, OnlineCommandTarget<'static>>,
    manual_repairs: Query<'w, 's, (), With<ManualStructureRepair>>,
    support_targets: Query<'w, 's, OnlineSupportTarget<'static>>,
    rally_points: Query<
        'w,
        's,
        (
            &'static NetworkEntityId,
            &'static Team,
            &'static mut RallyPoint,
        ),
        With<Structure>,
    >,
    structures: Query<'w, 's, StructurePrereqItem<'static>>,
    occupiers: Query<
        'w,
        's,
        PlacementOccupierItem<'static>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
}

#[derive(Default)]
struct HostLobbyRuntime {
    next_player_id: u64,
    peer_players: HashMap<PeerId, u64>,
    resume_players: HashMap<String, u64>,
}

impl HostLobbyRuntime {
    fn new(host_session_key: String) -> Self {
        Self {
            next_player_id: 2,
            peer_players: HashMap::new(),
            resume_players: HashMap::from([(host_session_key, 1)]),
        }
    }

    fn admit(
        &mut self,
        snapshot: &mut OnlineLobbySnapshot,
        peer: PeerId,
        session_key: String,
        player_name: String,
        allow_new_player: bool,
    ) -> Result<(u64, usize), &'static str> {
        if let Some(player_id) = self.resume_players.get(&session_key).copied() {
            let slot = snapshot
                .human_slot(player_id)
                .ok_or("the previous player slot is no longer available")?;
            self.peer_players.retain(|_, id| *id != player_id);
            self.peer_players.insert(peer, player_id);
            if let OnlineSlotOccupant::Human {
                name, connected, ..
            } = &mut snapshot.slots[slot].occupant
            {
                *name = player_name;
                *connected = true;
            }
            snapshot.revision = snapshot.revision.saturating_add(1);
            return Ok((player_id, slot));
        }

        if !allow_new_player {
            return Err("the match is already in progress");
        }

        let Some(slot) = snapshot
            .slots
            .iter()
            .position(|slot| slot.occupant == OnlineSlotOccupant::Open)
        else {
            return Err("the lobby has no open player slot");
        };
        let player_id = self.next_player_id;
        self.next_player_id = self.next_player_id.saturating_add(1);
        snapshot.slots[slot].occupant = OnlineSlotOccupant::Human {
            player_id,
            name: player_name,
            ready: false,
            connected: true,
        };
        snapshot.revision = snapshot.revision.saturating_add(1);
        self.resume_players.insert(session_key, player_id);
        self.peer_players.insert(peer, player_id);
        Ok((player_id, slot))
    }

    fn player_for_session(&self, session_key: &str) -> Option<u64> {
        self.resume_players.get(session_key).copied()
    }

    fn disconnect(&mut self, snapshot: &mut OnlineLobbySnapshot, peer: PeerId) -> Option<u64> {
        let Some(player_id) = self.peer_players.remove(&peer) else {
            return None;
        };
        let Some(slot) = snapshot.human_slot(player_id) else {
            return None;
        };
        if let OnlineSlotOccupant::Human {
            connected, ready, ..
        } = &mut snapshot.slots[slot].occupant
        {
            *connected = false;
            *ready = false;
        }
        snapshot.revision = snapshot.revision.saturating_add(1);
        Some(player_id)
    }

    fn retain_players(&mut self, retained: &HashSet<u64>) {
        self.peer_players
            .retain(|_, player_id| retained.contains(player_id));
        self.resume_players
            .retain(|_, player_id| retained.contains(player_id));
    }
}

#[derive(Resource, Default)]
pub(crate) struct OnlineLifecycleControl {
    local_return_to_lobby: bool,
    local_leave_session: bool,
    remote_return_requests: Vec<u64>,
    disconnected_players: HashMap<u64, f32>,
    forfeited_players: HashSet<u64>,
    session_closed_reason: Option<String>,
    transport_stopped_reason: Option<String>,
}

impl OnlineLifecycleControl {
    pub(crate) fn request_return_to_lobby(&mut self) {
        self.local_return_to_lobby = true;
    }

    pub(crate) fn request_leave_session(&mut self) {
        self.local_leave_session = true;
    }

    fn note_disconnect(&mut self, player_id: u64) {
        if !self.forfeited_players.contains(&player_id) {
            self.disconnected_players
                .insert(player_id, ONLINE_PLAYER_RECONNECT_GRACE_SECONDS);
        }
    }

    fn note_reconnect(&mut self, player_id: u64) {
        self.disconnected_players.remove(&player_id);
    }

    fn tick_disconnects(&mut self, delta_seconds: f32) -> Vec<u64> {
        for remaining in self.disconnected_players.values_mut() {
            *remaining -= delta_seconds.max(0.0);
        }
        let expired = self
            .disconnected_players
            .iter()
            .filter_map(|(player_id, remaining)| (*remaining <= 0.0).then_some(*player_id))
            .collect::<Vec<_>>();
        for player_id in &expired {
            self.disconnected_players.remove(player_id);
            self.forfeited_players.insert(*player_id);
        }
        expired
    }

    fn reset_match(&mut self) {
        self.local_return_to_lobby = false;
        self.local_leave_session = false;
        self.remote_return_requests.clear();
        self.disconnected_players.clear();
        self.forfeited_players.clear();
        self.session_closed_reason = None;
        self.transport_stopped_reason = None;
    }
}

#[derive(Resource)]
pub(crate) struct OnlineSession {
    phase: OnlinePhase,
    is_host: bool,
    service_url: String,
    player_name: String,
    room_code_input: String,
    join_token_input: String,
    focused_field: Option<OnlineTextField>,
    status: String,
    room: Option<RoomDescriptor>,
    host_token: Option<String>,
    service_config: Option<ServiceConfigResponse>,
    public_rooms: Vec<RoomDescriptor>,
    session_key: String,
    local_player_id: Option<u64>,
    assigned_slot: Option<usize>,
    host_peer: Option<PeerId>,
    lobby: Option<OnlineLobbySnapshot>,
    match_config: Option<OnlineMatchConfig>,
    host_runtime: Option<HostLobbyRuntime>,
    ui_dirty: bool,
    rendered_language: Language,
}

impl Default for OnlineSession {
    fn default() -> Self {
        Self {
            phase: OnlinePhase::Home,
            is_host: false,
            service_url: option_env!("OPEN_BEVY_SIGNALING_URL")
                .filter(|url| !url.is_empty())
                .unwrap_or(ONLINE_DEFAULT_SERVICE_URL)
                .to_string(),
            player_name: t("指挥官", "Commander").to_string(),
            room_code_input: String::new(),
            join_token_input: String::new(),
            focused_field: None,
            status: t(
                "选择创建房间或输入房间码加入",
                "Create a room or enter a code to join",
            )
            .to_string(),
            room: None,
            host_token: None,
            service_config: None,
            public_rooms: Vec::new(),
            session_key: new_session_key(),
            local_player_id: None,
            assigned_slot: None,
            host_peer: None,
            lobby: None,
            match_config: None,
            host_runtime: None,
            ui_dirty: true,
            rendered_language: current_language(),
        }
    }
}

impl OnlineSession {
    fn set_status(&mut self, status: impl Into<String>) {
        let mut status = status.into();
        truncate_utf8(&mut status, ONLINE_MAX_STATUS_BYTES);
        self.status = status;
        self.ui_dirty = true;
    }

    fn reset_connection(&mut self) {
        self.phase = OnlinePhase::Home;
        self.is_host = false;
        self.room = None;
        self.host_token = None;
        self.service_config = None;
        self.local_player_id = None;
        self.assigned_slot = None;
        self.host_peer = None;
        self.lobby = None;
        self.match_config = None;
        self.host_runtime = None;
        self.focused_field = None;
        self.ui_dirty = true;
    }
}

#[derive(Resource, Default)]
struct OnlineTransport {
    socket: Option<WebRtcTransport>,
}

#[derive(Resource, Clone)]
struct OnlineAsyncInbox(Arc<Mutex<VecDeque<OnlineAsyncResult>>>);

impl Default for OnlineAsyncInbox {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::new())))
    }
}

enum OnlineAsyncResult {
    Created(Result<(CreateRoomResponse, ServiceConfigResponse), String>),
    Joined(Result<(RoomDescriptor, ServiceConfigResponse), String>),
    Rooms(Result<RoomListResponse, String>),
    TransportStopped(String),
}

#[derive(Component)]
struct OnlineUiRoot;

#[derive(Component, Clone)]
enum OnlineUiAction {
    Focus(OnlineTextField),
    CreatePublic,
    RefreshRooms,
    JoinInputRoom,
    JoinRoom(String),
    Back,
    Disconnect,
    ToggleReady,
    CycleMap,
    CycleResources,
    CycleVictory,
    CycleOccupant(usize),
    CycleFaction(usize),
    CycleTeam(usize),
    CycleColor(usize),
    StartMatch,
}

#[derive(Component)]
struct OnlineButton {
    action: OnlineUiAction,
    enabled: bool,
}

pub(crate) fn add_online_scene(app: &mut App) -> &mut App {
    app.init_resource::<OnlineSession>()
        .init_resource::<OnlineTransport>()
        .init_resource::<OnlineAsyncInbox>()
        .init_resource::<OnlineMatchReplication>()
        .init_resource::<OnlineCommandOutbox>()
        .init_resource::<OnlineCommandInbox>()
        .init_resource::<OnlineLifecycleControl>()
        .add_systems(OnEnter(AppScreen::OnlineLobby), enter_online_lobby)
        .add_systems(OnEnter(AppScreen::InMatch), reset_online_match_replication)
        .add_systems(
            Update,
            (
                process_online_async_results,
                poll_online_transport,
                process_online_lifecycle,
                apply_pending_online_snapshot,
            )
                .chain()
                .before(SimulationPhase::UiAndManagement),
        )
        .add_systems(
            Update,
            interpolate_network_entities
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress)
                .run_if(online_client_match),
        )
        .add_systems(
            Update,
            (flush_online_player_commands, apply_online_player_commands)
                .chain()
                .in_set(SimulationPhase::UiAndManagement)
                .after(issue_orders)
                .after(structure_placement_input)
                .after(command_shortcuts)
                .after(command_buttons)
                .after(command_queue_controls)
                .after(production_queue_slot_buttons)
                .after(fire_support_power_on_left_click)
                .run_if(match_in_progress),
        )
        .add_systems(
            Last,
            broadcast_online_world_snapshot.run_if(in_state(AppScreen::InMatch)),
        )
        .add_systems(
            Update,
            (online_text_input, online_menu_buttons, rebuild_online_ui)
                .chain()
                .run_if(in_state(AppScreen::OnlineLobby)),
        );
    app
}

pub(crate) fn online_match_is_authoritative(session: Option<Res<OnlineSession>>) -> bool {
    session
        .as_deref()
        .is_none_or(|session| session.phase != OnlinePhase::InMatch || session.is_host)
}

pub(crate) fn online_match_uses_command_transport(session: Option<&OnlineSession>) -> bool {
    session.is_some_and(|session| session.phase == OnlinePhase::InMatch)
}

pub(crate) fn online_match_uses_global_result(session: Option<&OnlineSession>) -> bool {
    online_match_uses_command_transport(session)
}

pub(crate) fn online_match_is_host(session: Option<&OnlineSession>) -> bool {
    session.is_some_and(|session| session.phase == OnlinePhase::InMatch && session.is_host)
}

fn online_client_match(session: Option<Res<OnlineSession>>) -> bool {
    session
        .as_deref()
        .is_some_and(|session| session.phase == OnlinePhase::InMatch && !session.is_host)
}

fn reset_online_match_replication(
    mut replication: ResMut<OnlineMatchReplication>,
    mut outbox: ResMut<OnlineCommandOutbox>,
    mut inbox: ResMut<OnlineCommandInbox>,
    mut lifecycle: ResMut<OnlineLifecycleControl>,
) {
    *replication = OnlineMatchReplication::default();
    *outbox = OnlineCommandOutbox::default();
    *inbox = OnlineCommandInbox::default();
    lifecycle.reset_match();
}

fn enter_online_lobby(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut session: ResMut<OnlineSession>,
) {
    commands.spawn((
        Name::new("Online Lobby Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::OnlineLobby),
    ));
    setup_menu_backdrop(
        &mut commands,
        &asset_server,
        AppScreen::OnlineLobby,
        Color::srgba(0.0, 0.025, 0.022, 0.58),
    );
    session.ui_dirty = true;
}

fn rebuild_online_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut session: ResMut<OnlineSession>,
    roots: Query<Entity, With<OnlineUiRoot>>,
) {
    let language = current_language();
    if !session.ui_dirty && session.rendered_language == language {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    session.rendered_language = language;
    session.ui_dirty = false;
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            Name::new("Online Lobby UI"),
            OnlineUiRoot,
            DespawnOnExit(AppScreen::OnlineLobby),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(24)),
                ..default()
            },
        ))
        .with_children(|root| match session.phase {
            OnlinePhase::Home | OnlinePhase::Connecting => {
                spawn_online_home(root, &session, font.clone())
            }
            OnlinePhase::Lobby | OnlinePhase::InMatch => {
                spawn_online_lobby(root, &session, font.clone())
            }
        });
}

fn online_panel_node(width: f32) -> impl Bundle {
    (
        Node {
            width: Val::Percent(92.0),
            max_width: px(width),
            max_height: Val::Percent(94.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(10),
            padding: UiRect::all(px(18)),
            border: UiRect::all(px(1)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.54, 0.5)),
        BackgroundColor(Color::srgba(0.015, 0.03, 0.03, 0.96)),
    )
}

fn spawn_online_home(root: &mut ChildSpawnerCommands, session: &OnlineSession, font: Handle<Font>) {
    root.spawn(online_panel_node(1120.0))
        .with_children(|panel| {
            spawn_online_title(panel, t("Open RTS 联机对战", "Open RTS Online"), &font);
            panel.spawn((
                Text::new(session.status.clone()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.66, 0.85, 0.79)),
            ));
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    ..default()
                })
                .with_children(|columns| {
                    columns
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(10),
                            ..default()
                        })
                        .with_children(|left| {
                            spawn_online_field(
                                left,
                                t("玩家名", "Player name"),
                                &session.player_name,
                                OnlineTextField::PlayerName,
                                session.focused_field,
                                &font,
                            );
                            spawn_online_field(
                                left,
                                t("联机服务", "Online service"),
                                &session.service_url,
                                OnlineTextField::ServiceUrl,
                                session.focused_field,
                                &font,
                            );
                            spawn_online_action_button(
                                left,
                                OnlineUiAction::CreatePublic,
                                t("创建公开房间", "Create public room"),
                                &font,
                                true,
                            );
                            spawn_online_action_button(
                                left,
                                OnlineUiAction::RefreshRooms,
                                t("刷新公开房间", "Refresh public rooms"),
                                &font,
                                true,
                            );
                        });
                    columns
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(10),
                            ..default()
                        })
                        .with_children(|right| {
                            spawn_online_field(
                                right,
                                t("房间码", "Room code"),
                                &session.room_code_input,
                                OnlineTextField::RoomCode,
                                session.focused_field,
                                &font,
                            );
                            spawn_online_field(
                                right,
                                t("私人房间口令（可空）", "Private token (optional)"),
                                &session.join_token_input,
                                OnlineTextField::JoinToken,
                                session.focused_field,
                                &font,
                            );
                            spawn_online_action_button(
                                right,
                                OnlineUiAction::JoinInputRoom,
                                t("加入房间", "Join room"),
                                &font,
                                true,
                            );
                        });
                });

            panel.spawn(menu_section_header(
                "公开房间",
                "Public rooms",
                font.clone(),
            ));
            if session.public_rooms.is_empty() {
                panel.spawn((
                    Text::new(t("暂无房间，点击刷新", "No rooms found; refresh to search")),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.68, 0.66)),
                ));
            } else {
                for room in &session.public_rooms {
                    let map = room.metadata.get("map").map(String::as_str).unwrap_or("-");
                    let label = format!(
                        "{}  |  {}  |  {}/{}",
                        room.room_code, map, room.peer_count, room.max_peers
                    );
                    spawn_online_action_button(
                        panel,
                        OnlineUiAction::JoinRoom(room.room_code.to_string()),
                        label,
                        &font,
                        room.host_connected && room.peer_count < room.max_peers,
                    );
                }
            }
            spawn_online_action_button(panel, OnlineUiAction::Back, t("返回", "Back"), &font, true);
        });
}

fn spawn_online_lobby(
    root: &mut ChildSpawnerCommands,
    session: &OnlineSession,
    font: Handle<Font>,
) {
    let Some(lobby) = session.lobby.as_ref() else {
        spawn_online_home(root, session, font);
        return;
    };
    root.spawn(online_panel_node(1240.0))
        .with_children(|panel| {
            spawn_online_title(
                panel,
                format!(
                    "{}  {}",
                    t("联机作战室", "Online War Room"),
                    lobby.room_code
                ),
                &font,
            );
            panel.spawn((
                Text::new(session.status.clone()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.66, 0.85, 0.79)),
            ));
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|controls| {
                    let map_label = format!(
                        "{}: {}",
                        t("地图", "Map"),
                        localized_skirmish_map_name(lobby.map())
                    );
                    let resources = GODOT_STARTING_RESOURCE_OPTIONS
                        [lobby.starting_resources_index % GODOT_STARTING_RESOURCE_OPTIONS.len()]
                    .resources;
                    let resource_label = format!(
                        "{}: {}/{}",
                        t("初始资源", "Resources"),
                        resources.ore,
                        resources.crystal
                    );
                    let victory = VictoryCondition::ALL
                        [lobby.victory_condition_index % VictoryCondition::ALL.len()];
                    for (action, label) in [
                        (OnlineUiAction::CycleMap, map_label),
                        (OnlineUiAction::CycleResources, resource_label),
                        (
                            OnlineUiAction::CycleVictory,
                            format!("{}: {}", t("胜利", "Victory"), victory.label()),
                        ),
                    ] {
                        spawn_online_action_button(controls, action, label, &font, session.is_host);
                    }
                });
            panel.spawn(menu_section_header(
                "玩家槽位",
                "Player slots",
                font.clone(),
            ));
            for (slot_index, slot) in lobby.slots.iter().enumerate() {
                spawn_online_slot_row(panel, session, slot_index, slot, &font);
            }
            let local_ready = session
                .local_player_id
                .and_then(|id| lobby.human_slot(id))
                .and_then(|slot| match lobby.slots[slot].occupant {
                    OnlineSlotOccupant::Human { ready, .. } => Some(ready),
                    _ => None,
                })
                .unwrap_or(false);
            spawn_online_action_button(
                panel,
                OnlineUiAction::ToggleReady,
                if local_ready {
                    t("取消准备", "Not ready")
                } else {
                    t("准备", "Ready")
                },
                &font,
                true,
            );
            if session.is_host {
                spawn_online_action_button(
                    panel,
                    OnlineUiAction::StartMatch,
                    t("开始联机对战", "Start online match"),
                    &font,
                    lobby.can_start(),
                );
            }
            spawn_online_action_button(
                panel,
                OnlineUiAction::Disconnect,
                t("离开房间", "Leave room"),
                &font,
                true,
            );
        });
}

fn spawn_online_slot_row(
    parent: &mut ChildSpawnerCommands,
    session: &OnlineSession,
    slot_index: usize,
    slot: &OnlineLobbySlot,
    font: &Handle<Font>,
) {
    let local_player = session.local_player_id;
    let (occupant_label, occupant_player) = match &slot.occupant {
        OnlineSlotOccupant::Open => (t("开放", "Open").to_string(), None),
        OnlineSlotOccupant::Closed => (t("关闭", "Closed").to_string(), None),
        OnlineSlotOccupant::Ai(difficulty) => {
            (format!("AI {}", difficulty.to_game().short_label()), None)
        }
        OnlineSlotOccupant::Human {
            player_id,
            name,
            ready,
            connected,
        } => (
            format!(
                "{}{}{}",
                name,
                if *connected {
                    ""
                } else {
                    t("（重连中）", " (reconnecting)")
                },
                if *ready {
                    t("  已准备", "  Ready")
                } else {
                    ""
                }
            ),
            Some(*player_id),
        ),
    };
    let own_slot = occupant_player.is_some_and(|id| Some(id) == local_player);
    let host_editable = session.is_host && occupant_player.is_none();
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: px(42),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::all(px(4)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(if own_slot {
                Color::srgb(0.86, 0.62, 0.22)
            } else {
                Color::srgb(0.24, 0.34, 0.32)
            }),
            BackgroundColor(Color::srgba(0.03, 0.05, 0.052, 0.9)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{:02}", slot_index + 1)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.7, 0.3)),
                Node {
                    width: px(34),
                    ..default()
                },
            ));
            spawn_online_action_button(
                row,
                OnlineUiAction::CycleOccupant(slot_index),
                occupant_label,
                font,
                host_editable,
            );
            spawn_online_action_button(
                row,
                OnlineUiAction::CycleFaction(slot_index),
                slot.faction.label(),
                font,
                own_slot || host_editable,
            );
            spawn_online_action_button(
                row,
                OnlineUiAction::CycleTeam(slot_index),
                format!("{} {}", t("队", "Team"), slot.team_id + 1),
                font,
                own_slot || host_editable,
            );
            spawn_online_action_button(
                row,
                OnlineUiAction::CycleColor(slot_index),
                format!("{} {}", t("色", "Color"), slot.color_slot + 1),
                font,
                own_slot || host_editable,
            );
        });
}

fn spawn_online_title(
    parent: &mut ChildSpawnerCommands,
    text: impl Into<String>,
    font: &Handle<Font>,
) {
    parent.spawn((
        Text::new(text.into()),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(30.0),
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.72, 0.38)),
        Node {
            align_self: AlignSelf::Center,
            ..default()
        },
    ));
}

fn spawn_online_field(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    field: OnlineTextField,
    focused: Option<OnlineTextField>,
    font: &Handle<Font>,
) {
    parent.spawn((
        Text::new(label.to_string()),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.72, 0.82, 0.8)),
    ));
    spawn_online_action_button(
        parent,
        OnlineUiAction::Focus(field),
        if value.is_empty() {
            t("点击输入", "Click to type").to_string()
        } else {
            value.to_string()
        },
        font,
        true,
    );
    if focused == Some(field) {
        parent.spawn((
            Text::new(t("正在输入", "Typing")),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.54, 0.92, 0.82)),
        ));
    }
}

fn spawn_online_action_button(
    parent: &mut ChildSpawnerCommands,
    action: OnlineUiAction,
    label: impl Into<String>,
    font: &Handle<Font>,
    enabled: bool,
) {
    parent
        .spawn((
            Button,
            OnlineButton { action, enabled },
            Node {
                flex_grow: 1.0,
                min_width: px(90),
                min_height: px(38),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(10)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(if enabled {
                Color::srgb(0.3, 0.48, 0.44)
            } else {
                Color::srgb(0.16, 0.2, 0.2)
            }),
            BackgroundColor(if enabled {
                Color::srgba(0.05, 0.08, 0.08, 0.96)
            } else {
                Color::srgba(0.03, 0.04, 0.04, 0.8)
            }),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.into()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(if enabled {
                    Color::srgb(0.86, 0.92, 0.9)
                } else {
                    Color::srgb(0.38, 0.44, 0.43)
                }),
            ));
        });
}

fn online_text_input(mut events: MessageReader<KeyboardInput>, mut session: ResMut<OnlineSession>) {
    let Some(field) = session.focused_field else {
        return;
    };
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Escape | Key::Enter => {
                session.focused_field = None;
                session.ui_dirty = true;
            }
            Key::Backspace => {
                online_field_mut(&mut session, field).pop();
                session.ui_dirty = true;
            }
            Key::Character(_) => {
                if let Some(text) = event.text.as_ref() {
                    append_online_field(&mut session, field, text.as_str());
                    session.ui_dirty = true;
                }
            }
            _ => {}
        }
    }
}

fn online_field_mut(session: &mut OnlineSession, field: OnlineTextField) -> &mut String {
    match field {
        OnlineTextField::PlayerName => &mut session.player_name,
        OnlineTextField::ServiceUrl => &mut session.service_url,
        OnlineTextField::RoomCode => &mut session.room_code_input,
        OnlineTextField::JoinToken => &mut session.join_token_input,
    }
}

fn append_online_field(session: &mut OnlineSession, field: OnlineTextField, text: &str) {
    let value = online_field_mut(session, field);
    let max = match field {
        OnlineTextField::PlayerName => 32,
        OnlineTextField::ServiceUrl => 160,
        OnlineTextField::RoomCode => 12,
        OnlineTextField::JoinToken => 128,
    };
    for character in text.chars() {
        let character = if field == OnlineTextField::RoomCode {
            character.to_ascii_uppercase()
        } else {
            character
        };
        let valid = match field {
            OnlineTextField::RoomCode => character.is_ascii_alphanumeric(),
            OnlineTextField::PlayerName => {
                !character.is_control() && !matches!(character, '/' | '\\')
            }
            OnlineTextField::ServiceUrl | OnlineTextField::JoinToken => !character.is_control(),
        };
        if valid && value.len() + character.len_utf8() <= max {
            value.push(character);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn online_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    inbox: Res<OnlineAsyncInbox>,
    mut setup: ResMut<MatchSetupSettings>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(&Interaction, &OnlineButton, &mut BackgroundColor)>,
) {
    for (interaction, button, mut background) in &mut buttons {
        let clicked = button.enabled
            && *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left);
        if clicked {
            handle_online_action(
                button.action.clone(),
                &mut session,
                &mut transport,
                &inbox,
                &mut setup,
                &mut next_state,
            );
        }
        *background = BackgroundColor(if !button.enabled {
            Color::srgba(0.03, 0.04, 0.04, 0.8)
        } else {
            match interaction {
                Interaction::Pressed => Color::srgba(0.12, 0.24, 0.2, 0.98),
                Interaction::Hovered => Color::srgba(0.08, 0.16, 0.14, 0.98),
                Interaction::None => Color::srgba(0.05, 0.08, 0.08, 0.96),
            }
        });
    }
}

fn handle_online_action(
    action: OnlineUiAction,
    session: &mut OnlineSession,
    transport: &mut OnlineTransport,
    inbox: &OnlineAsyncInbox,
    setup: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
) {
    match action {
        OnlineUiAction::Focus(field) => {
            session.focused_field = Some(field);
            session.ui_dirty = true;
        }
        OnlineUiAction::CreatePublic => begin_create_room(session, inbox),
        OnlineUiAction::RefreshRooms => begin_refresh_rooms(session, inbox),
        OnlineUiAction::JoinInputRoom => {
            let room_code = session.room_code_input.clone();
            begin_join_room(session, inbox, room_code);
        }
        OnlineUiAction::JoinRoom(room_code) => begin_join_room(session, inbox, room_code),
        OnlineUiAction::Back => {
            disconnect_online(session, transport);
            next_state.set(AppScreen::MainMenu);
        }
        OnlineUiAction::Disconnect => disconnect_online(session, transport),
        OnlineUiAction::ToggleReady => {
            let ready = session
                .local_player_id
                .and_then(|id| session.lobby.as_ref()?.human_slot(id))
                .and_then(|slot| match session.lobby.as_ref()?.slots[slot].occupant {
                    OnlineSlotOccupant::Human { ready, .. } => Some(!ready),
                    _ => None,
                });
            if let Some(ready) = ready {
                submit_lobby_command(session, transport, OnlineLobbyCommand::Ready(ready));
            }
        }
        OnlineUiAction::CycleMap => {
            if let Some(lobby) = &session.lobby {
                let current = SKIRMISH_MAPS
                    .iter()
                    .position(|map| map.id == lobby.map_id)
                    .unwrap_or(0);
                let minimum = lobby
                    .slots
                    .iter()
                    .filter(|slot| matches!(slot.occupant, OnlineSlotOccupant::Human { .. }))
                    .count();
                if let Some(map) = (1..=SKIRMISH_MAPS.len())
                    .map(|offset| &SKIRMISH_MAPS[(current + offset) % SKIRMISH_MAPS.len()])
                    .find(|map| map.players >= minimum)
                {
                    submit_lobby_command(
                        session,
                        transport,
                        OnlineLobbyCommand::Map {
                            map_id: map.id.to_string(),
                        },
                    );
                }
            }
        }
        OnlineUiAction::CycleResources => {
            let next = session
                .lobby
                .as_ref()
                .map(|lobby| {
                    (lobby.starting_resources_index + 1) % GODOT_STARTING_RESOURCE_OPTIONS.len()
                })
                .unwrap_or(0);
            submit_lobby_command(
                session,
                transport,
                OnlineLobbyCommand::StartingResources(next),
            );
        }
        OnlineUiAction::CycleVictory => {
            let next = session
                .lobby
                .as_ref()
                .map(|lobby| (lobby.victory_condition_index + 1) % VictoryCondition::ALL.len())
                .unwrap_or(0);
            submit_lobby_command(
                session,
                transport,
                OnlineLobbyCommand::VictoryCondition(next),
            );
        }
        OnlineUiAction::CycleOccupant(slot) => {
            let next = session.lobby.as_ref().and_then(|lobby| {
                let occupant = &lobby.slots.get(slot)?.occupant;
                Some(match occupant {
                    OnlineSlotOccupant::Open => OnlineSlotOccupant::Ai(OnlineAiDifficulty::Easy),
                    OnlineSlotOccupant::Ai(_) => OnlineSlotOccupant::Closed,
                    OnlineSlotOccupant::Closed => OnlineSlotOccupant::Open,
                    OnlineSlotOccupant::Human { .. } => return None,
                })
            });
            if let Some(occupant) = next {
                submit_lobby_command(
                    session,
                    transport,
                    OnlineLobbyCommand::SlotOccupant { slot, occupant },
                );
            }
        }
        OnlineUiAction::CycleFaction(slot) => {
            if let Some(faction) = session
                .lobby
                .as_ref()
                .and_then(|lobby| lobby.slots.get(slot))
                .map(|slot| slot.faction.next())
            {
                submit_lobby_command(
                    session,
                    transport,
                    OnlineLobbyCommand::Faction { slot, faction },
                );
            }
        }
        OnlineUiAction::CycleTeam(slot) => {
            if let Some(team_id) = session
                .lobby
                .as_ref()
                .and_then(|lobby| lobby.slots.get(slot))
                .map(|slot| (slot.team_id + 1) % MAX_SKIRMISH_LOBBY_SLOTS)
            {
                submit_lobby_command(
                    session,
                    transport,
                    OnlineLobbyCommand::Team { slot, team_id },
                );
            }
        }
        OnlineUiAction::CycleColor(slot) => {
            if let Some(color_slot) = session
                .lobby
                .as_ref()
                .and_then(|lobby| lobby.slots.get(slot))
                .map(|slot| (slot.color_slot + 1) % PLAYER_COLOR_PALETTE.len())
            {
                submit_lobby_command(
                    session,
                    transport,
                    OnlineLobbyCommand::Color { slot, color_slot },
                );
            }
        }
        OnlineUiAction::StartMatch => {
            host_start_online_match(session, transport, setup, next_state)
        }
    }
}

fn begin_create_room(session: &mut OnlineSession, inbox: &OnlineAsyncInbox) {
    let Some((client, player_name, build_id)) = validated_online_request(session) else {
        return;
    };
    let request = CreateRoomRequest {
        game_id: default_game_id(),
        build_id,
        protocol_version: protocol_version(),
        max_peers: MAX_SKIRMISH_LOBBY_SLOTS as u16,
        visibility: RoomVisibility::Public,
        metadata: BTreeMap::from([
            (
                "mode".to_string(),
                "host-authoritative-skirmish".to_string(),
            ),
            ("map".to_string(), SKIRMISH_MAPS[0].id.to_string()),
        ]),
    };
    session.phase = OnlinePhase::Connecting;
    session.focused_field = None;
    session.set_status(t("正在创建房间…", "Creating room..."));
    let _ = player_name;
    spawn_online_request(inbox.clone(), async move {
        let result = async {
            let config = client.service_config().await.map_err(client_error_text)?;
            let room = client
                .create_room(&request)
                .await
                .map_err(client_error_text)?;
            Ok((room, config))
        }
        .await;
        OnlineAsyncResult::Created(result)
    });
}

fn begin_join_room(session: &mut OnlineSession, inbox: &OnlineAsyncInbox, room_code: String) {
    let Some((client, _player_name, _build_id)) = validated_online_request(session) else {
        return;
    };
    let room_code = match RoomCode::new(room_code.trim().to_ascii_uppercase()) {
        Ok(room_code) => room_code,
        Err(error) => {
            session.set_status(format!("{}: {error}", t("房间码无效", "Invalid room code")));
            return;
        }
    };
    session.room_code_input = room_code.to_string();
    session.phase = OnlinePhase::Connecting;
    session.focused_field = None;
    session.set_status(t("正在加入房间…", "Joining room..."));
    spawn_online_request(inbox.clone(), async move {
        let result = async {
            let config = client.service_config().await.map_err(client_error_text)?;
            let room = client
                .room(&default_game_id(), protocol_version(), &room_code)
                .await
                .map_err(client_error_text)?;
            Ok((room, config))
        }
        .await;
        OnlineAsyncResult::Joined(result)
    });
}

fn begin_refresh_rooms(session: &mut OnlineSession, inbox: &OnlineAsyncInbox) {
    let Some((client, _, _)) = validated_online_request(session) else {
        return;
    };
    session.set_status(t("正在查询公开房间…", "Searching public rooms..."));
    spawn_online_request(inbox.clone(), async move {
        OnlineAsyncResult::Rooms(
            client
                .list_rooms(&default_game_id(), protocol_version())
                .await
                .map_err(client_error_text),
        )
    });
}

fn validated_online_request(
    session: &mut OnlineSession,
) -> Option<(RoomServiceClient, PlayerName, BuildId)> {
    let player_name = match PlayerName::new(session.player_name.trim()) {
        Ok(name) => name,
        Err(error) => {
            session.set_status(format!(
                "{}: {error}",
                t("玩家名无效", "Invalid player name")
            ));
            return None;
        }
    };
    let client = match RoomServiceClient::new(session.service_url.trim()) {
        Ok(client) => client,
        Err(error) => {
            session.set_status(format!(
                "{}: {error}",
                t("服务地址无效", "Invalid service URL")
            ));
            return None;
        }
    };
    let build = option_env!("OPEN_BEVY_BUILD_ID")
        .filter(|value| !value.is_empty())
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let build_id = BuildId::new(build).expect("package/build id is valid");
    Some((client, player_name, build_id))
}

fn process_online_async_results(
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    inbox: Res<OnlineAsyncInbox>,
    mut lifecycle: ResMut<OnlineLifecycleControl>,
) {
    let results = inbox
        .0
        .lock()
        .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
        .unwrap_or_default();
    for result in results {
        match result {
            OnlineAsyncResult::Created(Ok((room, config))) => {
                let player_name = match PlayerName::new(session.player_name.trim()) {
                    Ok(name) => name,
                    Err(error) => {
                        session.reset_connection();
                        session.set_status(error.to_string());
                        continue;
                    }
                };
                let transport_config =
                    TransportConfig::host(&room, player_name, config.ice_servers.clone());
                if let Err(error) =
                    install_online_transport(&mut transport, &inbox, transport_config)
                {
                    session.reset_connection();
                    session.set_status(error);
                    continue;
                }
                session.is_host = true;
                session.phase = OnlinePhase::Lobby;
                session.room_code_input = room.room.room_code.to_string();
                session.host_token = Some(room.host_token.clone());
                session.service_config = Some(config);
                session.room = Some(room.room.clone());
                session.local_player_id = Some(1);
                session.assigned_slot = Some(0);
                session.host_runtime = Some(HostLobbyRuntime::new(session.session_key.clone()));
                session.lobby = Some(OnlineLobbySnapshot::new(
                    room.room.room_code.to_string(),
                    session.player_name.clone(),
                ));
                session.set_status(t(
                    "房间已创建，等待其他玩家加入",
                    "Room created; waiting for players",
                ));
            }
            OnlineAsyncResult::Created(Err(error)) => {
                session.reset_connection();
                session.set_status(error);
            }
            OnlineAsyncResult::Joined(Ok((room, config))) => {
                let player_name = match PlayerName::new(session.player_name.trim()) {
                    Ok(name) => name,
                    Err(error) => {
                        session.reset_connection();
                        session.set_status(error.to_string());
                        continue;
                    }
                };
                let ticket = (!session.join_token_input.is_empty())
                    .then(|| session.join_token_input.clone());
                let transport_config = TransportConfig::player(
                    &config.websocket_base_url,
                    &room,
                    player_name,
                    ticket,
                    config.ice_servers.clone(),
                );
                if let Err(error) =
                    install_online_transport(&mut transport, &inbox, transport_config)
                {
                    session.reset_connection();
                    session.set_status(error);
                    continue;
                }
                session.is_host = false;
                session.phase = OnlinePhase::Connecting;
                session.room_code_input = room.room_code.to_string();
                session.service_config = Some(config);
                session.room = Some(room);
                session.set_status(t(
                    "已连接信令，正在建立对等连接…",
                    "Signaling connected; opening peer channel...",
                ));
            }
            OnlineAsyncResult::Joined(Err(error)) => {
                session.reset_connection();
                session.set_status(error);
            }
            OnlineAsyncResult::Rooms(Ok(rooms)) => {
                session.public_rooms = rooms.rooms;
                let rooms_empty = session.public_rooms.is_empty();
                session.set_status(if rooms_empty {
                    t("未找到公开房间", "No public rooms found")
                } else {
                    t("公开房间列表已刷新", "Public room list refreshed")
                });
            }
            OnlineAsyncResult::Rooms(Err(error)) => session.set_status(error),
            OnlineAsyncResult::TransportStopped(error) => {
                if session.phase != OnlinePhase::Home {
                    lifecycle.transport_stopped_reason = Some(error.clone());
                    session.set_status(format!(
                        "{}: {error}",
                        t("网络连接已停止", "Network connection stopped")
                    ));
                }
            }
        }
    }
}

fn install_online_transport(
    transport: &mut OnlineTransport,
    inbox: &OnlineAsyncInbox,
    config: TransportConfig,
) -> Result<(), String> {
    if let Some(socket) = transport.socket.as_mut() {
        socket.close();
    }
    let (socket, message_loop) = WebRtcTransport::connect(config).map_err(client_error_text)?;
    transport.socket = Some(socket);
    spawn_message_loop(inbox.clone(), message_loop);
    Ok(())
}

fn disconnect_online(session: &mut OnlineSession, transport: &mut OnlineTransport) {
    if let Some(socket) = transport.socket.as_mut() {
        socket.close();
    }
    transport.socket = None;
    session.reset_connection();
    session.set_status(t("已离开联机房间", "Left online room"));
}

fn poll_online_transport(
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    mut replication: ResMut<OnlineMatchReplication>,
    mut command_inbox: ResMut<OnlineCommandInbox>,
    mut lifecycle: ResMut<OnlineLifecycleControl>,
    mut setup: ResMut<MatchSetupSettings>,
    mut next_state: ResMut<NextState<AppScreen>>,
) {
    let Some(socket) = transport.socket.as_mut() else {
        return;
    };
    let events = match socket.poll() {
        Ok(events) => events,
        Err(error) => {
            session.set_status(client_error_text(error));
            return;
        }
    };
    for event in events {
        match event {
            TransportEvent::PeerConnected(peer) => {
                if !session.is_host {
                    let hello = OnlineReliableMessage::Hello {
                        protocol: RTS_ONLINE_PROTOCOL,
                        session_key: session.session_key.clone(),
                        player_name: session.player_name.clone(),
                    };
                    let _ = send_online_message(socket, peer, &hello);
                }
            }
            TransportEvent::PeerDisconnected(peer) => {
                if session.is_host {
                    let in_match = session.phase == OnlinePhase::InMatch;
                    let OnlineSession {
                        host_runtime,
                        lobby,
                        ..
                    } = &mut *session;
                    let disconnected_player = host_runtime
                        .as_mut()
                        .zip(lobby.as_mut())
                        .and_then(|(runtime, lobby)| runtime.disconnect(lobby, peer));
                    if let Some(player_id) = disconnected_player {
                        if in_match {
                            lifecycle.note_disconnect(player_id);
                        }
                        broadcast_lobby_snapshot(&session, socket);
                        session.ui_dirty = true;
                    }
                } else if session.host_peer == Some(peer) {
                    session.host_peer = None;
                    session.set_status(t(
                        "房主连接中断，等待自动恢复…",
                        "Host disconnected; waiting for automatic resume...",
                    ));
                }
            }
            TransportEvent::ReliableMessage { peer, payload } => {
                let message = match postcard::from_bytes::<OnlineReliableMessage>(&payload) {
                    Ok(message) => message,
                    Err(error) => {
                        session.set_status(format!(
                            "{}: {error}",
                            t("收到无效联机消息", "Invalid online message")
                        ));
                        continue;
                    }
                };
                process_reliable_message(
                    &mut session,
                    socket,
                    peer,
                    message,
                    &mut command_inbox,
                    &mut lifecycle,
                    &mut setup,
                    &mut next_state,
                );
            }
            TransportEvent::SnapshotMessage { peer, payload } => {
                if session.is_host
                    || session.phase != OnlinePhase::InMatch
                    || session.host_peer != Some(peer)
                {
                    continue;
                }
                let Ok(snapshot) = postcard::from_bytes::<OnlineWorldSnapshot>(&payload) else {
                    continue;
                };
                queue_online_world_snapshot(&mut replication, snapshot);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_online_lifecycle(
    mut commands: Commands,
    real_time: Res<Time<Real>>,
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    mut lifecycle: ResMut<OnlineLifecycleControl>,
    match_state: Res<MatchState>,
    mut build_queue: ResMut<BuildQueue>,
    mut next_state: ResMut<NextState<AppScreen>>,
    team_entities: Query<(Entity, &Team), With<MatchScopedEntity>>,
    pending_paradrops: Query<(Entity, &PendingParadrop)>,
) {
    if let Some(reason) = lifecycle.session_closed_reason.take() {
        disconnect_online(&mut session, &mut transport);
        session.set_status(format!(
            "{}: {reason}",
            t("联机会话已结束", "Online session ended")
        ));
        lifecycle.reset_match();
        next_state.set(AppScreen::OnlineLobby);
        return;
    }
    if let Some(reason) = lifecycle.transport_stopped_reason.take() {
        disconnect_online(&mut session, &mut transport);
        session.set_status(format!(
            "{}: {reason}",
            t("网络连接已停止", "Network connection stopped")
        ));
        lifecycle.reset_match();
        next_state.set(AppScreen::OnlineLobby);
        return;
    }

    if lifecycle.local_leave_session {
        lifecycle.local_leave_session = false;
        if session.is_host
            && session.phase == OnlinePhase::InMatch
            && let Some(socket) = transport.socket.as_mut()
        {
            let _ = broadcast_online_message(
                socket,
                &OnlineReliableMessage::SessionClosed {
                    reason: t("房主已结束联机会话", "The host ended the online session")
                        .to_string(),
                },
            );
        }
        disconnect_online(&mut session, &mut transport);
        lifecycle.reset_match();
        next_state.set(AppScreen::MainMenu);
        return;
    }

    if session.phase != OnlinePhase::InMatch {
        lifecycle.local_return_to_lobby = false;
        lifecycle.remote_return_requests.clear();
        lifecycle.disconnected_players.clear();
        lifecycle.forfeited_players.clear();
        return;
    }

    if session.is_host {
        for player_id in lifecycle.tick_disconnects(real_time.delta_secs()) {
            forfeit_disconnected_online_player(
                &mut commands,
                &session,
                player_id,
                &mut build_queue,
                &team_entities,
                &pending_paradrops,
            );
        }

        let host_requested = std::mem::take(&mut lifecycle.local_return_to_lobby);
        let client_requested_after_match =
            !match_state.is_running() && !lifecycle.remote_return_requests.is_empty();
        lifecycle.remote_return_requests.clear();
        if host_requested || client_requested_after_match {
            host_return_to_online_lobby(
                &mut session,
                &mut transport,
                &mut lifecycle,
                &mut next_state,
            );
        }
        return;
    }

    if std::mem::take(&mut lifecycle.local_return_to_lobby) {
        if match_state.is_running() {
            session.set_status(t(
                "对局进行中，只有房主可以结束整场对局",
                "Only the host can end a running match",
            ));
        } else {
            match (transport.socket.as_mut(), session.host_peer) {
                (Some(socket), Some(host_peer)) => match send_online_message(
                    socket,
                    host_peer,
                    &OnlineReliableMessage::ReturnToLobbyRequest,
                ) {
                    Ok(()) => session.set_status(t(
                        "已请求返回联机作战室，等待房主确认…",
                        "Requested return to the online war room; waiting for host...",
                    )),
                    Err(error) => session.set_status(error),
                },
                _ => session.set_status(t(
                    "房主连接尚未恢复",
                    "The host connection has not recovered yet",
                )),
            }
        }
    }
}

fn forfeit_disconnected_online_player(
    commands: &mut Commands,
    session: &OnlineSession,
    player_id: u64,
    build_queue: &mut BuildQueue,
    team_entities: &Query<(Entity, &Team), With<MatchScopedEntity>>,
    pending_paradrops: &Query<(Entity, &PendingParadrop)>,
) {
    let Some(team) = session
        .match_config
        .as_ref()
        .and_then(|config| config.runtime_team_for_player(player_id))
    else {
        return;
    };
    build_queue.0.retain(|job| job.team != team);
    for (entity, entity_team) in team_entities.iter() {
        if *entity_team == team {
            commands.entity(entity).try_despawn();
        }
    }
    for (entity, pending) in pending_paradrops.iter() {
        if pending.team == team {
            commands.entity(entity).try_despawn();
        }
    }
}

fn host_return_to_online_lobby(
    session: &mut OnlineSession,
    transport: &mut OnlineTransport,
    lifecycle: &mut OnlineLifecycleControl,
    next_state: &mut NextState<AppScreen>,
) {
    let Some(snapshot) = prepare_online_lobby_for_rematch(session, lifecycle) else {
        session.set_status(t(
            "无法恢复联机作战室",
            "Could not restore the online war room",
        ));
        return;
    };
    if let Some(socket) = transport.socket.as_mut()
        && let Err(error) = broadcast_online_message(
            socket,
            &OnlineReliableMessage::ReturnToLobby(snapshot.clone()),
        )
    {
        session.set_status(error);
    }
    accept_online_return_to_lobby(session, lifecycle, snapshot, next_state);
}

fn prepare_online_lobby_for_rematch(
    session: &mut OnlineSession,
    lifecycle: &OnlineLifecycleControl,
) -> Option<OnlineLobbySnapshot> {
    let host_player_id = session.lobby.as_ref()?.host_player_id;
    let connected_players = session
        .host_runtime
        .as_ref()?
        .peer_players
        .values()
        .copied()
        .chain(std::iter::once(host_player_id))
        .collect::<HashSet<_>>();
    let mut retained_players = HashSet::new();
    let lobby = session.lobby.as_mut()?;
    for slot in &mut lobby.slots {
        let OnlineSlotOccupant::Human {
            player_id,
            ready,
            connected,
            ..
        } = &mut slot.occupant
        else {
            continue;
        };
        if connected_players.contains(player_id) && !lifecycle.forfeited_players.contains(player_id)
        {
            *ready = false;
            *connected = true;
            retained_players.insert(*player_id);
        } else {
            slot.occupant = OnlineSlotOccupant::Open;
        }
    }
    lobby.revision = lobby.revision.saturating_add(1);
    session
        .host_runtime
        .as_mut()?
        .retain_players(&retained_players);
    Some(lobby.clone())
}

fn broadcast_online_message(
    socket: &mut WebRtcTransport,
    message: &OnlineReliableMessage,
) -> Result<(), String> {
    let payload = postcard::to_allocvec(message).map_err(|error| error.to_string())?;
    socket
        .broadcast_reliable(&payload)
        .map_err(client_error_text)
}

fn flush_online_player_commands(
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    mut outbox: ResMut<OnlineCommandOutbox>,
    mut inbox: ResMut<OnlineCommandInbox>,
) {
    if session.phase != OnlinePhase::InMatch {
        outbox.pending.clear();
        return;
    }
    if session.is_host {
        let Some(player_id) = session.local_player_id else {
            return;
        };
        while let Some(envelope) = outbox.pending.pop_front() {
            enqueue_online_player_command(&mut inbox, player_id, envelope);
        }
        return;
    }

    let Some(host_peer) = session.host_peer else {
        return;
    };
    let Some(socket) = transport.socket.as_mut() else {
        return;
    };
    while let Some(envelope) = outbox.pending.pop_front() {
        let message = OnlineReliableMessage::PlayerCommand(envelope.clone());
        if let Err(error) = send_online_message(socket, host_peer, &message) {
            outbox.pending.push_front(envelope);
            session.set_status(format!(
                "{}: {error}",
                t("无法发送对局命令", "Could not send match command")
            ));
            break;
        }
    }
}

fn apply_online_player_commands(params: OnlineCommandApplyParams) {
    let OnlineCommandApplyParams {
        mut commands,
        session,
        mut inbox,
        asset_server,
        terrain,
        map_bounds,
        relations,
        mut next_id,
        mut economies,
        mut build_queue,
        mut support_cooldowns,
        mut battle_log,
        mut audio_feedback,
        network_entities,
        actors,
        holds,
        targets,
        manual_repairs,
        support_targets,
        mut rally_points,
        structures: structure_prereqs,
        occupiers,
    } = params;
    if session.phase != OnlinePhase::InMatch || !session.is_host {
        inbox.pending.clear();
        return;
    }
    let Some(config) = session.match_config.as_ref() else {
        inbox.pending.clear();
        return;
    };
    let entity_by_network_id = network_entities
        .iter()
        .map(|(entity, network_id)| (network_id.0, entity))
        .collect::<HashMap<_, _>>();
    let host_visible_team = session
        .local_player_id
        .and_then(|player_id| config.runtime_team_for_player(player_id))
        .unwrap_or(Team::Player(0));
    let mut accepted_structure_sites = Vec::<(Vec3, f32)>::new();
    let mut acted_structures = HashSet::<Entity>::new();

    while let Some(authorized) = inbox.pending.pop_front() {
        let Some(team) = config.runtime_team_for_player(authorized.player_id) else {
            continue;
        };
        match authorized.command {
            OnlinePlayerCommand::UnitOrders { orders, queue } => {
                if orders.is_empty() || orders.len() > ONLINE_MAX_UNIT_ORDERS_PER_COMMAND {
                    continue;
                }
                let mut commanded_ids = HashSet::with_capacity(orders.len());
                for command in orders {
                    if !commanded_ids.insert(command.unit_id) {
                        continue;
                    }
                    let Some(entity) = entity_by_network_id.get(&command.unit_id).copied() else {
                        continue;
                    };
                    let Ok((actor, _network_id, actor_team, unit, health, transform, order_state)) =
                        actors.get(entity)
                    else {
                        continue;
                    };
                    if *actor_team != team || health.current <= 0.0 {
                        continue;
                    }
                    let Some(order) = resolve_online_unit_order(
                        actor,
                        unit,
                        *actor_team,
                        transform,
                        command.order,
                        *map_bounds,
                        &relations,
                        &entity_by_network_id,
                        &targets,
                    ) else {
                        continue;
                    };
                    let (
                        move_order,
                        follow_order,
                        attack_order,
                        capture_order,
                        garrison_order,
                        harvest_order,
                        repair_order,
                        construct_order,
                        attack_move_order,
                        patrol_order,
                        order_queue,
                    ) = order_state;
                    issue_or_queue_unit_order(
                        &mut commands,
                        actor,
                        order,
                        queue,
                        true,
                        has_active_orders_in_query(
                            move_order,
                            follow_order,
                            attack_order,
                            capture_order,
                            garrison_order,
                            harvest_order,
                            repair_order,
                            construct_order,
                            attack_move_order,
                            patrol_order,
                        ),
                        order_queue,
                    );
                    commands
                        .entity(actor)
                        .try_insert(HoldPosition { enabled: false });
                }
            }
            OnlinePlayerCommand::UnitAction { units, action } => {
                let Some(units) = resolve_owned_online_unit_entities(
                    team,
                    &units,
                    &entity_by_network_id,
                    &actors,
                ) else {
                    continue;
                };
                match action {
                    OnlineUnitAction::Stop => {
                        for entity in units {
                            clear_order_state(&mut commands, entity);
                            commands.entity(entity).try_remove::<OrderQueue>();
                        }
                    }
                    OnlineUnitAction::ToggleHoldPosition => {
                        if !units.iter().all(|entity| {
                            actors
                                .get(*entity)
                                .is_ok_and(|(_, _, _, unit, ..)| unit_supports_hold_position(unit))
                        }) {
                            continue;
                        }
                        let all_holding = units.iter().all(|entity| {
                            holds
                                .get(*entity)
                                .ok()
                                .flatten()
                                .is_some_and(|hold| hold.enabled)
                        });
                        let enabled = !all_holding;
                        for entity in units {
                            commands.entity(entity).try_insert(HoldPosition { enabled });
                            if enabled {
                                clear_order_state(&mut commands, entity);
                                commands.entity(entity).try_remove::<OrderQueue>();
                            }
                        }
                    }
                    OnlineUnitAction::GuardArea => {
                        if !units.iter().all(|entity| {
                            actors
                                .get(*entity)
                                .is_ok_and(|(_, _, _, unit, ..)| can_unit_guard_area(unit))
                        }) {
                            continue;
                        }
                        for entity in units {
                            clear_order_state(&mut commands, entity);
                            commands
                                .entity(entity)
                                .try_remove::<OrderQueue>()
                                .try_insert(HoldPosition { enabled: false });
                        }
                    }
                    OnlineUnitAction::Scatter => {
                        let scatter_units = units
                            .iter()
                            .filter_map(|entity| {
                                let (_, _, _, unit, _, transform, _) = actors.get(*entity).ok()?;
                                unit_supports_patrol(unit)
                                    .then_some((*entity, transform.translation))
                            })
                            .collect::<Vec<_>>();
                        if scatter_units.len() != units.len() {
                            continue;
                        }
                        let positions = scatter_units
                            .iter()
                            .map(|(_, position)| *position)
                            .collect::<Vec<_>>();
                        for ((entity, _), target) in scatter_units
                            .iter()
                            .zip(scatter_target_positions(&positions))
                        {
                            clear_order_state(&mut commands, *entity);
                            commands
                                .entity(*entity)
                                .try_remove::<OrderQueue>()
                                .try_insert(HoldPosition { enabled: false })
                                .try_insert(MoveOrder {
                                    target: map_bounds.clamp_ground_point(target, 0.0),
                                });
                        }
                    }
                    OnlineUnitAction::ToggleDeployMode => {
                        if !units.iter().all(|entity| {
                            actors
                                .get(*entity)
                                .is_ok_and(|(_, _, _, unit, ..)| is_deployable_vehicle(unit.id))
                        }) {
                            continue;
                        }
                        for entity in units {
                            commands.entity(entity).try_insert(DeployModeToggleRequest);
                        }
                    }
                }
            }
            OnlinePlayerCommand::SetRallyPoints {
                structures,
                target,
                target_entity,
                mode,
            } => {
                if structures.is_empty()
                    || structures.len() > ONLINE_MAX_RALLY_STRUCTURES_PER_COMMAND
                {
                    continue;
                }
                let requested_point = Vec3::from_array(target);
                let rally_target = target_entity.and_then(|network_id| {
                    let entity = entity_by_network_id.get(&network_id).copied()?;
                    let (
                        target_team,
                        transform,
                        unit,
                        structure,
                        resource,
                        health,
                        _under_construction,
                        _garrison,
                    ) = targets.get(entity).ok()?;
                    let alive = health.is_none_or(|health| health.current > 0.0)
                        && resource.is_none_or(|resource| resource.amount > 0);
                    let permitted = resource.is_some()
                        || (relations.are_allied(team, *target_team)
                            && (unit.is_some() || structure.is_some()));
                    (alive && permitted).then_some((entity, transform.translation))
                });
                if target_entity.is_some() && rally_target.is_none() {
                    continue;
                }
                if rally_target.is_none()
                    && validated_terrain_target_in_bounds(requested_point, *map_bounds).is_none()
                {
                    continue;
                }
                let requested = structures.into_iter().collect::<HashSet<_>>();
                for (network_id, structure_team, mut rally_point) in &mut rally_points {
                    if *structure_team != team || !requested.contains(&network_id.0) {
                        continue;
                    }
                    apply_rally_point_command_in_bounds(
                        &mut rally_point,
                        requested_point,
                        rally_target,
                        mode.to_game(),
                        *map_bounds,
                    );
                }
            }
            OnlinePlayerCommand::TrainUnits {
                producers,
                unit_id,
                batch_to_limit,
            } => {
                let (Some(economies), Some(build_queue)) =
                    (economies.as_deref_mut(), build_queue.as_deref_mut())
                else {
                    continue;
                };
                let Some(def) = valid_online_registry_entity(&unit_id) else {
                    continue;
                };
                let faction = config.runtime_faction_for_player(authorized.player_id);
                let Some(producers) = resolve_online_producers(
                    team,
                    faction,
                    def.id,
                    &producers,
                    &entity_by_network_id,
                    &targets,
                ) else {
                    continue;
                };
                if !requirements_met(def, team, &structure_prereqs) {
                    continue;
                }
                let _ = enqueue_build_jobs_for_producers(
                    team,
                    faction,
                    BuildAction::Train(def.id),
                    def,
                    &producers,
                    batch_to_limit,
                    economies,
                    build_queue,
                );
            }
            OnlinePlayerCommand::CancelProduction {
                producers,
                product_id,
                local_index,
            } => {
                let (Some(economies), Some(build_queue)) =
                    (economies.as_deref_mut(), build_queue.as_deref_mut())
                else {
                    continue;
                };
                let Some(product) = valid_online_registry_entity(&product_id) else {
                    continue;
                };
                let Some(producer_entities) = resolve_owned_online_structure_entities(
                    team,
                    &producers,
                    ONLINE_MAX_PRODUCERS_PER_COMMAND,
                    &entity_by_network_id,
                    &targets,
                ) else {
                    continue;
                };
                if let Some(local_index) = local_index {
                    if producer_entities.len() != 1
                        || usize::from(local_index) >= PRODUCTION_QUEUE_LIMIT
                    {
                        continue;
                    }
                    let _ = cancel_queued_job_at_local_index(
                        team,
                        producer_entities[0],
                        usize::from(local_index),
                        build_queue,
                        economies,
                    );
                } else {
                    let _ = cancel_latest_queued_product_for_producers(
                        team,
                        product.id,
                        &producer_entities,
                        build_queue,
                        economies,
                    );
                }
            }
            OnlinePlayerCommand::PlaceStructure {
                constructors,
                structure_id,
                position,
                rotation_y_radians,
            } => {
                let (Some(asset_server), Some(terrain), Some(next_id), Some(economies)) = (
                    asset_server.as_deref(),
                    terrain.as_deref(),
                    next_id.as_deref_mut(),
                    economies.as_deref_mut(),
                ) else {
                    continue;
                };
                let Some(def) = valid_online_registry_entity(&structure_id) else {
                    continue;
                };
                let point = Vec3::from_array(position);
                if !point.is_finite()
                    || !rotation_y_radians.is_finite()
                    || accepted_structure_sites.iter().any(|(accepted, radius)| {
                        xz_distance(*accepted, point) < *radius + def.radius
                    })
                {
                    continue;
                }
                let Some(mut constructors) = resolve_online_constructors(
                    team,
                    &constructors,
                    &entity_by_network_id,
                    &actors,
                ) else {
                    continue;
                };
                if constructors.is_empty() {
                    let Some(constructor) = nearest_online_constructor(team, point, &actors) else {
                        continue;
                    };
                    constructors.push(constructor);
                }
                let faction = config.runtime_faction_for_player(authorized.player_id);
                let Ok((structure, _)) = place_structure_at_for_faction(
                    &mut commands,
                    asset_server,
                    next_id,
                    team,
                    faction,
                    host_visible_team,
                    def.id,
                    point,
                    normalize_structure_rotation_y(rotation_y_radians),
                    *map_bounds,
                    terrain,
                    economies,
                    &structure_prereqs,
                    &occupiers,
                ) else {
                    continue;
                };
                accepted_structure_sites.push((point, def.radius));
                assign_online_constructors(
                    &mut commands,
                    team,
                    structure,
                    point,
                    &constructors,
                    &actors,
                );
            }
            OnlinePlayerCommand::StructureAction { structures, action } => {
                let Some(structures) = resolve_owned_online_action_structures(
                    team,
                    &structures,
                    &entity_by_network_id,
                    &targets,
                ) else {
                    continue;
                };
                for entity in structures {
                    if acted_structures.contains(&entity) {
                        continue;
                    }
                    let Ok((_, _, _, structure, _, health, construction, _)) = targets.get(entity)
                    else {
                        continue;
                    };
                    let (Some(structure), Some(health)) = (structure, health) else {
                        continue;
                    };
                    match action {
                        OnlineStructureAction::Sell => {
                            let (Some(economies), Some(build_queue)) =
                                (economies.as_deref_mut(), build_queue.as_deref_mut())
                            else {
                                continue;
                            };
                            let refund = if let Some(construction) = construction {
                                construction_cancel_refund(construction.cost)
                            } else {
                                let Some(def) = registry::entity(structure.id) else {
                                    continue;
                                };
                                structure_sell_refund(def, health)
                            };
                            if !acted_structures.insert(entity) {
                                continue;
                            }
                            let economy = economies.get_mut(team);
                            economy.ore += refund.0;
                            economy.crystal += refund.1;
                            cancel_jobs_for_producer(build_queue, economies, entity);
                            commands.entity(entity).try_despawn();
                        }
                        OnlineStructureAction::Repair => {
                            let Some(economies) = economies.as_deref_mut() else {
                                continue;
                            };
                            if !structure_is_constructed(construction)
                                || health.current >= health.max
                                || manual_repairs.get(entity).is_ok()
                            {
                                continue;
                            }
                            let Some(def) = registry::entity(structure.id) else {
                                continue;
                            };
                            let cost = structure_repair_cost(def, health);
                            if !economies.get_mut(team).spend(cost)
                                || !acted_structures.insert(entity)
                            {
                                continue;
                            }
                            commands.entity(entity).try_insert(ManualStructureRepair {
                                points_remaining: missing_structure_hitpoints(health),
                            });
                        }
                        OnlineStructureAction::CancelConstruction => {
                            let (Some(economies), Some(build_queue), Some(construction)) = (
                                economies.as_deref_mut(),
                                build_queue.as_deref_mut(),
                                construction,
                            ) else {
                                continue;
                            };
                            if !acted_structures.insert(entity) {
                                continue;
                            }
                            let refund = construction_cancel_refund(construction.cost);
                            let economy = economies.get_mut(team);
                            economy.ore += refund.0;
                            economy.crystal += refund.1;
                            cancel_jobs_for_producer(build_queue, economies, entity);
                            commands.entity(entity).try_despawn();
                        }
                    }
                }
            }
            OnlinePlayerCommand::UseSupportPower { power, target } => {
                let (Some(economies), Some(support_cooldowns), Some(battle_log)) = (
                    economies.as_deref(),
                    support_cooldowns.as_deref_mut(),
                    battle_log.as_deref_mut(),
                ) else {
                    continue;
                };
                let Some(target) =
                    validated_terrain_target_in_bounds(Vec3::from_array(target), *map_bounds)
                else {
                    continue;
                };
                let power = power.to_game();
                let faction = config.runtime_faction_for_player(authorized.player_id);
                if !support_power_available_to_faction(Some(faction), power) {
                    continue;
                }
                let targets = support_targets
                    .iter()
                    .filter_map(
                        |(entity, target_team, transform, health, unit, structure)| {
                            (health.current > 0.0 && (unit.is_some() || structure.is_some()))
                                .then_some(SupportPowerTargetSnapshot {
                                    entity,
                                    team: *target_team,
                                    position: transform.translation,
                                    health: *health,
                                    mobile: unit.is_some_and(|unit| unit.speed > 0.0),
                                })
                        },
                    )
                    .collect::<Vec<_>>();
                if activate_support_power(
                    &mut commands,
                    target,
                    power,
                    team,
                    host_visible_team,
                    economies,
                    support_cooldowns,
                    battle_log,
                    &relations,
                    &structure_prereqs,
                    &targets,
                ) && let Some(audio_feedback) = audio_feedback.as_deref_mut()
                {
                    record_support_power_audio_feedback(
                        audio_feedback,
                        team,
                        host_visible_team,
                        power,
                    );
                }
            }
        }
    }
}

fn valid_online_registry_entity(id: &str) -> Option<&'static registry::EntityDef> {
    (!id.is_empty() && id.len() <= ONLINE_MAX_ENTITY_ID_BYTES)
        .then(|| registry::entity(id))
        .flatten()
}

fn resolve_owned_online_unit_entities(
    team: Team,
    requested: &[u64],
    entity_by_network_id: &HashMap<u64, Entity>,
    actors: &Query<OnlineCommandActor<'_>>,
) -> Option<Vec<Entity>> {
    if requested.is_empty() || requested.len() > ONLINE_MAX_UNIT_ACTIONS_PER_COMMAND {
        return None;
    }
    let mut seen = HashSet::with_capacity(requested.len());
    let mut units = Vec::with_capacity(requested.len());
    for network_id in requested {
        if !seen.insert(*network_id) {
            return None;
        }
        let entity = entity_by_network_id.get(network_id).copied()?;
        let (_, _, unit_team, _, health, _, _) = actors.get(entity).ok()?;
        if *unit_team != team || health.current <= 0.0 {
            return None;
        }
        units.push(entity);
    }
    Some(units)
}

fn resolve_owned_online_action_structures(
    team: Team,
    requested: &[u64],
    entity_by_network_id: &HashMap<u64, Entity>,
    targets: &Query<OnlineCommandTarget<'_>>,
) -> Option<Vec<Entity>> {
    if requested.is_empty() || requested.len() > ONLINE_MAX_STRUCTURE_ACTIONS_PER_COMMAND {
        return None;
    }
    let mut seen = HashSet::with_capacity(requested.len());
    let mut structures = Vec::with_capacity(requested.len());
    for network_id in requested {
        if !seen.insert(*network_id) {
            return None;
        }
        let entity = entity_by_network_id.get(network_id).copied()?;
        let (structure_team, _, _, structure, _, health, _, _) = targets.get(entity).ok()?;
        if *structure_team != team
            || structure.is_none()
            || health.is_none_or(|health| health.current <= 0.0)
        {
            return None;
        }
        structures.push(entity);
    }
    Some(structures)
}

fn resolve_online_producers(
    team: Team,
    faction: SkirmishFaction,
    product_id: &'static str,
    requested: &[u64],
    entity_by_network_id: &HashMap<u64, Entity>,
    targets: &Query<OnlineCommandTarget<'_>>,
) -> Option<Vec<(Entity, &'static str, Vec3)>> {
    if requested.is_empty() || requested.len() > ONLINE_MAX_PRODUCERS_PER_COMMAND {
        return None;
    }
    let faction = faction_def(faction)?;
    let mut seen = HashSet::with_capacity(requested.len());
    let mut producers = Vec::with_capacity(requested.len());
    for network_id in requested {
        if !seen.insert(*network_id) {
            return None;
        }
        let entity = entity_by_network_id.get(network_id).copied()?;
        let (producer_team, transform, _, structure, _, health, construction, _) =
            targets.get(entity).ok()?;
        let (Some(structure), Some(health)) = (structure, health) else {
            return None;
        };
        if *producer_team != team
            || health.current <= 0.0
            || !structure_is_constructed(construction)
            || !faction.can_produce(structure.id, product_id)
        {
            return None;
        }
        producers.push((entity, structure.id, transform.translation));
    }
    Some(producers)
}

fn resolve_owned_online_structure_entities(
    team: Team,
    requested: &[u64],
    maximum: usize,
    entity_by_network_id: &HashMap<u64, Entity>,
    targets: &Query<OnlineCommandTarget<'_>>,
) -> Option<Vec<Entity>> {
    if requested.is_empty() || requested.len() > maximum {
        return None;
    }
    let mut seen = HashSet::with_capacity(requested.len());
    let mut structures = Vec::with_capacity(requested.len());
    for network_id in requested {
        if !seen.insert(*network_id) {
            return None;
        }
        let entity = entity_by_network_id.get(network_id).copied()?;
        let (structure_team, _, _, structure, _, health, construction, _) =
            targets.get(entity).ok()?;
        if *structure_team != team
            || structure.is_none()
            || health.is_none_or(|health| health.current <= 0.0)
            || !structure_is_constructed(construction)
        {
            return None;
        }
        structures.push(entity);
    }
    Some(structures)
}

fn resolve_online_constructors(
    team: Team,
    requested: &[u64],
    entity_by_network_id: &HashMap<u64, Entity>,
    actors: &Query<OnlineCommandActor<'_>>,
) -> Option<Vec<Entity>> {
    if requested.len() > ONLINE_MAX_CONSTRUCTORS_PER_COMMAND {
        return None;
    }
    let mut seen = HashSet::with_capacity(requested.len());
    let mut constructors = Vec::with_capacity(requested.len());
    for network_id in requested {
        if !seen.insert(*network_id) {
            return None;
        }
        let entity = entity_by_network_id.get(network_id).copied()?;
        let (_, _, constructor_team, unit, health, _, _) = actors.get(entity).ok()?;
        if *constructor_team != team
            || health.current <= 0.0
            || !can_unit_construct_structures(unit)
        {
            return None;
        }
        constructors.push(entity);
    }
    Some(constructors)
}

fn assign_online_constructors(
    commands: &mut Commands,
    team: Team,
    target: Entity,
    target_position: Vec3,
    requested: &[Entity],
    actors: &Query<OnlineCommandActor<'_>>,
) -> bool {
    if !requested.is_empty() {
        for constructor in requested {
            issue_unit_order(commands, *constructor, UnitQueuedOrder::Construct(target));
        }
        return true;
    }

    let Some(constructor) = nearest_online_constructor(team, target_position, actors) else {
        return false;
    };
    issue_unit_order(commands, constructor, UnitQueuedOrder::Construct(target));
    true
}

fn nearest_online_constructor(
    team: Team,
    target_position: Vec3,
    actors: &Query<OnlineCommandActor<'_>>,
) -> Option<Entity> {
    actors
        .iter()
        .filter(|(_, _, actor_team, unit, health, _, _)| {
            **actor_team == team && health.current > 0.0 && can_unit_construct_structures(unit)
        })
        .map(|(entity, _, _, _, _, transform, _)| {
            (entity, xz_distance(transform.translation, target_position))
        })
        .min_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(entity, _)| entity)
}

#[allow(clippy::too_many_arguments)]
fn resolve_online_unit_order(
    actor: Entity,
    unit: &Unit,
    actor_team: Team,
    actor_transform: &Transform,
    order: OnlineUnitOrderKind,
    map_bounds: MapBounds,
    relations: &TeamRelations,
    entity_by_network_id: &HashMap<u64, Entity>,
    targets: &Query<OnlineCommandTarget<'_>>,
) -> Option<UnitQueuedOrder> {
    let resolve_target = |network_id: u64| entity_by_network_id.get(&network_id).copied();
    let ground_point =
        |point: [f32; 3]| validated_terrain_target_in_bounds(Vec3::from_array(point), map_bounds);
    match order {
        OnlineUnitOrderKind::Move { destination } => (unit.speed > 0.0)
            .then(|| ground_point(destination))
            .flatten()
            .map(UnitQueuedOrder::Move),
        OnlineUnitOrderKind::Attack { target } => {
            let target = resolve_target(target)?;
            let (target_team, _, target_unit, target_structure, _, health, _, _) =
                targets.get(target).ok()?;
            (target != actor
                && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
                && (target_unit.is_some() || target_structure.is_some())
                && health.is_some_and(|health| health.current > 0.0)
                && relations.are_enemies(actor_team, *target_team))
            .then_some(UnitQueuedOrder::Attack(target))
        }
        OnlineUnitOrderKind::Capture { target } => {
            let target = resolve_target(target)?;
            let (target_team, _, _, structure, _, health, under_construction, _) =
                targets.get(target).ok()?;
            (target != actor
                && can_unit_capture(unit)
                && structure.is_some()
                && health.is_some_and(|health| health.current > 0.0)
                && structure_is_constructed(under_construction)
                && can_capture_structure_team(actor_team, *target_team, relations))
            .then_some(UnitQueuedOrder::Capture(target))
        }
        OnlineUnitOrderKind::Garrison { target } => {
            let target = resolve_target(target)?;
            let (target_team, _, _, structure, _, health, under_construction, garrison) =
                targets.get(target).ok()?;
            let (Some(structure), Some(health), Some(garrison)) = (structure, health, garrison)
            else {
                return None;
            };
            (target != actor
                && can_unit_garrison(unit)
                && can_garrison_structure_target(
                    actor_team,
                    structure,
                    *target_team,
                    health,
                    garrison,
                    under_construction,
                    relations,
                ))
            .then_some(UnitQueuedOrder::Garrison(target))
        }
        OnlineUnitOrderKind::Harvest {
            target,
            destination,
        } => {
            let target = resolve_target(target)?;
            let (target_team, _, _, structure, resource, health, under_construction, _) =
                targets.get(target).ok()?;
            if !can_unit_collect_resources(unit) || target == actor {
                return None;
            }
            match destination {
                OnlineHarvestTarget::Resource => resource
                    .is_some_and(|resource| resource.amount > 0)
                    .then_some(UnitQueuedOrder::Harvest {
                        target,
                        state: HarvestState::MovingToResource,
                    }),
                OnlineHarvestTarget::Dropoff => (structure
                    .is_some_and(is_resource_dropoff_structure)
                    && *target_team == actor_team
                    && health.is_some_and(|health| health.current > 0.0)
                    && structure_is_constructed(under_construction))
                .then_some(UnitQueuedOrder::Harvest {
                    target,
                    state: HarvestState::MovingToDropoff,
                }),
            }
        }
        OnlineUnitOrderKind::Repair { target } => {
            let target = resolve_target(target)?;
            let (target_team, _, target_unit, structure, _, health, under_construction, _) =
                targets.get(target).ok()?;
            let health = health?;
            (target != actor
                && repair_capability(unit).is_some()
                && *target_team == actor_team
                && can_repair_order_target(target_unit, structure, under_construction, health))
            .then_some(UnitQueuedOrder::Repair(target))
        }
        OnlineUnitOrderKind::Construct { target } => {
            let target = resolve_target(target)?;
            let (target_team, _, _, structure, _, health, under_construction, _) =
                targets.get(target).ok()?;
            (target != actor
                && can_unit_construct_structures(unit)
                && *target_team == actor_team
                && structure.is_some()
                && under_construction.is_some()
                && health.is_some_and(|health| health.current > 0.0))
            .then_some(UnitQueuedOrder::Construct(target))
        }
        OnlineUnitOrderKind::Follow {
            target,
            offset,
            allow_enemy,
        } => {
            let target = resolve_target(target)?;
            let (target_team, _, target_unit, structure, resource, health, _, _) =
                targets.get(target).ok()?;
            let alive = health.is_none_or(|health| health.current > 0.0)
                && resource.is_none_or(|resource| resource.amount > 0);
            let targetable = target_unit.is_some() || structure.is_some() || resource.is_some();
            let offset = Vec3::from_array(offset);
            (target != actor
                && unit.speed > 0.0
                && offset.is_finite()
                && alive
                && targetable
                && (allow_enemy || relations.are_allied(actor_team, *target_team)))
            .then_some(if allow_enemy {
                UnitQueuedOrder::ForceFollow { target, offset }
            } else {
                UnitQueuedOrder::Follow { target, offset }
            })
        }
        OnlineUnitOrderKind::AttackMove { destination } => (unit.speed > 0.0
            && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some()))
        .then(|| ground_point(destination))
        .flatten()
        .map(UnitQueuedOrder::AttackMove),
        OnlineUnitOrderKind::Patrol {
            origin,
            destination,
        } => {
            let origin = ground_point(origin).unwrap_or(actor_transform.translation);
            (unit.speed > 0.0)
                .then(|| ground_point(destination))
                .flatten()
                .map(|destination| UnitQueuedOrder::Patrol {
                    origin,
                    destination,
                })
        }
    }
}

fn queue_online_world_snapshot(
    replication: &mut OnlineMatchReplication,
    snapshot: OnlineWorldSnapshot,
) -> bool {
    if snapshot.protocol != RTS_ONLINE_PROTOCOL
        || snapshot.tick <= replication.last_applied_tick
        || replication
            .pending_snapshot
            .as_ref()
            .is_some_and(|pending| pending.tick >= snapshot.tick)
    {
        return false;
    }
    replication.pending_snapshot = Some(snapshot);
    true
}

fn broadcast_online_world_snapshot(params: OnlineSnapshotBroadcastParams) {
    let OnlineSnapshotBroadcastParams {
        time,
        mut session,
        mut transport,
        mut replication,
        entities,
        network_entities,
        economies,
        build_queue,
        support_cooldowns,
        match_state,
    } = params;
    if session.phase != OnlinePhase::InMatch || !session.is_host {
        return;
    }
    let Some(socket) = transport.socket.as_mut() else {
        return;
    };
    replication.send_accumulator += time.delta_secs();
    if replication.send_accumulator < ONLINE_SNAPSHOT_INTERVAL_SECONDS {
        return;
    }
    replication.send_accumulator %= ONLINE_SNAPSHOT_INTERVAL_SECONDS;
    replication.next_tick = replication.next_tick.saturating_add(1);

    let mut entity_snapshots = entities
        .iter()
        .filter_map(
            |(
                network_id,
                transform,
                team,
                unit,
                structure,
                resource,
                supply_crate,
                health,
                visual_faction,
                cargo,
                construction,
                veterancy,
            )| {
                let kind = if let Some(unit) = unit {
                    OnlineEntityKind::Unit {
                        id: unit.id.to_string(),
                    }
                } else if let Some(structure) = structure {
                    OnlineEntityKind::Structure {
                        id: structure.id.to_string(),
                    }
                } else if let Some(resource) = resource {
                    OnlineEntityKind::Resource {
                        kind: OnlineResourceKind::from_game(resource.kind),
                        amount: resource.amount,
                    }
                } else if let Some(supply_crate) = supply_crate {
                    OnlineEntityKind::SupplyCrate {
                        effect: OnlineSupplyCrateEffect::from_game(supply_crate.effect),
                    }
                } else {
                    return None;
                };
                Some(OnlineEntitySnapshot {
                    id: network_id.0,
                    kind,
                    team: OnlineEntityTeam::from_game(*team),
                    translation: transform.translation.to_array(),
                    rotation: transform.rotation.to_array(),
                    scale: transform.scale.to_array(),
                    health: health.map(|health| [health.current, health.max]),
                    visual_faction: visual_faction
                        .map(|faction| OnlineFaction::from_game(faction.0)),
                    cargo: cargo.map(|cargo| [cargo.capacity, cargo.ore, cargo.crystal]),
                    construction: construction
                        .map(|construction| [construction.remaining, construction.total]),
                    veterancy: veterancy
                        .map(|veterancy| (veterancy.rank, veterancy.experience_points)),
                })
            },
        )
        .collect::<Vec<_>>();
    entity_snapshots.sort_unstable_by_key(|entity| entity.id);
    let network_id_by_entity = network_entities
        .iter()
        .map(|(entity, network_id)| (entity, network_id.0))
        .collect::<HashMap<_, _>>();
    let build_queue = build_queue
        .0
        .iter()
        .filter_map(|job| {
            Some(OnlineBuildJobSnapshot {
                team: OnlineEntityTeam::from_game(job.team),
                action: OnlineBuildActionSnapshot::from_game(job.action)?,
                producer_entity: *network_id_by_entity.get(&job.producer_entity)?,
                producer_id: job.producer_id.to_string(),
                timer: job.timer,
                origin: job.origin.to_array(),
                cost: [job.cost.ore, job.cost.crystal],
            })
        })
        .collect();
    let snapshot = OnlineWorldSnapshot {
        protocol: RTS_ONLINE_PROTOCOL,
        tick: replication.next_tick,
        entities: entity_snapshots,
        economies: economies
            .players
            .iter()
            .map(|economy| OnlineEconomySnapshot {
                ore: economy.ore,
                crystal: economy.crystal,
                power_used: economy.power_used,
                power_capacity: economy.power_capacity,
                power_sabotage_remaining: economy.power_sabotage_remaining,
                production_veterancy_ranks: economy.production_veterancy_ranks.to_vec(),
            })
            .collect(),
        build_queue,
        support_cooldowns: support_cooldowns.remaining.clone(),
        support_initial_charge_started: support_cooldowns.initial_charge_started.clone(),
        match_state: OnlineMatchStateSnapshot {
            start_time_sec: match_state.start_time_sec,
            remaining_teams: match_state.remaining_teams,
            remaining_anchors: match_state.remaining_anchors,
            active_anchor_teams: match_state.active_anchor_teams.clone(),
            finished: !match_state.is_running(),
        },
    };
    let payload = match postcard::to_allocvec(&snapshot) {
        Ok(payload) if payload.len() <= MAX_SNAPSHOT_PACKET_BYTES => payload,
        Ok(payload) => {
            session.set_status(format!(
                "{}: {}/{} bytes",
                t("对局快照过大", "Match snapshot is too large"),
                payload.len(),
                MAX_SNAPSHOT_PACKET_BYTES
            ));
            return;
        }
        Err(error) => {
            session.set_status(format!(
                "{}: {error}",
                t("无法编码对局快照", "Could not encode match snapshot")
            ));
            return;
        }
    };
    if let Err(error) = socket.broadcast_snapshot(&payload) {
        session.set_status(client_error_text(error));
    }
}

fn apply_pending_online_snapshot(params: OnlineSnapshotApplyParams) {
    let OnlineSnapshotApplyParams {
        mut commands,
        asset_server,
        session,
        mut replication,
        mut next_id,
        visible_player,
        relations,
        mut economies,
        mut build_queue,
        mut support_cooldowns,
        mut match_state,
        mut match_flow,
        mut audio_feedback,
        network_entities,
        mut entities,
    } = params;
    if session.phase != OnlinePhase::InMatch || session.is_host {
        replication.pending_snapshot = None;
        return;
    }
    let Some(snapshot) = replication.pending_snapshot.take() else {
        return;
    };
    if snapshot.tick <= replication.last_applied_tick {
        return;
    }
    replication.last_applied_tick = snapshot.tick;

    economies.players = snapshot
        .economies
        .iter()
        .map(|snapshot| {
            let mut economy = TeamEconomy::new(snapshot.ore, snapshot.crystal);
            economy.power_used = snapshot.power_used;
            economy.power_capacity = snapshot.power_capacity;
            economy.power_sabotage_remaining = snapshot.power_sabotage_remaining;
            for (target, source) in economy
                .production_veterancy_ranks
                .iter_mut()
                .zip(&snapshot.production_veterancy_ranks)
            {
                *target = *source;
            }
            economy
        })
        .collect();
    match_state.start_time_sec = snapshot.match_state.start_time_sec;
    match_state.remaining_teams = snapshot.match_state.remaining_teams;
    match_state.remaining_anchors = snapshot.match_state.remaining_anchors;
    match_state.active_anchor_teams = snapshot.match_state.active_anchor_teams;
    if snapshot.match_state.finished && match_state.is_running() {
        let phase = online_match_phase_for_perspective(
            controlled_player_team(Some(&visible_player)),
            &match_state.active_anchor_teams,
            &relations,
        );
        match phase {
            MatchPhase::HumanVictory => {
                record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Victory)
            }
            MatchPhase::HumanDefeat => {
                record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Defeat)
            }
            MatchPhase::Running | MatchPhase::MatchFinished => {}
        }
        let reason = match phase {
            MatchPhase::HumanVictory => t(
                "胜利：最后一个敌对阵营已被消灭",
                "Victory: the last hostile side was eliminated",
            ),
            MatchPhase::HumanDefeat => t(
                "失利：己方阵营未能坚持到对局结束",
                "Defeat: your side did not survive the match",
            ),
            MatchPhase::Running | MatchPhase::MatchFinished => t("战斗结束", "Battle Over"),
        };
        finalize_match(&mut match_state, &mut match_flow, phase, reason);
    }

    let entity_by_network_id = network_entities
        .iter()
        .map(|(entity, network_id)| (network_id.0, entity))
        .collect::<HashMap<_, _>>();
    build_queue.0 = snapshot
        .build_queue
        .iter()
        .filter_map(|job| {
            let producer_entity = entity_by_network_id.get(&job.producer_entity).copied()?;
            let producer_id = registry::entity(&job.producer_id)?.id;
            Some(BuildJob {
                team: job.team.to_game(),
                action: job.action.to_game()?,
                producer_entity,
                producer_id,
                timer: job.timer.max(0.0),
                origin: Vec3::from_array(job.origin),
                cost: registry::Cost {
                    ore: job.cost[0],
                    crystal: job.cost[1],
                },
            })
        })
        .collect();
    support_cooldowns.remaining = snapshot
        .support_cooldowns
        .into_iter()
        .take(MAX_SKIRMISH_LOBBY_SLOTS)
        .collect();
    support_cooldowns.initial_charge_started = snapshot
        .support_initial_charge_started
        .into_iter()
        .take(MAX_SKIRMISH_LOBBY_SLOTS)
        .collect();

    let mut incoming = snapshot
        .entities
        .into_iter()
        .map(|entity| (entity.id, entity))
        .collect::<HashMap<_, _>>();
    for (
        entity,
        network_id,
        mut transform,
        team,
        health,
        resource,
        cargo,
        construction,
        veterancy,
    ) in &mut entities
    {
        let Some(snapshot) = incoming.remove(&network_id.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let target = snapshot.transform();
        if transform.translation.distance(target.translation) > ONLINE_SNAPSHOT_SNAP_DISTANCE {
            *transform = target;
            commands.entity(entity).try_remove::<NetworkInterpolation>();
        } else {
            commands.entity(entity).try_insert(NetworkInterpolation {
                translation: target.translation,
                rotation: target.rotation,
                scale: target.scale,
            });
        }
        if *team != snapshot.team.to_game() {
            commands.entity(entity).try_insert(snapshot.team.to_game());
        }
        sync_optional_health(&mut commands, entity, health, snapshot.health);
        sync_optional_resource(&mut commands, entity, resource, &snapshot.kind);
        sync_optional_cargo(&mut commands, entity, cargo, snapshot.cargo);
        sync_optional_construction(
            &mut commands,
            entity,
            construction,
            snapshot.construction,
            &snapshot.kind,
        );
        sync_optional_veterancy(
            &mut commands,
            entity,
            veterancy,
            snapshot.veterancy,
            &snapshot.kind,
        );
        sync_visual_faction(&mut commands, entity, snapshot.visual_faction);
    }

    for snapshot in incoming.into_values() {
        spawn_online_snapshot_entity(
            &mut commands,
            &asset_server,
            &mut next_id,
            visible_player.team,
            &snapshot,
        );
    }
}

fn sync_optional_health(
    commands: &mut Commands,
    entity: Entity,
    health: Option<Mut<Health>>,
    snapshot: Option<[f32; 2]>,
) {
    match (health, snapshot) {
        (Some(mut health), Some([current, max])) => {
            health.current = current;
            health.max = max;
        }
        (None, Some([current, max])) => {
            commands.entity(entity).try_insert(Health { current, max });
        }
        (Some(_), None) => {
            commands.entity(entity).try_remove::<Health>();
        }
        (None, None) => {}
    }
}

fn sync_optional_resource(
    commands: &mut Commands,
    entity: Entity,
    resource: Option<Mut<ResourceNode>>,
    kind: &OnlineEntityKind,
) {
    match (resource, kind) {
        (Some(mut resource), OnlineEntityKind::Resource { kind, amount }) => {
            resource.kind = kind.to_game();
            resource.amount = *amount;
        }
        (None, OnlineEntityKind::Resource { kind, amount }) => {
            commands.entity(entity).try_insert(ResourceNode {
                kind: kind.to_game(),
                amount: *amount,
            });
        }
        (Some(_), _) => {
            commands.entity(entity).try_remove::<ResourceNode>();
        }
        (None, _) => {}
    }
}

fn sync_optional_cargo(
    commands: &mut Commands,
    entity: Entity,
    cargo: Option<Mut<ResourceCargo>>,
    snapshot: Option<[i32; 3]>,
) {
    match (cargo, snapshot) {
        (Some(mut cargo), Some([capacity, ore, crystal])) => {
            cargo.capacity = capacity;
            cargo.ore = ore;
            cargo.crystal = crystal;
        }
        (None, Some([capacity, ore, crystal])) => {
            commands.entity(entity).try_insert(ResourceCargo {
                capacity,
                ore,
                crystal,
            });
        }
        (Some(_), None) => {
            commands.entity(entity).try_remove::<ResourceCargo>();
        }
        (None, None) => {}
    }
}

fn sync_optional_construction(
    commands: &mut Commands,
    entity: Entity,
    construction: Option<Mut<UnderConstruction>>,
    snapshot: Option<[f32; 2]>,
    kind: &OnlineEntityKind,
) {
    match (construction, snapshot) {
        (Some(mut construction), Some([remaining, total])) => {
            construction.remaining = remaining;
            construction.total = total;
        }
        (None, Some([remaining, total])) => {
            let cost = match kind {
                OnlineEntityKind::Structure { id } => {
                    registry::entity(id).map_or(registry::Cost::default(), |def| def.cost)
                }
                _ => registry::Cost::default(),
            };
            commands.entity(entity).try_insert(UnderConstruction {
                remaining,
                total,
                cost,
                free_worker_origin: None,
            });
        }
        (Some(_), None) => {
            commands.entity(entity).try_remove::<UnderConstruction>();
        }
        (None, None) => {}
    }
}

fn sync_optional_veterancy(
    commands: &mut Commands,
    entity: Entity,
    veterancy: Option<Mut<Veterancy>>,
    snapshot: Option<(u8, u32)>,
    kind: &OnlineEntityKind,
) {
    match (veterancy, snapshot) {
        (Some(mut veterancy), Some((rank, experience_points))) => {
            veterancy.rank = rank;
            veterancy.experience_points = experience_points;
        }
        (None, Some((rank, experience_points))) => {
            let OnlineEntityKind::Unit { id } = kind else {
                return;
            };
            let Some(def) = registry::entity(id) else {
                return;
            };
            let Some(weapon) = def.weapon else {
                return;
            };
            commands.entity(entity).try_insert(Veterancy {
                rank,
                experience_points,
                base_health: def.health,
                base_damage: weapon.damage,
                base_range: weapon.range,
                base_vision: unit_vision_radius(def),
            });
        }
        (Some(_), None) => {
            commands.entity(entity).try_remove::<Veterancy>();
        }
        (None, None) => {}
    }
}

fn sync_visual_faction(commands: &mut Commands, entity: Entity, snapshot: Option<OnlineFaction>) {
    if let Some(faction) = snapshot {
        commands
            .entity(entity)
            .try_insert(VisualFaction(faction.to_game()));
    } else {
        commands.entity(entity).try_remove::<VisualFaction>();
    }
}

fn spawn_online_snapshot_entity(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    visible_team: Team,
    snapshot: &OnlineEntitySnapshot,
) {
    let team = snapshot.team.to_game();
    let visual_faction = snapshot.visual_faction.map(OnlineFaction::to_game);
    let transform = snapshot.transform();
    let entity = match &snapshot.kind {
        OnlineEntityKind::Unit { id } => {
            let Some(def) = registry::entity(id) else {
                return;
            };
            spawn_unit_with_visual_faction(
                commands,
                asset_server,
                next_id,
                def.id,
                team,
                transform.translation,
                snapshot.veterancy.map_or(0, |veterancy| veterancy.0),
                visual_faction,
                visible_team,
            )
        }
        OnlineEntityKind::Structure { id } => {
            let Some(def) = registry::entity(id) else {
                return;
            };
            if snapshot.construction.is_some() {
                spawn_structure_under_construction_with_visual_faction(
                    commands,
                    asset_server,
                    next_id,
                    def.id,
                    team,
                    transform.translation,
                    None,
                    0.0,
                    visible_team,
                    visual_faction,
                )
            } else {
                spawn_structure_for_visual_faction(
                    commands,
                    asset_server,
                    next_id,
                    def.id,
                    team,
                    visible_team,
                    transform.translation,
                    0.0,
                    visual_faction,
                )
            }
        }
        OnlineEntityKind::Resource { kind, amount } => spawn_resource_node(
            commands,
            asset_server,
            kind.to_game(),
            *amount,
            transform.translation,
        ),
        OnlineEntityKind::SupplyCrate { effect } => spawn_supply_crate(
            commands,
            asset_server,
            effect.to_game(),
            transform.translation,
        ),
    };
    commands
        .entity(entity)
        .try_insert((NetworkEntityId(snapshot.id), transform, team));
    if let Some([current, max]) = snapshot.health {
        commands.entity(entity).try_insert(Health { current, max });
    }
    if let Some([capacity, ore, crystal]) = snapshot.cargo {
        commands.entity(entity).try_insert(ResourceCargo {
            capacity,
            ore,
            crystal,
        });
    }
    if let Some([remaining, total]) = snapshot.construction {
        let cost = match &snapshot.kind {
            OnlineEntityKind::Structure { id } => {
                registry::entity(id).map_or(registry::Cost::default(), |def| def.cost)
            }
            _ => registry::Cost::default(),
        };
        commands.entity(entity).try_insert(UnderConstruction {
            remaining,
            total,
            cost,
            free_worker_origin: None,
        });
    }
    if let Some((rank, experience_points)) = snapshot.veterancy
        && let OnlineEntityKind::Unit { id } = &snapshot.kind
        && let Some(def) = registry::entity(id)
        && let Some(weapon) = def.weapon
    {
        commands.entity(entity).try_insert(Veterancy {
            rank,
            experience_points,
            base_health: def.health,
            base_damage: weapon.damage,
            base_range: weapon.range,
            base_vision: unit_vision_radius(def),
        });
    }
    sync_visual_faction(commands, entity, snapshot.visual_faction);
}

fn interpolate_network_entities(
    mut commands: Commands,
    time: Res<Time>,
    mut entities: Query<(Entity, &mut Transform, &NetworkInterpolation)>,
) {
    let alpha = 1.0 - (-18.0 * time.delta_secs()).exp();
    for (entity, mut transform, target) in &mut entities {
        transform.translation = transform.translation.lerp(target.translation, alpha);
        transform.rotation = transform.rotation.slerp(target.rotation, alpha);
        transform.scale = transform.scale.lerp(target.scale, alpha);
        if transform.translation.distance_squared(target.translation) < 0.0001
            && transform.rotation.angle_between(target.rotation) < 0.001
        {
            transform.translation = target.translation;
            transform.rotation = target.rotation;
            transform.scale = target.scale;
            commands.entity(entity).try_remove::<NetworkInterpolation>();
        }
    }
}

fn process_reliable_message(
    session: &mut OnlineSession,
    socket: &mut WebRtcTransport,
    peer: PeerId,
    message: OnlineReliableMessage,
    command_inbox: &mut OnlineCommandInbox,
    lifecycle: &mut OnlineLifecycleControl,
    setup: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
) {
    if session.is_host {
        match message {
            OnlineReliableMessage::Hello {
                protocol,
                session_key,
                player_name,
            } => {
                if protocol != RTS_ONLINE_PROTOCOL {
                    let _ = send_online_message(
                        socket,
                        peer,
                        &OnlineReliableMessage::Rejected {
                            reason: "RTS lobby protocol mismatch".to_string(),
                        },
                    );
                    return;
                }
                let allow_new_player = session.phase != OnlinePhase::InMatch;
                let resumed_player = session
                    .host_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.player_for_session(&session_key));
                if !allow_new_player && resumed_player.is_none() {
                    let _ = send_online_message(
                        socket,
                        peer,
                        &OnlineReliableMessage::Rejected {
                            reason: "the match is already in progress".to_string(),
                        },
                    );
                    return;
                }
                if resumed_player
                    .is_some_and(|player_id| lifecycle.forfeited_players.contains(&player_id))
                {
                    let _ = send_online_message(
                        socket,
                        peer,
                        &OnlineReliableMessage::Rejected {
                            reason: "the player already forfeited this match".to_string(),
                        },
                    );
                    return;
                }
                let admission = session
                    .host_runtime
                    .as_mut()
                    .zip(session.lobby.as_mut())
                    .ok_or("host lobby is unavailable")
                    .and_then(|(runtime, lobby)| {
                        runtime.admit(lobby, peer, session_key, player_name, allow_new_player)
                    });
                match admission {
                    Ok((player_id, assigned_slot)) => {
                        lifecycle.note_reconnect(player_id);
                        let snapshot = session.lobby.clone().expect("host lobby exists");
                        let _ = send_online_message(
                            socket,
                            peer,
                            &OnlineReliableMessage::Welcome {
                                player_id,
                                assigned_slot,
                                snapshot,
                                match_config: session.match_config.clone(),
                            },
                        );
                        broadcast_lobby_snapshot(session, socket);
                        session.set_status(t("玩家已加入", "Player joined"));
                    }
                    Err(reason) => {
                        let _ = send_online_message(
                            socket,
                            peer,
                            &OnlineReliableMessage::Rejected {
                                reason: reason.to_string(),
                            },
                        );
                    }
                }
            }
            OnlineReliableMessage::LobbyCommand(command) => {
                let Some(player_id) = session
                    .host_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.peer_players.get(&peer).copied())
                else {
                    return;
                };
                if apply_lobby_command(session, player_id, command) {
                    broadcast_lobby_snapshot(session, socket);
                }
            }
            OnlineReliableMessage::PlayerCommand(envelope)
                if session.phase == OnlinePhase::InMatch =>
            {
                let Some(player_id) = session
                    .host_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.peer_players.get(&peer).copied())
                else {
                    return;
                };
                enqueue_online_player_command(command_inbox, player_id, envelope);
            }
            OnlineReliableMessage::ReturnToLobbyRequest
                if session.phase == OnlinePhase::InMatch =>
            {
                let Some(player_id) = session
                    .host_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.peer_players.get(&peer).copied())
                else {
                    return;
                };
                if !lifecycle.remote_return_requests.contains(&player_id) {
                    lifecycle.remote_return_requests.push(player_id);
                }
            }
            _ => {}
        }
        return;
    }

    match message {
        OnlineReliableMessage::Welcome {
            player_id,
            assigned_slot,
            snapshot,
            match_config,
        } => accept_online_welcome(
            session,
            peer,
            player_id,
            assigned_slot,
            snapshot,
            match_config,
            setup,
            next_state,
        ),
        OnlineReliableMessage::LobbySnapshot(snapshot) if session.host_peer == Some(peer) => {
            session.lobby = Some(snapshot);
            session.ui_dirty = true;
        }
        OnlineReliableMessage::StartMatch(config) if session.host_peer == Some(peer) => {
            let Some(local_slot) = session.assigned_slot else {
                session.set_status(t("缺少本地玩家槽位", "Missing local player slot"));
                return;
            };
            match online_match_setup(&config, local_slot) {
                Ok(settings) => {
                    *setup = settings;
                    session.match_config = Some(config);
                    session.phase = OnlinePhase::InMatch;
                    next_state.set(AppScreen::InMatch);
                }
                Err(error) => session.set_status(error),
            }
        }
        OnlineReliableMessage::ReturnToLobby(snapshot) if session.host_peer == Some(peer) => {
            accept_online_return_to_lobby(session, lifecycle, snapshot, next_state);
        }
        OnlineReliableMessage::SessionClosed { reason } if session.host_peer == Some(peer) => {
            lifecycle.session_closed_reason = Some(reason);
        }
        OnlineReliableMessage::Rejected { reason } => {
            session.set_status(format!("{}: {reason}", t("加入被拒绝", "Join rejected")));
        }
        _ => {}
    }
}

fn accept_online_welcome(
    session: &mut OnlineSession,
    peer: PeerId,
    player_id: u64,
    assigned_slot: usize,
    snapshot: OnlineLobbySnapshot,
    match_config: Option<OnlineMatchConfig>,
    setup: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
) {
    session.host_peer = Some(peer);
    session.local_player_id = Some(player_id);
    session.assigned_slot = Some(assigned_slot);
    session.lobby = Some(snapshot);
    match match_config {
        Some(config) => match online_match_setup(&config, assigned_slot) {
            Ok(settings) => {
                *setup = settings;
                session.match_config = Some(config);
                session.phase = OnlinePhase::InMatch;
                session.set_status(t(
                    "已恢复主机连接，正在同步对局…",
                    "Host connection restored; synchronizing match...",
                ));
                next_state.set(AppScreen::InMatch);
            }
            Err(error) => session.set_status(error),
        },
        None => {
            session.match_config = None;
            session.phase = OnlinePhase::Lobby;
            session.set_status(t("已加入联机作战室", "Joined online war room"));
            next_state.set(AppScreen::OnlineLobby);
        }
    }
}

fn accept_online_return_to_lobby(
    session: &mut OnlineSession,
    lifecycle: &mut OnlineLifecycleControl,
    snapshot: OnlineLobbySnapshot,
    next_state: &mut NextState<AppScreen>,
) {
    session.lobby = Some(snapshot);
    session.match_config = None;
    session.phase = OnlinePhase::Lobby;
    session.set_status(t("已返回联机作战室", "Returned to online war room"));
    lifecycle.reset_match();
    next_state.set(AppScreen::OnlineLobby);
}

fn submit_lobby_command(
    session: &mut OnlineSession,
    transport: &mut OnlineTransport,
    command: OnlineLobbyCommand,
) {
    let Some(socket) = transport.socket.as_mut() else {
        return;
    };
    if session.is_host {
        if apply_lobby_command(session, 1, command) {
            broadcast_lobby_snapshot(session, socket);
        }
    } else if let Some(host_peer) = session.host_peer
        && let Err(error) = send_online_message(
            socket,
            host_peer,
            &OnlineReliableMessage::LobbyCommand(command),
        )
    {
        session.set_status(error);
    }
}

fn apply_lobby_command(
    session: &mut OnlineSession,
    player_id: u64,
    command: OnlineLobbyCommand,
) -> bool {
    let is_host = session
        .lobby
        .as_ref()
        .is_some_and(|lobby| lobby.host_player_id == player_id);
    let Some(lobby) = session.lobby.as_mut() else {
        return false;
    };
    let own_slot = lobby.human_slot(player_id);
    let changed = match command {
        OnlineLobbyCommand::Ready(ready) => own_slot.is_some_and(|slot| {
            if let OnlineSlotOccupant::Human { ready: current, .. } =
                &mut lobby.slots[slot].occupant
            {
                let changed = *current != ready;
                *current = ready;
                changed
            } else {
                false
            }
        }),
        OnlineLobbyCommand::Faction { slot, faction } => {
            editable_lobby_slot(lobby, own_slot, is_host, slot).is_some_and(|slot| {
                let changed = slot.faction != faction;
                slot.faction = faction;
                changed
            })
        }
        OnlineLobbyCommand::Team { slot, team_id } => {
            editable_lobby_slot(lobby, own_slot, is_host, slot).is_some_and(|slot| {
                let team_id = team_id % MAX_SKIRMISH_LOBBY_SLOTS;
                let changed = slot.team_id != team_id;
                slot.team_id = team_id;
                changed
            })
        }
        OnlineLobbyCommand::Color { slot, color_slot } => {
            editable_lobby_slot(lobby, own_slot, is_host, slot).is_some_and(|slot| {
                let color_slot = color_slot % PLAYER_COLOR_PALETTE.len();
                let changed = slot.color_slot != color_slot;
                slot.color_slot = color_slot;
                changed
            })
        }
        OnlineLobbyCommand::Map { map_id } if is_host => {
            let Some(map) = SKIRMISH_MAPS.iter().find(|map| map.id == map_id) else {
                return false;
            };
            let human_count = lobby
                .slots
                .iter()
                .filter(|slot| matches!(slot.occupant, OnlineSlotOccupant::Human { .. }))
                .count();
            if map.players < human_count || lobby.map_id == map.id {
                false
            } else {
                resize_lobby_for_map(lobby, map);
                true
            }
        }
        OnlineLobbyCommand::StartingResources(index) if is_host => {
            if index >= GODOT_STARTING_RESOURCE_OPTIONS.len() {
                false
            } else {
                let changed = lobby.starting_resources_index != index;
                lobby.starting_resources_index = index;
                changed
            }
        }
        OnlineLobbyCommand::VictoryCondition(index) if is_host => {
            if index >= VictoryCondition::ALL.len() {
                false
            } else {
                let changed = lobby.victory_condition_index != index;
                lobby.victory_condition_index = index;
                changed
            }
        }
        OnlineLobbyCommand::SlotOccupant { slot, occupant } if is_host => {
            let Some(target) = lobby.slots.get_mut(slot) else {
                return false;
            };
            if matches!(target.occupant, OnlineSlotOccupant::Human { .. })
                || matches!(occupant, OnlineSlotOccupant::Human { .. })
            {
                false
            } else {
                let changed = target.occupant != occupant;
                target.occupant = occupant;
                changed
            }
        }
        _ => false,
    };
    if changed {
        lobby.revision = lobby.revision.saturating_add(1);
        session.ui_dirty = true;
    }
    changed
}

fn editable_lobby_slot(
    lobby: &mut OnlineLobbySnapshot,
    own_slot: Option<usize>,
    is_host: bool,
    requested: usize,
) -> Option<&mut OnlineLobbySlot> {
    let slot = lobby.slots.get(requested)?;
    let permitted = own_slot == Some(requested)
        || (is_host && !matches!(slot.occupant, OnlineSlotOccupant::Human { .. }));
    permitted.then(|| &mut lobby.slots[requested])
}

fn resize_lobby_for_map(lobby: &mut OnlineLobbySnapshot, map: &SkirmishMapDef) {
    lobby.map_id = map.id.to_string();
    let previous_len = lobby.slots.len();
    lobby.slots.resize_with(map.players, || {
        let slot = previous_len;
        OnlineLobbySlot {
            occupant: OnlineSlotOccupant::Closed,
            faction: OnlineFaction::Alliance,
            team_id: slot,
            color_slot: slot,
            spawn_slot: slot,
        }
    });
    for (index, slot) in lobby.slots.iter_mut().enumerate() {
        slot.spawn_slot = index;
        if index >= previous_len {
            slot.team_id = index;
            slot.color_slot = index;
            slot.faction = OnlineFaction::from_game(DEFAULT_LOBBY_FACTIONS[index]);
        }
    }
}

fn broadcast_lobby_snapshot(session: &OnlineSession, socket: &mut WebRtcTransport) {
    let Some(snapshot) = session.lobby.clone() else {
        return;
    };
    if let Ok(payload) = postcard::to_allocvec(&OnlineReliableMessage::LobbySnapshot(snapshot)) {
        let _ = socket.broadcast_reliable(&payload);
    }
}

fn host_start_online_match(
    session: &mut OnlineSession,
    transport: &mut OnlineTransport,
    setup: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
) {
    if !session.is_host {
        return;
    }
    let Some(lobby) = session.lobby.as_ref() else {
        return;
    };
    if !lobby.can_start() {
        session.set_status(t(
            "至少需要两名已连接且已准备的玩家",
            "At least two connected, ready players are required",
        ));
        return;
    }
    let config = OnlineMatchConfig::from(lobby);
    let settings = match online_match_setup(&config, 0) {
        Ok(settings) => settings,
        Err(error) => {
            session.set_status(error);
            return;
        }
    };
    let Some(socket) = transport.socket.as_mut() else {
        return;
    };
    let payload = match postcard::to_allocvec(&OnlineReliableMessage::StartMatch(config.clone())) {
        Ok(payload) => payload,
        Err(error) => {
            session.set_status(format!(
                "{}: {error}",
                t("无法编码对局", "Could not encode match")
            ));
            return;
        }
    };
    if let Err(error) = socket.broadcast_reliable(&payload) {
        session.set_status(client_error_text(error));
        return;
    }
    *setup = settings;
    session.match_config = Some(config);
    session.phase = OnlinePhase::InMatch;
    next_state.set(AppScreen::InMatch);
}

fn online_match_setup(
    config: &OnlineMatchConfig,
    local_source_slot: usize,
) -> Result<MatchSetupSettings, String> {
    let map = SKIRMISH_MAPS
        .iter()
        .find(|map| map.id == config.map_id)
        .ok_or_else(|| "online match selected an unknown map".to_string())?;
    let starting_resources = GODOT_STARTING_RESOURCE_OPTIONS
        .get(config.starting_resources_index)
        .ok_or_else(|| "online match selected invalid starting resources".to_string())?
        .resources;
    let victory_condition = VictoryCondition::ALL
        .get(config.victory_condition_index)
        .copied()
        .ok_or_else(|| "online match selected an invalid victory condition".to_string())?;
    let local_runtime_slot = config
        .slots
        .iter()
        .position(|slot| slot.source_slot == local_source_slot)
        .ok_or_else(|| "local player slot is not active in the match".to_string())?;
    let player_controllers = config
        .slots
        .iter()
        .map(|slot| match slot.occupant {
            OnlineSlotOccupant::Human { .. } => SkirmishPlayerController::Human,
            OnlineSlotOccupant::Ai(difficulty) => {
                SkirmishPlayerController::Ai(difficulty.to_game())
            }
            OnlineSlotOccupant::Open | OnlineSlotOccupant::Closed => SkirmishPlayerController::None,
        })
        .collect::<Vec<_>>();
    let active_teams = vec![true; config.slots.len()];
    let team_ids = config
        .slots
        .iter()
        .map(|slot| slot.team_id)
        .collect::<Vec<_>>();
    Ok(MatchSetupSettings {
        map_path: map.godot_path,
        starting_resources,
        visible_player: VisiblePlayer::per_player(Team::Player(local_runtime_slot)),
        ai_difficulties: skirmish_ai_difficulties_from_controllers(&player_controllers),
        team_relations: skirmish_team_relations_from_team_ids(&active_teams, &team_ids),
        startup_loadout: StartupLoadoutMode::GodotSkirmish,
        victory_condition,
        active_teams,
        player_factions: config
            .slots
            .iter()
            .map(|slot| slot.faction.to_game())
            .collect(),
        player_color_slots: config.slots.iter().map(|slot| slot.color_slot).collect(),
        player_controllers,
        player_spawn_slots: config.slots.iter().map(|slot| slot.source_slot).collect(),
    })
}

fn send_online_message(
    socket: &mut WebRtcTransport,
    peer: PeerId,
    message: &OnlineReliableMessage,
) -> Result<(), String> {
    let payload = postcard::to_allocvec(message).map_err(|error| error.to_string())?;
    socket
        .send_reliable(peer, payload)
        .map_err(client_error_text)
}

fn client_error_text(error: ClientError) -> String {
    error.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_online_request<F>(inbox: OnlineAsyncInbox, future: F)
where
    F: Future<Output = OnlineAsyncResult> + Send + 'static,
{
    IoTaskPool::get()
        .spawn(async move {
            let result = future.await;
            if let Ok(mut queue) = inbox.0.lock() {
                queue.push_back(result);
            }
        })
        .detach();
}

#[cfg(target_arch = "wasm32")]
fn spawn_online_request<F>(inbox: OnlineAsyncInbox, future: F)
where
    F: Future<Output = OnlineAsyncResult> + 'static,
{
    IoTaskPool::get()
        .spawn_local(async move {
            let result = future.await;
            if let Ok(mut queue) = inbox.0.lock() {
                queue.push_back(result);
            }
        })
        .detach();
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_message_loop(inbox: OnlineAsyncInbox, message_loop: MessageLoopFuture) {
    IoTaskPool::get()
        .spawn(async move {
            if let Err(error) = message_loop.await
                && let Ok(mut queue) = inbox.0.lock()
            {
                queue.push_back(OnlineAsyncResult::TransportStopped(error.to_string()));
            }
        })
        .detach();
}

#[cfg(target_arch = "wasm32")]
fn spawn_message_loop(inbox: OnlineAsyncInbox, message_loop: MessageLoopFuture) {
    IoTaskPool::get()
        .spawn_local(async move {
            if let Err(error) = message_loop.await
                && let Ok(mut queue) = inbox.0.lock()
            {
                queue.push_back(OnlineAsyncResult::TransportStopped(error.to_string()));
            }
        })
        .detach();
}

fn new_session_key() -> String {
    Uuid::new_v4().simple().to_string()
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    if value.len() <= max_bytes {
        return;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_keys_are_random_browser_safe_identifiers() {
        let first = new_session_key();
        let second = new_session_key();
        assert_ne!(first, second);
        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    fn sample_world_snapshot(tick: u64, entity_count: usize) -> OnlineWorldSnapshot {
        OnlineWorldSnapshot {
            protocol: RTS_ONLINE_PROTOCOL,
            tick,
            entities: (0..entity_count)
                .map(|index| OnlineEntitySnapshot {
                    id: index as u64 + 1,
                    kind: OnlineEntityKind::Unit {
                        id: "HeavyAssaultVehicle".to_string(),
                    },
                    team: OnlineEntityTeam::Player(index % 8),
                    translation: [index as f32, 0.5, index as f32 * 0.25],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0; 3],
                    health: Some([75.0, 100.0]),
                    visual_faction: Some(OnlineFaction::Alliance),
                    cargo: None,
                    construction: None,
                    veterancy: Some((2, 12)),
                })
                .collect(),
            economies: (0..8)
                .map(|_| OnlineEconomySnapshot {
                    ore: 999,
                    crystal: 333,
                    power_used: 70,
                    power_capacity: 100,
                    power_sabotage_remaining: 0.0,
                    production_veterancy_ranks: vec![3; 3],
                })
                .collect(),
            build_queue: Vec::new(),
            support_cooldowns: vec![[12.0; SupportPowerKind::ALL.len()]; 8],
            support_initial_charge_started: vec![[true; SupportPowerKind::ALL.len()]; 8],
            match_state: OnlineMatchStateSnapshot {
                start_time_sec: 600.0,
                remaining_teams: 8,
                remaining_anchors: 8,
                active_anchor_teams: vec![true; 8],
                finished: false,
            },
        }
    }

    #[test]
    fn network_entity_namespaces_never_overlap() {
        let dynamic = NetworkEntityId::dynamic(u32::MAX);
        let resource = NetworkEntityId::map_resource(0);
        let next_resource = NetworkEntityId::map_resource(1);
        let crate_id = NetworkEntityId::supply_crate(0);

        assert_ne!(dynamic, resource);
        assert_ne!(resource, crate_id);
        assert_ne!(resource, next_resource);
    }

    #[test]
    fn world_snapshot_rejects_stale_and_wrong_protocol_packets() {
        let mut replication = OnlineMatchReplication {
            last_applied_tick: 10,
            ..default()
        };
        assert!(!queue_online_world_snapshot(
            &mut replication,
            sample_world_snapshot(10, 1)
        ));
        assert!(queue_online_world_snapshot(
            &mut replication,
            sample_world_snapshot(12, 1)
        ));
        assert!(!queue_online_world_snapshot(
            &mut replication,
            sample_world_snapshot(11, 1)
        ));
        let mut wrong_protocol = sample_world_snapshot(13, 1);
        wrong_protocol.protocol += 1;
        assert!(!queue_online_world_snapshot(
            &mut replication,
            wrong_protocol
        ));
        assert_eq!(replication.pending_snapshot.unwrap().tick, 12);
    }

    #[test]
    fn large_eight_player_snapshot_fits_unreliable_channel() {
        let mut snapshot = sample_world_snapshot(42, 512);
        snapshot.build_queue = (0..128)
            .map(|index| OnlineBuildJobSnapshot {
                team: OnlineEntityTeam::Player(index % 8),
                action: OnlineBuildActionSnapshot::Train("Worker".to_string()),
                producer_entity: index as u64 + 1,
                producer_id: "CommandCenter".to_string(),
                timer: 4.5,
                origin: [index as f32, 0.0, index as f32 * 0.25],
                cost: [4, 2],
            })
            .collect();
        let encoded = postcard::to_allocvec(&snapshot).unwrap();
        assert!(
            encoded.len() <= MAX_SNAPSHOT_PACKET_BYTES,
            "{} byte snapshot exceeds {} byte channel limit",
            encoded.len(),
            MAX_SNAPSHOT_PACKET_BYTES
        );
        assert_eq!(
            postcard::from_bytes::<OnlineWorldSnapshot>(&encoded).unwrap(),
            snapshot
        );
    }

    #[test]
    fn lobby_has_exactly_one_row_per_map_spawn() {
        let lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        assert_eq!(lobby.slots.len(), lobby.map().players);
        assert_eq!(lobby.slots[0].spawn_slot, 0);
        assert_eq!(
            lobby.slots.last().unwrap().spawn_slot,
            lobby.map().players - 1
        );
    }

    #[test]
    fn reconnecting_player_keeps_public_identity_and_slot() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        let mut runtime = HostLobbyRuntime::new("host-key".to_string());
        let first_peer = PeerId("00000000-0000-0000-0000-000000000001".parse().unwrap());
        let (player_id, slot) = runtime
            .admit(
                &mut lobby,
                first_peer,
                "resume-key".to_string(),
                "Player".to_string(),
                true,
            )
            .unwrap();
        assert_eq!(runtime.disconnect(&mut lobby, first_peer), Some(player_id));
        let second_peer = PeerId("00000000-0000-0000-0000-000000000002".parse().unwrap());
        let resumed = runtime
            .admit(
                &mut lobby,
                second_peer,
                "resume-key".to_string(),
                "Player".to_string(),
                false,
            )
            .unwrap();
        assert_eq!(resumed, (player_id, slot));
    }

    #[test]
    fn running_match_rejects_unknown_late_joiners() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        let mut runtime = HostLobbyRuntime::new("host-key".to_string());
        let peer = PeerId("00000000-0000-0000-0000-000000000004".parse().unwrap());

        let result = runtime.admit(
            &mut lobby,
            peer,
            "unknown-key".to_string(),
            "Late Player".to_string(),
            false,
        );

        assert_eq!(result, Err("the match is already in progress"));
        assert!(runtime.peer_players.is_empty());
        assert_eq!(lobby.active_slot_count(), 1);
    }

    #[test]
    fn disconnect_grace_allows_resume_then_forfeits_after_expiry() {
        let mut lifecycle = OnlineLifecycleControl::default();
        lifecycle.note_disconnect(2);
        assert!(lifecycle.tick_disconnects(29.0).is_empty());
        lifecycle.note_reconnect(2);
        assert!(lifecycle.tick_disconnects(2.0).is_empty());
        assert!(!lifecycle.forfeited_players.contains(&2));

        lifecycle.note_disconnect(2);
        assert_eq!(lifecycle.tick_disconnects(30.0), vec![2]);
        assert!(lifecycle.forfeited_players.contains(&2));
        lifecycle.note_reconnect(2);
        assert!(lifecycle.forfeited_players.contains(&2));
    }

    #[test]
    fn rematch_lobby_keeps_connected_players_and_reclaims_forfeited_slots() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Connected".to_string(),
            ready: true,
            connected: true,
        };
        lobby.slots[2].occupant = OnlineSlotOccupant::Human {
            player_id: 3,
            name: "Forfeited".to_string(),
            ready: true,
            connected: false,
        };
        let peer = PeerId("00000000-0000-0000-0000-000000000005".parse().unwrap());
        let mut runtime = HostLobbyRuntime::new("host-key".to_string());
        runtime.peer_players.insert(peer, 2);
        runtime.resume_players.insert("player-2".to_string(), 2);
        runtime.resume_players.insert("player-3".to_string(), 3);
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.lobby = Some(lobby);
        session.host_runtime = Some(runtime);
        let mut lifecycle = OnlineLifecycleControl::default();
        lifecycle.forfeited_players.insert(3);

        let rematch = prepare_online_lobby_for_rematch(&mut session, &lifecycle).unwrap();

        assert!(matches!(
            rematch.slots[0].occupant,
            OnlineSlotOccupant::Human {
                player_id: 1,
                ready: false,
                connected: true,
                ..
            }
        ));
        assert!(matches!(
            rematch.slots[1].occupant,
            OnlineSlotOccupant::Human {
                player_id: 2,
                ready: false,
                connected: true,
                ..
            }
        ));
        assert_eq!(rematch.slots[2].occupant, OnlineSlotOccupant::Open);
        let runtime = session.host_runtime.as_ref().unwrap();
        assert_eq!(runtime.player_for_session("player-2"), Some(2));
        assert_eq!(runtime.player_for_session("player-3"), None);
    }

    #[test]
    fn reconnect_welcome_keeps_client_inside_running_match() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        let config = OnlineMatchConfig::from(&lobby);
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.match_config = Some(config.clone());
        let peer = PeerId("00000000-0000-0000-0000-000000000003".parse().unwrap());
        let mut setup = MatchSetupSettings::default();
        let mut next_state = NextState::<AppScreen>::default();

        accept_online_welcome(
            &mut session,
            peer,
            2,
            1,
            lobby,
            Some(config),
            &mut setup,
            &mut next_state,
        );

        assert_eq!(session.phase, OnlinePhase::InMatch);
        assert_eq!(session.host_peer, Some(peer));
        assert_eq!(session.local_player_id, Some(2));
        assert_eq!(session.assigned_slot, Some(1));
    }

    #[test]
    fn lobby_welcome_returns_reconnecting_client_from_stale_match() {
        let lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.match_config = Some(OnlineMatchConfig::from(&lobby));
        let peer = PeerId("00000000-0000-0000-0000-000000000006".parse().unwrap());
        let mut setup = MatchSetupSettings::default();
        let mut next_state = NextState::<AppScreen>::default();

        accept_online_welcome(
            &mut session,
            peer,
            1,
            0,
            lobby,
            None,
            &mut setup,
            &mut next_state,
        );

        assert_eq!(session.phase, OnlinePhase::Lobby);
        assert!(session.match_config.is_none());
    }

    #[test]
    fn disconnect_expiry_removes_owned_entities_and_queued_production() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: false,
        };
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.match_config = Some(OnlineMatchConfig::from(&lobby));
        let mut lifecycle = OnlineLifecycleControl::default();
        lifecycle.note_disconnect(2);
        lifecycle.disconnected_players.insert(2, 0.0);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<AppScreen>()
            .insert_resource(session)
            .insert_resource(OnlineTransport::default())
            .insert_resource(lifecycle)
            .insert_resource(MatchState::default())
            .insert_resource(BuildQueue::default())
            .add_systems(Update, process_online_lifecycle);
        let producer = app
            .world_mut()
            .spawn((Team::Player(1), MatchScopedEntity))
            .id();
        let pending = app
            .world_mut()
            .spawn((
                PendingParadrop {
                    remaining: 3.0,
                    team: Team::Player(1),
                    target: Vec3::ZERO,
                    unit_paths: &[],
                },
                MatchScopedEntity,
            ))
            .id();
        app.world_mut()
            .resource_mut::<BuildQueue>()
            .0
            .push(BuildJob {
                team: Team::Player(1),
                action: BuildAction::Train("Worker"),
                producer_entity: producer,
                producer_id: "CommandCenter",
                timer: 2.0,
                origin: Vec3::ZERO,
                cost: registry::Cost::default(),
            });

        app.update();

        assert!(app.world().get_entity(producer).is_err());
        assert!(app.world().get_entity(pending).is_err());
        assert!(app.world().resource::<BuildQueue>().0.is_empty());
        assert!(
            app.world()
                .resource::<OnlineLifecycleControl>()
                .forfeited_players
                .contains(&2)
        );
    }

    #[test]
    fn client_return_request_waits_for_match_end_then_restores_everyone() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        let peer = PeerId("00000000-0000-0000-0000-000000000007".parse().unwrap());
        let mut runtime = HostLobbyRuntime::new("host-key".to_string());
        runtime.peer_players.insert(peer, 2);
        runtime.resume_players.insert("player-key".to_string(), 2);
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.match_config = Some(OnlineMatchConfig::from(&lobby));
        session.lobby = Some(lobby);
        session.host_runtime = Some(runtime);
        let mut lifecycle = OnlineLifecycleControl::default();
        lifecycle.remote_return_requests.push(2);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<AppScreen>()
            .insert_resource(session)
            .insert_resource(OnlineTransport::default())
            .insert_resource(lifecycle)
            .insert_resource(MatchState::default())
            .insert_resource(BuildQueue::default())
            .add_systems(Update, process_online_lifecycle);

        app.update();
        assert_eq!(
            app.world().resource::<OnlineSession>().phase,
            OnlinePhase::InMatch
        );

        app.world_mut().resource_mut::<MatchState>().phase = MatchPhase::HumanVictory;
        app.world_mut()
            .resource_mut::<OnlineLifecycleControl>()
            .remote_return_requests
            .push(2);
        app.update();

        let session = app.world().resource::<OnlineSession>();
        assert_eq!(session.phase, OnlinePhase::Lobby);
        assert!(session.match_config.is_none());
        assert!(matches!(
            session.lobby.as_ref().unwrap().slots[1].occupant,
            OnlineSlotOccupant::Human {
                player_id: 2,
                ready: false,
                connected: true,
                ..
            }
        ));
    }

    #[test]
    fn online_match_menu_keeps_authoritative_actions_host_only() {
        let active_teams = ActiveTeams(vec![true, true]);
        let visible_player = VisiblePlayer::default();
        let mut client = OnlineSession::default();
        client.phase = OnlinePhase::InMatch;
        assert!(!match_menu_action_enabled(
            MatchMenuAction::Restart,
            &visible_player,
            &active_teams,
            true,
            Some(&client),
        ));
        assert!(!match_menu_action_enabled(
            MatchMenuAction::SetSpeed(MatchSpeedPreset::Fast),
            &visible_player,
            &active_teams,
            true,
            Some(&client),
        ));
        assert!(!match_menu_action_enabled(
            MatchMenuAction::ReturnToSetup,
            &visible_player,
            &active_teams,
            true,
            Some(&client),
        ));
        assert!(match_menu_action_enabled(
            MatchMenuAction::ReturnToSetup,
            &visible_player,
            &active_teams,
            false,
            Some(&client),
        ));
        assert!(!match_end_action_enabled(
            MatchEndAction::Restart,
            true,
            true,
        ));

        client.is_host = true;
        assert!(match_menu_action_enabled(
            MatchMenuAction::SetSpeed(MatchSpeedPreset::Fast),
            &visible_player,
            &active_teams,
            true,
            Some(&client),
        ));
        assert!(match_menu_action_enabled(
            MatchMenuAction::ReturnToSetup,
            &visible_player,
            &active_teams,
            true,
            Some(&client),
        ));
    }

    #[test]
    fn online_host_elimination_does_not_stop_remaining_opponents() {
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);

        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(MatchState::default())
            .insert_resource(MatchFlow::default())
            .insert_resource(AudioFeedback::default())
            .insert_resource(TeamRelations::default())
            .insert_resource(VisiblePlayer::default())
            .insert_resource(MatchSetupSettings::default())
            .add_systems(Update, evaluate_match_end);
        app.world_mut().spawn((
            Structure {
                id: "CommandCenter",
            },
            Team::Player(1),
            Health::new(100.0),
        ));
        let last_opponent = app
            .world_mut()
            .spawn((
                Structure {
                    id: "CommandCenter",
                },
                Team::Player(2),
                Health::new(100.0),
            ))
            .id();

        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        assert!(app.world().resource::<MatchFlow>().is_active());

        app.world_mut().despawn(last_opponent);
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::HumanDefeat
        );
        assert!(!app.world().resource::<MatchFlow>().is_active());
    }

    #[test]
    fn online_match_conversion_preserves_all_active_slots() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        lobby.slots[2].occupant = OnlineSlotOccupant::Ai(OnlineAiDifficulty::Normal);
        let config = OnlineMatchConfig::from(&lobby);
        let host = online_match_setup(&config, 0).unwrap();
        let client = online_match_setup(&config, 1).unwrap();
        assert_eq!(host.active_teams.len(), 3);
        assert_eq!(host.player_spawn_slots, vec![0, 1, 2]);
        assert_eq!(host.player_factions, client.player_factions);
        assert_eq!(host.team_relations, client.team_relations);
        assert_eq!(host.visible_player.team, Team::Player(0));
        assert_eq!(client.visible_player.team, Team::Player(1));
        assert!(!team_uses_automatic_construction(
            Team::Player(0),
            Some(Team::Player(0)),
            Some(&host),
            true,
        ));
        assert!(!team_uses_automatic_construction(
            Team::Player(1),
            Some(Team::Player(0)),
            Some(&host),
            true,
        ));
        assert!(team_uses_automatic_construction(
            Team::Player(2),
            Some(Team::Player(0)),
            Some(&host),
            true,
        ));
        assert!(team_uses_automatic_construction(
            Team::Player(0),
            None,
            Some(&host),
            false,
        ));
    }

    #[test]
    fn player_command_sequences_reject_replays_and_wrong_protocols() {
        let mut inbox = OnlineCommandInbox::default();
        let command = OnlinePlayerCommand::UnitOrders {
            orders: vec![OnlineUnitOrderCommand {
                unit_id: 7,
                order: OnlineUnitOrderKind::Move {
                    destination: [1.0, 0.0, 2.0],
                },
            }],
            queue: false,
        };
        let envelope = OnlinePlayerCommandEnvelope {
            protocol: RTS_ONLINE_PROTOCOL,
            sequence: 1,
            command: command.clone(),
        };
        assert!(enqueue_online_player_command(
            &mut inbox,
            2,
            envelope.clone()
        ));
        assert!(!enqueue_online_player_command(&mut inbox, 2, envelope));
        assert!(!enqueue_online_player_command(
            &mut inbox,
            2,
            OnlinePlayerCommandEnvelope {
                protocol: RTS_ONLINE_PROTOCOL + 1,
                sequence: 2,
                command,
            }
        ));
        assert_eq!(inbox.pending.len(), 1);
    }

    #[test]
    fn maximum_unit_order_batch_fits_reliable_channel() {
        let message = OnlineReliableMessage::PlayerCommand(OnlinePlayerCommandEnvelope {
            protocol: RTS_ONLINE_PROTOCOL,
            sequence: u64::MAX,
            command: OnlinePlayerCommand::UnitOrders {
                orders: (0..ONLINE_MAX_UNIT_ORDERS_PER_COMMAND)
                    .map(|index| OnlineUnitOrderCommand {
                        unit_id: index as u64 + 1,
                        order: OnlineUnitOrderKind::Patrol {
                            origin: [index as f32, 0.0, 1.0],
                            destination: [2.0, 0.0, index as f32],
                        },
                    })
                    .collect(),
                queue: true,
            },
        });
        let encoded = postcard::to_allocvec(&message).unwrap();
        assert!(
            encoded.len() <= open_bevy_net::MAX_RELIABLE_PACKET_BYTES,
            "{} byte command exceeds {} byte reliable channel limit",
            encoded.len(),
            open_bevy_net::MAX_RELIABLE_PACKET_BYTES
        );
        assert_eq!(
            postcard::from_bytes::<OnlineReliableMessage>(&encoded).unwrap(),
            message
        );
    }

    #[test]
    fn maximum_instant_action_batches_fit_reliable_channel() {
        for command in [
            OnlinePlayerCommand::UnitAction {
                units: (1..=ONLINE_MAX_UNIT_ACTIONS_PER_COMMAND as u64).collect(),
                action: OnlineUnitAction::Scatter,
            },
            OnlinePlayerCommand::StructureAction {
                structures: (1..=ONLINE_MAX_STRUCTURE_ACTIONS_PER_COMMAND as u64).collect(),
                action: OnlineStructureAction::Sell,
            },
        ] {
            let message = OnlineReliableMessage::PlayerCommand(OnlinePlayerCommandEnvelope {
                protocol: RTS_ONLINE_PROTOCOL,
                sequence: u64::MAX,
                command,
            });
            let encoded = postcard::to_allocvec(&message).unwrap();
            assert!(
                encoded.len() <= open_bevy_net::MAX_RELIABLE_PACKET_BYTES,
                "{} byte command exceeds {} byte reliable channel limit",
                encoded.len(),
                open_bevy_net::MAX_RELIABLE_PACKET_BYTES
            );
            assert_eq!(
                postcard::from_bytes::<OnlineReliableMessage>(&encoded).unwrap(),
                message
            );
        }
    }

    #[test]
    fn host_applies_owned_unit_orders_and_rejects_opponent_control() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .add_systems(Update, apply_online_player_commands);
        let host_unit = app
            .world_mut()
            .spawn((
                NetworkEntityId(10),
                Team::Player(0),
                Unit {
                    id: "Worker",
                    speed: 5.0,
                    can_crush: false,
                    can_be_crushed: true,
                },
                Health::new(10.0),
                Transform::default(),
            ))
            .id();
        let player_unit = app
            .world_mut()
            .spawn((
                NetworkEntityId(20),
                Team::Player(1),
                Unit {
                    id: "Worker",
                    speed: 5.0,
                    can_crush: false,
                    can_be_crushed: true,
                },
                Health::new(10.0),
                Transform::default(),
            ))
            .id();
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            for (sequence, unit_id, destination) in
                [(1, 10, [9.0, 0.0, 9.0]), (2, 20, [4.0, 0.0, 5.0])]
            {
                assert!(enqueue_online_player_command(
                    &mut inbox,
                    2,
                    OnlinePlayerCommandEnvelope {
                        protocol: RTS_ONLINE_PROTOCOL,
                        sequence,
                        command: OnlinePlayerCommand::UnitOrders {
                            orders: vec![OnlineUnitOrderCommand {
                                unit_id,
                                order: OnlineUnitOrderKind::Move { destination },
                            }],
                            queue: false,
                        },
                    }
                ));
            }
        }
        app.update();

        assert!(app.world().get::<MoveOrder>(host_unit).is_none());
        assert_eq!(
            app.world().get::<MoveOrder>(player_unit).unwrap().target,
            Vec3::new(4.0, 0.0, 5.0)
        );
    }

    #[test]
    fn host_applies_owned_unit_actions_and_rejects_opponent_control() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .add_systems(Update, apply_online_player_commands);
        let spawn_unit = |world: &mut World, network_id, team, x| {
            world
                .spawn((
                    NetworkEntityId(network_id),
                    team,
                    Unit {
                        id: "LightRifleInfantry",
                        speed: 3.4,
                        can_crush: false,
                        can_be_crushed: true,
                    },
                    Health::new(10.0),
                    Transform::from_xyz(x, 0.0, 0.0),
                    HoldPosition { enabled: false },
                    MoveOrder {
                        target: Vec3::new(x + 5.0, 0.0, 0.0),
                    },
                ))
                .id()
        };
        let host_unit = spawn_unit(app.world_mut(), 10, Team::Player(0), -4.0);
        let player_unit = spawn_unit(app.world_mut(), 20, Team::Player(1), 4.0);
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            for (sequence, unit_id) in [(1, 10), (2, 20)] {
                assert!(enqueue_online_player_command(
                    &mut inbox,
                    2,
                    OnlinePlayerCommandEnvelope {
                        protocol: RTS_ONLINE_PROTOCOL,
                        sequence,
                        command: OnlinePlayerCommand::UnitAction {
                            units: vec![unit_id],
                            action: OnlineUnitAction::ToggleHoldPosition,
                        },
                    }
                ));
            }
        }
        app.update();

        assert!(!app.world().get::<HoldPosition>(host_unit).unwrap().enabled);
        assert!(app.world().get::<MoveOrder>(host_unit).is_some());
        assert!(
            app.world()
                .get::<HoldPosition>(player_unit)
                .unwrap()
                .enabled
        );
        assert!(app.world().get::<MoveOrder>(player_unit).is_none());

        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            assert!(enqueue_online_player_command(
                &mut inbox,
                2,
                OnlinePlayerCommandEnvelope {
                    protocol: RTS_ONLINE_PROTOCOL,
                    sequence: 3,
                    command: OnlinePlayerCommand::UnitAction {
                        units: vec![20],
                        action: OnlineUnitAction::Scatter,
                    },
                }
            ));
        }
        app.update();
        assert!(app.world().get::<MoveOrder>(player_unit).is_some());
        assert!(
            !app.world()
                .get::<HoldPosition>(player_unit)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn host_structure_actions_are_owned_and_idempotent_per_tick() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut economies = Economies::default();
        economies.get_mut(Team::Player(1)).ore = 100;
        economies.get_mut(Team::Player(1)).crystal = 100;
        let damaged_health = Health {
            current: 50.0,
            max: 100.0,
        };
        let repair_cost =
            structure_repair_cost(registry::entity("PowerReactor").unwrap(), &damaged_health);
        let foundation_cost = registry::Cost {
            ore: 13,
            crystal: 7,
        };

        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .insert_resource(economies)
            .insert_resource(BuildQueue::default())
            .add_systems(Update, apply_online_player_commands);
        let enemy = app
            .world_mut()
            .spawn((
                NetworkEntityId(10),
                Team::Player(0),
                Structure { id: "PowerReactor" },
                Health::new(100.0),
                Transform::default(),
            ))
            .id();
        let damaged = app
            .world_mut()
            .spawn((
                NetworkEntityId(20),
                Team::Player(1),
                Structure { id: "PowerReactor" },
                damaged_health,
                Transform::default(),
            ))
            .id();
        let foundation = app
            .world_mut()
            .spawn((
                NetworkEntityId(21),
                Team::Player(1),
                Structure { id: "PowerReactor" },
                Health::new(100.0),
                Transform::default(),
                UnderConstruction {
                    remaining: 5.0,
                    total: 5.0,
                    cost: foundation_cost,
                    free_worker_origin: None,
                },
            ))
            .id();
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            let commands = [
                (1, vec![10], OnlineStructureAction::Sell),
                (2, vec![20], OnlineStructureAction::Repair),
                (3, vec![20], OnlineStructureAction::Repair),
                (4, vec![21], OnlineStructureAction::CancelConstruction),
                (5, vec![21], OnlineStructureAction::CancelConstruction),
            ];
            for (sequence, structures, action) in commands {
                assert!(enqueue_online_player_command(
                    &mut inbox,
                    2,
                    OnlinePlayerCommandEnvelope {
                        protocol: RTS_ONLINE_PROTOCOL,
                        sequence,
                        command: OnlinePlayerCommand::StructureAction { structures, action },
                    }
                ));
            }
        }
        app.update();

        assert!(app.world().get_entity(enemy).is_ok());
        assert!(app.world().get::<ManualStructureRepair>(damaged).is_some());
        assert!(app.world().get_entity(foundation).is_err());
        let economy = app.world().resource::<Economies>().get(Team::Player(1));
        assert_eq!(economy.ore, 100 - repair_cost.ore + foundation_cost.ore);
        assert_eq!(
            economy.crystal,
            100 - repair_cost.crystal + foundation_cost.crystal
        );
    }

    #[test]
    fn host_validates_faction_tech_and_cooldown_for_support_powers() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        lobby.slots[1].faction = OnlineFaction::Alliance;
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut economies = Economies::default();
        let _ = economies.get_mut(Team::Player(1));
        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .insert_resource(economies)
            .insert_resource(BuildQueue::default())
            .insert_resource(SupportCooldowns::default())
            .insert_resource(BattleLog::default())
            .insert_resource(AudioFeedback::default())
            .add_systems(Update, apply_online_player_commands);
        app.world_mut().spawn((
            NetworkEntityId(20),
            Team::Player(1),
            Structure { id: "RadarUplink" },
            Health::new(100.0),
            Transform::default(),
        ));
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            for (sequence, power) in [
                (1, OnlineSupportPower::OrbitalStrike),
                (2, OnlineSupportPower::RadarSweep),
                (3, OnlineSupportPower::RadarSweep),
            ] {
                assert!(enqueue_online_player_command(
                    &mut inbox,
                    2,
                    OnlinePlayerCommandEnvelope {
                        protocol: RTS_ONLINE_PROTOCOL,
                        sequence,
                        command: OnlinePlayerCommand::UseSupportPower {
                            power,
                            target: [4.0, 0.0, 3.0],
                        },
                    }
                ));
            }
        }
        app.update();

        let cooldowns = app.world().resource::<SupportCooldowns>();
        assert_eq!(
            cooldowns.remaining_for(Team::Player(1), SupportPowerKind::OrbitalStrike),
            0.0
        );
        assert_eq!(
            cooldowns.remaining_for(Team::Player(1), SupportPowerKind::RadarSweep),
            SupportPowerKind::RadarSweep.definition().cooldown
        );
    }

    #[test]
    fn host_validates_training_ownership_and_authoritative_refunds() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        lobby.slots[1].faction = OnlineFaction::Alliance;
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut economies = Economies::default();
        for team in [Team::Player(0), Team::Player(1)] {
            let economy = economies.get_mut(team);
            economy.ore = 1_000;
            economy.crystal = 1_000;
        }
        let starting_ore = economies.get(Team::Player(1)).ore;
        let starting_crystal = economies.get(Team::Player(1)).crystal;

        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .insert_resource(economies)
            .insert_resource(BuildQueue::default())
            .add_systems(Update, apply_online_player_commands);
        app.world_mut().spawn((
            NetworkEntityId(10),
            Team::Player(0),
            Structure {
                id: "CommandCenter",
            },
            Health::new(100.0),
            Transform::default(),
        ));
        let player_producer = app
            .world_mut()
            .spawn((
                NetworkEntityId(20),
                Team::Player(1),
                Structure {
                    id: "CommandCenter",
                },
                Health::new(100.0),
                Transform::from_xyz(8.0, 0.0, 4.0),
            ))
            .id();
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            for (sequence, producer) in [(1, 10), (2, 20)] {
                assert!(enqueue_online_player_command(
                    &mut inbox,
                    2,
                    OnlinePlayerCommandEnvelope {
                        protocol: RTS_ONLINE_PROTOCOL,
                        sequence,
                        command: OnlinePlayerCommand::TrainUnits {
                            producers: vec![producer],
                            unit_id: "Worker".to_string(),
                            batch_to_limit: false,
                        },
                    }
                ));
            }
        }
        app.update();

        let queue = app.world().resource::<BuildQueue>();
        assert_eq!(queue.0.len(), 1);
        assert_eq!(queue.0[0].producer_entity, player_producer);
        assert_eq!(queue.0[0].team, Team::Player(1));
        assert_eq!(queue.0[0].action, BuildAction::Train("Worker"));
        let charged = queue.0[0].cost;
        assert_eq!(
            app.world().resource::<Economies>().get(Team::Player(1)).ore,
            starting_ore - charged.ore
        );
        assert_eq!(
            app.world()
                .resource::<Economies>()
                .get(Team::Player(1))
                .crystal,
            starting_crystal - charged.crystal
        );

        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            assert!(enqueue_online_player_command(
                &mut inbox,
                2,
                OnlinePlayerCommandEnvelope {
                    protocol: RTS_ONLINE_PROTOCOL,
                    sequence: 3,
                    command: OnlinePlayerCommand::CancelProduction {
                        producers: vec![20],
                        product_id: "Worker".to_string(),
                        local_index: None,
                    },
                }
            ));
        }
        app.update();

        assert!(app.world().resource::<BuildQueue>().0.is_empty());
        let economy = app.world().resource::<Economies>().get(Team::Player(1));
        assert_eq!(economy.ore, starting_ore);
        assert_eq!(economy.crystal, starting_crystal);
    }

    #[test]
    fn host_places_owned_structure_and_assigns_authorized_worker() {
        let mut lobby = OnlineLobbySnapshot::new("ABC123".to_string(), "Host".to_string());
        lobby.slots[1].occupant = OnlineSlotOccupant::Human {
            player_id: 2,
            name: "Player".to_string(),
            ready: true,
            connected: true,
        };
        lobby.slots[1].faction = OnlineFaction::Alliance;
        let mut session = OnlineSession::default();
        session.phase = OnlinePhase::InMatch;
        session.is_host = true;
        session.local_player_id = Some(1);
        session.match_config = Some(OnlineMatchConfig::from(&lobby));

        let mut economies = Economies::default();
        let economy = economies.get_mut(Team::Player(1));
        economy.ore = 1_000;
        economy.crystal = 1_000;
        let structure_cost = registry::entity("PowerReactor").unwrap().cost;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<WorldAsset>()
            .insert_resource(session)
            .insert_resource(OnlineCommandInbox::default())
            .insert_resource(MapBounds::default())
            .insert_resource(TeamRelations::default())
            .insert_resource(TerrainHeightField::default())
            .insert_resource(NextSpawnId(100))
            .insert_resource(economies)
            .insert_resource(BuildQueue::default())
            .add_systems(Update, apply_online_player_commands);
        app.world_mut().spawn((
            NetworkEntityId(20),
            Team::Player(1),
            Structure {
                id: "CommandCenter",
            },
            Health::new(100.0),
            Transform::from_xyz(10.0, 0.0, 0.0),
            Selectable { radius: 2.0 },
        ));
        let worker = app
            .world_mut()
            .spawn((
                NetworkEntityId(30),
                Team::Player(1),
                Unit {
                    id: "Worker",
                    speed: 3.0,
                    can_crush: false,
                    can_be_crushed: true,
                },
                Health::new(6.0),
                Transform::from_xyz(14.0, 0.0, 0.0),
                Selectable { radius: 0.35 },
            ))
            .id();
        {
            let mut inbox = app.world_mut().resource_mut::<OnlineCommandInbox>();
            assert!(enqueue_online_player_command(
                &mut inbox,
                2,
                OnlinePlayerCommandEnvelope {
                    protocol: RTS_ONLINE_PROTOCOL,
                    sequence: 1,
                    command: OnlinePlayerCommand::PlaceStructure {
                        constructors: vec![30],
                        structure_id: "PowerReactor".to_string(),
                        position: [17.0, 0.0, 0.0],
                        rotation_y_radians: 0.0,
                    },
                }
            ));
        }
        app.update();

        let mut structures = app
            .world_mut()
            .query::<(Entity, &Structure, &Team, Option<&UnderConstruction>)>();
        let placed = structures
            .iter(app.world())
            .find_map(|(entity, structure, team, construction)| {
                (structure.id == "PowerReactor"
                    && *team == Team::Player(1)
                    && construction.is_some())
                .then_some(entity)
            })
            .expect("host should place the validated structure");
        assert_eq!(
            app.world().get::<ConstructOrder>(worker).unwrap().target,
            placed
        );
        let economy = app.world().resource::<Economies>().get(Team::Player(1));
        assert_eq!(economy.ore, 1_000 - structure_cost.ore);
        assert_eq!(economy.crystal, 1_000 - structure_cost.crystal);
    }
}
