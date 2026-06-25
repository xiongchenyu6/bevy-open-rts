#[cfg(feature = "audio")]
use bevy::audio::Volume;
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    camera::{RenderTargetInfo, ScalingMode, Viewport},
    ecs::query::Or,
    ecs::system::SystemParam,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    math::primitives::{ConicalFrustum, Cuboid, Cylinder, Torus},
    prelude::*,
    render::error_handler::{ErrorType, RenderError, RenderErrorHandler, RenderErrorPolicy},
    window::{PrimaryWindow, WindowResolution},
};
use bevy_common_assets::{json::JsonAssetPlugin, ron::RonAssetPlugin};
use serde::Deserialize;
use std::collections::{BTreeMap, VecDeque};

mod generated_registry;

use generated_registry as registry;

#[derive(Asset, TypePath, Deserialize)]
#[allow(dead_code)]
struct RtsDataManifest {
    name: String,
}

fn handle_render_error(
    error: &RenderError,
    _main_world: &mut World,
    _render_world: &mut World,
) -> RenderErrorPolicy {
    match error.ty {
        ErrorType::DeviceLost => RenderErrorPolicy::StopRendering,
        ErrorType::Validation => RenderErrorPolicy::Ignore,
        ErrorType::OutOfMemory | ErrorType::Internal => RenderErrorPolicy::StopRendering,
    }
}

const MAP_HALF_EXTENT: f32 = 24.0;
const EDGE_PAN_PX: f32 = 28.0;
const CAMERA_MIN_DISTANCE: f32 = 5.5;
const CAMERA_DEFAULT_DISTANCE: f32 = 7.0;
const CAMERA_MAX_DISTANCE: f32 = 9.0;
const CAMERA_NEAR_PLANE: f32 = 0.05;
const CAMERA_FAR_PLANE: f32 = 300.0;
const CAMERA_DEFAULT_YAW: f32 = -0.72;
const CAMERA_DEFAULT_PITCH: f32 = -1.02;
const CAMERA_BOUNDS_MARGIN: f32 = 1.2;
const CAMERA_PAN_SPEED_MULTIPLIER: f32 = 0.48;
const CAMERA_MOUSE_ROTATION_SPEED: f32 = 0.005;
const CAMERA_START_PRIMARY_UNITS: &[&str] = &["MobileConstructionVehicle"];
const CAMERA_START_PRIMARY_STRUCTURES: &[&str] = &["CommandCenter"];
const RESOURCE_ORDER_GROUND_SNAP_RADIUS_M: f32 = 2.2;
const RESOURCE_ORDER_COLLECTOR_GROUND_SNAP_RADIUS_M: f32 = 7.0;
const RESOURCE_ORDER_SCREEN_PICK_MIN_RADIUS_PX: f32 = 34.0;
const RESOURCE_ORDER_SCREEN_PICK_MAX_RADIUS_PX: f32 = 78.0;
const RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX: f32 = 148.0;
const ENEMY_ORDER_SCREEN_PICK_MIN_RADIUS_PX: f32 = 32.0;
const ENEMY_ORDER_SCREEN_PICK_MAX_RADIUS_PX: f32 = 96.0;
const DEFAULT_MODEL_FALLBACK: &str = "models/kenney-spacekit/rover.glb";
const COMMAND_SLOT_COUNT: usize = 24;
const COMMAND_KEY_CANCEL: &str = "cancel";
const COMMAND_KEY_GUARD_AREA: &str = "guard_area";
const COMMAND_KEY_SCATTER: &str = "scatter";
const COMMAND_KEY_HOLD_POSITION: &str = "hold_position";
const COMMAND_KEY_MINIMAP_MOVE: &str = "minimap_move";
const COMMAND_KEY_DEPLOY_MCV: &str = "deploy_mcv";
const COMMAND_KEY_TOGGLE_DEPLOY: &str = "toggle_deploy";
const MOVE_ORDER_REACHED_DISTANCE_M: f32 = 0.22;
const CONTACT_ACTION_REACHED_TOLERANCE_M: f32 = MOVE_ORDER_REACHED_DISTANCE_M;
const ATTACK_MOVE_REACHED_DISTANCE: f32 = 2.0;
const PATROL_TURN_DISTANCE: f32 = 2.0;
const SCATTER_DISTANCE: f32 = 4.0;
const DRAG_SELECT_THRESHOLD: f32 = 6.0;
const SELECTION_DRAG_INTERRUPT_MARGIN_PX: f32 = 1.0;
const CAMERA_ROTATE_SPEED: f32 = 2.0;
const DOUBLE_CLICK_MIN_SECONDS: f32 = 0.05;
const DOUBLE_CLICK_MAX_SECONDS: f32 = 0.6;
const SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PX: f32 = 38.0;
const SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PER_METER_PX: f32 = 18.0;
const FOG_REVEAL_RADIUS: f32 = 11.5;
const FOG_COMPENSATION: f32 = 2.0;
const MATCH_END_TITLE_COLOR: Color = Color::srgb(0.98, 0.96, 0.42);
const MATCH_END_BG_COLOR: Color = Color::srgba(0.04, 0.05, 0.08, 0.86);
const MATCH_END_TITLE_FONT_SIZE: f32 = 34.0;
const MATCH_END_TEXT_FONT_SIZE: f32 = 19.0;
const CLICK_MARKER_TTL_SECONDS: f32 = 0.5;
const CLICK_MARKER_RADIUS_START: f32 = 0.7;
const CLICK_MARKER_RADIUS_END: f32 = 0.05;
const UNIT_ADHERENCE_MARGIN_M: f32 = 0.3;
const CAPTURE_ENTRY_MARGIN_M: f32 = 1.3;
const FOLLOW_TARGET_DISTANCE_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
const RESOURCE_ENTRY_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
const RESOURCE_DROPOFF_ENTRY_MARGIN_M: f32 = 1.2;
const REPAIR_ADHERENCE_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
const REPAIR_ENTRY_MARGIN_M: f32 = 1.0;
const RESOURCE_SEARCH_RADIUS_M: f32 = 30.0;
const ORE_PURIFIER_BONUS_RATIO: f32 = 0.25;
const SUPPLY_CRATE_PICKUP_RADIUS: f32 = 0.85;
const SUPPLY_CRATE_RESOURCE_ORE: i32 = 6;
const SUPPLY_CRATE_RESOURCE_CRYSTAL: i32 = 1;
const SUPPLY_CRATE_REPAIR_RADIUS: f32 = 3.5;
const SUPPLY_CRATE_REPAIR_AMOUNT: f32 = 8.0;
const AI_SUPPLY_CRATE_COLLECTION_LIMIT: usize = 2;
const LOW_POWER_PRODUCTION_SPEED_MULTIPLIER: f32 = 0.5;
const PRODUCTION_QUEUE_LIMIT: usize = 5;
const PRODUCTION_QUEUE_HUD_SLOT_COUNT: usize = 24;
const STRUCTURE_SELL_REFUND_RATIO: f32 = 0.5;
const STRUCTURE_MANUAL_REPAIR_COST_RATIO: f32 = 0.5;
const STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND: f32 = 3.0;
const AI_REPAIR_MIN_MISSING_HITPOINT_RATIO: f32 = 0.25;
const AI_REPAIR_MAX_STARTS_PER_REFRESH: usize = 2;
const AI_REPAIR_REFRESH_INTERVAL_SECONDS: f32 = 0.5;
const AI_OPENING_ATTACK_GRACE_SECONDS: f32 = 45.0;
const AI_TECH_BUNKER_GARRISON_REFRESH_INTERVAL_SECONDS: f32 = 1.0;
const AI_TECH_BUNKER_GARRISON_SEARCH_RADIUS: f32 = 16.0;
const AI_SUPPORT_MIN_CLUSTER_TARGETS: usize = 2;
const AI_SUPPORT_ORBITAL_STRIKE_MIN_SCORE: f32 = 3.0;
const AI_SUPPORT_WEATHER_STORM_MIN_SCORE: f32 = 5.0;
const AI_SUPPORT_STRATEGIC_MISSILE_MIN_SCORE: f32 = 5.0;
const AI_SUPPORT_NANITE_REPAIR_MIN_MISSING_HP: f32 = 4.0;
const AI_SUPPORT_CHRONO_RELAY_MIN_MOBILE_UNITS: usize = 2;
const AI_SUPPORT_SHIELD_OVERDRIVE_MIN_SCORE: f32 = 2.0;
const AI_SUPPORT_SHIELD_OVERDRIVE_MOBILE_PRESSURE_BONUS: f32 = 12.0;
const AI_SUPPORT_SHIELD_PRESSURE_EXTRA_RADIUS: f32 = 4.0;
const AI_SUPPORT_SHIELD_PRESSURE_DISTANCE_WEIGHT: f32 = 0.3;
const STRUCTURE_CONSTRUCTION_PROGRESS_PER_SECOND: f32 = 0.3;
const CONSTRUCTION_ENTRY_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
const BASE_CONSTRUCTION_RADIUS_M: f32 = 9.0;
const AI_CONSTRUCTION_REFRESH_INTERVAL_SECONDS: f32 = 0.5;
const SHIELD_TROOPER_PASSIVE_DAMAGE_SCALE: f32 = 0.65;
const SIEGE_DRILL_DEPLOYED_ATTACK_RANGE: f32 = 6.5;
const SIEGE_DRILL_DEPLOYED_ATTACK_INTERVAL: f32 = 1.0;
const SIEGE_DRILL_DEPLOYED_STRUCTURE_DAMAGE_MULTIPLIER: f32 = 3.6;
const SIEGE_DRILL_DEPLOYED_SIGHT_RANGE: f32 = 9.5;
const VETERANCY_MAX_RANK: u8 = 2;
const VETERANCY_DAMAGE_MULTIPLIER_BY_RANK: [f32; 3] = [1.0, 1.25, 1.5];
const VETERANCY_HP_MULTIPLIER_BY_RANK: [f32; 3] = [1.0, 1.2, 1.5];
const VETERANCY_RANGE_BONUS_BY_RANK: [f32; 3] = [0.0, 0.5, 1.0];
const VETERANCY_SIGHT_BONUS_BY_RANK: [f32; 3] = [0.0, 1.0, 2.0];
const VETERANCY_ELITE_REGEN_PER_SECOND: f32 = 1.0;
const VETERANCY_KILLS_BY_RANK: [u32; 3] = [0, 2, 5];
const VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS: f32 = 1.1;
const CRUSH_DAMAGE: f32 = 999.0;
const CRUSH_RADIUS_MARGIN_M: f32 = 0.15;
const CRUSH_MIN_FRAME_DISPLACEMENT_M: f32 = 0.005;
const MOVEMENT_OBSTACLE_CLEARANCE_M: f32 = 0.18;
const MOVEMENT_OBSTACLE_LOOKAHEAD_M: f32 = 2.4;
const MOVEMENT_OBSTACLE_STEER_WEIGHT: f32 = 1.15;
const COMBAT_WRECKAGE_LIFETIME_SECONDS: f32 = 10.0;
const STRUCTURE_FIREBALL_LIFETIME_SECONDS: f32 = 1.4;
const STRUCTURE_SMOKE_COLUMN_LIFETIME_SECONDS: f32 = 4.5;
const BATTLE_LOG_ENTRY_TTL_SECONDS: f32 = 6.5;
const BATTLE_EVENT_PING_LIFETIME_SECONDS: f32 = 4.0;
const BATTLE_LOG_MAX_ENTRIES: usize = 5;
const BATTLE_LOG_UNDER_ATTACK_COOLDOWN_SECONDS: f32 = 7.0;
const BATTLE_LOG_TOP_PX: f32 = 74.0;
const BATTLE_LOG_RIGHT_PX: f32 = 18.0;
const BATTLE_LOG_WIDTH_PX: f32 = 390.0;
const BATTLE_LOG_HIT_HEIGHT_PX: f32 = 168.0;
const MINIMAP_SIZE_PX: f32 = 158.0;
const MINIMAP_RIGHT_PX: f32 = 18.0;
const MINIMAP_BOTTOM_PX: f32 = 146.0;
const MINIMAP_ENTITY_MARKER_PX: f32 = 4.6;
const MINIMAP_STRUCTURE_MARKER_PX: f32 = 6.2;
const MINIMAP_RESOURCE_MARKER_PX: f32 = 3.8;
const INFILTRATION_RESOURCE_STEAL_MIN: i32 = 1;
const PRODUCTION_VETERANCY_PRODUCER_COUNT: usize = 3;
const TERRAIN_TARGET_MAP_MARGIN_M: f32 = 2.5;
const MAX_SKIRMISH_LOBBY_SLOTS: usize = 8;
const DEFAULT_LOBBY_CONTROLLERS: [SkirmishPlayerController; MAX_SKIRMISH_LOBBY_SLOTS] = [
    SkirmishPlayerController::Human,
    SkirmishPlayerController::Ai(AiDifficulty::Easy),
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
];
const DEFAULT_LOBBY_FACTIONS: [SkirmishFaction; MAX_SKIRMISH_LOBBY_SLOTS] = [
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
    SkirmishFaction::Chaos,
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
    SkirmishFaction::Chaos,
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
];
const DEFAULT_LOBBY_TEAM_IDS: [u8; MAX_SKIRMISH_LOBBY_SLOTS] = [0, 1, 2, 3, 4, 5, 6, 7];
const DEFAULT_LOBBY_COLOR_SLOTS: [usize; MAX_SKIRMISH_LOBBY_SLOTS] = [0, 1, 2, 3, 4, 5, 6, 7];
const PLAYER_COLOR_PALETTE: [[f32; 3]; 20] = [
    [0.26, 0.72, 0.38],
    [0.86, 0.26, 0.22],
    [0.58, 0.36, 0.86],
    [0.20, 0.68, 0.82],
    [0.92, 0.62, 0.18],
    [0.82, 0.28, 0.62],
    [0.62, 0.78, 0.22],
    [0.24, 0.42, 0.88],
    [0.80, 0.46, 0.22],
    [0.20, 0.76, 0.58],
    [0.62, 0.34, 0.36],
    [0.46, 0.62, 0.92],
    [0.72, 0.70, 0.36],
    [0.36, 0.56, 0.32],
    [0.78, 0.48, 0.86],
    [0.90, 0.38, 0.42],
    [0.38, 0.78, 0.72],
    [0.54, 0.50, 0.26],
    [0.30, 0.50, 0.66],
    [0.76, 0.54, 0.54],
];
const SKIRMISH_MAP_PREVIEW_SIZE: Vec2 = Vec2::new(360.0, 220.0);
const SKIRMISH_MAP_PREVIEW_PADDING: f32 = 12.0;
const SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS: usize = 4;
const MINE_DEPLOY_OFFSETS: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (1.0, -1.0),
    (-1.0, 1.0),
    (1.0, 1.0),
    (0.0, -1.0),
    (1.0, 0.0),
    (0.0, 1.0),
    (-1.0, 0.0),
];
const AI_CAPTURE_INTERVAL_SECONDS: f32 = 4.5;
const AI_CAPTURE_ENGINEER_LIMIT: usize = 1;
const AI_CAPTURE_NEUTRAL_TECH_TARGET_BONUS: f32 = 18.0;
const AI_SABOTEUR_INTERVAL_SECONDS: f32 = 5.0;
const AI_SABOTEUR_LIMIT: usize = 1;
const AI_SABOTEUR_ID: &str = "SaboteurInfiltrator";
const STRUCTURE_PLACEMENT_ROTATION_STEP_RADIANS: f32 = std::f32::consts::FRAC_PI_4;
const STRUCTURE_PLACEMENT_ROTATION_DEAD_ZONE_M: f32 = 0.1;
const COMMAND_SLOT_HOTKEYS: [CommandHotkey; COMMAND_SLOT_COUNT] = [
    CommandHotkey::new("Q", KeyCode::KeyQ),
    CommandHotkey::new("W", KeyCode::KeyW),
    CommandHotkey::new("E", KeyCode::KeyE),
    CommandHotkey::new("R", KeyCode::KeyR),
    CommandHotkey::new("T", KeyCode::KeyT),
    CommandHotkey::new("Y", KeyCode::KeyY),
    CommandHotkey::new("A", KeyCode::KeyA),
    CommandHotkey::new("S", KeyCode::KeyS),
    CommandHotkey::new("D", KeyCode::KeyD),
    CommandHotkey::new("F", KeyCode::KeyF),
    CommandHotkey::new("G", KeyCode::KeyG),
    CommandHotkey::new("H", KeyCode::KeyH),
    CommandHotkey::new("Z", KeyCode::KeyZ),
    CommandHotkey::new("X", KeyCode::KeyX),
    CommandHotkey::new("C", KeyCode::KeyC),
    CommandHotkey::new("V", KeyCode::KeyV),
    CommandHotkey::new("B", KeyCode::KeyB),
    CommandHotkey::new("N", KeyCode::KeyN),
    CommandHotkey::new("1", KeyCode::Digit1),
    CommandHotkey::new("2", KeyCode::Digit2),
    CommandHotkey::new("3", KeyCode::Digit3),
    CommandHotkey::new("4", KeyCode::Digit4),
    CommandHotkey::new("5", KeyCode::Digit5),
    CommandHotkey::new("6", KeyCode::Digit6),
];
const GROUP_SLOT_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
const CAMERA_BOOKMARK_KEYS: [KeyCode; 4] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportPowerKind {
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

#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum SimulationPhase {
    UiAndManagement,
    BuildProcessing,
    Combat,
    PostCombat,
}

#[derive(States, Default, Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AppScreen {
    #[default]
    MainMenu,
    InMatch,
    RestartingMatch,
}

#[derive(Component)]
struct MatchScopedEntity;

impl SupportPowerKind {
    const ALL: [Self; 9] = [
        Self::RadarSweep,
        Self::OrbitalStrike,
        Self::EmpPulse,
        Self::ChronoRelay,
        Self::ShieldOverdrive,
        Self::NaniteRepairSwarm,
        Self::WeatherStorm,
        Self::StrategicMissile,
        Self::Paradrop,
    ];

    fn idx(self) -> usize {
        match self {
            Self::RadarSweep => 0,
            Self::OrbitalStrike => 1,
            Self::EmpPulse => 2,
            Self::ChronoRelay => 3,
            Self::ShieldOverdrive => 4,
            Self::NaniteRepairSwarm => 5,
            Self::WeatherStorm => 6,
            Self::StrategicMissile => 7,
            Self::Paradrop => 8,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::RadarSweep => "雷达扫描",
            Self::OrbitalStrike => "轨道打击",
            Self::EmpPulse => "EMP脉冲",
            Self::ChronoRelay => "时光回响",
            Self::ShieldOverdrive => "护盾超载",
            Self::NaniteRepairSwarm => "纳米修复",
            Self::WeatherStorm => "气象风暴",
            Self::StrategicMissile => "战略导弹",
            Self::Paradrop => "空投",
        }
    }

    fn is_superweapon(self) -> bool {
        matches!(self, Self::WeatherStorm | Self::StrategicMissile)
    }

    fn hotkey(self) -> KeyCode {
        match self {
            Self::RadarSweep => KeyCode::F1,
            Self::OrbitalStrike => KeyCode::F2,
            Self::EmpPulse => KeyCode::F3,
            Self::ChronoRelay => KeyCode::F4,
            Self::ShieldOverdrive => KeyCode::F5,
            Self::NaniteRepairSwarm => KeyCode::F6,
            Self::WeatherStorm => KeyCode::F7,
            Self::StrategicMissile => KeyCode::F8,
            Self::Paradrop => KeyCode::F9,
        }
    }

    fn definition(self) -> SupportPowerDef {
        match self {
            Self::RadarSweep => SupportPowerDef {
                requirements: &["RadarUplink"],
                cooldown: 18.0,
                radius: 12.0,
                duration: 8.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::OrbitalStrike => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 45.0,
                radius: 3.4,
                duration: 0.0,
                impact_delay: 0.7,
                requires_power: true,
                damage: 8.0,
                damage_scale: 1.2,
                initial_cooldown: 0.0,
                healing: 0.0,
                unit_paths: &[],
            },
            Self::EmpPulse => SupportPowerDef {
                requirements: &["RoboticsBay"],
                cooldown: 36.0,
                radius: 4.8,
                duration: 5.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::ChronoRelay => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 38.0,
                radius: 4.6,
                duration: 7.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.75,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::ShieldOverdrive => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 55.0,
                radius: 4.8,
                duration: 8.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 0.25,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::NaniteRepairSwarm => SupportPowerDef {
                requirements: &["RoboticsBay"],
                cooldown: 42.0,
                radius: 5.2,
                duration: 0.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 10.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::WeatherStorm => SupportPowerDef {
                requirements: &["WeatherControlSpire"],
                cooldown: 90.0,
                radius: 6.4,
                duration: 0.0,
                impact_delay: 1.8,
                requires_power: true,
                damage: 12.0,
                damage_scale: 1.6,
                healing: 0.0,
                initial_cooldown: 90.0,
                unit_paths: &[],
            },
            Self::StrategicMissile => SupportPowerDef {
                requirements: &["WeatherControlSpire"],
                cooldown: 105.0,
                radius: 4.8,
                duration: 0.0,
                impact_delay: 1.4,
                requires_power: true,
                damage: 20.0,
                damage_scale: 2.0,
                healing: 0.0,
                initial_cooldown: 105.0,
                unit_paths: &[],
            },
            Self::Paradrop => SupportPowerDef {
                requirements: &["TechAirport"],
                cooldown: 52.0,
                radius: 2.4,
                duration: 0.0,
                impact_delay: 1.1,
                requires_power: false,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &["LightRifleInfantry", "LightRifleInfantry", "RocketInfantry"],
            },
        }
    }
}

const AI_SUPPORT_POWER_PRIORITY: [SupportPowerKind; 9] = [
    SupportPowerKind::EmpPulse,
    SupportPowerKind::NaniteRepairSwarm,
    SupportPowerKind::ShieldOverdrive,
    SupportPowerKind::ChronoRelay,
    SupportPowerKind::WeatherStorm,
    SupportPowerKind::StrategicMissile,
    SupportPowerKind::OrbitalStrike,
    SupportPowerKind::Paradrop,
    SupportPowerKind::RadarSweep,
];

#[derive(Clone, Copy)]
struct SupportPowerDef {
    requirements: &'static [&'static str],
    cooldown: f32,
    radius: f32,
    duration: f32,
    impact_delay: f32,
    requires_power: bool,
    damage: f32,
    damage_scale: f32,
    healing: f32,
    unit_paths: &'static [&'static str],
    initial_cooldown: f32,
}

#[derive(Resource)]
struct CommandMode {
    attack_move: bool,
    patrol: bool,
    rally_point: bool,
    support_power: Option<SupportPowerKind>,
    pending_structure_placement: Option<PendingStructurePlacement>,
}

impl CommandMode {
    fn has_pending_interaction(&self) -> bool {
        self.attack_move
            || self.patrol
            || self.rally_point
            || self.support_power.is_some()
            || self.pending_structure_placement.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingStructurePlacement {
    id: &'static str,
    rotation_y_radians: f32,
    position: Option<Vec3>,
    drag_rotation_origin: Option<Vec3>,
}

impl PendingStructurePlacement {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            rotation_y_radians: 0.0,
            position: None,
            drag_rotation_origin: None,
        }
    }

    fn rotation_y_radians(self) -> f32 {
        self.rotation_y_radians
    }
}

#[derive(Resource, Default, Clone, Copy)]
struct StructurePlacementFeedback {
    validity: Option<StructurePlacementValidity>,
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
struct MatchMenuState {
    visible: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MatchSpeedPreset {
    Slow,
    #[default]
    Normal,
    Fast,
    Faster,
    Max,
}

impl MatchSpeedPreset {
    const ALL: [Self; 5] = [
        Self::Slow,
        Self::Normal,
        Self::Fast,
        Self::Faster,
        Self::Max,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Slow => "0.75x",
            Self::Normal => "1x",
            Self::Fast => "1.25x",
            Self::Faster => "1.5x",
            Self::Max => "2x",
        }
    }

    fn scale(self) -> f32 {
        match self {
            Self::Slow => 0.75,
            Self::Normal => 1.0,
            Self::Fast => 1.25,
            Self::Faster => 1.5,
            Self::Max => 2.0,
        }
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
struct MatchSpeed {
    preset: MatchSpeedPreset,
}

const MATCH_BRIEFING_AUTO_HIDE_SECONDS: f32 = 14.0;

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct MatchBriefingState {
    visible: bool,
    elapsed_seconds: f32,
    auto_hide_seconds: f32,
}

impl Default for MatchBriefingState {
    fn default() -> Self {
        Self {
            visible: false,
            elapsed_seconds: 0.0,
            auto_hide_seconds: MATCH_BRIEFING_AUTO_HIDE_SECONDS,
        }
    }
}

impl MatchBriefingState {
    fn show(&mut self) {
        self.visible = true;
        self.elapsed_seconds = 0.0;
    }

    fn dismiss(&mut self) {
        self.visible = false;
        self.elapsed_seconds = 0.0;
    }
}

#[derive(Component)]
struct EmpDisabled {
    remaining: f32,
}

#[derive(Component)]
struct ChronoRelay {
    remaining: f32,
    speed_multiplier: f32,
}

#[derive(Component)]
struct SupportShield {
    remaining: f32,
    damage_scale: f32,
}

fn queue_apply_emp_disabled(commands: &mut Commands, entity: Entity, duration: f32) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        entity_mut.remove::<(
            MoveOrder,
            FollowOrder,
            AttackOrder,
            CaptureOrder,
            GarrisonOrder,
            HarvestOrder,
            RepairOrder,
            ConstructOrder,
            AttackMoveOrder,
            PatrolOrder,
            OrderQueue,
        )>();
        if let Some(mut disabled) = entity_mut.get_mut::<EmpDisabled>() {
            disabled.remaining = disabled.remaining.max(duration);
            return;
        }
        entity_mut.insert(EmpDisabled {
            remaining: duration,
        });
    });
}

fn queue_apply_chrono_relay(
    commands: &mut Commands,
    entity: Entity,
    duration: f32,
    speed_multiplier: f32,
) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        if let Some(mut chrono) = entity_mut.get_mut::<ChronoRelay>() {
            chrono.remaining = chrono.remaining.max(duration);
            chrono.speed_multiplier = chrono.speed_multiplier.max(speed_multiplier);
            return;
        }
        entity_mut.insert(ChronoRelay {
            remaining: duration,
            speed_multiplier,
        });
    });
}

fn queue_apply_support_shield(
    commands: &mut Commands,
    entity: Entity,
    duration: f32,
    damage_scale: f32,
) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        let damage_scale = damage_scale.clamp(0.0, 1.0);
        if let Some(mut shield) = entity_mut.get_mut::<SupportShield>() {
            shield.remaining = shield.remaining.max(duration);
            shield.damage_scale = damage_scale;
            return;
        }
        entity_mut.insert(SupportShield {
            remaining: duration,
            damage_scale,
        });
    });
}

#[derive(Component)]
struct PassiveSupportShield {
    damage_scale: f32,
}

#[derive(Component)]
struct MobileShieldProjector {
    refresh_remaining: f32,
    radius: f32,
    duration: f32,
    damage_scale: f32,
}

#[derive(Component, Clone, Copy)]
struct RallyPoint {
    target: Option<Vec3>,
    target_unit: Option<Entity>,
}

#[derive(Component)]
struct RepairAura {
    rate: f32,
    radius: f32,
    mode: RepairAuraMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepairAuraMode {
    AllEligible,
    NearestEligible,
}

#[derive(Component)]
struct HealingAura {
    rate: f32,
    radius: f32,
}

#[derive(Component)]
struct ManualStructureRepair {
    points_remaining: f32,
}

#[derive(Component, Clone, Copy)]
struct UnderConstruction {
    remaining: f32,
    total: f32,
    cost: registry::Cost,
    free_harvester_origin: Option<Vec3>,
}

type StructurePrereqItem<'a> = (
    &'a Structure,
    &'a Team,
    &'a Transform,
    Option<&'a UnderConstruction>,
);
type StructureEntityItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    Option<&'a UnderConstruction>,
);
type ProductionHotkeyStructureItem<'a> = (
    Entity,
    &'a Team,
    &'a Structure,
    &'a Health,
    &'a VisibilityState,
    Option<&'a UnderConstruction>,
);
type SelectedSellStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a UnderConstruction>,
);
type SelectedRepairStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a ManualStructureRepair>,
    Option<&'a UnderConstruction>,
);
type CommandOrderStateItem<'a> = (
    Option<&'a MoveOrder>,
    Option<&'a FollowOrder>,
    Option<&'a AttackOrder>,
    Option<&'a CaptureOrder>,
    Option<&'a GarrisonOrder>,
    Option<&'a HarvestOrder>,
    Option<&'a RepairOrder>,
    Option<&'a ConstructOrder>,
    Option<&'a AttackMoveOrder>,
    Option<&'a PatrolOrder>,
    Option<&'a OrderQueue>,
);
type SelectedCommandUnitItem<'a> = (
    Entity,
    &'a Unit,
    &'a Team,
    &'a Transform,
    &'a HoldPosition,
    CommandOrderStateItem<'a>,
);
type CommandPanelUnitItem<'a> = (
    &'a Unit,
    &'a Team,
    Option<&'a MoveOrder>,
    Option<&'a FollowOrder>,
    Option<&'a AttackOrder>,
    Option<&'a CaptureOrder>,
    Option<&'a GarrisonOrder>,
    Option<&'a HarvestOrder>,
    Option<&'a RepairOrder>,
    Option<&'a ConstructOrder>,
    Option<&'a AttackMoveOrder>,
    Option<&'a PatrolOrder>,
    Option<&'a OrderQueue>,
);
type SelectedOrderUnitItem<'a> = (
    Entity,
    &'a Transform,
    &'a Unit,
    &'a Team,
    CommandOrderStateItem<'a>,
    Option<&'a ResourceCargo>,
);
type SelectableOrderTargetItem<'a> = (
    Entity,
    &'a Transform,
    &'a Selectable,
    &'a Team,
    &'a VisibilityState,
    Option<&'a ResourceNode>,
    Option<&'a SupplyCrate>,
    Option<&'a Health>,
    Option<&'a Unit>,
    Option<&'a Structure>,
    Option<&'a UnderConstruction>,
);
type SelectedCommandUnitFilter = (With<Selected>, With<Unit>, Without<Structure>);
type SelectedOrderUnitFilter = (With<Selected>, With<Unit>, Without<Structure>);
type SelectedRallyPointFilter = (With<Selected>, With<Structure>, Without<Unit>);
type PlacementOccupierItem<'a> = (
    Entity,
    &'a Transform,
    &'a Selectable,
    Option<&'a Health>,
    Option<&'a ResourceNode>,
);
type AiRepairStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a ManualStructureRepair>,
    Option<&'a UnderConstruction>,
);
type CaptureStructureTargetItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    &'a Health,
    Option<&'a UnderConstruction>,
);
type AiOpenBunkerItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    &'a Health,
    &'a Garrison,
    Option<&'a UnderConstruction>,
);
type AiGarrisonUnitItem<'a> = (
    Entity,
    &'a Unit,
    &'a Team,
    &'a Transform,
    &'a Health,
    Option<&'a OrderQueue>,
);

#[derive(Component, Clone, Copy)]
struct DeployedSiegeMode {
    previous_hold_position: bool,
    base_speed: f32,
    base_attack_range: f32,
    base_attack_interval: f32,
    base_structure_damage_multiplier: f32,
    base_sight_range: f32,
}

#[derive(Component)]
struct DeployModeToggleRequest;

#[derive(Component)]
struct AiAttackWaveMember;

#[derive(Component)]
struct SupportWarning {
    remaining: f32,
    radius: f32,
    color: Color,
}

#[derive(Component)]
struct TemporarySupportReveal {
    remaining: f32,
    radius: f32,
}

#[derive(Component)]
struct PendingOrbitalStrike {
    remaining: f32,
    radius: f32,
    damage: f32,
    impact_scale: f32,
    team: Team,
}

#[derive(Component)]
struct PendingParadrop {
    remaining: f32,
    team: Team,
    target: Vec3,
    unit_paths: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct SupportPowerTargetSnapshot {
    entity: Entity,
    team: Team,
    position: Vec3,
    health: Health,
    mobile: bool,
}

#[derive(Resource)]
struct SupportCooldowns {
    remaining: Vec<[f32; SupportPowerKind::ALL.len()]>,
    initial_charge_started: Vec<[bool; SupportPowerKind::ALL.len()]>,
}

impl SupportCooldowns {
    fn ensure_team(&mut self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        if self.remaining.len() <= index {
            self.remaining
                .resize(index + 1, [0.0; SupportPowerKind::ALL.len()]);
            self.initial_charge_started
                .resize(index + 1, [false; SupportPowerKind::ALL.len()]);
        }
        Some(index)
    }

    fn ready(&self, team: Team, power: SupportPowerKind) -> bool {
        self.remaining_for(team, power) <= 0.0
    }

    fn remaining_for(&self, team: Team, power: SupportPowerKind) -> f32 {
        team.economy_index()
            .and_then(|index| self.remaining.get(index))
            .map_or(0.0, |remaining| remaining[power.idx()])
    }

    fn set(&mut self, team: Team, power: SupportPowerKind, base: f32) {
        if let Some(index) = self.ensure_team(team) {
            self.remaining[index][power.idx()] = base;
        }
    }
}

impl Default for SupportCooldowns {
    fn default() -> Self {
        Self {
            remaining: Vec::new(),
            initial_charge_started: Vec::new(),
        }
    }
}

#[derive(Resource)]
struct MatchState {
    phase: MatchPhase,
    result_reason: &'static str,
    start_time_sec: f32,
    remaining_teams: u32,
    remaining_anchors: u32,
    enemy_units_destroyed: u32,
    enemy_structures_destroyed: u32,
    units_lost: u32,
    structures_lost: u32,
}

impl MatchState {
    fn is_running(&self) -> bool {
        matches!(self.phase, MatchPhase::Running)
    }

    fn finish_if_not_set(&mut self, reason: MatchPhase, reason_text: &'static str) {
        if !self.is_running() {
            return;
        }
        self.phase = reason;
        self.result_reason = reason_text;
    }
}

fn match_in_progress(
    app_screen: Res<State<AppScreen>>,
    match_menu: Res<MatchMenuState>,
    match_state: Res<MatchState>,
    match_flow: Res<MatchFlow>,
) -> bool {
    *app_screen.get() == AppScreen::InMatch
        && !match_menu.visible
        && match_state.is_running()
        && match_flow.is_active()
}

fn finalize_match(
    match_state: &mut MatchState,
    match_flow: &mut MatchFlow,
    phase: MatchPhase,
    reason: &'static str,
) {
    match_state.finish_if_not_set(phase, reason);
    match_flow.active = false;
}

impl Default for MatchState {
    fn default() -> Self {
        Self {
            phase: MatchPhase::Running,
            result_reason: "",
            start_time_sec: 0.0,
            remaining_teams: 0,
            remaining_anchors: 0,
            enemy_units_destroyed: 0,
            enemy_structures_destroyed: 0,
            units_lost: 0,
            structures_lost: 0,
        }
    }
}

#[derive(Resource)]
struct MatchFlow {
    active: bool,
}

impl MatchFlow {
    fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for MatchFlow {
    fn default() -> Self {
        Self { active: true }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum MatchPhase {
    #[default]
    Running,
    HumanDefeat,
    HumanVictory,
    MatchFinished,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct VisibilityState {
    visible: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct FogMemoryVisible;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct FogMemoryStructureRemnant {
    radius: f32,
}

#[derive(Component)]
struct VisionRadius(f32);

#[derive(Component)]
struct MatchEndOverlay;

#[derive(Component)]
struct MatchEndTitle;

#[derive(Component)]
struct MatchEndReason;

#[derive(Component)]
struct MatchEndStats;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MatchEndButton {
    action: MatchEndAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchEndAction {
    Restart,
    ReturnToSetup,
    ExitToMenu,
}

#[derive(Component)]
struct MatchMenuOverlay;

#[derive(Component)]
struct MatchMenuStatusText;

#[derive(Component)]
struct MatchBriefingPanel;

#[derive(Component)]
struct MatchBriefingText;

#[derive(Component)]
struct MatchBriefingReopenButton;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MatchBriefingButton {
    action: MatchBriefingAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchBriefingAction {
    Show,
    Dismiss,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MatchMenuButton {
    action: MatchMenuAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchMenuAction {
    Resume,
    SetSpeed(MatchSpeedPreset),
    PreviousPerspective,
    NextPerspective,
    Restart,
    ReturnToSetup,
}

#[derive(Component)]
struct ClickMarker {
    ttl: f32,
    radius: f32,
}

#[derive(Component)]
struct CombatWreckage {
    remaining: f32,
}

#[derive(Component)]
struct ScorchMark;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructureDestructionVfxKind {
    ExplosionFireball,
    SmokeColumn,
}

#[derive(Component, Clone, Copy)]
struct StructureDestructionVfx {
    kind: StructureDestructionVfxKind,
    remaining: f32,
    total: f32,
    radius: f32,
    team: Team,
}

#[derive(Component, Clone, Copy)]
struct VeterancyPromotionEffect {
    rank: u8,
    remaining: f32,
    total: f32,
    radius: f32,
    team: Team,
}

impl Default for CommandMode {
    fn default() -> Self {
        Self {
            attack_move: false,
            patrol: false,
            rally_point: false,
            support_power: None,
            pending_structure_placement: None,
        }
    }
}

#[derive(Clone, Copy)]
struct SpawnSpec {
    id: &'static str,
    offset: (f32, f32),
}

#[derive(Clone, Copy)]
struct ResourceSpec {
    kind: ResourceKind,
    amount: i32,
    position: (f32, f32),
}

#[derive(Clone, Copy)]
struct NamedSupplyCrateSpec {
    name: &'static str,
    effect: SupplyCrateEffect,
    position: (f32, f32),
}

#[derive(Clone, Copy)]
struct NeutralTechSpec {
    name: &'static str,
    id: &'static str,
    position: (f32, f32),
}

#[derive(Clone, Copy)]
struct SkirmishMapDef {
    id: &'static str,
    godot_path: &'static str,
    name: &'static str,
    name_key: &'static str,
    players: usize,
    size: (f32, f32),
    spawn_points: &'static [(f32, f32)],
    resources: &'static [ResourceSpec],
    neutral_tech: &'static [NeutralTechSpec],
    supply_crates: &'static [NamedSupplyCrateSpec],
}

impl SkirmishMapDef {
    fn contains_point(self, point: (f32, f32)) -> bool {
        point.0 >= 0.0 && point.1 >= 0.0 && point.0 <= self.size.0 && point.1 <= self.size.1
    }

    fn is_catalog_consistent(self) -> bool {
        !self.id.is_empty()
            && !self.godot_path.is_empty()
            && !self.name.is_empty()
            && !self.name_key.is_empty()
            && self.players > 0
            && self.size.0 > 0.0
            && self.size.1 > 0.0
            && self.spawn_points.len() == self.players
            && self.resources.len() >= self.players * 3
            && self
                .spawn_points
                .iter()
                .copied()
                .all(|point| self.contains_point(point))
            && self
                .resources
                .iter()
                .all(|resource| self.contains_point(resource.position))
            && self.neutral_tech.iter().all(|tech| {
                !tech.name.is_empty() && !tech.id.is_empty() && self.contains_point(tech.position)
            })
            && self.supply_crates.iter().all(|crate_spec| {
                !crate_spec.name.is_empty()
                    && matches!(
                        crate_spec.effect,
                        SupplyCrateEffect::Resources
                            | SupplyCrateEffect::Repair
                            | SupplyCrateEffect::Veterancy
                    )
                    && self.contains_point(crate_spec.position)
            })
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct SelectedSkirmishMap {
    godot_path: &'static str,
}

impl Default for SelectedSkirmishMap {
    fn default() -> Self {
        Self {
            godot_path: SKIRMISH_MAPS[0].godot_path,
        }
    }
}

impl SelectedSkirmishMap {
    fn definition(self) -> &'static SkirmishMapDef {
        skirmish_map_by_path(self.godot_path).unwrap_or(&SKIRMISH_MAPS[0])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StartingResources {
    ore: i32,
    crystal: i32,
}

impl StartingResources {
    const fn new(ore: i32, crystal: i32) -> Self {
        Self { ore, crystal }
    }

    const fn godot_standard() -> Self {
        Self::new(8, 4)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StartingResourceOption {
    key: &'static str,
    resources: StartingResources,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkirmishFaction {
    Alliance,
    Demon,
    Chaos,
}

impl SkirmishFaction {
    const ALL: [Self; 3] = [Self::Alliance, Self::Demon, Self::Chaos];

    fn registry_id(self) -> &'static str {
        match self {
            Self::Alliance => "alliance",
            Self::Demon => "demon",
            Self::Chaos => "chaos",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Alliance => "人族",
            Self::Demon => "魔族",
            Self::Chaos => "混沌族",
        }
    }

    fn from_team(team: Team) -> Self {
        match team {
            Team::Player(index) => DEFAULT_LOBBY_FACTIONS
                .get(index)
                .copied()
                .unwrap_or(Self::Alliance),
            Team::Neutral => Self::Alliance,
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|faction| *faction == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

const GODOT_STANDARD_STARTING_RESOURCE_INDEX: usize = 1;
const DEFAULT_STARTING_RESOURCE_INDEX: usize = 3;
const SKIRMISH_TEAM_OPTION_COUNT: u8 = MAX_SKIRMISH_LOBBY_SLOTS as u8;
const BEVY_PLAYTEST_STARTING_RESOURCES: StartingResources = StartingResources::new(260, 80);
fn default_active_teams() -> Vec<bool> {
    DEFAULT_LOBBY_CONTROLLERS
        .into_iter()
        .map(SkirmishPlayerController::is_active)
        .collect()
}

fn default_player_factions() -> Vec<SkirmishFaction> {
    DEFAULT_LOBBY_FACTIONS.to_vec()
}

fn default_player_color_slots() -> Vec<usize> {
    DEFAULT_LOBBY_COLOR_SLOTS.to_vec()
}

fn default_player_controllers() -> Vec<SkirmishPlayerController> {
    DEFAULT_LOBBY_CONTROLLERS.to_vec()
}

fn default_player_spawn_slots() -> Vec<usize> {
    (0..MAX_SKIRMISH_LOBBY_SLOTS).collect()
}

const GODOT_STARTING_RESOURCE_OPTIONS: &[StartingResourceOption] = &[
    StartingResourceOption {
        key: "STARTING_RESOURCES_LOW",
        resources: StartingResources::new(4, 2),
    },
    StartingResourceOption {
        key: "STARTING_RESOURCES_STANDARD",
        resources: StartingResources::godot_standard(),
    },
    StartingResourceOption {
        key: "STARTING_RESOURCES_HIGH",
        resources: StartingResources::new(16, 8),
    },
    StartingResourceOption {
        key: "STARTING_RESOURCES_RICH",
        resources: StartingResources::new(32, 16),
    },
];

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct MatchSetupSettings {
    map_path: &'static str,
    starting_resources: StartingResources,
    visible_player: VisiblePlayer,
    ai_difficulties: AiDifficultySettings,
    team_relations: TeamRelations,
    startup_loadout: StartupLoadoutMode,
    active_teams: Vec<bool>,
    player_factions: Vec<SkirmishFaction>,
    player_color_slots: Vec<usize>,
    player_controllers: Vec<SkirmishPlayerController>,
    player_spawn_slots: Vec<usize>,
}

impl Default for MatchSetupSettings {
    fn default() -> Self {
        Self {
            map_path: SKIRMISH_MAPS[0].godot_path,
            starting_resources: StartingResources::new(32, 16),
            visible_player: VisiblePlayer::default(),
            ai_difficulties: skirmish_ai_difficulties_from_controllers(
                &default_player_controllers(),
            ),
            team_relations: TeamRelations::default(),
            startup_loadout: StartupLoadoutMode::PlaytestExpanded,
            active_teams: default_active_teams(),
            player_factions: default_player_factions(),
            player_color_slots: default_player_color_slots(),
            player_controllers: default_player_controllers(),
            player_spawn_slots: default_player_spawn_slots(),
        }
    }
}

impl MatchSetupSettings {
    #[cfg(test)]
    fn with_map(mut self, map_path: &'static str) -> Self {
        self.map_path = map_path;
        self
    }

    #[cfg(test)]
    fn with_starting_resources(mut self, starting_resources: StartingResources) -> Self {
        self.starting_resources = starting_resources;
        self
    }

    #[cfg(test)]
    fn with_visible_player(mut self, visible_player: VisiblePlayer) -> Self {
        self.visible_player = visible_player;
        self
    }

    #[cfg(test)]
    fn with_ai_difficulties(mut self, ai_difficulties: AiDifficultySettings) -> Self {
        self.ai_difficulties = ai_difficulties;
        self
    }

    #[cfg(test)]
    fn with_startup_loadout(mut self, startup_loadout: StartupLoadoutMode) -> Self {
        self.startup_loadout = startup_loadout;
        self
    }

    fn team_active(&self, team: Team) -> bool {
        team.economy_index()
            .and_then(|index| self.active_teams.get(index).copied())
            .unwrap_or(false)
    }

    fn player_faction(&self, team: Team) -> SkirmishFaction {
        team.economy_index()
            .and_then(|index| self.player_factions.get(index).copied())
            .unwrap_or_else(|| SkirmishFaction::from_team(team))
    }

    fn player_spawn_slot(&self, team: Team) -> usize {
        team.economy_index()
            .and_then(|index| self.player_spawn_slots.get(index).copied())
            .unwrap_or_else(|| team.economy_index().unwrap_or(0))
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct ActiveTeams(Vec<bool>);

impl Default for ActiveTeams {
    fn default() -> Self {
        Self(default_active_teams())
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct PlayerFactions(Vec<SkirmishFaction>);

impl Default for PlayerFactions {
    fn default() -> Self {
        Self(default_player_factions())
    }
}

impl PlayerFactions {
    fn faction(&self, team: Team) -> Option<SkirmishFaction> {
        team.economy_index()
            .and_then(|index| self.0.get(index).copied())
    }

    fn slot_faction(&self, team: Team) -> SkirmishFaction {
        self.faction(team)
            .unwrap_or_else(|| SkirmishFaction::from_team(team))
    }
}

fn faction_def(faction: SkirmishFaction) -> Option<&'static registry::FactionDef> {
    registry::faction(faction.registry_id())
}

fn slot_faction_from_option(
    player_factions: Option<&PlayerFactions>,
    team: Team,
) -> SkirmishFaction {
    player_factions.map_or_else(
        || SkirmishFaction::from_team(team),
        |factions| factions.slot_faction(team),
    )
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct PlayerColorSlots(Vec<usize>);

impl Default for PlayerColorSlots {
    fn default() -> Self {
        Self(default_player_color_slots())
    }
}

impl PlayerColorSlots {
    fn slot(&self, team: Team) -> Option<usize> {
        team.economy_index()
            .and_then(|index| self.0.get(index).copied())
            .map(|slot| slot % PLAYER_COLOR_PALETTE.len())
    }

    fn color(&self, team: Team) -> Color {
        self.slot(team)
            .map(player_color)
            .unwrap_or_else(|| Color::srgb(0.74, 0.77, 0.72))
    }

    fn color_rgb(&self, team: Team) -> [f32; 3] {
        self.slot(team)
            .map(player_color_rgb)
            .unwrap_or([0.74, 0.77, 0.72])
    }

    fn minimap_color(&self, team: Team) -> Color {
        self.slot(team)
            .map(|slot| player_color_with_alpha(slot, 0.95))
            .unwrap_or_else(|| Color::srgba(0.78, 0.78, 0.68, 0.86))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkirmishPlayerController {
    None,
    Human,
    Ai(AiDifficulty),
}

impl SkirmishPlayerController {
    fn is_active(self) -> bool {
        self != Self::None
    }

    fn is_human(self) -> bool {
        self == Self::Human
    }

    fn ai_difficulty(self) -> Option<AiDifficulty> {
        match self {
            Self::Ai(difficulty) => Some(difficulty),
            Self::None | Self::Human => None,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::None => Self::Human,
            Self::Human => Self::Ai(AiDifficulty::Beginner),
            Self::Ai(AiDifficulty::Beginner) => Self::Ai(AiDifficulty::Easy),
            Self::Ai(AiDifficulty::Easy) => Self::Ai(AiDifficulty::Normal),
            Self::Ai(AiDifficulty::Normal) => Self::Ai(AiDifficulty::Hard),
            Self::Ai(AiDifficulty::Hard) => Self::None,
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Human => "人族玩家",
            Self::Ai(AiDifficulty::Beginner) => "AI新手",
            Self::Ai(AiDifficulty::Easy) => "AI简单",
            Self::Ai(AiDifficulty::Normal) => "AI普通",
            Self::Ai(AiDifficulty::Hard) => "AI困难",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SkirmishMatchMode {
    #[default]
    OneVsOne,
    FreeForAll,
    AiVsAi,
    AlliedTwoVsOne,
}

impl SkirmishMatchMode {
    const ALL: [Self; 4] = [
        Self::OneVsOne,
        Self::FreeForAll,
        Self::AiVsAi,
        Self::AlliedTwoVsOne,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::OneVsOne => "one_vs_one",
            Self::FreeForAll => "free_for_all",
            Self::AiVsAi => "ai_vs_ai",
            Self::AlliedTwoVsOne => "allied_two_vs_one",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OneVsOne => "1v1",
            Self::FreeForAll => "自由混战",
            Self::AiVsAi => "AI对战",
            Self::AlliedTwoVsOne => "盟军2v1",
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct SkirmishMenuSelection {
    map_index: usize,
    starting_resource_index: usize,
    match_mode: SkirmishMatchMode,
    ai_difficulty: AiDifficulty,
    lobby_controllers: [SkirmishPlayerController; MAX_SKIRMISH_LOBBY_SLOTS],
    lobby_factions: [SkirmishFaction; MAX_SKIRMISH_LOBBY_SLOTS],
    lobby_team_ids: [u8; MAX_SKIRMISH_LOBBY_SLOTS],
    lobby_color_slots: [usize; MAX_SKIRMISH_LOBBY_SLOTS],
}

impl Default for SkirmishMenuSelection {
    fn default() -> Self {
        Self {
            map_index: 0,
            starting_resource_index: DEFAULT_STARTING_RESOURCE_INDEX,
            match_mode: SkirmishMatchMode::OneVsOne,
            ai_difficulty: AiDifficulty::Easy,
            lobby_controllers: DEFAULT_LOBBY_CONTROLLERS,
            lobby_factions: DEFAULT_LOBBY_FACTIONS,
            lobby_team_ids: DEFAULT_LOBBY_TEAM_IDS,
            lobby_color_slots: DEFAULT_LOBBY_COLOR_SLOTS,
        }
    }
}

impl SkirmishMenuSelection {
    fn map(self) -> &'static SkirmishMapDef {
        if self.map_choice_is_random() {
            return largest_skirmish_map();
        }
        &SKIRMISH_MAPS[self.map_index.min(SKIRMISH_MAPS.len().saturating_sub(1))]
    }

    fn map_choice_is_random(self) -> bool {
        is_random_map_index(self.map_index)
    }

    fn map_label(self) -> &'static str {
        if self.map_choice_is_random() {
            RANDOM_MAP_LABEL
        } else {
            self.map().name
        }
    }

    fn from_match_setup(settings: MatchSetupSettings) -> Self {
        let map_index = SKIRMISH_MAPS
            .iter()
            .position(|map| map.godot_path == settings.map_path)
            .unwrap_or(0);
        let starting_resource_index = GODOT_STARTING_RESOURCE_OPTIONS
            .iter()
            .position(|option| option.resources == settings.starting_resources)
            .unwrap_or(GODOT_STANDARD_STARTING_RESOURCE_INDEX);
        let focus_team = if settings.visible_player.team.economy_index().is_some() {
            settings.visible_player.team
        } else {
            Team::Player(0)
        };
        Self {
            map_index,
            starting_resource_index,
            match_mode: if settings.visible_player.is_spectator() {
                SkirmishMatchMode::AiVsAi
            } else {
                skirmish_mode_from_match_setup(&settings)
            },
            ai_difficulty: settings.ai_difficulties.default_ai_difficulty(focus_team),
            lobby_controllers: lobby_controllers_from_match_setup(&settings),
            lobby_factions: lobby_factions_from_match_setup(&settings),
            lobby_team_ids: lobby_team_ids_from_match_setup(&settings),
            lobby_color_slots: lobby_color_slots_from_match_setup(&settings),
        }
    }

    fn starting_resources(self) -> StartingResources {
        GODOT_STARTING_RESOURCE_OPTIONS
            .get(self.starting_resource_index)
            .unwrap_or(&GODOT_STARTING_RESOURCE_OPTIONS[GODOT_STANDARD_STARTING_RESOURCE_INDEX])
            .resources
    }

    fn active_teams(self) -> Vec<bool> {
        skirmish_active_teams_from_controllers(&self.runtime_player_controllers())
    }

    fn active_team_count(self) -> usize {
        self.active_lobby_slot_count()
    }

    fn lobby_slot_limit(self) -> usize {
        if self.map_choice_is_random() {
            largest_skirmish_map().players.min(MAX_SKIRMISH_LOBBY_SLOTS)
        } else {
            self.map().players.min(MAX_SKIRMISH_LOBBY_SLOTS)
        }
    }

    fn active_lobby_slot_count(self) -> usize {
        (0..self.lobby_slot_limit())
            .filter(|slot| self.lobby_controllers[*slot].is_active())
            .count()
    }

    fn active_lobby_slots(self) -> Vec<usize> {
        (0..self.lobby_slot_limit())
            .filter(|slot| self.lobby_controllers[*slot].is_active())
            .collect()
    }

    fn runtime_slot_for_team(self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        self.active_lobby_slots().get(index).copied()
    }

    fn runtime_player_controllers(self) -> Vec<SkirmishPlayerController> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_controllers[slot])
            .collect()
    }

    fn runtime_player_factions(self) -> Vec<SkirmishFaction> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_factions[slot])
            .collect()
    }

    fn runtime_team_ids(self) -> Vec<u8> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_team_ids[slot] % SKIRMISH_TEAM_OPTION_COUNT)
            .collect()
    }

    fn runtime_color_slots(self) -> Vec<usize> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_color_slots[slot] % PLAYER_COLOR_PALETTE.len())
            .collect()
    }

    fn runtime_spawn_slots(self) -> Vec<usize> {
        self.active_lobby_slots()
    }

    fn resolved_map(self, seed: u32) -> &'static SkirmishMapDef {
        if self.map_choice_is_random() {
            random_map_for_required_slots(self.required_player_slots(), seed)
        } else {
            self.map()
        }
    }

    fn required_player_slots(self) -> usize {
        self.active_lobby_slot_count().max(2)
    }

    fn selected_map_player_slots(self) -> usize {
        if self.map_choice_is_random() {
            random_map_for_required_slots(self.required_player_slots(), 0).players
        } else {
            self.map().players
        }
    }

    fn start_status(self) -> SkirmishStartStatus {
        skirmish_start_status_for_setup(
            self.required_player_slots(),
            self.selected_map_player_slots(),
            &self.active_teams(),
            &self.team_relations(),
        )
    }

    fn can_start(self) -> bool {
        self.start_status().can_start()
    }

    fn match_setup_with_map_seed(self, seed: u32) -> MatchSetupSettings {
        let active_teams = self.active_teams();
        let player_controllers = self.runtime_player_controllers();
        let player_factions = self.runtime_player_factions();
        let player_color_slots = self.runtime_color_slots();
        let team_ids = self.runtime_team_ids();
        let visible_player = if let Some(team) = self.human_team() {
            VisiblePlayer::per_player(team)
        } else {
            VisiblePlayer::all_players(self.focus_team())
        };
        let ai_difficulties = skirmish_ai_difficulties_from_controllers(&player_controllers);
        MatchSetupSettings {
            map_path: self.resolved_map(seed).godot_path,
            starting_resources: self.starting_resources(),
            visible_player,
            ai_difficulties,
            team_relations: skirmish_team_relations_from_team_ids(&active_teams, &team_ids),
            active_teams,
            player_factions,
            player_color_slots,
            player_controllers,
            player_spawn_slots: self.runtime_spawn_slots(),
            ..MatchSetupSettings::default()
        }
    }

    #[cfg(test)]
    fn match_setup(self) -> MatchSetupSettings {
        self.match_setup_with_map_seed(0)
    }

    fn set_match_mode(&mut self, mode: SkirmishMatchMode) {
        self.match_mode = mode;
    }

    fn lobby_slot_in_selected_map(self, slot: usize) -> bool {
        slot < self.lobby_slot_limit()
    }

    fn select_lobby_slot(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        for (other_slot, controller) in self.lobby_controllers.iter_mut().enumerate() {
            if other_slot != slot && controller.is_human() {
                *controller = SkirmishPlayerController::Ai(self.ai_difficulty);
            }
        }
        self.lobby_controllers[slot] = SkirmishPlayerController::Human;
    }

    fn set_lobby_slot_controller(&mut self, slot: usize, controller: SkirmishPlayerController) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        if controller.is_human() {
            self.select_lobby_slot(slot);
        } else {
            self.lobby_controllers[slot] = controller;
        }
    }

    fn cycle_lobby_slot_controller(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.set_lobby_slot_controller(slot, self.lobby_controllers[slot].next());
    }

    fn cycle_lobby_slot_faction(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_factions[slot] = self.lobby_factions[slot].next();
    }

    fn cycle_lobby_slot_team_id(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_team_ids[slot] = (self.lobby_team_ids[slot] + 1) % SKIRMISH_TEAM_OPTION_COUNT;
    }

    fn cycle_lobby_slot_color(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_color_slots[slot] =
            (self.lobby_color_slots[slot] + 1) % PLAYER_COLOR_PALETTE.len();
    }

    fn team_id(self, team: Team) -> Option<u8> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_team_ids[slot] % SKIRMISH_TEAM_OPTION_COUNT)
    }

    fn player_faction(self, team: Team) -> Option<SkirmishFaction> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_factions[slot])
    }

    fn focus_faction(self) -> SkirmishFaction {
        self.player_faction(self.focus_team())
            .unwrap_or_else(|| SkirmishFaction::from_team(self.focus_team()))
    }

    fn player_color_slot(self, team: Team) -> Option<usize> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_color_slots[slot] % PLAYER_COLOR_PALETTE.len())
    }

    fn set_ai_difficulty(&mut self, difficulty: AiDifficulty) {
        self.ai_difficulty = difficulty;
        for controller in &mut self.lobby_controllers {
            if matches!(controller, SkirmishPlayerController::Ai(_)) {
                *controller = SkirmishPlayerController::Ai(difficulty);
            }
        }
    }

    fn player_controller(self, team: Team) -> Option<SkirmishPlayerController> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_controllers[slot])
    }

    fn human_lobby_slot(self) -> Option<usize> {
        (0..self.lobby_slot_limit()).find(|slot| self.lobby_controllers[*slot].is_human())
    }

    fn human_team(self) -> Option<Team> {
        player_teams(self.active_lobby_slot_count()).find(|team| {
            self.player_controller(*team)
                .is_some_and(|controller| controller.is_human())
        })
    }

    fn focus_team(self) -> Team {
        self.human_team()
            .or_else(|| {
                player_teams(self.active_lobby_slot_count()).find(|team| {
                    self.player_controller(*team)
                        .is_some_and(|controller| controller.is_active())
                })
            })
            .unwrap_or(Team::Player(0))
    }

    fn focus_lobby_slot(self) -> Option<usize> {
        self.human_lobby_slot()
            .or_else(|| self.active_lobby_slots().into_iter().next())
    }

    fn team_relations(self) -> TeamRelations {
        let active_teams = self.active_teams();
        let team_ids = self.runtime_team_ids();
        skirmish_team_relations_from_team_ids(&active_teams, &team_ids)
    }
}

fn match_setup_from_menu_selection(
    selection: SkirmishMenuSelection,
    random_map_cursor: &mut RandomMapCursor,
) -> Option<MatchSetupSettings> {
    if !selection.can_start() {
        return None;
    }
    let seed = if selection.map_choice_is_random() {
        random_map_cursor.next_seed()
    } else {
        0
    };
    Some(selection.match_setup_with_map_seed(seed))
}

fn request_shared_match_scene_start(
    setup_settings: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
    settings: MatchSetupSettings,
) {
    *setup_settings = settings;
    next_state.set(AppScreen::InMatch);
}

fn start_shared_match_from_menu_selection(
    selection: SkirmishMenuSelection,
    setup_settings: &mut MatchSetupSettings,
    random_map_cursor: &mut RandomMapCursor,
    next_state: &mut NextState<AppScreen>,
) -> bool {
    let Some(settings) = match_setup_from_menu_selection(selection, random_map_cursor) else {
        return false;
    };
    request_shared_match_scene_start(setup_settings, next_state, settings);
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkirmishStartStatus {
    Ready,
    MapTooSmall {
        required_slots: usize,
        available_slots: usize,
    },
    NotEnoughPlayers {
        active_players: usize,
        required_players: usize,
    },
    NoOpposingTeams,
}

fn skirmish_start_status(required_slots: usize, available_slots: usize) -> SkirmishStartStatus {
    if available_slots < required_slots {
        SkirmishStartStatus::MapTooSmall {
            required_slots,
            available_slots,
        }
    } else {
        SkirmishStartStatus::Ready
    }
}

fn skirmish_start_status_for_setup(
    required_slots: usize,
    available_slots: usize,
    active_teams: &[bool],
    relations: &TeamRelations,
) -> SkirmishStartStatus {
    let map_status = skirmish_start_status(required_slots, available_slots);
    if !map_status.can_start() {
        return map_status;
    }
    let active_players = active_teams.iter().filter(|active| **active).count();
    const REQUIRED_SKIRMISH_PLAYERS: usize = 2;
    if active_players < REQUIRED_SKIRMISH_PLAYERS {
        return SkirmishStartStatus::NotEnoughPlayers {
            active_players,
            required_players: REQUIRED_SKIRMISH_PLAYERS,
        };
    }
    if skirmish_has_opposing_active_teams(active_teams, relations) {
        SkirmishStartStatus::Ready
    } else {
        SkirmishStartStatus::NoOpposingTeams
    }
}

impl SkirmishStartStatus {
    fn can_start(self) -> bool {
        matches!(self, Self::Ready)
    }

    fn summary_label(self) -> String {
        match self {
            Self::Ready => "状态: 可开始".to_string(),
            Self::MapTooSmall {
                required_slots,
                available_slots,
            } => {
                format!("状态: 地图出生点不足({available_slots}/{required_slots})")
            }
            Self::NotEnoughPlayers {
                active_players,
                required_players,
            } => {
                format!(
                    "状态: 需要至少{required_players}名玩家({active_players}/{required_players})"
                )
            }
            Self::NoOpposingTeams => "状态: 缺少敌对队伍".to_string(),
        }
    }
}

const RANDOM_MAP_LABEL: &str = "随机地图";

#[derive(Clone, Debug)]
struct RouletteWheel<T> {
    values_w_accumulated_shares: Vec<(T, f32)>,
}

impl<T> RouletteWheel<T> {
    fn new(value_to_share_mapping: impl IntoIterator<Item = (T, f32)>) -> Self {
        let values_w_positive_shares = value_to_share_mapping
            .into_iter()
            .filter(|(_, share)| *share > 0.0)
            .collect::<Vec<_>>();
        let total_share = values_w_positive_shares
            .iter()
            .map(|(_, share)| *share)
            .sum::<f32>();

        if total_share <= 0.0 {
            return Self {
                values_w_accumulated_shares: Vec::new(),
            };
        }

        let mut accumulated_share = 0.0;
        let mut values_w_accumulated_shares = Vec::with_capacity(values_w_positive_shares.len());
        for (value, share) in values_w_positive_shares {
            accumulated_share += share / total_share;
            values_w_accumulated_shares.push((value, accumulated_share));
        }

        Self {
            values_w_accumulated_shares,
        }
    }

    fn get_value(&self, probability: f32) -> Option<&T> {
        if self.values_w_accumulated_shares.is_empty() {
            return None;
        }

        let normalized_probability = probability.clamp(0.0, 1.0);
        for (value, accumulated_share) in &self.values_w_accumulated_shares {
            if normalized_probability <= *accumulated_share {
                return Some(value);
            }
        }

        self.values_w_accumulated_shares
            .last()
            .map(|(value, _)| value)
    }
}

fn random_map_index() -> usize {
    SKIRMISH_MAPS.len()
}

fn is_random_map_index(index: usize) -> bool {
    index == random_map_index()
}

fn random_map_candidates_for_required_slots(
    required_player_slots: usize,
) -> impl Iterator<Item = &'static SkirmishMapDef> {
    let required_player_slots = required_player_slots.max(2);
    SKIRMISH_MAPS
        .iter()
        .filter(move |map| map.players >= required_player_slots)
}

fn random_map_for_required_slots(
    required_player_slots: usize,
    seed: u32,
) -> &'static SkirmishMapDef {
    let candidates =
        random_map_candidates_for_required_slots(required_player_slots).collect::<Vec<_>>();
    if candidates.is_empty() {
        return largest_skirmish_map();
    }

    let probability = roulette_bucket_probability(seed, candidates.len());
    let wheel = RouletteWheel::new(candidates.into_iter().map(|map| (map, 1.0)));
    wheel
        .get_value(probability)
        .copied()
        .unwrap_or_else(largest_skirmish_map)
}

fn roulette_bucket_probability(seed: u32, bucket_count: usize) -> f32 {
    if bucket_count == 0 {
        return 0.0;
    }
    let bucket = seed as usize % bucket_count;
    (bucket as f32 + 0.5) / bucket_count as f32
}

fn largest_skirmish_map() -> &'static SkirmishMapDef {
    SKIRMISH_MAPS
        .iter()
        .max_by(|left, right| {
            left.players
                .cmp(&right.players)
                .then_with(|| skirmish_map_area(left).total_cmp(&skirmish_map_area(right)))
        })
        .unwrap_or(&SKIRMISH_MAPS[0])
}

fn skirmish_map_area(map: &SkirmishMapDef) -> f32 {
    map.size.0 * map.size.1
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct RandomMapCursor(u32);

impl Default for RandomMapCursor {
    fn default() -> Self {
        Self(0x4f1b_2c3d)
    }
}

impl RandomMapCursor {
    fn next_seed(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

fn skirmish_active_teams_from_controllers(controllers: &[SkirmishPlayerController]) -> Vec<bool> {
    controllers
        .iter()
        .copied()
        .map(SkirmishPlayerController::is_active)
        .collect()
}

fn skirmish_ai_difficulties_from_controllers(
    controllers: &[SkirmishPlayerController],
) -> AiDifficultySettings {
    let mut settings = AiDifficultySettings::default();
    for (index, controller) in controllers.iter().copied().enumerate() {
        if let Some(difficulty) = controller.ai_difficulty() {
            settings.set_difficulty(Team::Player(index), difficulty);
        }
    }
    settings
}

fn skirmish_player_controllers_from_match_setup(
    settings: &MatchSetupSettings,
) -> Vec<SkirmishPlayerController> {
    let mut controllers = vec![SkirmishPlayerController::None; settings.active_teams.len()];
    for (index, active) in settings.active_teams.iter().copied().enumerate() {
        if !active {
            continue;
        }
        let team = Team::Player(index);
        controllers[index] = if settings.visible_player.control == PlayerControlMode::Player
            && settings.visible_player.team == team
        {
            SkirmishPlayerController::Human
        } else {
            SkirmishPlayerController::Ai(settings.ai_difficulties.difficulty(team))
        };
    }
    controllers
}

fn lobby_controllers_from_match_setup(
    settings: &MatchSetupSettings,
) -> [SkirmishPlayerController; MAX_SKIRMISH_LOBBY_SLOTS] {
    let mut controllers = [SkirmishPlayerController::None; MAX_SKIRMISH_LOBBY_SLOTS];
    let runtime_controllers = skirmish_player_controllers_from_match_setup(settings);
    for (runtime_index, controller) in runtime_controllers.into_iter().enumerate() {
        let slot = settings
            .player_spawn_slots
            .get(runtime_index)
            .copied()
            .unwrap_or(runtime_index);
        if slot < MAX_SKIRMISH_LOBBY_SLOTS {
            controllers[slot] = controller;
        }
    }
    controllers
}

fn lobby_factions_from_match_setup(
    settings: &MatchSetupSettings,
) -> [SkirmishFaction; MAX_SKIRMISH_LOBBY_SLOTS] {
    let mut factions = DEFAULT_LOBBY_FACTIONS;
    for (runtime_index, faction) in settings.player_factions.iter().copied().enumerate() {
        let slot = settings
            .player_spawn_slots
            .get(runtime_index)
            .copied()
            .unwrap_or(runtime_index);
        if slot < MAX_SKIRMISH_LOBBY_SLOTS {
            factions[slot] = faction;
        }
    }
    factions
}

fn lobby_team_ids_from_match_setup(
    settings: &MatchSetupSettings,
) -> [u8; MAX_SKIRMISH_LOBBY_SLOTS] {
    let mut team_ids = DEFAULT_LOBBY_TEAM_IDS;
    let runtime_team_ids =
        skirmish_team_ids_from_relations(&settings.active_teams, &settings.team_relations);
    for (runtime_index, team_id) in runtime_team_ids.into_iter().enumerate() {
        let slot = settings
            .player_spawn_slots
            .get(runtime_index)
            .copied()
            .unwrap_or(runtime_index);
        if slot < MAX_SKIRMISH_LOBBY_SLOTS {
            team_ids[slot] = team_id % SKIRMISH_TEAM_OPTION_COUNT;
        }
    }
    team_ids
}

fn lobby_color_slots_from_match_setup(
    settings: &MatchSetupSettings,
) -> [usize; MAX_SKIRMISH_LOBBY_SLOTS] {
    let mut color_slots = DEFAULT_LOBBY_COLOR_SLOTS;
    for (runtime_index, color_slot) in settings.player_color_slots.iter().copied().enumerate() {
        let slot = settings
            .player_spawn_slots
            .get(runtime_index)
            .copied()
            .unwrap_or(runtime_index);
        if slot < MAX_SKIRMISH_LOBBY_SLOTS {
            color_slots[slot] = color_slot % PLAYER_COLOR_PALETTE.len();
        }
    }
    color_slots
}

fn skirmish_mode_from_active_teams(active_teams: &[bool]) -> SkirmishMatchMode {
    if active_teams.iter().filter(|active| **active).count() >= 3 {
        SkirmishMatchMode::FreeForAll
    } else {
        SkirmishMatchMode::OneVsOne
    }
}

fn skirmish_mode_from_match_setup(settings: &MatchSetupSettings) -> SkirmishMatchMode {
    if settings
        .active_teams
        .iter()
        .filter(|active| **active)
        .count()
        >= 3
        && skirmish_has_cross_team_alliance(&settings.team_relations, &settings.active_teams)
    {
        SkirmishMatchMode::AlliedTwoVsOne
    } else {
        skirmish_mode_from_active_teams(&settings.active_teams)
    }
}

fn skirmish_team_relations_from_team_ids(active_teams: &[bool], team_ids: &[u8]) -> TeamRelations {
    let mut relations = TeamRelations::default();
    relations.ensure_player_count(active_teams.len());
    for left_index in 0..active_teams.len() {
        if !active_teams[left_index] {
            continue;
        }
        let left = Team::Player(left_index);
        for right_index in 0..active_teams.len() {
            let right = Team::Player(right_index);
            if !active_teams[right_index] || left == right {
                continue;
            }
            relations.set_allied(
                left,
                right,
                team_ids.get(left_index).copied().unwrap_or(0)
                    == team_ids.get(right_index).copied().unwrap_or(0),
            );
        }
    }
    relations
}

fn skirmish_team_ids_from_relations(active_teams: &[bool], relations: &TeamRelations) -> Vec<u8> {
    let mut team_ids = (0..active_teams.len())
        .map(|index| (index % SKIRMISH_TEAM_OPTION_COUNT as usize) as u8)
        .collect::<Vec<_>>();
    let mut assigned = vec![false; active_teams.len()];
    let mut next_team_id = 0u8;
    for leader_index in 0..active_teams.len() {
        let leader = Team::Player(leader_index);
        if !active_teams[leader_index] || assigned[leader_index] {
            continue;
        }
        let team_id = next_team_id.min(SKIRMISH_TEAM_OPTION_COUNT.saturating_sub(1));
        next_team_id = next_team_id.saturating_add(1);
        for member_index in 0..active_teams.len() {
            let member = Team::Player(member_index);
            if active_teams[member_index] && relations.are_allied(leader, member) {
                team_ids[member_index] = team_id;
                assigned[member_index] = true;
            }
        }
    }
    team_ids
}

fn skirmish_has_opposing_active_teams(active_teams: &[bool], relations: &TeamRelations) -> bool {
    for left_index in 0..active_teams.len() {
        if !active_teams[left_index] {
            continue;
        }
        let left = Team::Player(left_index);
        for right_index in 0..active_teams.len() {
            let right = Team::Player(right_index);
            if active_teams[right_index] && relations.are_enemies(left, right) {
                return true;
            }
        }
    }
    false
}

fn allied_skirmish_ally(player_team: Team, active_team_count: usize) -> Option<Team> {
    default_skirmish_opponent(player_team, active_team_count)
}

fn allied_skirmish_enemy(player_team: Team, active_team_count: usize) -> Option<Team> {
    let ally = allied_skirmish_ally(player_team, active_team_count)?;
    player_teams(active_team_count).find(|team| *team != player_team && *team != ally)
}

fn skirmish_has_cross_team_alliance(relations: &TeamRelations, active_teams: &[bool]) -> bool {
    for left_index in 0..active_teams.len() {
        if !active_teams[left_index] {
            continue;
        }
        let left = Team::Player(left_index);
        for right_index in 0..active_teams.len() {
            let right = Team::Player(right_index);
            if left == right {
                continue;
            }
            if active_teams[right_index] && relations.are_allied(left, right) {
                return true;
            }
        }
    }
    false
}

fn default_skirmish_opponent(player_team: Team, active_team_count: usize) -> Option<Team> {
    player_teams(active_team_count).find(|team| *team != player_team)
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MainMenuAction {
    SelectMap(usize),
    SelectStartingResources(usize),
    SelectMatchMode(SkirmishMatchMode),
    SelectAiDifficulty(AiDifficulty),
    SelectLobbySlot(usize),
    CycleLobbySlotController(usize),
    CycleLobbySlotFaction(usize),
    CycleLobbySlotTeamId(usize),
    CycleLobbySlotColor(usize),
    StartMatch,
}

impl MainMenuAction {
    fn is_selected(self, selection: SkirmishMenuSelection) -> bool {
        match self {
            MainMenuAction::SelectMap(index) => index == selection.map_index,
            MainMenuAction::SelectStartingResources(index) => {
                index == selection.starting_resource_index
            }
            MainMenuAction::SelectMatchMode(mode) => mode == selection.match_mode,
            MainMenuAction::SelectAiDifficulty(difficulty) => difficulty == selection.ai_difficulty,
            MainMenuAction::SelectLobbySlot(slot) => selection.human_lobby_slot() == Some(slot),
            MainMenuAction::CycleLobbySlotController(_) => false,
            MainMenuAction::CycleLobbySlotFaction(_) => false,
            MainMenuAction::CycleLobbySlotTeamId(_) => false,
            MainMenuAction::CycleLobbySlotColor(_) => false,
            MainMenuAction::StartMatch => false,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MainMenuButton {
    action: MainMenuAction,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MainMenuButtonLabel {
    action: MainMenuAction,
}

#[derive(Component)]
struct MainMenuSummaryText;

#[derive(Component)]
struct MainMenuBriefStatusText;

#[derive(Component)]
struct MainMenuFactionInfoText;

#[derive(Component)]
struct MainMenuScrollArea;

#[derive(Component)]
struct MainMenuLobbySlotRow;

#[derive(Component)]
struct MainMenuLobbyListRoot {
    font: Handle<Font>,
}

#[derive(Component)]
struct SkirmishMapPreviewRoot;

#[derive(Component)]
struct SkirmishMapPreviewElement;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct SkirmishMapPreviewMarker {
    kind: SkirmishMapPreviewMarkerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkirmishMapPreviewMarkerKind {
    Spawn,
    Ore,
    Crystal,
    NeutralTech,
    SupplyCrate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkirmishMapPreviewRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MinimapContentRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct MapBounds {
    half_width: f32,
    half_depth: f32,
}

impl Default for MapBounds {
    fn default() -> Self {
        Self {
            half_width: MAP_HALF_EXTENT,
            half_depth: MAP_HALF_EXTENT,
        }
    }
}

impl MapBounds {
    fn from_size(size: (f32, f32)) -> Self {
        Self {
            half_width: size.0 * 0.5,
            half_depth: size.1 * 0.5,
        }
    }

    fn from_map(map: &SkirmishMapDef) -> Self {
        Self::from_size(map.size)
    }

    fn contains_ground_point(self, point: Vec3) -> bool {
        point.x >= -self.half_width
            && point.x <= self.half_width
            && point.z >= -self.half_depth
            && point.z <= self.half_depth
    }

    fn clamp_ground_point(self, point: Vec3, margin: f32) -> Vec3 {
        let half_width = (self.half_width - margin).max(0.0);
        let half_depth = (self.half_depth - margin).max(0.0);
        Vec3::new(
            point.x.clamp(-half_width, half_width),
            point.y,
            point.z.clamp(-half_depth, half_depth),
        )
    }

    fn minimap_local_position(self, world: Vec3) -> Vec2 {
        let rect = self.minimap_content_rect();
        let x = ((world.x + self.half_width) / (self.half_width * 2.0)).clamp(0.0, 1.0);
        let z = ((world.z + self.half_depth) / (self.half_depth * 2.0)).clamp(0.0, 1.0);
        Vec2::new(
            rect.left + x * rect.width,
            rect.top + (1.0 - z) * rect.height,
        )
    }

    fn minimap_world_position(self, local: Vec2) -> Vec3 {
        let rect = self.minimap_content_rect();
        let x = ((local.x - rect.left) / rect.width).clamp(0.0, 1.0);
        let z = (1.0 - ((local.y - rect.top) / rect.height)).clamp(0.0, 1.0);
        Vec3::new(
            x * self.half_width * 2.0 - self.half_width,
            0.0,
            z * self.half_depth * 2.0 - self.half_depth,
        )
    }

    fn minimap_world_position_checked(self, local: Vec2) -> Option<Vec3> {
        let rect = self.minimap_content_rect();
        (local.x >= rect.left
            && local.x <= rect.left + rect.width
            && local.y >= rect.top
            && local.y <= rect.top + rect.height)
            .then(|| self.minimap_world_position(local))
    }

    fn minimap_content_rect(self) -> MinimapContentRect {
        let map_width = (self.half_width * 2.0).max(1.0);
        let map_height = (self.half_depth * 2.0).max(1.0);
        let scale = (MINIMAP_SIZE_PX / map_width).min(MINIMAP_SIZE_PX / map_height);
        let width = map_width * scale;
        let height = map_height * scale;
        MinimapContentRect {
            left: (MINIMAP_SIZE_PX - width) * 0.5,
            top: (MINIMAP_SIZE_PX - height) * 0.5,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy)]
struct TeamStartup {
    structures: &'static [SpawnSpec],
    units: &'static [SpawnSpec],
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartupLoadoutMode {
    PlaytestExpanded,
    GodotSkirmish,
}

#[derive(Clone, Copy)]
struct TeamAiProfile {
    production_priority: &'static [&'static str],
    defense_priority: &'static [&'static str],
    defense_limits: &'static [(&'static str, usize)],
    expected_command_centers: usize,
    expected_workers: usize,
    expected_refineries: usize,
    expected_ore_harvesters: usize,
    expected_battlegroups: usize,
    expected_units_in_battlegroup: usize,
    active_offense_enabled: bool,
    capture_enabled: bool,
    saboteur_enabled: bool,
    support_powers_enabled: bool,
    production_interval: f32,
    attack_interval: f32,
    build_interval: f32,
    capture_interval: f32,
    saboteur_interval: f32,
    support_interval: f32,
    defense_limit_bonus: usize,
}

#[derive(Clone, Copy, Default)]
struct AiProductionCounts {
    workers: usize,
    ore_harvesters: usize,
    battle_units: usize,
}

#[derive(Clone, Copy)]
enum AiStructureBuildKind {
    Economy,
    Defense,
    Offense,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AiDifficulty {
    Beginner,
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    const ALL: [Self; 4] = [Self::Beginner, Self::Easy, Self::Normal, Self::Hard];

    fn label(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner AI",
            Self::Easy => "Easy AI",
            Self::Normal => "Normal AI",
            Self::Hard => "Hard AI",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
}

const HUMAN_STARTUP: TeamStartup = TeamStartup {
    structures: &[
        SpawnSpec {
            id: "CommandCenter",
            offset: (0.0, 0.0),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-3.0, 2.7),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-6.2, 0.3),
        },
        SpawnSpec {
            id: "Refinery",
            offset: (3.0, -2.6),
        },
        SpawnSpec {
            id: "VehicleFactory",
            offset: (3.6, 2.8),
        },
        SpawnSpec {
            id: "Barracks",
            offset: (-3.8, -2.8),
        },
        SpawnSpec {
            id: "AircraftFactory",
            offset: (0.0, 4.4),
        },
    ],
    units: &[
        SpawnSpec {
            id: "Worker",
            offset: (-1.8, -2.2),
        },
        SpawnSpec {
            id: "ScoutRover",
            offset: (0.7, -3.0),
        },
        SpawnSpec {
            id: "OreHarvester",
            offset: (2.3, -3.8),
        },
    ],
};

const DEMON_STARTUP: TeamStartup = TeamStartup {
    structures: &[
        SpawnSpec {
            id: "CommandCenter",
            offset: (0.0, 0.0),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-3.0, 2.7),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-6.2, 0.3),
        },
        SpawnSpec {
            id: "Refinery",
            offset: (3.0, -2.6),
        },
        SpawnSpec {
            id: "VehicleFactory",
            offset: (3.6, 2.8),
        },
        SpawnSpec {
            id: "Barracks",
            offset: (-3.8, -2.8),
        },
        SpawnSpec {
            id: "AircraftFactory",
            offset: (0.0, 4.4),
        },
        SpawnSpec {
            id: "RadarUplink",
            offset: (0.0, -0.7),
        },
    ],
    units: &[
        SpawnSpec {
            id: "Worker",
            offset: (-1.8, -2.2),
        },
        SpawnSpec {
            id: "FlameAssaultBuggy",
            offset: (0.7, -3.0),
        },
        SpawnSpec {
            id: "RocketInfantry",
            offset: (2.3, -3.8),
        },
    ],
};

const CHAOS_STARTUP: TeamStartup = TeamStartup {
    structures: &[
        SpawnSpec {
            id: "CommandCenter",
            offset: (0.0, 0.0),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-3.0, 2.7),
        },
        SpawnSpec {
            id: "PowerReactor",
            offset: (-6.2, 0.3),
        },
        SpawnSpec {
            id: "Refinery",
            offset: (3.0, -2.6),
        },
        SpawnSpec {
            id: "VehicleFactory",
            offset: (3.6, 2.8),
        },
        SpawnSpec {
            id: "Barracks",
            offset: (-3.8, -2.8),
        },
        SpawnSpec {
            id: "AircraftFactory",
            offset: (0.0, 4.4),
        },
        SpawnSpec {
            id: "RadarUplink",
            offset: (0.0, -0.7),
        },
        SpawnSpec {
            id: "RoboticsBay",
            offset: (0.6, -7.0),
        },
    ],
    units: &[
        SpawnSpec {
            id: "Worker",
            offset: (-1.8, -2.2),
        },
        SpawnSpec {
            id: "FieldMedic",
            offset: (0.7, -3.0),
        },
        SpawnSpec {
            id: "ShieldTrooper",
            offset: (2.3, -3.8),
        },
    ],
};

const GODOT_SKIRMISH_STARTUP: TeamStartup = TeamStartup {
    structures: &[SpawnSpec {
        id: "CommandCenter",
        offset: (0.0, 0.0),
    }],
    units: &[
        SpawnSpec {
            id: "Drone",
            offset: (-2.0, -2.0),
        },
        SpawnSpec {
            id: "Worker",
            offset: (-3.0, 3.0),
        },
        SpawnSpec {
            id: "Worker",
            offset: (3.0, 3.0),
        },
    ],
};

const SKIRMISH_MAP_ORE_AMOUNT: i32 = 240;
const SKIRMISH_MAP_CRYSTAL_AMOUNT: i32 = 140;

macro_rules! map_ore {
    ($x:expr, $z:expr) => {
        ResourceSpec {
            kind: ResourceKind::Ore,
            amount: SKIRMISH_MAP_ORE_AMOUNT,
            position: ($x, $z),
        }
    };
}

macro_rules! map_crystal {
    ($x:expr, $z:expr) => {
        ResourceSpec {
            kind: ResourceKind::Crystal,
            amount: SKIRMISH_MAP_CRYSTAL_AMOUNT,
            position: ($x, $z),
        }
    };
}

const PLAIN_AND_SIMPLE_SPAWNS: &[(f32, f32)] =
    &[(10.0, 7.0), (40.0, 7.0), (40.0, 43.0), (10.0, 43.0)];

const PLAIN_AND_SIMPLE_RESOURCES: &[ResourceSpec] = &[
    map_ore!(7.52981, 15.5708),
    map_ore!(9.4963, 15.3833),
    map_ore!(9.07351, 17.1366),
    map_ore!(40.5191, 14.7519),
    map_ore!(41.412, 13.361),
    map_ore!(42.5933, 14.65),
    map_crystal!(14.2571, 15.2579),
    map_crystal!(36.3143, 14.8547),
    map_ore!(40.685, 34.3027),
    map_ore!(41.533, 35.8383),
    map_ore!(42.7592, 34.2008),
    map_crystal!(36.4802, 34.4055),
    map_ore!(9.6306, 33.1921),
    map_ore!(8.46063, 34.3596),
    map_ore!(7.58274, 32.8476),
    map_crystal!(13.7599, 33.992),
];

const FOUR_CORNERS_SPAWNS: &[(f32, f32)] =
    &[(10.0, 10.0), (62.0, 10.0), (62.0, 62.0), (10.0, 62.0)];

const FOUR_CORNERS_RESOURCES: &[ResourceSpec] = &[
    map_ore!(15.0, 16.0),
    map_ore!(18.0, 15.0),
    map_ore!(17.0, 18.0),
    map_crystal!(22.0, 20.0),
    map_ore!(57.0, 16.0),
    map_ore!(54.0, 15.0),
    map_ore!(55.0, 18.0),
    map_crystal!(50.0, 20.0),
    map_ore!(57.0, 56.0),
    map_ore!(54.0, 57.0),
    map_ore!(55.0, 54.0),
    map_crystal!(50.0, 52.0),
    map_ore!(15.0, 56.0),
    map_ore!(18.0, 57.0),
    map_ore!(17.0, 54.0),
    map_crystal!(22.0, 52.0),
    map_ore!(32.0, 36.0),
    map_ore!(40.0, 36.0),
    map_crystal!(36.0, 32.0),
    map_crystal!(36.0, 40.0),
];

const FOUR_CORNERS_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (24.0, 36.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (48.0, 36.0),
    },
    NeutralTechSpec {
        name: "NorthTechAirport",
        id: "TechAirport",
        position: (36.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthTechAirport",
        id: "TechAirport",
        position: (36.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechHospital",
        id: "TechHospital",
        position: (48.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechHospital",
        id: "TechHospital",
        position: (24.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechRepairDepot",
        id: "TechRepairDepot",
        position: (24.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechRepairDepot",
        id: "TechRepairDepot",
        position: (48.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthCenterTechBunker",
        id: "TechBunker",
        position: (36.0, 28.0),
    },
    NeutralTechSpec {
        name: "SouthCenterTechBunker",
        id: "TechBunker",
        position: (36.0, 44.0),
    },
];

const FOUR_CORNERS_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "NorthWestResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (27.0, 27.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthEastRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (45.0, 27.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthEastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (45.0, 45.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthWestResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (27.0, 45.0),
    },
];

const TECH_DIVIDE_SPAWNS: &[(f32, f32)] = &[
    (10.0, 16.0),
    (10.0, 42.0),
    (10.0, 68.0),
    (74.0, 16.0),
    (74.0, 42.0),
    (74.0, 68.0),
];

const TECH_DIVIDE_RESOURCES: &[ResourceSpec] = &[
    map_ore!(15.0, 12.0),
    map_ore!(18.0, 15.0),
    map_ore!(14.0, 18.0),
    map_crystal!(21.0, 20.0),
    map_ore!(14.0, 38.0),
    map_ore!(18.0, 42.0),
    map_ore!(14.0, 46.0),
    map_crystal!(22.0, 42.0),
    map_ore!(14.0, 66.0),
    map_ore!(18.0, 69.0),
    map_ore!(15.0, 72.0),
    map_crystal!(21.0, 64.0),
    map_ore!(70.0, 12.0),
    map_ore!(66.0, 15.0),
    map_ore!(69.0, 18.0),
    map_crystal!(63.0, 20.0),
    map_ore!(70.0, 38.0),
    map_ore!(66.0, 42.0),
    map_ore!(70.0, 46.0),
    map_crystal!(62.0, 42.0),
    map_ore!(69.0, 66.0),
    map_ore!(66.0, 69.0),
    map_ore!(70.0, 72.0),
    map_crystal!(63.0, 64.0),
    map_ore!(38.0, 42.0),
    map_ore!(46.0, 42.0),
    map_crystal!(42.0, 38.0),
    map_crystal!(42.0, 46.0),
];

const TECH_DIVIDE_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "NorthOilDerrick",
        id: "TechOilDerrick",
        position: (42.0, 16.0),
    },
    NeutralTechSpec {
        name: "SouthOilDerrick",
        id: "TechOilDerrick",
        position: (42.0, 68.0),
    },
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (30.0, 42.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (54.0, 42.0),
    },
    NeutralTechSpec {
        name: "NorthTechAirport",
        id: "TechAirport",
        position: (42.0, 28.0),
    },
    NeutralTechSpec {
        name: "SouthTechAirport",
        id: "TechAirport",
        position: (42.0, 56.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechHospital",
        id: "TechHospital",
        position: (24.0, 30.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechHospital",
        id: "TechHospital",
        position: (60.0, 54.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechRepairDepot",
        id: "TechRepairDepot",
        position: (24.0, 54.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechRepairDepot",
        id: "TechRepairDepot",
        position: (60.0, 30.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechBunker",
        id: "TechBunker",
        position: (36.0, 36.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechBunker",
        id: "TechBunker",
        position: (48.0, 36.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechBunker",
        id: "TechBunker",
        position: (48.0, 48.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechBunker",
        id: "TechBunker",
        position: (36.0, 48.0),
    },
];

const TECH_DIVIDE_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "NorthResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (42.0, 22.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (42.0, 62.0),
    },
    NamedSupplyCrateSpec {
        name: "WestVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (26.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "EastResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (58.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthWestRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (30.0, 30.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthEastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (54.0, 54.0),
    },
];

const BIG_ARENA_SPAWNS: &[(f32, f32)] = &[
    (10.0, 30.0),
    (35.0, 10.0),
    (65.0, 10.0),
    (90.0, 30.0),
    (90.0, 70.0),
    (65.0, 90.0),
    (35.0, 90.0),
    (10.0, 70.0),
];

const BIG_ARENA_RESOURCES: &[ResourceSpec] = &[
    map_ore!(13.8154, 23.5681),
    map_ore!(12.3442, 21.3867),
    map_ore!(9.51845, 22.2094),
    map_crystal!(3.60055, 23.3712),
    map_ore!(27.4944, 3.46134),
    map_ore!(25.3909, 5.04189),
    map_ore!(26.3566, 7.82198),
    map_crystal!(27.8187, 13.6729),
    map_ore!(73.3526, 12.7052),
    map_ore!(75.2773, 10.9112),
    map_ore!(74.0231, 8.24873),
    map_crystal!(71.9507, 2.58511),
    map_ore!(94.8869, 25.3542),
    map_ore!(93.3679, 23.2059),
    map_ore!(90.561, 24.0908),
    map_crystal!(84.6702, 25.3831),
    map_ore!(84.7421, 73.7477),
    map_ore!(86.059, 76.0256),
    map_ore!(88.9349, 75.4003),
    map_crystal!(94.9189, 74.6505),
    map_ore!(71.2856, 94.4847),
    map_ore!(73.2918, 92.7823),
    map_ore!(72.163, 90.0643),
    map_crystal!(70.3568, 84.3103),
    map_ore!(29.1414, 85.0513),
    map_ore!(27.0299, 86.6211),
    map_ore!(27.9815, 89.4061),
    map_crystal!(29.4138, 95.2644),
    map_ore!(4.39248, 74.789),
    map_ore!(5.9203, 76.9311),
    map_ore!(8.72352, 76.0348),
    map_crystal!(14.609, 74.7184),
];

const BIG_ARENA_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "NorthOilDerrick",
        id: "TechOilDerrick",
        position: (50.0, 34.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (66.0, 50.0),
    },
    NeutralTechSpec {
        name: "SouthOilDerrick",
        id: "TechOilDerrick",
        position: (50.0, 66.0),
    },
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (34.0, 50.0),
    },
    NeutralTechSpec {
        name: "WestTechAirport",
        id: "TechAirport",
        position: (38.0, 44.0),
    },
    NeutralTechSpec {
        name: "EastTechAirport",
        id: "TechAirport",
        position: (62.0, 56.0),
    },
    NeutralTechSpec {
        name: "NorthTechHospital",
        id: "TechHospital",
        position: (56.0, 38.0),
    },
    NeutralTechSpec {
        name: "SouthTechHospital",
        id: "TechHospital",
        position: (44.0, 62.0),
    },
    NeutralTechSpec {
        name: "WestTechRepairDepot",
        id: "TechRepairDepot",
        position: (38.0, 56.0),
    },
    NeutralTechSpec {
        name: "EastTechRepairDepot",
        id: "TechRepairDepot",
        position: (62.0, 44.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechBunker",
        id: "TechBunker",
        position: (44.0, 38.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechBunker",
        id: "TechBunker",
        position: (62.0, 38.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechBunker",
        id: "TechBunker",
        position: (56.0, 62.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechBunker",
        id: "TechBunker",
        position: (38.0, 62.0),
    },
];

const BIG_ARENA_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "CenterResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (50.0, 50.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (50.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "EastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (58.0, 50.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (50.0, 58.0),
    },
    NamedSupplyCrateSpec {
        name: "WestRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (42.0, 50.0),
    },
];

const EMPTY_NEUTRAL_TECH: &[NeutralTechSpec] = &[];
const EMPTY_NAMED_CRATES: &[NamedSupplyCrateSpec] = &[];

const SKIRMISH_MAPS: &[SkirmishMapDef] = &[
    SkirmishMapDef {
        id: "plain_and_simple",
        godot_path: "res://source/match/maps/PlainAndSimple.tscn",
        name: "Plain & Simple",
        name_key: "MAP_NAME_PLAIN_AND_SIMPLE",
        players: 4,
        size: (50.0, 50.0),
        spawn_points: PLAIN_AND_SIMPLE_SPAWNS,
        resources: PLAIN_AND_SIMPLE_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
    },
    SkirmishMapDef {
        id: "four_corners",
        godot_path: "res://source/match/maps/FourCorners.tscn",
        name: "Four Corners",
        name_key: "MAP_NAME_FOUR_CORNERS",
        players: 4,
        size: (72.0, 72.0),
        spawn_points: FOUR_CORNERS_SPAWNS,
        resources: FOUR_CORNERS_RESOURCES,
        neutral_tech: FOUR_CORNERS_NEUTRAL_TECH,
        supply_crates: FOUR_CORNERS_CRATES,
    },
    SkirmishMapDef {
        id: "tech_divide",
        godot_path: "res://source/match/maps/TechDivide.tscn",
        name: "Tech Divide",
        name_key: "MAP_NAME_TECH_DIVIDE",
        players: 6,
        size: (84.0, 84.0),
        spawn_points: TECH_DIVIDE_SPAWNS,
        resources: TECH_DIVIDE_RESOURCES,
        neutral_tech: TECH_DIVIDE_NEUTRAL_TECH,
        supply_crates: TECH_DIVIDE_CRATES,
    },
    SkirmishMapDef {
        id: "big_arena",
        godot_path: "res://source/match/maps/BigArena.tscn",
        name: "Big Arena",
        name_key: "MAP_NAME_BIG_ARENA",
        players: 8,
        size: (100.0, 100.0),
        spawn_points: BIG_ARENA_SPAWNS,
        resources: BIG_ARENA_RESOURCES,
        neutral_tech: BIG_ARENA_NEUTRAL_TECH,
        supply_crates: BIG_ARENA_CRATES,
    },
];

#[cfg(test)]
fn skirmish_maps() -> &'static [SkirmishMapDef] {
    SKIRMISH_MAPS
}

fn skirmish_map_by_path(path: &str) -> Option<&'static SkirmishMapDef> {
    SKIRMISH_MAPS.iter().find(|map| map.godot_path == path)
}

fn map_local_to_world(map: &SkirmishMapDef, point: (f32, f32)) -> Vec3 {
    Vec3::new(point.0 - map.size.0 * 0.5, 0.0, point.1 - map.size.1 * 0.5)
}

fn team_start_position(map: &SkirmishMapDef, team: Team) -> Vec3 {
    let spawn_index = team.economy_index().unwrap_or(0);
    team_start_position_for_spawn_slot(map, spawn_index)
}

fn team_start_position_for_spawn_slot(map: &SkirmishMapDef, spawn_index: usize) -> Vec3 {
    let spawn_point = map
        .spawn_points
        .get(spawn_index)
        .copied()
        .or_else(|| map.spawn_points.first().copied())
        .unwrap_or((map.size.0 * 0.5, map.size.1 * 0.5));
    map_local_to_world(map, spawn_point)
}

#[allow(dead_code)]
fn team_start_camera_focus(map: &SkirmishMapDef, team: Team, loadout: StartupLoadoutMode) -> Vec3 {
    team_start_camera_focus_for_faction(map, team, SkirmishFaction::from_team(team), loadout)
}

fn team_start_camera_focus_for_faction(
    map: &SkirmishMapDef,
    team: Team,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    let base = team_start_position(map, team);
    team_start_camera_focus_from_base(base, faction, loadout)
}

fn team_start_camera_focus_for_spawn_slot(
    map: &SkirmishMapDef,
    spawn_index: usize,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    let base = team_start_position_for_spawn_slot(map, spawn_index);
    team_start_camera_focus_from_base(base, faction, loadout)
}

fn team_start_camera_focus_from_base(
    base: Vec3,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    base + startup_camera_focus_offset(faction_startup_for_loadout(faction, loadout))
}

fn startup_camera_focus_offset(startup: &TeamStartup) -> Vec3 {
    startup_spawn_offset(startup.units, CAMERA_START_PRIMARY_UNITS)
        .or_else(|| startup_spawn_offset(startup.structures, CAMERA_START_PRIMARY_STRUCTURES))
        .or_else(|| startup_aabb_pivot_offset(startup.units))
        .unwrap_or(Vec3::ZERO)
}

fn startup_spawn_offset(spawns: &[SpawnSpec], priority_ids: &[&str]) -> Option<Vec3> {
    priority_ids.iter().find_map(|priority_id| {
        spawns
            .iter()
            .find(|spawn| spawn.id == *priority_id)
            .map(spawn_offset_to_ground_vec)
    })
}

fn startup_aabb_pivot_offset(spawns: &[SpawnSpec]) -> Option<Vec3> {
    if spawns.is_empty() {
        return None;
    }
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_z, mut max_z) = (f32::INFINITY, f32::NEG_INFINITY);
    for spawn in spawns {
        min_x = min_x.min(spawn.offset.0);
        max_x = max_x.max(spawn.offset.0);
        min_z = min_z.min(spawn.offset.1);
        max_z = max_z.max(spawn.offset.1);
    }
    Some(Vec3::new((min_x + max_x) * 0.5, 0.0, (min_z + max_z) * 0.5))
}

fn spawn_offset_to_ground_vec(spawn: &SpawnSpec) -> Vec3 {
    Vec3::new(spawn.offset.0, 0.0, spawn.offset.1)
}

const HUMAN_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "ScoutRover",
    "RocketInfantry",
    "InterceptorVTOL",
    "OreHarvester",
    "MirageScoutTank",
    "FieldMedic",
    "BomberVTOL",
    "FlameAssaultBuggy",
    "ShieldTrooper",
    "RocketGunship",
    "DroneMineLayer",
    "FlakRocketTeam",
    "HeavyBombardmentAirship",
    "TeslaCrawlerMk2",
    "FlakRocketTeamMk2",
    "SiegeAirship",
    "RocketTrooperRobot",
    "HeavyMachinegunTrooper",
    "ModularMissileCarrier",
    "ShockTrooper",
    "JammerVehicle",
    "GrenadierTrooper",
    "AntiAirWalker",
    "MortarTeam",
    "FlakHoverTank",
    "CryoSprayer",
    "MobileRepairCrawler",
    "SniperScout",
    "MobileShieldProjector",
    "RailSniperTeam",
    "LongbowMissileCrawler",
    "PhaseSaboteur",
    "SiegeArtilleryVehicle",
    "PulseRifleCommando",
    "SiegeDrillTank",
    "TacticalOfficer",
    "LanceBeamTank",
    "RailgunTank",
    "HammerSiegeTank",
    "HeavySiegeWalker",
    "RailArtilleryWalker",
];

const DEMON_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "FlameAssaultBuggy",
    "RocketInfantry",
    "BomberVTOL",
    "OreHarvester",
    "ScoutRover",
    "HeavyMachinegunTrooper",
    "HeavyBombardmentAirship",
    "FlakHoverTank",
    "ShockTrooper",
    "SiegeAirship",
    "SiegeArtilleryVehicle",
    "GrenadierTrooper",
    "SiegeDrillTank",
    "MortarTeam",
    "HammerSiegeTank",
    "PulseRifleCommando",
    "HeavySiegeWalker",
];

const CHAOS_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "MirageScoutTank",
    "TeslaCrawlerMk2",
    "ShieldTrooper",
    "InterceptorVTOL",
    "ScoutRover",
    "FieldMedic",
    "Drone",
    "OreHarvester",
    "DroneMineLayer",
    "FlakRocketTeam",
    "RocketGunship",
    "FlakRocketTeamMk2",
    "HeavyBombardmentAirship",
    "RocketTrooperRobot",
    "CryoSprayer",
    "SiegeAirship",
    "ModularMissileCarrier",
    "SniperScout",
    "JammerVehicle",
    "RailSniperTeam",
    "AntiAirWalker",
    "PhaseSaboteur",
    "MobileRepairCrawler",
    "TacticalOfficer",
    "MobileShieldProjector",
    "LongbowMissileCrawler",
    "LanceBeamTank",
    "RailgunTank",
    "RailArtilleryWalker",
];

const AI_OFFENSE_STRUCTURE_PRIORITY: &[&str] = &["VehicleFactory", "Barracks", "AircraftFactory"];

const HUMAN_AI_DEFENSE_PRIORITY: &[&str] = &[
    "AntiGroundTurret",
    "AntiAirTurret",
    "TeslaFenceSegment",
    "ArcCoilDefenseTower",
    "LanceBeamDefenseTower",
    "PrismDefenseObelisk",
    "RailCannonBunker",
];
const HUMAN_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("AntiGroundTurret", 3),
    ("AntiAirTurret", 2),
    ("TeslaFenceSegment", 1),
    ("ArcCoilDefenseTower", 1),
    ("LanceBeamDefenseTower", 1),
    ("PrismDefenseObelisk", 1),
    ("RailCannonBunker", 1),
];
const DEMON_AI_DEFENSE_PRIORITY: &[&str] = &[
    "AntiAirTurret",
    "AntiGroundTurret",
    "ArcCoilDefenseTower",
    "LanceBeamDefenseTower",
];
const DEMON_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("AntiAirTurret", 2),
    ("AntiGroundTurret", 2),
    ("ArcCoilDefenseTower", 1),
    ("LanceBeamDefenseTower", 1),
];
const CHAOS_AI_DEFENSE_PRIORITY: &[&str] = &[
    "TeslaFenceSegment",
    "ArcCoilDefenseTower",
    "PrismDefenseObelisk",
    "RailCannonBunker",
];
const CHAOS_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("TeslaFenceSegment", 1),
    ("ArcCoilDefenseTower", 1),
    ("PrismDefenseObelisk", 1),
    ("RailCannonBunker", 1),
];

const HUMAN_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: HUMAN_AI_PRODUCTION_PRIORITY,
    defense_priority: HUMAN_AI_DEFENSE_PRIORITY,
    defense_limits: HUMAN_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_ore_harvesters: 2,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
};

const DEMON_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: DEMON_AI_PRODUCTION_PRIORITY,
    defense_priority: DEMON_AI_DEFENSE_PRIORITY,
    defense_limits: DEMON_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_ore_harvesters: 2,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
};

const CHAOS_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: CHAOS_AI_PRODUCTION_PRIORITY,
    defense_priority: CHAOS_AI_DEFENSE_PRIORITY,
    defense_limits: CHAOS_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_ore_harvesters: 2,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
};

const BEGINNER_AI_PRODUCTION_PRIORITY: &[&str] = &[];
const BEGINNER_AI_DEFENSE_PRIORITY: &[&str] = &[];
const BEGINNER_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameAppMode {
    Interactive,
    Headless,
}

#[derive(Default)]
pub struct SharedMatchScenePlugin;

impl Plugin for SharedMatchScenePlugin {
    fn build(&self, app: &mut App) {
        add_shared_match_scene(app);
    }
}

pub fn run_game_app() {
    build_game_app(GameAppMode::Interactive).run();
}

fn add_shared_match_resources(app: &mut App) -> &mut App {
    app.init_state::<AppScreen>()
        .init_resource::<Economies>()
        .init_resource::<TeamRelations>()
        .init_resource::<BuildQueue>()
        .init_resource::<NextSpawnId>()
        .init_resource::<AiDirector>()
        .init_resource::<AiDifficultySettings>()
        .init_resource::<ActiveTeams>()
        .init_resource::<PlayerFactions>()
        .init_resource::<PlayerColorSlots>()
        .init_resource::<VisiblePlayer>()
        .init_resource::<SelectedSkirmishMap>()
        .init_resource::<MatchSetupSettings>()
        .init_resource::<SkirmishMenuSelection>()
        .init_resource::<RandomMapCursor>()
        .init_resource::<MapBounds>()
        .init_resource::<CommandMode>()
        .init_resource::<StructurePlacementFeedback>()
        .init_resource::<MatchMenuState>()
        .init_resource::<MatchSpeed>()
        .init_resource::<MatchBriefingState>()
        .init_resource::<SelectionDragState>()
        .init_resource::<UnitGroups>()
        .init_resource::<CameraBookmarks>()
        .init_resource::<CameraMouseRotation>()
        .init_resource::<DoubleClickState>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<LatestBattleEvent>()
        .insert_resource(MatchFlow { active: false })
        .init_resource::<MatchState>()
        .init_resource::<SupportCooldowns>()
        .init_resource::<KillCredits>()
        .init_resource::<BattleLog>()
        .init_resource::<AudioFeedback>()
        .init_resource::<ObjectiveTrackerState>()
        .insert_resource(RtsCamera::default())
}

fn add_main_menu_scene(app: &mut App) -> &mut App {
    app.add_systems(
        OnEnter(AppScreen::MainMenu),
        (
            restore_main_menu_selection_from_match_setup,
            setup_main_menu,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            main_menu_scroll,
            main_menu_buttons,
            update_main_menu_summary,
            update_skirmish_map_preview,
            update_main_menu_lobby_slots,
        )
            .chain()
            .run_if(in_state(AppScreen::MainMenu)),
    )
}

/// Registers the live match scene shared by `cargo run`, capture, and gameplay tests.
pub fn add_shared_match_scene(app: &mut App) -> &mut App {
    add_shared_match_resources(app)
        .add_systems(
            OnEnter(AppScreen::InMatch),
            (
                apply_match_setup_settings,
                begin_match_from_setup,
                setup_support_cooldowns,
                setup,
            )
                .chain(),
        )
        .add_systems(OnEnter(AppScreen::RestartingMatch), advance_match_restart)
        .add_systems(
            OnExit(AppScreen::InMatch),
            (
                stop_match_flow_on_exit,
                reset_match_speed_on_exit,
                cleanup_match_scoped_entities,
            )
                .chain(),
        );
    add_runtime_systems(app);
    app
}

/// Registers the real game scene flow used by `cargo run`: setup menu plus shared match runtime.
pub fn add_game_scenes(app: &mut App) -> &mut App {
    app.add_plugins(SharedMatchScenePlugin);
    add_main_menu_scene(app);
    app
}

/// Advances an app with [`add_shared_match_scene`] registered into the live match scene.
pub fn enter_shared_match_scene(app: &mut App) {
    start_shared_match_scene_with_current_setup(app);
}

/// Starts the shared live match scene with the app's current [`MatchSetupSettings`].
pub fn start_shared_match_scene_with_current_setup(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
}

fn start_shared_match_scene_with_settings(app: &mut App, settings: MatchSetupSettings) {
    app.insert_resource(settings);
    start_shared_match_scene_with_current_setup(app);
}

fn add_headless_game_plugins(app: &mut App) -> &mut App {
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        },
        bevy::gizmos::GizmoPlugin,
    ))
    .add_message::<MouseMotion>()
    .add_message::<MouseWheel>()
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
    .init_asset::<WorldAsset>()
    .init_asset::<Font>();
    #[cfg(feature = "audio")]
    app.init_asset::<bevy::audio::AudioSource>();
    app
}

pub fn build_game_app(mode: GameAppMode) -> App {
    let mut app = App::new();
    match mode {
        GameAppMode::Interactive => {
            app.add_plugins(
                DefaultPlugins
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Bevy Open RTS".to_string(),
                            resolution: WindowResolution::new(1280, 720),
                            canvas: Some("#bevy-canvas".to_string()),
                            fit_canvas_to_parent: true,
                            prevent_default_event_handling: true,
                            ..default()
                        }),
                        exit_condition: bevy::window::ExitCondition::OnPrimaryClosed,
                        ..default()
                    })
                    .set(AssetPlugin {
                        meta_check: AssetMetaCheck::Never,
                        ..default()
                    })
                    .set(bevy::log::LogPlugin {
                        filter: format!(
                            "{},bevy_ecs::world::command_queue=error,icu_provider=error,icu_segmenter=error,parley=error",
                            bevy::log::DEFAULT_FILTER
                        ),
                        ..default()
                    }),
            );
        }
        GameAppMode::Headless => {
            add_headless_game_plugins(&mut app);
        }
    };
    app.insert_resource(ClearColor(Color::srgb(0.028, 0.034, 0.045)))
        .add_plugins((
            JsonAssetPlugin::<RtsDataManifest>::new(&["rts.json"]),
            RonAssetPlugin::<RtsDataManifest>::new(&["rts.ron"]),
        ))
        .insert_resource(RenderErrorHandler(handle_render_error));
    add_game_scenes(&mut app);
    app
}

pub fn build_capture_match_app() -> App {
    build_capture_match_app_with_settings(MatchSetupSettings::default())
}

fn build_capture_match_app_with_settings(settings: MatchSetupSettings) -> App {
    let mut app = App::new();
    add_headless_game_plugins(&mut app);
    app.add_plugins(SharedMatchScenePlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    start_shared_match_scene_with_settings(&mut app, settings);
    app
}

#[derive(Clone, Debug, Default)]
pub struct CaptureMatchSnapshot {
    pub entities: Vec<CaptureEntitySnapshot>,
    pub phase: CaptureMatchPhase,
    pub elapsed_seconds: f32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
    pub enemy_units_destroyed: u32,
    pub enemy_structures_destroyed: u32,
    pub players: Vec<CaptureTeamStats>,
}

#[derive(Clone, Debug)]
pub struct CaptureEntitySnapshot {
    pub x: f32,
    pub z: f32,
    pub team: CaptureTeam,
    pub kind: CaptureEntityKind,
    pub visible: bool,
    pub health_ratio: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureTeam {
    Player(usize),
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureEntityKind {
    Unit,
    Structure,
    Resource,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaptureMatchPhase {
    #[default]
    Running,
    HumanDefeat,
    HumanVictory,
    MatchFinished,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureTeamStats {
    pub units: u32,
    pub structures: u32,
    pub ore: i32,
    pub crystal: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaptureProofFaction {
    #[default]
    Human,
    Demon,
    Chaos,
}

impl CaptureProofFaction {
    pub const ALL: [Self; 3] = [Self::Human, Self::Demon, Self::Chaos];

    pub fn key(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Demon => "demon",
            Self::Chaos => "chaos",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Human => "人族",
            Self::Demon => "魔族",
            Self::Chaos => "混沌族",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "human" | "alliance" | "ren" | "人族" => Some(Self::Human),
            "demon" | "mo" | "魔族" => Some(Self::Demon),
            "chaos" | "hun" | "混沌族" => Some(Self::Chaos),
            _ => None,
        }
    }

    fn skirmish_faction(self) -> SkirmishFaction {
        match self {
            Self::Human => SkirmishFaction::Alliance,
            Self::Demon => SkirmishFaction::Demon,
            Self::Chaos => SkirmishFaction::Chaos,
        }
    }

    fn proof_vehicle(self) -> &'static str {
        match self {
            Self::Human | Self::Demon => "Tank",
            Self::Chaos => "MirageScoutTank",
        }
    }

    fn proof_vehicle_target(self) -> usize {
        match self {
            Self::Human | Self::Demon => 12,
            Self::Chaos => 12,
        }
    }

    fn mouse_victory_vehicle_target(self) -> usize {
        match self {
            Self::Human | Self::Demon => 5,
            Self::Chaos => 8,
        }
    }

    fn allied_victory_vehicle_target(self) -> usize {
        match self {
            Self::Human | Self::Demon => self.mouse_victory_vehicle_target() + 3,
            Self::Chaos => self.mouse_victory_vehicle_target(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureMatchProof {
    pub faction: CaptureProofFaction,
    pub product_id: &'static str,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub elapsed_seconds: u32,
    pub produced_units: u32,
    pub player_units: u32,
    pub enemy_units_destroyed: u32,
    pub enemy_structures_destroyed: u32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CaptureMatchProof {
    pub fn succeeded(self) -> bool {
        self.phase == CaptureMatchPhase::HumanVictory && self.remaining_teams <= 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureHarvestProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub harvest_ordered: bool,
    pub ore_before: i32,
    pub ore_after: i32,
    pub resource_before: i32,
    pub resource_after: i32,
    pub product_id: &'static str,
    pub produced_units: u32,
}

impl CaptureHarvestProof {
    pub fn succeeded(&self) -> bool {
        self.harvest_ordered
            && self.ore_after > self.ore_before
            && self.resource_after < self.resource_before
            && self.produced_units > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureDualHarvestProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub ore_before: i32,
    pub ore_after: i32,
    pub crystal_before: i32,
    pub crystal_after: i32,
    pub ore_harvest_ordered: bool,
    pub crystal_harvest_ordered: bool,
}

impl CaptureDualHarvestProof {
    pub fn succeeded(&self) -> bool {
        self.ore_harvest_ordered
            && self.crystal_harvest_ordered
            && self.ore_after > self.ore_before
            && self.crystal_after > self.crystal_before
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureSupplyCrateProof {
    pub faction: CaptureProofFaction,
    pub map_id: &'static str,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub collector_id: &'static str,
    pub collector_selected: bool,
    pub move_ordered: bool,
    pub crate_consumed: bool,
    pub ore_before: i32,
    pub ore_after: i32,
    pub crystal_before: i32,
    pub crystal_after: i32,
}

impl CaptureSupplyCrateProof {
    pub fn succeeded(&self) -> bool {
        self.collector_selected
            && self.move_ordered
            && self.crate_consumed
            && self.ore_after > self.ore_before
            && self.crystal_after > self.crystal_before
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureTechOilProof {
    pub faction: CaptureProofFaction,
    pub map_id: &'static str,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub radar_constructed: bool,
    pub robotics_constructed: bool,
    pub engineer_produced: bool,
    pub engineer_selected: bool,
    pub capture_ordered: bool,
    pub captured: bool,
    pub engineer_consumed: bool,
    pub ore_before: i32,
    pub ore_after: i32,
    pub capture_bonus_ore: i32,
}

impl CaptureTechOilProof {
    pub fn succeeded(&self) -> bool {
        self.radar_constructed
            && self.robotics_constructed
            && self.engineer_produced
            && self.engineer_selected
            && self.capture_ordered
            && self.captured
            && self.engineer_consumed
            && self.ore_after >= self.ore_before + self.capture_bonus_ore
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureAiPressureProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub ai_team: CaptureTeam,
    pub ai_units_before: u32,
    pub ai_units_peak: u32,
    pub ai_attack_orders: u32,
    pub player_health_before: f32,
    pub player_health_after: f32,
}

impl CaptureAiPressureProof {
    pub fn succeeded(&self) -> bool {
        let damage_or_defeat = self.player_health_after < self.player_health_before
            || self.phase == CaptureMatchPhase::HumanDefeat;
        self.ai_units_peak > self.ai_units_before
            && (self.ai_attack_orders > 0 || damage_or_defeat)
            && damage_or_defeat
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureAiVsAiProof {
    pub focus_faction: CaptureProofFaction,
    pub map_id: &'static str,
    pub match_mode_id: &'static str,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub focus_team: CaptureTeam,
    pub opponent_team: CaptureTeam,
    pub focus_units_before: u32,
    pub focus_units_peak: u32,
    pub opponent_units_before: u32,
    pub opponent_units_peak: u32,
    pub focus_attack_orders: u32,
    pub opponent_attack_orders: u32,
    pub focus_health_before: f32,
    pub focus_health_after: f32,
    pub opponent_health_before: f32,
    pub opponent_health_after: f32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CaptureAiVsAiProof {
    pub fn succeeded(&self) -> bool {
        let both_ai_sides_produced = self.focus_units_peak > self.focus_units_before
            && self.opponent_units_peak > self.opponent_units_before;
        let any_attack_orders = self.focus_attack_orders + self.opponent_attack_orders > 0;
        let any_side_damaged = self.focus_health_after < self.focus_health_before
            || self.opponent_health_after < self.opponent_health_before
            || (self.phase == CaptureMatchPhase::MatchFinished && self.remaining_teams <= 1);
        self.match_mode_id == SkirmishMatchMode::AiVsAi.id()
            && matches!(
                self.phase,
                CaptureMatchPhase::Running | CaptureMatchPhase::MatchFinished
            )
            && both_ai_sides_produced
            && any_attack_orders
            && any_side_damaged
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureBuildProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub structure_id: &'static str,
    pub placement_started: bool,
    pub placed: bool,
    pub construct_ordered: bool,
    pub constructed: bool,
    pub product_id: &'static str,
    pub produced_units: u32,
}

impl CaptureBuildProof {
    pub fn succeeded(&self) -> bool {
        self.placement_started
            && self.placed
            && self.construct_ordered
            && self.constructed
            && self.produced_units > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureVictoryProof {
    pub faction: CaptureProofFaction,
    pub map_id: &'static str,
    pub match_mode_id: &'static str,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub product_id: &'static str,
    pub target_units: u32,
    pub produced_units: u32,
    pub attack_orders: u32,
    pub player_units: u32,
    pub enemy_units_destroyed: u32,
    pub enemy_structures_destroyed: u32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CaptureVictoryProof {
    pub fn succeeded(&self) -> bool {
        let max_remaining_teams = if self.match_mode_id == "allied_two_vs_one" {
            2
        } else {
            1
        };
        self.phase == CaptureMatchPhase::HumanVictory
            && self.produced_units > 0
            && self.attack_orders > 0
            && self.enemy_structures_destroyed > 0
            && self.remaining_teams <= max_remaining_teams
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureEconomyVictoryProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub ore_before: i32,
    pub ore_after_harvest: i32,
    pub ore_after: i32,
    pub crystal_before: i32,
    pub crystal_after_harvest: i32,
    pub crystal_after: i32,
    pub ore_harvest_ordered: bool,
    pub crystal_harvest_ordered: bool,
    pub product_id: &'static str,
    pub target_units: u32,
    pub produced_units: u32,
    pub attack_orders: u32,
    pub player_units: u32,
    pub enemy_units_destroyed: u32,
    pub enemy_structures_destroyed: u32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CaptureEconomyVictoryProof {
    pub fn succeeded(&self) -> bool {
        let needs_crystal = registry::entity(self.product_id)
            .is_some_and(|def| def.cost.crystal > 0 && self.target_units > 0);
        self.phase == CaptureMatchPhase::HumanVictory
            && self.ore_harvest_ordered
            && self.ore_after_harvest > self.ore_before
            && (!needs_crystal
                || (self.crystal_harvest_ordered
                    && self.crystal_after_harvest > self.crystal_before))
            && self.produced_units >= self.target_units
            && self.attack_orders > 0
            && self.enemy_structures_destroyed > 0
            && self.remaining_teams <= 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturePlayableProof {
    pub faction: CaptureProofFaction,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub ore_before: i32,
    pub ore_after_harvest: i32,
    pub ore_after: i32,
    pub crystal_before: i32,
    pub crystal_after_harvest: i32,
    pub crystal_after: i32,
    pub ore_harvest_ordered: bool,
    pub crystal_harvest_ordered: bool,
    pub structure_id: &'static str,
    pub placement_started: bool,
    pub placed: bool,
    pub construct_ordered: bool,
    pub constructed: bool,
    pub barracks_product_id: &'static str,
    pub barracks_units: u32,
    pub vehicle_product_id: &'static str,
    pub target_units: u32,
    pub produced_units: u32,
    pub attack_orders: u32,
    pub player_units: u32,
    pub enemy_units_destroyed: u32,
    pub enemy_structures_destroyed: u32,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CapturePlayableProof {
    pub fn succeeded(&self) -> bool {
        self.phase == CaptureMatchPhase::HumanVictory
            && self.ore_harvest_ordered
            && self.ore_after_harvest > self.ore_before
            && self.crystal_harvest_ordered
            && self.crystal_after_harvest > self.crystal_before
            && self.placement_started
            && self.placed
            && self.construct_ordered
            && self.constructed
            && self.barracks_units > 0
            && self.produced_units >= self.target_units
            && self.attack_orders > 0
            && self.enemy_structures_destroyed > 0
            && self.remaining_teams <= 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureLobbySlotsProof {
    pub map_id: &'static str,
    pub map_players: usize,
    pub configured_slots: usize,
    pub runtime_players: usize,
    pub active_players: usize,
    pub economy_rows: usize,
    pub unit_teams: usize,
    pub structure_teams: usize,
    pub command_center_teams: usize,
    pub spawn_anchor_matches: usize,
    pub visible_player_index: Option<usize>,
    pub phase: CaptureMatchPhase,
    pub frames: usize,
    pub remaining_teams: u32,
    pub remaining_anchors: u32,
}

impl CaptureLobbySlotsProof {
    pub fn succeeded(&self) -> bool {
        self.map_players >= 8
            && self.configured_slots == self.map_players
            && self.runtime_players == self.configured_slots
            && self.active_players == self.configured_slots
            && self.economy_rows >= self.configured_slots
            && self.unit_teams == self.configured_slots
            && self.structure_teams == self.configured_slots
            && self.command_center_teams == self.configured_slots
            && self.spawn_anchor_matches == self.configured_slots
            && self.visible_player_index == Some(0)
            && self.phase == CaptureMatchPhase::Running
            && self.remaining_teams == self.configured_slots as u32
            && self.remaining_anchors >= self.configured_slots as u32
    }
}

pub fn start_default_match_for_capture(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    enter_shared_match_scene(app);
}

pub fn advance_capture_match(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

pub fn capture_match_snapshot(app: &mut App) -> CaptureMatchSnapshot {
    let world = app.world_mut();
    let mut entities = Vec::new();
    let player_count = world
        .get_resource::<ActiveTeams>()
        .map(|active| active.0.len())
        .unwrap_or(0)
        .max(world.resource::<Economies>().players.len());
    let mut players = vec![CaptureTeamStats::default(); player_count];
    {
        let mut query = world.query::<(
            &Transform,
            Option<&Team>,
            Option<&Unit>,
            Option<&Structure>,
            Option<&ResourceNode>,
            Option<&Health>,
            Option<&VisibilityState>,
        )>();
        for (transform, team, unit, structure, resource, health, visibility) in query.iter(world) {
            let kind = if resource.is_some() {
                CaptureEntityKind::Resource
            } else if structure.is_some() {
                CaptureEntityKind::Structure
            } else if unit.is_some() {
                CaptureEntityKind::Unit
            } else {
                continue;
            };
            let health_ratio = health.map_or(1.0, |health| {
                if health.max <= f32::EPSILON {
                    0.0
                } else {
                    (health.current / health.max).clamp(0.0, 1.0)
                }
            });
            if health_ratio <= 0.0 || resource.is_some_and(|resource| resource.amount <= 0) {
                continue;
            }
            if let Some(team) = team {
                if let Some(stats) = capture_player_stats_mut(&mut players, *team) {
                    match kind {
                        CaptureEntityKind::Unit => stats.units += 1,
                        CaptureEntityKind::Structure => stats.structures += 1,
                        CaptureEntityKind::Resource => {}
                    }
                }
            }
            entities.push(CaptureEntitySnapshot {
                x: transform.translation.x,
                z: transform.translation.z,
                team: team.map_or(CaptureTeam::Neutral, |team| capture_team_from_team(*team)),
                kind,
                visible: visibility.is_none_or(|visibility| visibility.visible),
                health_ratio,
            });
        }
    }
    let economies = world.resource::<Economies>();
    for (index, economy) in economies.players.iter().enumerate() {
        if let Some(stats) = players.get_mut(index) {
            stats.ore = economy.ore;
            stats.crystal = economy.crystal;
        }
    }
    let match_state = world.resource::<MatchState>();
    CaptureMatchSnapshot {
        entities,
        phase: match match_state.phase {
            MatchPhase::Running => CaptureMatchPhase::Running,
            MatchPhase::HumanDefeat => CaptureMatchPhase::HumanDefeat,
            MatchPhase::HumanVictory => CaptureMatchPhase::HumanVictory,
            MatchPhase::MatchFinished => CaptureMatchPhase::MatchFinished,
        },
        elapsed_seconds: match_state.start_time_sec,
        remaining_teams: match_state.remaining_teams,
        remaining_anchors: match_state.remaining_anchors,
        enemy_units_destroyed: match_state.enemy_units_destroyed,
        enemy_structures_destroyed: match_state.enemy_structures_destroyed,
        players,
    }
}

fn capture_player_stats_mut(
    players: &mut [CaptureTeamStats],
    team: Team,
) -> Option<&mut CaptureTeamStats> {
    team.economy_index()
        .and_then(|index| players.get_mut(index))
}

pub fn run_capture_default_match_proof(max_frames: usize) -> CaptureMatchProof {
    run_capture_match_proof_for_faction(CaptureProofFaction::Human, max_frames)
}

pub fn build_capture_match_app_for_faction(faction: CaptureProofFaction) -> App {
    build_capture_match_app_with_settings(capture_match_setup_for_faction(faction))
}

pub fn build_real_menu_match_app_for_faction(faction: CaptureProofFaction) -> App {
    build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Easy)
}

#[derive(Clone, Copy, Debug)]
struct RealMenuMatchStart {
    map_index: usize,
    faction: CaptureProofFaction,
    ai_difficulty: AiDifficulty,
    starting_resource_index: usize,
    match_mode: SkirmishMatchMode,
}

impl RealMenuMatchStart {
    fn new(faction: CaptureProofFaction) -> Self {
        Self {
            map_index: 0,
            faction,
            ai_difficulty: AiDifficulty::Easy,
            starting_resource_index: DEFAULT_STARTING_RESOURCE_INDEX,
            match_mode: SkirmishMatchMode::OneVsOne,
        }
    }

    fn with_map_index(mut self, map_index: usize) -> Self {
        self.map_index = map_index;
        self
    }

    fn with_ai_difficulty(mut self, ai_difficulty: AiDifficulty) -> Self {
        self.ai_difficulty = ai_difficulty;
        self
    }

    fn with_starting_resource_index(mut self, starting_resource_index: usize) -> Self {
        self.starting_resource_index = starting_resource_index;
        self
    }

    fn with_match_mode(mut self, match_mode: SkirmishMatchMode) -> Self {
        self.match_mode = match_mode;
        self
    }
}

fn build_real_menu_match_app_for_faction_with_ai(
    faction: CaptureProofFaction,
    ai_difficulty: AiDifficulty,
) -> App {
    build_real_menu_match_app_for_faction_with_ai_and_starting_resources(
        faction,
        ai_difficulty,
        DEFAULT_STARTING_RESOURCE_INDEX,
    )
}

fn build_real_menu_match_app_for_faction_with_ai_and_starting_resources(
    faction: CaptureProofFaction,
    ai_difficulty: AiDifficulty,
    starting_resource_index: usize,
) -> App {
    build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_ai_difficulty(ai_difficulty)
            .with_starting_resource_index(starting_resource_index),
    )
}

fn build_real_menu_match_app(match_start: RealMenuMatchStart) -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.update();

    drive_main_menu_action(
        &mut app,
        MainMenuAction::SelectMap(match_start.map_index),
        1,
    );
    drive_main_menu_action(&mut app, MainMenuAction::SelectLobbySlot(0), 1);
    drive_main_menu_slot_faction(&mut app, 0, match_start.faction.skirmish_faction());
    drive_main_menu_action(
        &mut app,
        MainMenuAction::SelectMatchMode(match_start.match_mode),
        1,
    );
    if match_start.match_mode == SkirmishMatchMode::AiVsAi {
        drive_main_menu_action(&mut app, MainMenuAction::CycleLobbySlotController(0), 1);
    }
    drive_main_menu_action(
        &mut app,
        MainMenuAction::SelectStartingResources(match_start.starting_resource_index),
        1,
    );
    drive_main_menu_action(
        &mut app,
        MainMenuAction::SelectAiDifficulty(match_start.ai_difficulty),
        1,
    );
    drive_main_menu_action(&mut app, MainMenuAction::StartMatch, 3);
    app
}

fn drive_main_menu_slot_faction(app: &mut App, slot: usize, faction: SkirmishFaction) {
    for _ in 0..SkirmishFaction::ALL.len() {
        let Some(selection) = app.world().get_resource::<SkirmishMenuSelection>() else {
            return;
        };
        if selection
            .lobby_factions
            .get(slot)
            .is_some_and(|current| *current == faction)
        {
            return;
        }
        drive_main_menu_action(app, MainMenuAction::CycleLobbySlotFaction(slot), 1);
    }
}

fn build_real_default_menu_match_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.update();

    drive_main_menu_action(&mut app, MainMenuAction::StartMatch, 3);
    app
}

fn build_real_menu_full_lobby_match_app(map_index: usize) -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.update();

    let map = SKIRMISH_MAPS
        .get(map_index)
        .unwrap_or_else(|| largest_skirmish_map());
    drive_main_menu_action(&mut app, MainMenuAction::SelectMap(map_index), 2);
    for slot in 2..map.players {
        drive_main_menu_action(&mut app, MainMenuAction::CycleLobbySlotController(slot), 1);
        drive_main_menu_action(&mut app, MainMenuAction::CycleLobbySlotController(slot), 1);
    }
    drive_main_menu_action(
        &mut app,
        MainMenuAction::SelectAiDifficulty(AiDifficulty::Easy),
        1,
    );
    drive_main_menu_action(&mut app, MainMenuAction::SelectLobbySlot(0), 1);
    drive_main_menu_action(&mut app, MainMenuAction::StartMatch, 5);
    app
}

pub fn run_real_menu_lobby_slots_proof(max_frames: usize) -> CaptureLobbySlotsProof {
    let mut app = build_real_menu_full_lobby_match_app(3);
    for _ in 0..max_frames {
        app.update();
    }
    capture_lobby_slots_proof(&mut app, max_frames)
}

fn capture_lobby_slots_proof(app: &mut App, frames: usize) -> CaptureLobbySlotsProof {
    let world = app.world_mut();
    let settings = world.resource::<MatchSetupSettings>().clone();
    let map = skirmish_map_by_path(settings.map_path).unwrap_or_else(|| largest_skirmish_map());
    let mut unit_teams = Vec::new();
    let mut structure_teams = Vec::new();
    let mut command_center_teams = Vec::new();
    let mut spawn_anchor_matches = Vec::new();

    {
        let mut query = world.query::<(
            &Team,
            Option<&Unit>,
            Option<&Structure>,
            &Transform,
            Option<&Health>,
        )>();
        for (team, unit, structure, transform, health) in query.iter(world) {
            if health.is_some_and(|health| health.current <= 0.0) {
                continue;
            }
            if unit.is_some() {
                mark_player_flag(&mut unit_teams, *team);
            }
            if let Some(structure) = structure {
                mark_player_flag(&mut structure_teams, *team);
                if structure.id == "CommandCenter" {
                    if let Some(index) = mark_player_flag(&mut command_center_teams, *team) {
                        let spawn_slot = settings.player_spawn_slot(*team);
                        if settings.active_teams.get(index).copied().unwrap_or(false)
                            && map.spawn_points.get(spawn_slot).is_some_and(|spawn_point| {
                                let expected = map_local_to_world(map, *spawn_point);
                                xz_distance(transform.translation, expected) <= 2.0
                            })
                        {
                            mark_player_flag(&mut spawn_anchor_matches, *team);
                        }
                    }
                }
            }
        }
    }

    let match_state = world.resource::<MatchState>();
    let phase = match match_state.phase {
        MatchPhase::Running => CaptureMatchPhase::Running,
        MatchPhase::HumanDefeat => CaptureMatchPhase::HumanDefeat,
        MatchPhase::HumanVictory => CaptureMatchPhase::HumanVictory,
        MatchPhase::MatchFinished => CaptureMatchPhase::MatchFinished,
    };
    let economy_rows = world.resource::<Economies>().players.len();

    CaptureLobbySlotsProof {
        map_id: map.id,
        map_players: map.players,
        configured_slots: map.players.min(settings.active_teams.len()),
        runtime_players: settings.active_teams.len(),
        active_players: settings
            .active_teams
            .iter()
            .filter(|active| **active)
            .count(),
        economy_rows,
        unit_teams: player_flag_count(&unit_teams),
        structure_teams: player_flag_count(&structure_teams),
        command_center_teams: player_flag_count(&command_center_teams),
        spawn_anchor_matches: player_flag_count(&spawn_anchor_matches),
        visible_player_index: settings.visible_player.team.economy_index(),
        phase,
        frames,
        remaining_teams: match_state.remaining_teams,
        remaining_anchors: match_state.remaining_anchors,
    }
}

fn mark_player_flag(flags: &mut Vec<bool>, team: Team) -> Option<usize> {
    let index = team.economy_index()?;
    if flags.len() <= index {
        flags.resize(index + 1, false);
    }
    flags[index] = true;
    Some(index)
}

fn player_flag_count(flags: &[bool]) -> usize {
    flags.iter().filter(|flag| **flag).count()
}

pub fn run_capture_match_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureMatchProof {
    let mut app = build_capture_match_app_for_faction(faction);
    run_capture_match_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_match_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureMatchProof {
    let mut app = build_real_menu_match_app_for_faction(faction);
    run_capture_match_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_harvest_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureHarvestProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Beginner);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.25),
    ));
    run_real_menu_harvest_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_dual_harvest_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureDualHarvestProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai_and_starting_resources(
        faction,
        AiDifficulty::Beginner,
        0,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_dual_harvest_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_supply_crate_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureSupplyCrateProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_map_index(1)
            .with_ai_difficulty(AiDifficulty::Beginner),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.25),
    ));
    run_real_menu_supply_crate_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_tech_oil_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureTechOilProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_map_index(1)
            .with_ai_difficulty(AiDifficulty::Beginner),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_tech_oil_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_tech_oil_proofs(max_frames: usize) -> Vec<CaptureTechOilProof> {
    CaptureProofFaction::ALL
        .into_iter()
        .map(|faction| run_real_menu_tech_oil_proof_for_faction(faction, max_frames))
        .collect()
}

pub fn run_real_menu_ai_pressure_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureAiPressureProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Hard);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_ai_pressure_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_ai_pressure_proofs(max_frames: usize) -> Vec<CaptureAiPressureProof> {
    CaptureProofFaction::ALL
        .into_iter()
        .map(|faction| run_real_menu_ai_pressure_proof_for_faction(faction, max_frames))
        .collect()
}

pub fn run_real_menu_ai_vs_ai_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureAiVsAiProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_match_mode(SkirmishMatchMode::AiVsAi)
            .with_ai_difficulty(AiDifficulty::Hard),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_ai_vs_ai_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_ai_vs_ai_proofs(max_frames: usize) -> Vec<CaptureAiVsAiProof> {
    CaptureProofFaction::ALL
        .into_iter()
        .map(|faction| run_real_menu_ai_vs_ai_proof_for_faction(faction, max_frames))
        .collect()
}

pub fn run_real_menu_build_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureBuildProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Beginner);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_build_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_victory_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureVictoryProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Beginner);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_victory_proof(&mut app, faction, max_frames)
}

pub fn run_real_default_menu_victory_proof(max_frames: usize) -> CaptureVictoryProof {
    let mut app = build_real_default_menu_match_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_victory_proof_with_target_units(
        &mut app,
        CaptureProofFaction::Human,
        max_frames,
        PRODUCTION_QUEUE_LIMIT + 1,
        false,
    )
}

pub fn run_real_menu_selected_faction_victory_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureVictoryProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai(faction, AiDifficulty::Beginner);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_victory_proof_with_target_units(
        &mut app,
        faction,
        max_frames,
        faction.mouse_victory_vehicle_target(),
        false,
    )
}

pub fn run_real_menu_selected_map_victory_proof(
    map_index: usize,
    max_frames: usize,
) -> CaptureVictoryProof {
    run_real_menu_selected_map_victory_proof_for_faction(
        CaptureProofFaction::Human,
        map_index,
        max_frames,
    )
}

pub fn run_real_menu_selected_map_victory_proof_for_faction(
    faction: CaptureProofFaction,
    map_index: usize,
    max_frames: usize,
) -> CaptureVictoryProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_map_index(map_index)
            .with_ai_difficulty(AiDifficulty::Beginner),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_victory_proof_with_target_units(
        &mut app,
        faction,
        max_frames,
        faction
            .mouse_victory_vehicle_target()
            .max(PRODUCTION_QUEUE_LIMIT + 1),
        false,
    )
}

pub fn run_real_menu_all_maps_victory_proofs(max_frames: usize) -> Vec<CaptureVictoryProof> {
    CaptureProofFaction::ALL
        .into_iter()
        .flat_map(|faction| {
            SKIRMISH_MAPS.iter().enumerate().map(move |(map_index, _)| {
                run_real_menu_selected_map_victory_proof_for_faction(faction, map_index, max_frames)
            })
        })
        .collect()
}

pub fn run_real_menu_allied_victory_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureVictoryProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_match_mode(SkirmishMatchMode::AlliedTwoVsOne)
            .with_ai_difficulty(AiDifficulty::Beginner),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_victory_proof_with_target_units(
        &mut app,
        faction,
        max_frames,
        faction.allied_victory_vehicle_target(),
        false,
    )
}

pub fn run_real_menu_allied_victory_proofs(max_frames: usize) -> Vec<CaptureVictoryProof> {
    CaptureProofFaction::ALL
        .into_iter()
        .map(|faction| run_real_menu_allied_victory_proof_for_faction(faction, max_frames))
        .collect()
}

pub fn run_real_menu_economy_victory_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureEconomyVictoryProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai_and_starting_resources(
        faction,
        AiDifficulty::Beginner,
        0,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_economy_victory_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_playable_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CapturePlayableProof {
    let mut app = build_real_menu_match_app_for_faction_with_ai_and_starting_resources(
        faction,
        AiDifficulty::Beginner,
        0,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_playable_proof(&mut app, faction, max_frames)
}

pub fn run_real_menu_free_for_all_playable_proof_for_faction(
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CapturePlayableProof {
    let mut app = build_real_menu_match_app(
        RealMenuMatchStart::new(faction)
            .with_ai_difficulty(AiDifficulty::Beginner)
            .with_starting_resource_index(0)
            .with_match_mode(SkirmishMatchMode::FreeForAll),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0),
    ));
    run_real_menu_playable_proof_with_target_units(
        &mut app,
        faction,
        max_frames,
        faction.mouse_victory_vehicle_target() * 2,
    )
}

pub fn run_capture_match_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureMatchProof {
    let max_frames = max_frames.max(1);
    let player_team = CAPTURE_PLAYER_TEAM;
    let product_id = faction.proof_vehicle();
    let units_before = capture_unit_count_by_id(app.world_mut(), player_team, product_id);
    let mut produced_units = 0usize;

    for frame in 0..max_frames {
        if app.world().resource::<MatchState>().phase != MatchPhase::Running {
            return capture_match_proof_from_app(app, faction, frame, produced_units, units_before);
        }
        advance_capture_match_proof_frame(app, faction, frame);
        produced_units = produced_units.max(capture_unit_count_by_id(
            app.world_mut(),
            player_team,
            product_id,
        ));
    }

    capture_match_proof_from_app(app, faction, max_frames, produced_units, units_before)
}

pub fn advance_capture_match_proof_frame(
    app: &mut App,
    faction: CaptureProofFaction,
    frame: usize,
) {
    if app.world().resource::<MatchState>().phase != MatchPhase::Running {
        return;
    }
    capture_queue_player_units(
        app,
        CAPTURE_PLAYER_TEAM,
        faction.proof_vehicle(),
        faction.proof_vehicle_target(),
    );
    if frame % 30 == 0 {
        capture_order_player_attackers(app, CAPTURE_PLAYER_TEAM);
    }
    app.update();
}

fn capture_match_proof_from_app(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    produced_units: usize,
    units_before: usize,
) -> CaptureMatchProof {
    capture_match_proof_status(
        app,
        faction,
        frames,
        produced_units.saturating_sub(units_before) as u32,
    )
}

pub fn capture_match_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    produced_units: u32,
) -> CaptureMatchProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CaptureMatchProof {
        faction,
        product_id: faction.proof_vehicle(),
        phase: snapshot.phase,
        frames,
        elapsed_seconds: snapshot.elapsed_seconds.round() as u32,
        produced_units,
        player_units: player_stats.units,
        enemy_units_destroyed: snapshot.enemy_units_destroyed,
        enemy_structures_destroyed: snapshot.enemy_structures_destroyed,
        remaining_teams: snapshot.remaining_teams,
        remaining_anchors: snapshot.remaining_anchors,
    }
}

pub fn capture_proof_unit_count(app: &mut App, faction: CaptureProofFaction) -> u32 {
    capture_unit_count_by_id(
        app.world_mut(),
        CAPTURE_PLAYER_TEAM,
        faction.proof_vehicle(),
    ) as u32
}

fn run_real_menu_harvest_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureHarvestProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let mut frames = 0usize;
    let mut harvest_ordered = false;
    let mut product_id = "";
    let mut produced_units = 0u32;

    {
        let mut economies = app.world_mut().resource_mut::<Economies>();
        let economy = economies.get_mut(team);
        economy.ore = 0;
        economy.crystal = 0;
    }

    let Some((harvester, harvester_position)) =
        capture_first_alive_resource_collector(app.world_mut(), team)
    else {
        return capture_harvest_proof_status(
            app,
            faction,
            frames,
            false,
            0,
            0,
            0,
            product_id,
            produced_units,
        );
    };
    let Some((resource, resource_before, resource_position)) =
        capture_nearest_visible_resource(app.world_mut(), ResourceKind::Ore, harvester_position)
    else {
        return capture_harvest_proof_status(
            app,
            faction,
            frames,
            false,
            0,
            0,
            0,
            product_id,
            produced_units,
        );
    };
    let ore_before = capture_team_ore(app.world(), team);

    if capture_world_left_click(app, harvester_position)
        && app
            .world()
            .get_entity(harvester)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
        && capture_world_right_click(app, resource_position)
    {
        harvest_ordered = app
            .world()
            .get::<HarvestOrder>(harvester)
            .is_some_and(|order| order.resource == Some(resource));
    }

    for _ in 0..max_frames {
        frames += 1;
        app.update();
        if capture_team_ore(app.world(), team) > ore_before {
            break;
        }
    }

    let resource_after_harvest = capture_resource_amount(app.world(), resource);
    if harvest_ordered
        && capture_team_ore(app.world(), team) > ore_before
        && resource_after_harvest < resource_before
    {
        let mut train_button = None;
        while train_button.is_none() && frames < max_frames {
            for producer_id in ["Barracks", "CommandCenter"] {
                if let Some(command) =
                    capture_affordable_train_command_from_producer(app, team, producer_id)
                {
                    train_button = Some(command);
                    break;
                }
            }
            if train_button.is_none() {
                frames += 1;
                app.update();
            }
        }

        if let Some((button, train_id)) = train_button {
            product_id = train_id;
            let units_before = capture_unit_count_by_id(app.world_mut(), team, train_id);
            capture_click_command_button(app, button);
            while frames < max_frames {
                frames += 1;
                app.update();
                let current = capture_unit_count_by_id(app.world_mut(), team, train_id);
                if current > units_before {
                    produced_units = current.saturating_sub(units_before) as u32;
                    break;
                }
            }
        }
    }

    let resource_after = capture_resource_amount(app.world(), resource);
    capture_harvest_proof_status(
        app,
        faction,
        frames,
        harvest_ordered,
        ore_before,
        resource_before,
        resource_after,
        product_id,
        produced_units,
    )
}

fn capture_harvest_proof_status(
    app: &App,
    faction: CaptureProofFaction,
    frames: usize,
    harvest_ordered: bool,
    ore_before: i32,
    resource_before: i32,
    resource_after: i32,
    product_id: &'static str,
    produced_units: u32,
) -> CaptureHarvestProof {
    let snapshot_phase = match app.world().resource::<MatchState>().phase {
        MatchPhase::Running => CaptureMatchPhase::Running,
        MatchPhase::HumanDefeat => CaptureMatchPhase::HumanDefeat,
        MatchPhase::HumanVictory => CaptureMatchPhase::HumanVictory,
        MatchPhase::MatchFinished => CaptureMatchPhase::MatchFinished,
    };
    CaptureHarvestProof {
        faction,
        phase: snapshot_phase,
        frames,
        harvest_ordered,
        ore_before,
        ore_after: capture_team_ore(app.world(), CAPTURE_PLAYER_TEAM),
        resource_before,
        resource_after,
        product_id,
        produced_units,
    }
}

fn run_real_menu_dual_harvest_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureDualHarvestProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let resources_before = capture_team_resources(app.world(), team);
    let mut frames = 0usize;
    let ore_harvest_ordered = capture_harvest_kind_until_resource_increases(
        app,
        team,
        ResourceKind::Ore,
        max_frames,
        &mut frames,
    );
    let crystal_harvest_ordered = if frames < max_frames {
        capture_harvest_kind_until_resource_increases(
            app,
            team,
            ResourceKind::Crystal,
            max_frames,
            &mut frames,
        )
    } else {
        false
    };

    capture_dual_harvest_proof_status(
        app,
        faction,
        frames,
        resources_before,
        ore_harvest_ordered,
        crystal_harvest_ordered,
    )
}

fn capture_dual_harvest_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    resources_before: (i32, i32),
    ore_harvest_ordered: bool,
    crystal_harvest_ordered: bool,
) -> CaptureDualHarvestProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CaptureDualHarvestProof {
        faction,
        phase: snapshot.phase,
        frames,
        ore_before: resources_before.0,
        ore_after: player_stats.ore,
        crystal_before: resources_before.1,
        crystal_after: player_stats.crystal,
        ore_harvest_ordered,
        crystal_harvest_ordered,
    }
}

fn run_real_menu_supply_crate_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureSupplyCrateProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let mut frames = 0usize;
    let resources_before = capture_team_resources(app.world(), team);
    let mut collector_id = "";
    let mut collector_selected = false;
    let mut move_ordered = false;
    let mut crate_consumed = false;

    let Some((collector, collector_position, id)) =
        capture_first_alive_supply_crate_collector(app.world_mut(), team)
    else {
        return capture_supply_crate_proof_status(
            app,
            faction,
            frames,
            collector_id,
            collector_selected,
            move_ordered,
            crate_consumed,
            resources_before,
        );
    };
    collector_id = id;

    let Some((crate_entity, crate_position)) = capture_nearest_supply_crate(
        app.world_mut(),
        SupplyCrateEffect::Resources,
        collector_position,
    ) else {
        return capture_supply_crate_proof_status(
            app,
            faction,
            frames,
            collector_id,
            collector_selected,
            move_ordered,
            crate_consumed,
            resources_before,
        );
    };
    if let Ok(mut entity) = app.world_mut().get_entity_mut(crate_entity) {
        entity.insert((VisibilityState { visible: true }, Visibility::Visible));
    }

    collector_selected = capture_world_left_click(app, collector_position)
        && app
            .world()
            .get_entity(collector)
            .is_ok_and(|entity| entity.get::<Selected>().is_some());
    let crate_ground = Vec3::new(crate_position.x, 0.0, crate_position.z);
    if collector_selected && capture_world_right_click(app, crate_ground) {
        move_ordered = app
            .world()
            .get::<MoveOrder>(collector)
            .is_some_and(|order| xz_distance(order.target, crate_position) < 0.08);
    }

    while frames < max_frames {
        frames += 1;
        app.update();
        if app.world().get_entity(crate_entity).is_err() {
            crate_consumed = true;
            break;
        }
        if app.world().resource::<MatchState>().phase != MatchPhase::Running {
            break;
        }
    }

    capture_supply_crate_proof_status(
        app,
        faction,
        frames,
        collector_id,
        collector_selected,
        move_ordered,
        crate_consumed,
        resources_before,
    )
}

fn capture_supply_crate_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    collector_id: &'static str,
    collector_selected: bool,
    move_ordered: bool,
    crate_consumed: bool,
    resources_before: (i32, i32),
) -> CaptureSupplyCrateProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CaptureSupplyCrateProof {
        faction,
        map_id: capture_selected_map_id(app),
        phase: snapshot.phase,
        frames,
        collector_id,
        collector_selected,
        move_ordered,
        crate_consumed,
        ore_before: resources_before.0,
        ore_after: player_stats.ore,
        crystal_before: resources_before.1,
        crystal_after: player_stats.crystal,
    }
}

fn run_real_menu_tech_oil_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureTechOilProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let mut frames = 0usize;
    let mut robotics_constructed = false;
    let mut engineer_produced = false;
    let mut engineer_selected = false;
    let mut capture_ordered = false;
    let mut captured = false;
    let mut engineer_consumed = false;
    let mut ore_before = capture_team_ore(app.world(), team);
    let capture_bonus_ore =
        registry::entity("TechOilDerrick").map_or(0, |def| def.capture_bonus_ore);

    let radar_constructed = capture_ensure_constructed_structure_from_worker(
        app,
        team,
        "RadarUplink",
        max_frames,
        &mut frames,
    )
    .constructed;
    if frames < max_frames {
        robotics_constructed = capture_ensure_constructed_structure_from_worker(
            app,
            team,
            "RoboticsBay",
            max_frames,
            &mut frames,
        )
        .constructed;
    }

    let mut engineer_entity = None;
    if radar_constructed && robotics_constructed && frames < max_frames {
        engineer_entity = capture_train_unit_from_producer(
            app,
            team,
            "CommandCenter",
            "EngineerDrone",
            max_frames,
            &mut frames,
        );
        engineer_produced = engineer_entity.is_some();
    }

    if let Some(engineer) = engineer_entity {
        let engineer_position =
            capture_entity_position(app.world(), engineer).unwrap_or(Vec3::ZERO);
        if let Some((oil, oil_position, _)) = capture_nearest_structure_by_id_and_team(
            app.world_mut(),
            Team::Neutral,
            "TechOilDerrick",
            engineer_position,
            Some(true),
        ) {
            ore_before = capture_team_ore(app.world(), team);
            if let Ok(mut entity) = app.world_mut().get_entity_mut(engineer) {
                entity.insert((VisibilityState { visible: true }, Visibility::Visible));
            }
            engineer_selected = capture_world_left_click_projected(app, engineer_position)
                && app
                    .world()
                    .get_entity(engineer)
                    .is_ok_and(|entity| entity.get::<Selected>().is_some());
            if let Ok(mut entity) = app.world_mut().get_entity_mut(oil) {
                entity.insert((VisibilityState { visible: true }, Visibility::Visible));
            }
            if engineer_selected
                && capture_world_right_click(app, Vec3::new(oil_position.x, 0.0, oil_position.z))
            {
                capture_ordered = app
                    .world()
                    .get::<CaptureOrder>(engineer)
                    .is_some_and(|order| order.target == oil);
            }

            while frames < max_frames {
                frames += 1;
                app.update();
                if capture_entity_team(app.world(), oil) == Some(team) {
                    captured = true;
                    break;
                }
                if app.world().resource::<MatchState>().phase != MatchPhase::Running {
                    break;
                }
            }
            engineer_consumed = app.world().get_entity(engineer).is_err();
        }
    }

    CaptureTechOilProof {
        faction,
        map_id: capture_selected_map_id(app),
        phase: capture_match_phase(app.world()),
        frames,
        radar_constructed,
        robotics_constructed,
        engineer_produced,
        engineer_selected,
        capture_ordered,
        captured,
        engineer_consumed,
        ore_before,
        ore_after: capture_team_ore(app.world(), team),
        capture_bonus_ore,
    }
}

fn run_real_menu_ai_pressure_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureAiPressureProof {
    let max_frames = max_frames.max(1);
    let player_team = CAPTURE_PLAYER_TEAM;
    let Some(ai_team) = capture_primary_active_enemy_team(app.world(), player_team) else {
        return capture_ai_pressure_proof_status(app, faction, 0, Team::Neutral, 0, 0, 0, 0.0, 0.0);
    };

    let ai_units_before = capture_alive_unit_count(app.world_mut(), ai_team) as u32;
    let player_health_before = capture_team_total_health(app.world_mut(), player_team);
    let mut ai_units_peak = ai_units_before;
    let mut ai_attack_orders_peak = 0u32;
    let mut player_health_min = player_health_before;
    let mut frames = 0usize;

    while frames < max_frames && app.world().resource::<MatchState>().phase == MatchPhase::Running {
        frames += 1;
        app.update();
        ai_units_peak =
            ai_units_peak.max(capture_alive_unit_count(app.world_mut(), ai_team) as u32);
        ai_attack_orders_peak = ai_attack_orders_peak.max(capture_attack_orders_against_team(
            app.world_mut(),
            ai_team,
            player_team,
        ) as u32);
        player_health_min =
            player_health_min.min(capture_team_total_health(app.world_mut(), player_team));
        if ai_units_peak > ai_units_before
            && ai_attack_orders_peak > 0
            && player_health_min < player_health_before
        {
            break;
        }
    }

    capture_ai_pressure_proof_status(
        app,
        faction,
        frames,
        ai_team,
        ai_units_before,
        ai_units_peak,
        ai_attack_orders_peak,
        player_health_before,
        player_health_min,
    )
}

fn capture_ai_pressure_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    ai_team: Team,
    ai_units_before: u32,
    ai_units_peak: u32,
    ai_attack_orders: u32,
    player_health_before: f32,
    player_health_after: f32,
) -> CaptureAiPressureProof {
    let snapshot = capture_match_snapshot(app);
    CaptureAiPressureProof {
        faction,
        phase: snapshot.phase,
        frames,
        ai_team: capture_team_from_team(ai_team),
        ai_units_before,
        ai_units_peak,
        ai_attack_orders,
        player_health_before,
        player_health_after,
    }
}

fn run_real_menu_ai_vs_ai_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureAiVsAiProof {
    let max_frames = max_frames.max(1);
    let Some((focus_team, opponent_team)) =
        capture_ai_vs_ai_focus_and_opponent(app.world(), CAPTURE_PLAYER_TEAM)
    else {
        return capture_ai_vs_ai_proof_status(
            app,
            faction,
            0,
            Team::Neutral,
            Team::Neutral,
            0,
            0,
            0,
            0,
            0,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
    };

    let focus_units_before = capture_alive_unit_count(app.world_mut(), focus_team) as u32;
    let opponent_units_before = capture_alive_unit_count(app.world_mut(), opponent_team) as u32;
    let focus_health_before = capture_team_total_health(app.world_mut(), focus_team);
    let opponent_health_before = capture_team_total_health(app.world_mut(), opponent_team);
    let mut focus_units_peak = focus_units_before;
    let mut opponent_units_peak = opponent_units_before;
    let mut focus_attack_orders_peak = 0u32;
    let mut opponent_attack_orders_peak = 0u32;
    let mut focus_health_min = focus_health_before;
    let mut opponent_health_min = opponent_health_before;
    let mut frames = 0usize;

    while frames < max_frames && app.world().resource::<MatchState>().phase == MatchPhase::Running {
        frames += 1;
        app.update();
        focus_units_peak =
            focus_units_peak.max(capture_alive_unit_count(app.world_mut(), focus_team) as u32);
        opponent_units_peak = opponent_units_peak
            .max(capture_alive_unit_count(app.world_mut(), opponent_team) as u32);
        focus_attack_orders_peak = focus_attack_orders_peak.max(
            capture_attack_orders_against_team(app.world_mut(), focus_team, opponent_team) as u32,
        );
        opponent_attack_orders_peak = opponent_attack_orders_peak.max(
            capture_attack_orders_against_team(app.world_mut(), opponent_team, focus_team) as u32,
        );
        focus_health_min =
            focus_health_min.min(capture_team_total_health(app.world_mut(), focus_team));
        opponent_health_min =
            opponent_health_min.min(capture_team_total_health(app.world_mut(), opponent_team));
        if focus_units_peak > focus_units_before
            && opponent_units_peak > opponent_units_before
            && focus_attack_orders_peak + opponent_attack_orders_peak > 0
            && (focus_health_min < focus_health_before
                || opponent_health_min < opponent_health_before)
        {
            break;
        }
    }

    capture_ai_vs_ai_proof_status(
        app,
        faction,
        frames,
        focus_team,
        opponent_team,
        focus_units_before,
        focus_units_peak,
        opponent_units_before,
        opponent_units_peak,
        focus_attack_orders_peak,
        opponent_attack_orders_peak,
        focus_health_before,
        focus_health_min,
        opponent_health_before,
        opponent_health_min,
    )
}

fn capture_ai_vs_ai_focus_and_opponent(world: &World, focus_team: Team) -> Option<(Team, Team)> {
    let active_teams = world.resource::<ActiveTeams>();
    let relations = world.resource::<TeamRelations>().clone();
    let candidates = player_teams(active_teams.0.len())
        .filter(|team| team_is_active(*team, Some(active_teams)))
        .collect::<Vec<_>>();
    if team_is_active(focus_team, Some(active_teams))
        && let Some(opponent) = candidates
            .iter()
            .copied()
            .find(|team| *team != focus_team && relations.are_enemies(focus_team, *team))
    {
        return Some((focus_team, opponent));
    }

    candidates.iter().copied().find_map(|first| {
        candidates
            .iter()
            .copied()
            .find(|second| *second != first && relations.are_enemies(first, *second))
            .map(|second| (first, second))
    })
}

#[allow(clippy::too_many_arguments)]
fn capture_ai_vs_ai_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    focus_team: Team,
    opponent_team: Team,
    focus_units_before: u32,
    focus_units_peak: u32,
    opponent_units_before: u32,
    opponent_units_peak: u32,
    focus_attack_orders: u32,
    opponent_attack_orders: u32,
    focus_health_before: f32,
    focus_health_after: f32,
    opponent_health_before: f32,
    opponent_health_after: f32,
) -> CaptureAiVsAiProof {
    let snapshot = capture_match_snapshot(app);
    CaptureAiVsAiProof {
        focus_faction: faction,
        map_id: capture_selected_map_id(app),
        match_mode_id: capture_match_mode_id(app),
        phase: snapshot.phase,
        frames,
        focus_team: capture_team_from_team(focus_team),
        opponent_team: capture_team_from_team(opponent_team),
        focus_units_before,
        focus_units_peak,
        opponent_units_before,
        opponent_units_peak,
        focus_attack_orders,
        opponent_attack_orders,
        focus_health_before,
        focus_health_after,
        opponent_health_before,
        opponent_health_after,
        remaining_teams: snapshot.remaining_teams,
        remaining_anchors: snapshot.remaining_anchors,
    }
}

fn run_real_menu_build_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureBuildProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let structure_id = "Barracks";
    let mut frames = 0usize;
    let mut placement_started = false;
    let mut placed = false;
    let mut construct_ordered = false;
    let mut constructed = false;
    let mut product_id = "";
    let mut produced_units = 0u32;

    {
        let mut economies = app.world_mut().resource_mut::<Economies>();
        let economy = economies.get_mut(team);
        economy.ore = economy.ore.max(200);
        economy.crystal = economy.crystal.max(200);
    }

    let Some((worker, worker_position)) =
        capture_ensure_builder_worker(app, team, max_frames, &mut frames)
    else {
        return capture_build_proof_status(
            app,
            faction,
            frames,
            structure_id,
            placement_started,
            placed,
            construct_ordered,
            constructed,
            product_id,
            produced_units,
        );
    };

    if capture_world_left_click(app, worker_position)
        && app
            .world()
            .get_entity(worker)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
    {
        app.update();
        if let Some(button) =
            capture_enabled_command_for_action(app, BuildAction::Build(structure_id))
        {
            capture_click_command_button(app, button);
            placement_started = app
                .world()
                .resource::<CommandMode>()
                .pending_structure_placement
                .is_some_and(|pending| pending.id == structure_id);
        }
    }

    let mut placed_structure = None;
    if placement_started
        && let Some(placement) =
            capture_valid_structure_placement_point_near_team_base(app, team, structure_id)
        && capture_world_left_click(app, placement)
    {
        placed_structure = capture_structure_by_id_near(
            app.world_mut(),
            team,
            structure_id,
            placement,
            Some(false),
        );
        placed = placed_structure.is_some()
            && app
                .world()
                .resource::<CommandMode>()
                .pending_structure_placement
                .is_none();
    }

    if let Some((structure, structure_position, _)) = placed_structure {
        if let Ok(mut entity) = app.world_mut().get_entity_mut(structure) {
            entity.insert((VisibilityState { visible: true }, Visibility::Visible));
        }
        let current_worker_position =
            capture_entity_position(app.world(), worker).unwrap_or(worker_position);
        if capture_world_left_click(app, current_worker_position)
            && app
                .world()
                .get_entity(worker)
                .is_ok_and(|entity| entity.get::<Selected>().is_some())
            && capture_world_right_click(app, structure_position)
        {
            construct_ordered = app
                .world()
                .get::<ConstructOrder>(worker)
                .is_some_and(|order| order.target == structure);
        }

        while frames < max_frames {
            frames += 1;
            app.update();
            constructed = capture_structure_constructed(app.world(), structure);
            if constructed {
                break;
            }
        }

        if constructed
            && capture_world_left_click(app, structure_position)
            && app
                .world()
                .get_entity(structure)
                .is_ok_and(|entity| entity.get::<Selected>().is_some())
        {
            app.update();
            let Some((button, train_id)) = capture_first_affordable_train_command(app, team) else {
                return capture_build_proof_status(
                    app,
                    faction,
                    frames,
                    structure_id,
                    placement_started,
                    placed,
                    construct_ordered,
                    constructed,
                    product_id,
                    produced_units,
                );
            };
            product_id = train_id;
            let units_before = capture_unit_count_by_id(app.world_mut(), team, train_id);
            capture_click_command_button(app, button);
            while frames < max_frames {
                frames += 1;
                app.update();
                let current = capture_unit_count_by_id(app.world_mut(), team, train_id);
                if current > units_before {
                    produced_units = current.saturating_sub(units_before) as u32;
                    break;
                }
            }
        }
    }

    capture_build_proof_status(
        app,
        faction,
        frames,
        structure_id,
        placement_started,
        placed,
        construct_ordered,
        constructed,
        product_id,
        produced_units,
    )
}

fn capture_build_proof_status(
    app: &App,
    faction: CaptureProofFaction,
    frames: usize,
    structure_id: &'static str,
    placement_started: bool,
    placed: bool,
    construct_ordered: bool,
    constructed: bool,
    product_id: &'static str,
    produced_units: u32,
) -> CaptureBuildProof {
    let phase = match app.world().resource::<MatchState>().phase {
        MatchPhase::Running => CaptureMatchPhase::Running,
        MatchPhase::HumanDefeat => CaptureMatchPhase::HumanDefeat,
        MatchPhase::HumanVictory => CaptureMatchPhase::HumanVictory,
        MatchPhase::MatchFinished => CaptureMatchPhase::MatchFinished,
    };
    CaptureBuildProof {
        faction,
        phase,
        frames,
        structure_id,
        placement_started,
        placed,
        construct_ordered,
        constructed,
        product_id,
        produced_units,
    }
}

fn run_real_menu_victory_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureVictoryProof {
    run_real_menu_victory_proof_with_target_units(
        app,
        faction,
        max_frames,
        faction.mouse_victory_vehicle_target(),
        true,
    )
}

fn run_real_menu_victory_proof_with_target_units(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
    target_units: usize,
    grant_resources: bool,
) -> CaptureVictoryProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let product_id = faction.proof_vehicle();
    let mut frames = 0usize;
    let mut attack_orders = 0u32;

    let Some(product_def) = registry::entity(product_id) else {
        return capture_victory_proof_status(
            app,
            faction,
            frames,
            product_id,
            target_units as u32,
            0,
            attack_orders,
        );
    };
    if grant_resources {
        let mut economies = app.world_mut().resource_mut::<Economies>();
        let economy = economies.get_mut(team);
        economy.ore = economy
            .ore
            .max(product_def.cost.ore * target_units as i32 + 320);
        economy.crystal = economy
            .crystal
            .max(product_def.cost.crystal * target_units as i32 + 320);
    }

    let produced_units = capture_train_attack_group_from_factory_command_panel(
        app,
        team,
        product_id,
        target_units,
        max_frames,
        &mut frames,
    );
    attack_orders +=
        capture_finish_match_with_mouse_attackers(app, team, max_frames, &mut frames) as u32;

    capture_victory_proof_status(
        app,
        faction,
        frames,
        product_id,
        target_units as u32,
        produced_units,
        attack_orders,
    )
}

fn run_real_menu_economy_victory_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CaptureEconomyVictoryProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let product_id = faction.proof_vehicle();
    let target_units = faction.mouse_victory_vehicle_target();
    let Some(product_def) = registry::entity(product_id) else {
        return capture_economy_victory_proof_status(
            app,
            faction,
            0,
            (0, 0),
            (0, 0),
            false,
            false,
            product_id,
            target_units as u32,
            0,
            0,
        );
    };
    let required_ore = product_def.cost.ore * target_units as i32;
    let required_crystal = product_def.cost.crystal * target_units as i32;
    let resources_before = capture_team_resources(app.world(), team);
    let mut frames = 0usize;
    let mut ore_harvest_ordered = false;
    let mut crystal_harvest_ordered = false;

    while frames < max_frames {
        let (ore, crystal) = capture_team_resources(app.world(), team);
        if ore >= required_ore && crystal >= required_crystal {
            break;
        }
        if ore < required_ore {
            if !capture_harvest_kind_until_resource_increases(
                app,
                team,
                ResourceKind::Ore,
                max_frames,
                &mut frames,
            ) {
                break;
            }
            ore_harvest_ordered = true;
            continue;
        }
        if crystal < required_crystal {
            if !capture_harvest_kind_until_resource_increases(
                app,
                team,
                ResourceKind::Crystal,
                max_frames,
                &mut frames,
            ) {
                break;
            }
            crystal_harvest_ordered = true;
            continue;
        }
    }

    let resources_after_harvest = capture_team_resources(app.world(), team);
    let produced_units = capture_train_attack_group_from_factory_command_panel(
        app,
        team,
        product_id,
        target_units,
        max_frames,
        &mut frames,
    );
    let attack_orders =
        capture_finish_match_with_mouse_attackers(app, team, max_frames, &mut frames) as u32;

    capture_economy_victory_proof_status(
        app,
        faction,
        frames,
        resources_before,
        resources_after_harvest,
        ore_harvest_ordered,
        crystal_harvest_ordered,
        product_id,
        target_units as u32,
        produced_units,
        attack_orders,
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CaptureBarracksBuildResult {
    placement_started: bool,
    placed: bool,
    construct_ordered: bool,
    constructed: bool,
    product_id: &'static str,
    produced_units: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CaptureStructureBuildOnlyResult {
    placement_started: bool,
    placed: bool,
    construct_ordered: bool,
    constructed: bool,
}

fn run_real_menu_playable_proof(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
) -> CapturePlayableProof {
    run_real_menu_playable_proof_with_target_units(
        app,
        faction,
        max_frames,
        faction.mouse_victory_vehicle_target(),
    )
}

fn run_real_menu_playable_proof_with_target_units(
    app: &mut App,
    faction: CaptureProofFaction,
    max_frames: usize,
    target_units: usize,
) -> CapturePlayableProof {
    let max_frames = max_frames.max(1);
    let team = CAPTURE_PLAYER_TEAM;
    let vehicle_id = faction.proof_vehicle();
    let Some(vehicle_def) = registry::entity(vehicle_id) else {
        return capture_playable_proof_status(
            app,
            faction,
            0,
            (0, 0),
            (0, 0),
            (false, false),
            CaptureBarracksBuildResult::default(),
            vehicle_id,
            target_units as u32,
            0,
            0,
        );
    };
    let Some(barracks_def) = registry::entity("Barracks") else {
        return capture_playable_proof_status(
            app,
            faction,
            0,
            (0, 0),
            (0, 0),
            (false, false),
            CaptureBarracksBuildResult::default(),
            vehicle_id,
            target_units as u32,
            0,
            0,
        );
    };

    let resources_before = capture_team_resources(app.world(), team);
    let mut frames = 0usize;
    let mut harvest_flags = (false, false);
    let mut harvest_peaks = resources_before;
    let upfront_ore = vehicle_def.cost.ore * target_units as i32 + barracks_def.cost.ore + 2;
    let upfront_crystal =
        vehicle_def.cost.crystal * target_units as i32 + barracks_def.cost.crystal + 1;
    capture_harvest_until_resources(
        app,
        team,
        upfront_ore,
        upfront_crystal,
        max_frames,
        &mut frames,
        &mut harvest_flags,
        &mut harvest_peaks,
    );
    capture_force_sample_harvests(
        app,
        team,
        max_frames,
        &mut frames,
        &mut harvest_flags,
        &mut harvest_peaks,
    );

    let build_result =
        capture_build_barracks_and_train_unit_from_mouse(app, team, max_frames, &mut frames);

    let vehicle_ore = vehicle_def.cost.ore * target_units as i32;
    let vehicle_crystal = vehicle_def.cost.crystal * target_units as i32;
    capture_harvest_until_resources(
        app,
        team,
        vehicle_ore,
        vehicle_crystal,
        max_frames,
        &mut frames,
        &mut harvest_flags,
        &mut harvest_peaks,
    );

    let produced_units = capture_train_attack_group_from_factory_command_panel(
        app,
        team,
        vehicle_id,
        target_units,
        max_frames,
        &mut frames,
    );
    let attack_orders =
        capture_finish_match_with_mouse_attackers(app, team, max_frames, &mut frames) as u32;

    capture_playable_proof_status(
        app,
        faction,
        frames,
        resources_before,
        harvest_peaks,
        harvest_flags,
        build_result,
        vehicle_id,
        target_units as u32,
        produced_units,
        attack_orders,
    )
}

fn capture_victory_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    product_id: &'static str,
    target_units: u32,
    produced_units: u32,
    attack_orders: u32,
) -> CaptureVictoryProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CaptureVictoryProof {
        faction,
        map_id: capture_selected_map_id(app),
        match_mode_id: capture_match_mode_id(app),
        phase: snapshot.phase,
        frames,
        product_id,
        target_units,
        produced_units,
        attack_orders,
        player_units: player_stats.units,
        enemy_units_destroyed: snapshot.enemy_units_destroyed,
        enemy_structures_destroyed: snapshot.enemy_structures_destroyed,
        remaining_teams: snapshot.remaining_teams,
        remaining_anchors: snapshot.remaining_anchors,
    }
}

fn capture_selected_map_id(app: &App) -> &'static str {
    app.world()
        .get_resource::<SelectedSkirmishMap>()
        .map(|map| map.definition().id)
        .unwrap_or("unknown")
}

fn capture_match_mode_id(app: &App) -> &'static str {
    let Some(settings) = app.world().get_resource::<MatchSetupSettings>() else {
        return "unknown";
    };
    if app
        .world()
        .get_resource::<VisiblePlayer>()
        .is_some_and(|visible_player| visible_player.is_spectator())
    {
        SkirmishMatchMode::AiVsAi.id()
    } else {
        skirmish_mode_from_match_setup(settings).id()
    }
}

fn capture_economy_victory_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    resources_before: (i32, i32),
    resources_after_harvest: (i32, i32),
    ore_harvest_ordered: bool,
    crystal_harvest_ordered: bool,
    product_id: &'static str,
    target_units: u32,
    produced_units: u32,
    attack_orders: u32,
) -> CaptureEconomyVictoryProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CaptureEconomyVictoryProof {
        faction,
        phase: snapshot.phase,
        frames,
        ore_before: resources_before.0,
        ore_after_harvest: resources_after_harvest.0,
        ore_after: player_stats.ore,
        crystal_before: resources_before.1,
        crystal_after_harvest: resources_after_harvest.1,
        crystal_after: player_stats.crystal,
        ore_harvest_ordered,
        crystal_harvest_ordered,
        product_id,
        target_units,
        produced_units,
        attack_orders,
        player_units: player_stats.units,
        enemy_units_destroyed: snapshot.enemy_units_destroyed,
        enemy_structures_destroyed: snapshot.enemy_structures_destroyed,
        remaining_teams: snapshot.remaining_teams,
        remaining_anchors: snapshot.remaining_anchors,
    }
}

fn capture_playable_proof_status(
    app: &mut App,
    faction: CaptureProofFaction,
    frames: usize,
    resources_before: (i32, i32),
    resources_after_harvest: (i32, i32),
    harvest_flags: (bool, bool),
    build_result: CaptureBarracksBuildResult,
    vehicle_id: &'static str,
    target_units: u32,
    produced_units: u32,
    attack_orders: u32,
) -> CapturePlayableProof {
    let snapshot = capture_match_snapshot(app);
    let player_stats = capture_stats_for_team(&snapshot, CAPTURE_PLAYER_TEAM);
    CapturePlayableProof {
        faction,
        phase: snapshot.phase,
        frames,
        ore_before: resources_before.0,
        ore_after_harvest: resources_after_harvest.0,
        ore_after: player_stats.ore,
        crystal_before: resources_before.1,
        crystal_after_harvest: resources_after_harvest.1,
        crystal_after: player_stats.crystal,
        ore_harvest_ordered: harvest_flags.0,
        crystal_harvest_ordered: harvest_flags.1,
        structure_id: "Barracks",
        placement_started: build_result.placement_started,
        placed: build_result.placed,
        construct_ordered: build_result.construct_ordered,
        constructed: build_result.constructed,
        barracks_product_id: build_result.product_id,
        barracks_units: build_result.produced_units,
        vehicle_product_id: vehicle_id,
        target_units,
        produced_units,
        attack_orders,
        player_units: player_stats.units,
        enemy_units_destroyed: snapshot.enemy_units_destroyed,
        enemy_structures_destroyed: snapshot.enemy_structures_destroyed,
        remaining_teams: snapshot.remaining_teams,
        remaining_anchors: snapshot.remaining_anchors,
    }
}

fn capture_team_resources(world: &World, team: Team) -> (i32, i32) {
    let economy = world.resource::<Economies>().get(team);
    (economy.ore, economy.crystal)
}

fn capture_resource_value(resources: (i32, i32), kind: ResourceKind) -> i32 {
    match kind {
        ResourceKind::Ore => resources.0,
        ResourceKind::Crystal => resources.1,
    }
}

fn capture_harvest_kind_until_resource_increases(
    app: &mut App,
    team: Team,
    kind: ResourceKind,
    max_frames: usize,
    frames: &mut usize,
) -> bool {
    let before = capture_team_resources(app.world(), team);
    if !capture_order_nearest_resource_harvest(app, team, kind) {
        return false;
    }
    let before_value = capture_resource_value(before, kind);
    while *frames < max_frames {
        *frames += 1;
        app.update();
        let current = capture_resource_value(capture_team_resources(app.world(), team), kind);
        if current > before_value {
            return true;
        }
        if app.world().resource::<MatchState>().phase != MatchPhase::Running {
            break;
        }
    }
    false
}

fn capture_harvest_until_resources(
    app: &mut App,
    team: Team,
    required_ore: i32,
    required_crystal: i32,
    max_frames: usize,
    frames: &mut usize,
    harvest_flags: &mut (bool, bool),
    harvest_peaks: &mut (i32, i32),
) {
    while *frames < max_frames {
        let (ore, crystal) = capture_team_resources(app.world(), team);
        if ore >= required_ore && crystal >= required_crystal {
            break;
        }
        if ore < required_ore {
            if !capture_harvest_kind_until_resource_increases(
                app,
                team,
                ResourceKind::Ore,
                max_frames,
                frames,
            ) {
                break;
            }
            harvest_flags.0 = true;
            let resources = capture_team_resources(app.world(), team);
            harvest_peaks.0 = harvest_peaks.0.max(resources.0);
            harvest_peaks.1 = harvest_peaks.1.max(resources.1);
            continue;
        }
        if crystal < required_crystal {
            if !capture_harvest_kind_until_resource_increases(
                app,
                team,
                ResourceKind::Crystal,
                max_frames,
                frames,
            ) {
                break;
            }
            harvest_flags.1 = true;
            let resources = capture_team_resources(app.world(), team);
            harvest_peaks.0 = harvest_peaks.0.max(resources.0);
            harvest_peaks.1 = harvest_peaks.1.max(resources.1);
            continue;
        }
    }
}

fn capture_force_sample_harvests(
    app: &mut App,
    team: Team,
    max_frames: usize,
    frames: &mut usize,
    harvest_flags: &mut (bool, bool),
    harvest_peaks: &mut (i32, i32),
) {
    if !harvest_flags.0
        && capture_harvest_kind_until_resource_increases(
            app,
            team,
            ResourceKind::Ore,
            max_frames,
            frames,
        )
    {
        harvest_flags.0 = true;
        let resources = capture_team_resources(app.world(), team);
        harvest_peaks.0 = harvest_peaks.0.max(resources.0);
        harvest_peaks.1 = harvest_peaks.1.max(resources.1);
    }

    if !harvest_flags.1
        && capture_harvest_kind_until_resource_increases(
            app,
            team,
            ResourceKind::Crystal,
            max_frames,
            frames,
        )
    {
        harvest_flags.1 = true;
        let resources = capture_team_resources(app.world(), team);
        harvest_peaks.0 = harvest_peaks.0.max(resources.0);
        harvest_peaks.1 = harvest_peaks.1.max(resources.1);
    }
}

fn capture_order_nearest_resource_harvest(app: &mut App, team: Team, kind: ResourceKind) -> bool {
    let Some((harvester, harvester_position)) =
        capture_first_alive_resource_collector(app.world_mut(), team)
    else {
        return false;
    };
    let Some((resource, _, resource_position)) =
        capture_nearest_visible_resource(app.world_mut(), kind, harvester_position)
    else {
        return false;
    };
    capture_world_left_click(app, harvester_position)
        && app
            .world()
            .get_entity(harvester)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
        && capture_world_right_click(app, resource_position)
        && app
            .world()
            .get::<HarvestOrder>(harvester)
            .is_some_and(|order| order.resource == Some(resource))
}

fn capture_ensure_constructed_structure_from_worker(
    app: &mut App,
    team: Team,
    structure_id: &'static str,
    max_frames: usize,
    frames: &mut usize,
) -> CaptureStructureBuildOnlyResult {
    let mut result = CaptureStructureBuildOnlyResult::default();
    if capture_constructed_producer(app.world_mut(), team, structure_id).is_some() {
        result.constructed = true;
        return result;
    }

    if capture_ensure_builder_worker(app, team, max_frames, frames).is_none() {
        return result;
    }
    if capture_select_first_alive_unit_by_id(app, team, "Worker").is_some() {
        app.update();
    }
    if let Some(button) = capture_enabled_command_for_action(app, BuildAction::Build(structure_id))
    {
        capture_click_command_button(app, button);
        result.placement_started = app
            .world()
            .resource::<CommandMode>()
            .pending_structure_placement
            .is_some_and(|pending| pending.id == structure_id);
    }

    let mut placed_structure = None;
    if result.placement_started
        && let Some(placement) =
            capture_valid_structure_placement_point_near_team_base(app, team, structure_id)
        && capture_world_left_click(app, placement)
    {
        placed_structure = capture_structure_by_id_near(
            app.world_mut(),
            team,
            structure_id,
            placement,
            Some(false),
        );
        result.placed = placed_structure.is_some()
            && app
                .world()
                .resource::<CommandMode>()
                .pending_structure_placement
                .is_none();
    }

    if let Some((structure, structure_position, _)) = placed_structure {
        if let Ok(mut entity) = app.world_mut().get_entity_mut(structure) {
            entity.insert((VisibilityState { visible: true }, Visibility::Visible));
        }
        if let Some((worker, _)) = capture_select_first_alive_unit_by_id(app, team, "Worker")
            && capture_world_right_click(app, structure_position)
        {
            result.construct_ordered = app
                .world()
                .get::<ConstructOrder>(worker)
                .is_some_and(|order| order.target == structure);
        }

        while *frames < max_frames {
            *frames += 1;
            app.update();
            result.constructed = capture_structure_constructed(app.world(), structure);
            if result.constructed {
                break;
            }
        }
    }

    result
}

fn capture_build_barracks_and_train_unit_from_mouse(
    app: &mut App,
    team: Team,
    max_frames: usize,
    frames: &mut usize,
) -> CaptureBarracksBuildResult {
    let structure_id = "Barracks";
    let mut result = CaptureBarracksBuildResult::default();

    if capture_ensure_builder_worker(app, team, max_frames, frames).is_none() {
        return result;
    }

    if capture_select_first_alive_unit_by_id(app, team, "Worker").is_some() {
        app.update();
    }
    if let Some(button) = capture_enabled_command_for_action(app, BuildAction::Build(structure_id))
    {
        capture_click_command_button(app, button);
        result.placement_started = app
            .world()
            .resource::<CommandMode>()
            .pending_structure_placement
            .is_some_and(|pending| pending.id == structure_id);
    }

    let mut placed_structure = None;
    if result.placement_started
        && let Some(placement) =
            capture_valid_structure_placement_point_near_team_base(app, team, structure_id)
        && capture_world_left_click(app, placement)
    {
        placed_structure = capture_structure_by_id_near(
            app.world_mut(),
            team,
            structure_id,
            placement,
            Some(false),
        );
        result.placed = placed_structure.is_some()
            && app
                .world()
                .resource::<CommandMode>()
                .pending_structure_placement
                .is_none();
    }

    if let Some((structure, structure_position, _)) = placed_structure {
        if let Ok(mut entity) = app.world_mut().get_entity_mut(structure) {
            entity.insert((VisibilityState { visible: true }, Visibility::Visible));
        }
        if let Some((worker, _)) = capture_select_first_alive_unit_by_id(app, team, "Worker")
            && capture_world_right_click(app, structure_position)
        {
            result.construct_ordered = app
                .world()
                .get::<ConstructOrder>(worker)
                .is_some_and(|order| order.target == structure);
        }

        while *frames < max_frames {
            *frames += 1;
            app.update();
            result.constructed = capture_structure_constructed(app.world(), structure);
            if result.constructed {
                break;
            }
        }

        if result.constructed
            && capture_world_left_click(app, structure_position)
            && app
                .world()
                .get_entity(structure)
                .is_ok_and(|entity| entity.get::<Selected>().is_some())
        {
            app.update();
            if let Some((button, train_id)) = capture_first_affordable_train_command(app, team) {
                result.product_id = train_id;
                let units_before = capture_unit_count_by_id(app.world_mut(), team, train_id);
                capture_click_command_button(app, button);
                while *frames < max_frames {
                    *frames += 1;
                    app.update();
                    let current = capture_unit_count_by_id(app.world_mut(), team, train_id);
                    if current > units_before {
                        result.produced_units = current.saturating_sub(units_before) as u32;
                        break;
                    }
                }
            }
        }
    }

    result
}

fn capture_train_attack_group_from_factory_command_panel(
    app: &mut App,
    team: Team,
    product_id: &'static str,
    target_units: usize,
    max_frames: usize,
    frames: &mut usize,
) -> u32 {
    let Some((factory, _, factory_position)) =
        capture_constructed_producer(app.world_mut(), team, "VehicleFactory")
    else {
        return 0;
    };
    let units_before = capture_unit_count_by_id(app.world_mut(), team, product_id);
    let mut produced_units = 0u32;

    if capture_world_left_click(app, factory_position)
        && app
            .world()
            .get_entity(factory)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
    {
        app.update();
        while *frames < max_frames {
            let produced = capture_unit_count_by_id(app.world_mut(), team, product_id);
            produced_units = produced.saturating_sub(units_before) as u32;
            if produced >= units_before + target_units {
                break;
            }
            let started = produced + capture_queued_train_jobs(app.world(), team, product_id);
            if started < units_before + target_units
                && let Some(button) =
                    capture_enabled_command_for_action(app, BuildAction::Train(product_id))
            {
                capture_click_command_button(app, button);
            }
            *frames += 1;
            app.update();
        }
    }
    produced_units
}

fn capture_finish_match_with_mouse_attackers(
    app: &mut App,
    team: Team,
    max_frames: usize,
    frames: &mut usize,
) -> usize {
    let mut attack_orders = 0usize;
    while *frames < max_frames && app.world().resource::<MatchState>().phase == MatchPhase::Running
    {
        let Some((target, target_position)) = capture_first_enemy_anchor(app.world_mut(), team)
        else {
            break;
        };
        if let Ok(mut entity) = app.world_mut().get_entity_mut(target) {
            entity.insert((VisibilityState { visible: true }, Visibility::Visible));
        }
        let attackers = capture_alive_attackers(app.world_mut(), team);
        if attackers.is_empty() {
            break;
        }
        attack_orders +=
            capture_mouse_order_attackers_to_target(app, &attackers, target, target_position);

        for _ in 0..260 {
            if *frames >= max_frames
                || app.world().get_entity(target).is_err()
                || app.world().resource::<MatchState>().phase != MatchPhase::Running
            {
                break;
            }
            *frames += 1;
            app.update();
        }
    }

    for _ in 0..8 {
        if *frames >= max_frames
            || app.world().resource::<MatchState>().phase == MatchPhase::HumanVictory
        {
            break;
        }
        *frames += 1;
        app.update();
    }
    attack_orders
}

fn drive_main_menu_action(app: &mut App, action: MainMenuAction, followup_updates: usize) {
    let button_entity = {
        let world = app.world_mut();
        let mut buttons = world.query::<(Entity, &MainMenuButton)>();
        buttons
            .iter(world)
            .find_map(|(entity, button)| (button.action == action).then_some(entity))
            .expect("main menu button should exist")
    };
    app.world_mut()
        .entity_mut(button_entity)
        .insert(Interaction::Pressed);
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }
    app.update();
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
        mouse.clear();
    }
    if let Ok(mut button) = app.world_mut().get_entity_mut(button_entity) {
        button.insert(Interaction::None);
    }
    for _ in 0..followup_updates {
        app.update();
    }
}

fn capture_first_alive_resource_collector(world: &mut World, team: Team) -> Option<(Entity, Vec3)> {
    let mut units = world.query::<(
        Entity,
        &Team,
        &Unit,
        &Transform,
        &Health,
        Option<&ResourceCargo>,
    )>();
    units
        .iter(world)
        .filter_map(|(entity, unit_team, unit, transform, health, cargo)| {
            (*unit_team == team
                && health.current > 0.0
                && cargo.is_some_and(|cargo| cargo.capacity > 0)
                && can_unit_collect_resources(unit))
            .then_some((entity, transform.translation, unit.id))
        })
        .min_by_key(|(_, _, id)| match *id {
            "OreHarvester" => 0,
            "Worker" => 1,
            _ => 2,
        })
        .map(|(entity, position, _)| (entity, position))
}

fn capture_first_alive_unit_by_id(
    world: &mut World,
    team: Team,
    id: &'static str,
) -> Option<(Entity, Vec3)> {
    let mut units = world.query::<(Entity, &Team, &Unit, &Transform, &Health)>();
    units
        .iter(world)
        .find_map(|(entity, unit_team, unit, transform, health)| {
            (*unit_team == team && unit.id == id && health.current > 0.0)
                .then_some((entity, transform.translation))
        })
}

fn capture_first_alive_supply_crate_collector(
    world: &mut World,
    team: Team,
) -> Option<(Entity, Vec3, &'static str)> {
    let mut units = world.query::<(Entity, &Team, &Unit, &MovementDomain, &Transform, &Health)>();
    units
        .iter(world)
        .filter_map(|(entity, unit_team, unit, domain, transform, health)| {
            (*unit_team == team
                && health.current > 0.0
                && unit.speed > 0.0
                && *domain == MovementDomain::Terrain)
                .then_some((entity, transform.translation, unit.id))
        })
        .min_by_key(|(_, _, id)| match *id {
            "ScoutRover" => 0,
            "FlameAssaultBuggy" => 1,
            "Worker" => 2,
            _ => 3,
        })
}

fn capture_alive_units_by_id(
    world: &mut World,
    team: Team,
    id: &'static str,
) -> Vec<(Entity, Vec3)> {
    let mut units = world.query::<(Entity, &Team, &Unit, &Transform, &Health)>();
    units
        .iter(world)
        .filter_map(|(entity, unit_team, unit, transform, health)| {
            (*unit_team == team && unit.id == id && health.current > 0.0)
                .then_some((entity, transform.translation))
        })
        .collect()
}

fn capture_select_first_alive_unit_by_id(
    app: &mut App,
    team: Team,
    id: &'static str,
) -> Option<(Entity, Vec3)> {
    let units = capture_alive_units_by_id(app.world_mut(), team, id);
    for (entity, position) in units {
        let current_position = capture_entity_position(app.world(), entity).unwrap_or(position);
        if capture_world_left_click(app, current_position)
            && app
                .world()
                .get_entity(entity)
                .is_ok_and(|entity_ref| entity_ref.get::<Selected>().is_some())
        {
            return Some((entity, current_position));
        }
    }
    None
}

fn capture_ensure_builder_worker(
    app: &mut App,
    team: Team,
    max_frames: usize,
    frames: &mut usize,
) -> Option<(Entity, Vec3)> {
    if let Some(worker) = capture_first_alive_unit_by_id(app.world_mut(), team, "Worker") {
        return Some(worker);
    }
    let (command_center, _, command_center_position) =
        capture_constructed_producer(app.world_mut(), team, "CommandCenter")?;
    if !capture_world_left_click(app, command_center_position)
        || !app
            .world()
            .get_entity(command_center)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
    {
        return None;
    }
    app.update();
    let button = capture_enabled_command_for_action(app, BuildAction::Train("Worker"))?;
    capture_click_command_button(app, button);
    while *frames < max_frames {
        *frames += 1;
        app.update();
        if let Some(worker) = capture_first_alive_unit_by_id(app.world_mut(), team, "Worker") {
            return Some(worker);
        }
    }
    None
}

fn capture_nearest_visible_resource(
    world: &mut World,
    kind: ResourceKind,
    origin: Vec3,
) -> Option<(Entity, i32, Vec3)> {
    let mut resources = world.query::<(Entity, &ResourceNode, &Transform, &VisibilityState)>();
    resources
        .iter(world)
        .filter_map(|(entity, resource, transform, visibility)| {
            (resource.kind == kind && resource.amount > 0 && visibility.visible).then_some((
                entity,
                resource.amount,
                transform.translation,
            ))
        })
        .min_by(|(_, _, lhs), (_, _, rhs)| {
            xz_distance(*lhs, origin)
                .partial_cmp(&xz_distance(*rhs, origin))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn capture_nearest_supply_crate(
    world: &mut World,
    effect: SupplyCrateEffect,
    origin: Vec3,
) -> Option<(Entity, Vec3)> {
    let mut crates = world.query::<(Entity, &SupplyCrate, &Transform)>();
    crates
        .iter(world)
        .filter_map(|(entity, supply_crate, transform)| {
            (supply_crate.effect == effect).then_some((entity, transform.translation))
        })
        .min_by(|(_, lhs), (_, rhs)| {
            xz_distance(*lhs, origin)
                .partial_cmp(&xz_distance(*rhs, origin))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn capture_resource_amount(world: &World, entity: Entity) -> i32 {
    world
        .get::<ResourceNode>(entity)
        .map_or(0, |resource| resource.amount)
}

fn capture_entity_position(world: &World, entity: Entity) -> Option<Vec3> {
    world
        .get::<Transform>(entity)
        .map(|transform| transform.translation)
}

fn capture_entity_team(world: &World, entity: Entity) -> Option<Team> {
    world.get::<Team>(entity).copied()
}

fn capture_match_phase(world: &World) -> CaptureMatchPhase {
    match world.resource::<MatchState>().phase {
        MatchPhase::Running => CaptureMatchPhase::Running,
        MatchPhase::HumanDefeat => CaptureMatchPhase::HumanDefeat,
        MatchPhase::HumanVictory => CaptureMatchPhase::HumanVictory,
        MatchPhase::MatchFinished => CaptureMatchPhase::MatchFinished,
    }
}

fn capture_team_ore(world: &World, team: Team) -> i32 {
    world.resource::<Economies>().get(team).ore
}

fn capture_first_affordable_train_command(
    app: &mut App,
    team: Team,
) -> Option<(Entity, &'static str)> {
    let world = app.world_mut();
    let (ore, crystal) = {
        let economy = world.resource::<Economies>().get(team);
        (economy.ore, economy.crystal)
    };
    let mut slots = world.query::<(Entity, &CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    let mut candidates = slots
        .iter(world)
        .filter_map(|(entity, slot, action, availability)| match action {
            BuildAction::Train(product_id) if availability.enabled => {
                let def = registry::entity(product_id)?;
                (ore >= def.cost.ore && crystal >= def.cost.crystal).then_some((
                    slot.0,
                    entity,
                    *product_id,
                ))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(slot, _, _)| *slot);
    candidates
        .into_iter()
        .next()
        .map(|(_, entity, product_id)| (entity, product_id))
}

fn capture_train_unit_from_producer(
    app: &mut App,
    team: Team,
    producer_id: &'static str,
    product_id: &'static str,
    max_frames: usize,
    frames: &mut usize,
) -> Option<Entity> {
    let (producer, _, producer_position) =
        capture_constructed_producer(app.world_mut(), team, producer_id)?;
    if !capture_world_left_click(app, producer_position)
        || !app
            .world()
            .get_entity(producer)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
    {
        return None;
    }
    app.update();
    let button = capture_enabled_command_for_action(app, BuildAction::Train(product_id))?;
    let units_before = capture_alive_units_by_id(app.world_mut(), team, product_id);
    capture_click_command_button(app, button);
    while *frames < max_frames {
        *frames += 1;
        app.update();
        if let Some((entity, _)) = capture_alive_units_by_id(app.world_mut(), team, product_id)
            .into_iter()
            .find(|(entity, _)| !units_before.iter().any(|(before, _)| before == entity))
        {
            return Some(entity);
        }
    }
    None
}

fn capture_enabled_command_for_action(app: &mut App, target: BuildAction) -> Option<Entity> {
    let world = app.world_mut();
    let mut slots = world.query::<(Entity, &BuildAction, &CommandSlotAvailability)>();
    slots
        .iter(world)
        .find_map(|(entity, action, availability)| {
            (*action == target && availability.enabled).then_some(entity)
        })
}

fn capture_affordable_train_command_from_producer(
    app: &mut App,
    team: Team,
    producer_id: &'static str,
) -> Option<(Entity, &'static str)> {
    let (producer, _, producer_position) =
        capture_constructed_producer(app.world_mut(), team, producer_id)?;
    if !capture_world_left_click(app, producer_position)
        || !app
            .world()
            .get_entity(producer)
            .is_ok_and(|entity| entity.get::<Selected>().is_some())
    {
        return None;
    }
    app.update();
    capture_first_affordable_train_command(app, team)
}

fn capture_valid_structure_placement_point_near_team_base(
    app: &mut App,
    team: Team,
    id: &'static str,
) -> Option<Vec3> {
    let def = registry::entity(id)?;
    let bounds = *app.world().resource::<MapBounds>();
    let anchors = {
        let world = app.world_mut();
        let mut structures =
            world.query::<(&Structure, &Team, &Transform, Option<&UnderConstruction>)>();
        structures
            .iter(world)
            .filter_map(
                |(structure, structure_team, transform, under_construction)| {
                    if *structure_team != team || !structure_is_constructed(under_construction) {
                        return None;
                    }
                    let structure_def = registry::entity(structure.id)?;
                    Some((transform.translation, structure_def.radius))
                },
            )
            .collect::<Vec<_>>()
    };
    if anchors.is_empty() {
        return None;
    }
    let occupiers = {
        let world = app.world_mut();
        let mut occupiers = world.query_filtered::<(
            &Transform,
            &Selectable,
            Option<&Health>,
            Option<&ResourceNode>,
        ), Or<(With<Unit>, With<Structure>, With<ResourceNode>)>>(
        );
        occupiers
            .iter(world)
            .filter_map(|(transform, selectable, health, resource_node)| {
                if health.is_some_and(|health| health.current <= 0.0)
                    || resource_node.is_some_and(|resource| resource.amount <= 0)
                {
                    return None;
                }
                Some((transform.translation, selectable.radius))
            })
            .collect::<Vec<_>>()
    };

    let sample_count = 24;
    for (anchor, anchor_radius) in anchors.iter().copied() {
        for ring in 0..20 {
            let radius = anchor_radius + def.radius + 1.0 + ring as f32 * 0.75;
            for sample in 0..sample_count {
                let angle = (sample as f32 / sample_count as f32) * std::f32::consts::TAU
                    + ring as f32 * 0.23;
                let candidate = bounds.clamp_ground_point(
                    anchor + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
                    def.radius,
                );
                if !map_contains_ground_point_in_bounds(candidate, bounds) {
                    continue;
                }
                let in_base_radius = anchors.iter().any(|(base, base_radius)| {
                    xz_distance(*base, candidate)
                        <= *base_radius + def.radius + BASE_CONSTRUCTION_RADIUS_M
                });
                if !in_base_radius {
                    continue;
                }
                let collides = occupiers.iter().any(|(position, radius)| {
                    xz_distance(*position, candidate) <= *radius + def.radius
                });
                if !collides {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn capture_structure_by_id_near(
    world: &mut World,
    team: Team,
    id: &'static str,
    position: Vec3,
    constructed: Option<bool>,
) -> Option<(Entity, Vec3, bool)> {
    let mut structures = world.query::<(
        Entity,
        &Structure,
        &Team,
        &Transform,
        Option<&UnderConstruction>,
    )>();
    structures.iter(world).find_map(
        |(entity, structure, structure_team, transform, under_construction)| {
            let is_constructed = structure_is_constructed(under_construction);
            (structure.id == id
                && *structure_team == team
                && xz_distance(transform.translation, position) < 0.05
                && constructed.is_none_or(|expected| expected == is_constructed))
            .then_some((entity, transform.translation, is_constructed))
        },
    )
}

fn capture_nearest_structure_by_id_and_team(
    world: &mut World,
    team: Team,
    id: &'static str,
    origin: Vec3,
    constructed: Option<bool>,
) -> Option<(Entity, Vec3, bool)> {
    let mut structures = world.query::<(
        Entity,
        &Structure,
        &Team,
        &Transform,
        Option<&UnderConstruction>,
    )>();
    structures
        .iter(world)
        .filter_map(
            |(entity, structure, structure_team, transform, under_construction)| {
                let is_constructed = structure_is_constructed(under_construction);
                (structure.id == id
                    && *structure_team == team
                    && constructed.is_none_or(|expected| expected == is_constructed))
                .then_some((entity, transform.translation, is_constructed))
            },
        )
        .min_by(|(_, lhs, _), (_, rhs, _)| {
            xz_distance(*lhs, origin)
                .partial_cmp(&xz_distance(*rhs, origin))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn capture_structure_constructed(world: &World, entity: Entity) -> bool {
    world.get_entity(entity).is_ok_and(|entity_ref| {
        entity_ref
            .get::<Health>()
            .is_some_and(|health| health.current > 0.0)
            && entity_ref.get::<Structure>().is_some()
            && entity_ref.get::<UnderConstruction>().is_none()
    })
}

fn capture_click_command_button(app: &mut App, button: Entity) {
    app.world_mut()
        .entity_mut(button)
        .insert(Interaction::Pressed);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
        mouse.clear();
    }
    app.world_mut().entity_mut(button).insert(Interaction::None);
    app.update();
}

fn capture_world_left_click(app: &mut App, position: Vec3) -> bool {
    capture_world_mouse_click(app, position, MouseButton::Left)
}

fn capture_world_left_click_projected(app: &mut App, position: Vec3) -> bool {
    capture_world_mouse_click_projected(app, position, MouseButton::Left)
}

fn capture_world_left_click_additive(app: &mut App, position: Vec3) -> bool {
    capture_world_mouse_click_with_shift(app, position, MouseButton::Left, true)
}

fn capture_world_right_click(app: &mut App, position: Vec3) -> bool {
    capture_world_mouse_click(app, position, MouseButton::Right)
}

fn capture_world_mouse_click(app: &mut App, position: Vec3, button: MouseButton) -> bool {
    capture_world_mouse_click_with_shift(app, position, button, false)
}

fn capture_world_mouse_click_projected(app: &mut App, position: Vec3, button: MouseButton) -> bool {
    if !capture_attach_window_to_main_camera(app, position) {
        return false;
    }
    let Some(cursor) = capture_screen_position_for_world(app, position) else {
        return false;
    };
    if !capture_set_cursor(app, cursor) {
        return false;
    }
    let cursor_blocked = {
        let world = app.world_mut();
        let mut window_q = world.query_filtered::<&Window, With<PrimaryWindow>>();
        let Ok(window) = window_q.single(world) else {
            return false;
        };
        match button {
            MouseButton::Left => cursor_is_over_hud(window),
            MouseButton::Right => cursor_blocks_world_order_controls(window, cursor),
            _ => false,
        }
    };
    if cursor_blocked {
        return false;
    }
    {
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.release(KeyCode::ShiftLeft);
        keyboard.release(KeyCode::ShiftRight);
        keyboard.clear();
    }
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(button);
    }
    app.update();
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(button);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    true
}

fn capture_world_mouse_click_with_shift(
    app: &mut App,
    position: Vec3,
    button: MouseButton,
    shift_pressed: bool,
) -> bool {
    if !capture_attach_window_to_main_camera(app, position) {
        return false;
    }
    let Some(cursor) = capture_screen_position_for_world(app, position) else {
        return false;
    };
    if !capture_set_cursor(app, cursor) {
        return false;
    }
    let cursor_blocked = {
        let world = app.world_mut();
        let mut window_q = world.query_filtered::<&Window, With<PrimaryWindow>>();
        let Ok(window) = window_q.single(world) else {
            return false;
        };
        match button {
            MouseButton::Left => cursor_is_over_hud(window),
            MouseButton::Right => cursor_blocks_world_order_controls(window, cursor),
            _ => false,
        }
    };
    if cursor_blocked {
        return false;
    }
    let Some(ground_position) = capture_ground_position_for_cursor(app) else {
        return false;
    };
    if xz_distance(ground_position, position) > 0.05 {
        return false;
    }
    {
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if shift_pressed {
            keyboard.press(KeyCode::ShiftLeft);
        } else {
            keyboard.release(KeyCode::ShiftLeft);
        }
        keyboard.release(KeyCode::ShiftRight);
        keyboard.clear();
    }
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(button);
    }
    app.update();
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(button);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    {
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.release(KeyCode::ShiftLeft);
        keyboard.clear();
    }
    true
}

fn capture_attach_window_to_main_camera(app: &mut App, focus: Vec3) -> bool {
    let window_entity = {
        let world = app.world_mut();
        let mut window_q = world.query_filtered::<Entity, With<PrimaryWindow>>();
        window_q.iter(world).next()
    };
    if let Some(window_entity) = window_entity {
        if let Some(mut window) = app
            .world_mut()
            .entity_mut(window_entity)
            .get_mut::<Window>()
        {
            window.set_cursor_position(Some(Vec2::new(640.0, 360.0)));
        }
    } else {
        let mut window = Window {
            resolution: WindowResolution::new(1280, 720),
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(640.0, 360.0)));
        app.world_mut().spawn((window, PrimaryWindow));
    }

    let camera_state = RtsCamera::focused_on(focus);
    let camera_transform = camera_transform_from_state(&camera_state);
    let mut projection = camera_projection_from_state(&camera_state);
    projection.update(1280.0, 720.0);
    *app.world_mut().resource_mut::<RtsCamera>() = camera_state;

    let world = app.world_mut();
    let mut camera_q = world.query_filtered::<(
        &mut Camera,
        &mut Transform,
        &mut GlobalTransform,
        &mut Projection,
    ), With<MainCamera>>();
    let Ok((mut camera, mut transform, mut global_transform, mut current_projection)) =
        camera_q.single_mut(world)
    else {
        return false;
    };
    camera.viewport = Some(Viewport {
        physical_size: UVec2::new(1280, 720),
        ..default()
    });
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: UVec2::new(1280, 720),
        scale_factor: 1.0,
    });
    camera.computed.clip_from_view = projection.get_clip_from_view();
    *transform = camera_transform;
    *global_transform = GlobalTransform::from(camera_transform);
    *current_projection = projection;
    true
}

fn capture_set_cursor(app: &mut App, cursor: Vec2) -> bool {
    let world = app.world_mut();
    let mut window_q = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    let Ok(mut window) = window_q.single_mut(world) else {
        return false;
    };
    window.set_cursor_position(Some(cursor));
    true
}

fn capture_screen_position_for_world(app: &mut App, position: Vec3) -> Option<Vec2> {
    let world = app.world_mut();
    let mut camera_q = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
    let (camera, camera_transform) = camera_q.single(world).ok()?;
    camera.world_to_viewport(camera_transform, position).ok()
}

fn capture_ground_position_for_cursor(app: &mut App) -> Option<Vec3> {
    let cursor = {
        let world = app.world_mut();
        let mut window_q = world.query_filtered::<&Window, With<PrimaryWindow>>();
        window_q.single(world).ok()?.cursor_position()?
    };
    let world = app.world_mut();
    let mut camera_q = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
    let (camera, camera_transform) = camera_q.single(world).ok()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
}

const CAPTURE_PLAYER_TEAM: Team = Team::Player(0);

fn capture_match_setup_for_faction(faction: CaptureProofFaction) -> MatchSetupSettings {
    let mut lobby_controllers = DEFAULT_LOBBY_CONTROLLERS;
    let mut lobby_factions = DEFAULT_LOBBY_FACTIONS;
    for (slot, controller) in lobby_controllers.iter_mut().enumerate() {
        if slot == 0 {
            *controller = SkirmishPlayerController::Human;
        } else if controller.is_human() {
            *controller = SkirmishPlayerController::Ai(AiDifficulty::Easy);
        }
    }
    lobby_factions[0] = faction.skirmish_faction();
    let selection = SkirmishMenuSelection {
        map_index: 0,
        starting_resource_index: DEFAULT_STARTING_RESOURCE_INDEX,
        match_mode: SkirmishMatchMode::OneVsOne,
        ai_difficulty: AiDifficulty::Easy,
        lobby_controllers,
        lobby_factions,
        lobby_team_ids: DEFAULT_LOBBY_TEAM_IDS,
        lobby_color_slots: DEFAULT_LOBBY_COLOR_SLOTS,
    };
    selection.match_setup_with_map_seed(0)
}

fn capture_stats_for_team(snapshot: &CaptureMatchSnapshot, team: Team) -> CaptureTeamStats {
    team.economy_index()
        .and_then(|index| snapshot.players.get(index).copied())
        .unwrap_or_default()
}

fn capture_team_from_team(team: Team) -> CaptureTeam {
    match team {
        Team::Player(index) => CaptureTeam::Player(index),
        Team::Neutral => CaptureTeam::Neutral,
    }
}

fn capture_primary_active_enemy_team(world: &World, player_team: Team) -> Option<Team> {
    let relations = world.resource::<TeamRelations>();
    let active_teams = world.resource::<ActiveTeams>();
    player_teams(active_teams.0.len()).find(|team| {
        team_is_active(*team, Some(active_teams)) && relations.are_enemies(player_team, *team)
    })
}

fn capture_alive_unit_count(world: &mut World, team: Team) -> usize {
    let mut units = world.query::<(&Team, &Health, &Unit)>();
    units
        .iter(world)
        .filter(|(unit_team, health, _)| **unit_team == team && health.current > 0.0)
        .count()
}

fn capture_team_total_health(world: &mut World, team: Team) -> f32 {
    let mut health_q = world.query::<(&Team, &Health)>();
    health_q
        .iter(world)
        .filter_map(|(entity_team, health)| {
            (*entity_team == team && health.current > 0.0).then_some(health.current)
        })
        .sum()
}

fn capture_attack_orders_against_team(
    world: &mut World,
    attacker_team: Team,
    target_team: Team,
) -> usize {
    let mut units = world.query::<(&Team, &Health, &AttackOrder)>();
    let targets = units
        .iter(world)
        .filter_map(|(team, health, attack_order)| {
            (*team == attacker_team && health.current > 0.0).then_some(attack_order.target)
        })
        .collect::<Vec<_>>();
    targets
        .into_iter()
        .filter(|target| {
            world
                .get::<Team>(*target)
                .is_some_and(|team| *team == target_team)
        })
        .count()
}

fn capture_queue_player_units(
    app: &mut App,
    team: Team,
    product_id: &'static str,
    target_units: usize,
) -> usize {
    let world = app.world_mut();
    let produced = capture_unit_count_by_id(world, team, product_id);
    let queued = capture_queued_train_jobs(world, team, product_id);
    let Some((factory, producer_id, origin)) =
        capture_constructed_producer(world, team, "VehicleFactory")
    else {
        return 0;
    };
    let queue_len = {
        let build_queue = world.resource::<BuildQueue>();
        producer_build_queue_len(build_queue, factory)
    };
    let capacity = PRODUCTION_QUEUE_LIMIT.saturating_sub(queue_len);
    let needed = target_units.saturating_sub(produced + queued).min(capacity);
    if needed == 0 {
        return 0;
    }

    let Some(def) = registry::entity(product_id) else {
        return 0;
    };
    {
        let mut economies = world.resource_mut::<Economies>();
        let economy = economies.get_mut(team);
        economy.ore = economy.ore.max(def.cost.ore * needed as i32 + 120);
        economy.crystal = economy.crystal.max(def.cost.crystal * needed as i32 + 120);
    }
    {
        let mut build_queue = world.resource_mut::<BuildQueue>();
        for _ in 0..needed {
            build_queue.0.push(BuildJob {
                team,
                action: BuildAction::Train(product_id),
                producer_entity: factory,
                producer_id,
                timer: 0.25,
                origin,
            });
        }
    }
    needed
}

fn capture_constructed_producer(
    world: &mut World,
    team: Team,
    id: &'static str,
) -> Option<(Entity, &'static str, Vec3)> {
    let mut query = world.query::<(
        Entity,
        &Structure,
        &Team,
        &Transform,
        Option<&UnderConstruction>,
    )>();
    query.iter(world).find_map(
        |(entity, structure, structure_team, transform, under_construction)| {
            (*structure_team == team
                && structure.id == id
                && structure_is_constructed(under_construction))
            .then_some((entity, structure.id, transform.translation))
        },
    )
}

fn capture_unit_count_by_id(world: &mut World, team: Team, id: &'static str) -> usize {
    let mut query = world.query::<(&Team, &Unit, &Health)>();
    query
        .iter(world)
        .filter(|(unit_team, unit, health)| {
            **unit_team == team && unit.id == id && health.current > 0.0
        })
        .count()
}

fn capture_queued_train_jobs(world: &World, team: Team, product_id: &'static str) -> usize {
    world
        .resource::<BuildQueue>()
        .0
        .iter()
        .filter(|job| {
            job.team == team && matches!(job.action, BuildAction::Train(id) if id == product_id)
        })
        .count()
}

fn capture_order_player_attackers(app: &mut App, player_team: Team) -> usize {
    let Some((target, _)) = capture_first_enemy_anchor(app.world_mut(), player_team) else {
        return 0;
    };
    if let Ok(mut entity) = app.world_mut().get_entity_mut(target) {
        entity.insert((VisibilityState { visible: true }, Visibility::Visible));
    }
    let attackers = capture_alive_attackers(app.world_mut(), player_team);
    for attacker in &attackers {
        if let Ok(mut entity) = app.world_mut().get_entity_mut(*attacker) {
            entity.remove::<(
                MoveOrder,
                FollowOrder,
                CaptureOrder,
                GarrisonOrder,
                HarvestOrder,
                RepairOrder,
                ConstructOrder,
                AttackMoveOrder,
                PatrolOrder,
                OrderQueue,
            )>();
            entity.insert(AttackOrder { target });
        }
    }
    attackers.len()
}

fn capture_alive_attackers(world: &mut World, team: Team) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Team, &Health, &Weapon)>();
    query
        .iter(world)
        .filter_map(|(entity, unit_team, health, _)| {
            (*unit_team == team && health.current > 0.0).then_some(entity)
        })
        .collect()
}

fn capture_mouse_order_attackers_to_target(
    app: &mut App,
    attackers: &[Entity],
    target: Entity,
    target_position: Vec3,
) -> usize {
    let positions = attackers
        .iter()
        .filter_map(|entity| {
            app.world().get_entity(*entity).ok().and_then(|entity_ref| {
                let health = entity_ref.get::<Health>()?;
                let transform = entity_ref.get::<Transform>()?;
                (health.current > 0.0).then_some((*entity, transform.translation))
            })
        })
        .collect::<Vec<_>>();

    if positions.is_empty() {
        return 0;
    }

    for (index, (_, position)) in positions.iter().copied().enumerate() {
        let selected = if index == 0 {
            capture_world_left_click(app, position)
        } else {
            capture_world_left_click_additive(app, position)
        };
        if !selected {
            continue;
        }
    }

    let selected_attackers = positions
        .iter()
        .filter(|(entity, _)| {
            app.world()
                .get_entity(*entity)
                .is_ok_and(|entity_ref| entity_ref.get::<Selected>().is_some())
        })
        .map(|(entity, _)| *entity)
        .collect::<Vec<_>>();
    if selected_attackers.is_empty() {
        return 0;
    }

    let target_health_before = app
        .world()
        .get::<Health>(target)
        .map(|health| health.current);
    if !capture_world_right_click(app, target_position) {
        return 0;
    }
    let target_health_after = app
        .world()
        .get::<Health>(target)
        .map(|health| health.current);
    let target_destroyed = app.world().get_entity(target).is_err();
    let target_damaged = target_health_before
        .zip(target_health_after)
        .is_some_and(|(before, after)| after < before);

    selected_attackers
        .iter()
        .filter(|attacker| {
            target_destroyed
                || target_damaged
                || app.world().get_entity(**attacker).is_ok_and(|entity| {
                    entity
                        .get::<AttackOrder>()
                        .is_some_and(|order| order.target == target)
                        || entity
                            .get::<MoveOrder>()
                            .is_some_and(|order| xz_distance(order.target, target_position) <= 8.0)
                })
        })
        .count()
}

fn capture_first_enemy_anchor(world: &mut World, player_team: Team) -> Option<(Entity, Vec3)> {
    let relations = world.resource::<TeamRelations>().clone();
    let mut best = None;
    {
        let mut structures = world.query::<(Entity, &Structure, &Team, &Transform, &Health)>();
        for (entity, structure, team, transform, health) in structures.iter(world) {
            if health.current > 0.0
                && relations.are_enemies(player_team, *team)
                && is_structure_elimination_anchor(structure)
            {
                best = Some((entity, transform.translation));
                break;
            }
        }
    }
    if best.is_some() {
        return best;
    }
    let mut units = world.query::<(Entity, &Unit, &Team, &Transform, &Health)>();
    units
        .iter(world)
        .find_map(|(entity, unit, team, transform, health)| {
            (health.current > 0.0
                && relations.are_enemies(player_team, *team)
                && is_worker_elimination_anchor(unit))
            .then_some((entity, transform.translation))
        })
}

fn cleanup_match_scoped_entities(
    mut commands: Commands,
    entities: Query<Entity, With<MatchScopedEntity>>,
) {
    for entity in &entities {
        commands.entity(entity).try_despawn();
    }
}

fn stop_match_flow_on_exit(mut match_flow: ResMut<MatchFlow>, mut menu: ResMut<MatchMenuState>) {
    match_flow.active = false;
    menu.visible = false;
}

fn reset_match_speed_on_exit(
    mut match_speed: ResMut<MatchSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    *match_speed = MatchSpeed::default();
    virtual_time.set_relative_speed(MatchSpeedPreset::Normal.scale());
}

fn advance_match_restart(mut next_state: ResMut<NextState<AppScreen>>) {
    next_state.set(AppScreen::InMatch);
}

fn add_runtime_systems(app: &mut App) -> &mut App {
    app.add_systems(
        Update,
        match_menu_input
            .in_set(SimulationPhase::UiAndManagement)
            .before(update_command_mode)
            .run_if(in_state(AppScreen::InMatch)),
    );
    app.add_systems(
        Update,
        (match_menu_buttons, update_match_menu_overlay)
            .chain()
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(in_state(AppScreen::InMatch)),
    );
    app.add_systems(
        Update,
        (match_briefing_buttons, update_match_briefing_overlay)
            .chain()
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(in_state(AppScreen::InMatch)),
    );
    app.add_systems(
        Update,
        structure_placement_input
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    );
    app.add_systems(
        Update,
        (
            rotate_camera
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            camera_control
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            minimap_input
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            update_command_mode
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            select_entities
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            selection_hotkeys
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            focus_latest_battle_event
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            issue_orders
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            command_queue_controls
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            refresh_command_panel
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            command_shortcuts
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            command_buttons
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            production_queue_slot_buttons
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            update_deploy_mode_requests
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            update_support_cooldowns
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            economy_tick
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            ai_director
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            auto_assign_ai_construction_workers
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            auto_assign_ai_resource_collectors
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            auto_assign_ai_supply_crate_collectors
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        battle_log_entry_buttons
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_selection_drag_box
            .in_set(SimulationPhase::UiAndManagement)
            .after(select_entities)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        restore_ai_attack_wave_orders
            .after(ai_director)
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_ai_tech_bunker_garrisons
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_rally_point_targets
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        (
            monitor_low_power_audio_feedback.run_if(match_in_progress),
            play_pending_audio_feedback,
            update_match_clock.run_if(match_in_progress),
        )
            .chain()
            .in_set(SimulationPhase::UiAndManagement),
    )
    .configure_sets(
        Update,
        (
            SimulationPhase::UiAndManagement,
            SimulationPhase::BuildProcessing,
            SimulationPhase::Combat,
            SimulationPhase::PostCombat,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (process_build_queue, progress_under_construction_structures)
            .chain()
            .in_set(SimulationPhase::BuildProcessing)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        clear_emp_disabled_orders
            .in_set(SimulationPhase::Combat)
            .before(update_attack_move_and_patrol_orders)
            .before(update_ai_siege_drill_deploy_mode)
            .before(chase_attack_targets)
            .before(update_capture_orders)
            .before(update_garrison_orders)
            .before(update_harvest_orders)
            .before(update_follow_orders)
            .before(update_repair_orders)
            .before(update_construct_orders)
            .before(move_units)
            .before(progress_queued_orders)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        (
            update_attack_move_and_patrol_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_ai_siege_drill_deploy_mode
                .in_set(SimulationPhase::Combat)
                .before(chase_attack_targets)
                .run_if(match_in_progress),
            chase_attack_targets
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_capture_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_garrison_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_harvest_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_manual_structure_repairs
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_mine_layers
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_follow_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_repair_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_construct_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            move_units
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_mines
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            collect_supply_crates
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            update_veterancy_regeneration
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            combat
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            progress_queued_orders
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        (
            update_support_effects.run_if(match_in_progress),
            update_repair_and_healing_auras.run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        cleanup_dead_entities
            .before(evaluate_match_end)
            .in_set(SimulationPhase::PostCombat)
            .run_if(match_in_progress)
            .run_if(resource_exists::<AssetServer>),
    )
    .add_systems(
        Update,
        (
            apply_kill_credits
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            evaluate_match_end
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_visibility
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_pulses
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_click_markers
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_combat_wreckage
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_structure_destruction_vfx
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_veterancy_promotion_effects
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_battle_log
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_minimap
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_hud
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            draw_world_overlays
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        update_objective_tracker_hud
            .in_set(SimulationPhase::PostCombat)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        (match_end_buttons, update_match_end_overlay)
            .chain()
            .run_if(in_state(AppScreen::InMatch)),
    )
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource)]
struct RtsCamera {
    focus: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
    edge_pan_active: bool,
}

impl Default for RtsCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            distance: CAMERA_DEFAULT_DISTANCE,
            yaw: CAMERA_DEFAULT_YAW,
            pitch: CAMERA_DEFAULT_PITCH,
            edge_pan_active: false,
        }
    }
}

impl RtsCamera {
    fn focused_on(focus: Vec3) -> Self {
        Self { focus, ..default() }
    }
}

#[derive(Resource, Default)]
struct CameraMouseRotation {
    active: bool,
    start_yaw: f32,
    accumulated_x: f32,
    last_middle_press_time: Option<f32>,
}

fn camera_transform_from_state(camera: &RtsCamera) -> Transform {
    let offset = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0)
        * Vec3::new(0.0, 0.0, camera.distance);
    Transform::from_translation(camera.focus + offset).looking_at(camera.focus, Vec3::Y)
}

fn camera_projection_from_state(camera: &RtsCamera) -> Projection {
    Projection::Orthographic(OrthographicProjection {
        near: CAMERA_NEAR_PLANE,
        far: CAMERA_FAR_PLANE,
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: camera.distance,
        },
        ..OrthographicProjection::default_3d()
    })
}

#[derive(Resource, Default, Debug)]
struct SelectionDragState {
    active: bool,
    dragging: bool,
    start: Vec2,
    started_in_hud: bool,
}

#[derive(Component)]
struct SelectionDragBox;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScreenRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

#[derive(Resource)]
struct UnitGroups {
    slots: [Vec<Entity>; 9],
    last_accessed: Option<usize>,
}

impl Default for UnitGroups {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| Vec::new()),
            last_accessed: None,
        }
    }
}

#[derive(Resource)]
struct CameraBookmarks {
    slots: [Option<CameraBookmark>; 4],
}

impl Default for CameraBookmarks {
    fn default() -> Self {
        Self {
            slots: [None, None, None, None],
        }
    }
}

#[derive(Clone, Copy)]
struct CameraBookmark {
    focus: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

impl CameraBookmark {
    fn capture(camera: &RtsCamera) -> Self {
        Self {
            focus: camera.focus,
            distance: camera.distance,
            yaw: camera.yaw,
            pitch: camera.pitch,
        }
    }

    fn restore(self, camera: &mut RtsCamera) {
        camera.focus = self.focus;
        camera.distance = self.distance;
        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
    }

    fn restore_safely(self, camera: &mut RtsCamera, bounds: MapBounds) {
        self.restore(camera);
        clamp_camera_view_safely(camera, bounds);
    }
}

#[derive(Resource, Debug)]
struct DoubleClickState {
    last_click_time: f32,
    last_unit: Option<Entity>,
    last_unit_type: Option<&'static str>,
}

#[derive(Resource, Default)]
struct LatestBattleEvent {
    focus: Option<Vec3>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BattleEventPingKind {
    Generic,
    SupportPower,
    EnemySupportPower,
    EnemySuperweapon,
}

#[derive(Clone)]
struct BattleLogEntry {
    message: String,
    remaining: f32,
    focus: Option<Vec3>,
    ping_kind: BattleEventPingKind,
    minimap_ping_active: bool,
    minimap_ping_remaining: f32,
}

#[derive(Resource, Default)]
struct BattleLog {
    entries: VecDeque<BattleLogEntry>,
    under_attack_cooldown: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitVoiceEvent {
    Hello,
    Ack1,
    Ack2,
    Training,
    UnitReady,
    ConstructionComplete,
    NotEnoughResources,
    SupportPowerReady,
    SupportPowerFired,
    EnemySupportPowerFired,
    EnemySuperweaponReady,
    EnemySuperweaponLaunched,
    Victory,
    Defeat,
    BaseUnderAttack,
    UnitUnderAttack,
    UnitLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SoundEffectKind {
    Select,
    Command,
    ProductionStart,
    ProductionReady,
    ConstructionStarted,
    ConstructionCanceled,
    Error,
    LowPower,
    RepairStarted,
    StructureCaptured,
    StructureLost,
    StructureSold,
    SupplyCrate,
    UnitPromoted,
    SupportPowerReady,
    SupportPowerFire,
    SuperweaponWarning,
    WeaponHit,
    Explosion,
}

#[derive(Resource)]
struct AudioFeedback {
    pending_voice: Option<UnitVoiceEvent>,
    pending_sound: Option<SoundEffectKind>,
    last_voice: Option<UnitVoiceEvent>,
    last_sound: Option<SoundEffectKind>,
    last_command_key: Option<&'static str>,
    last_low_power: Option<bool>,
    next_ack_is_first: bool,
}

impl Default for AudioFeedback {
    fn default() -> Self {
        Self {
            pending_voice: None,
            pending_sound: None,
            last_voice: None,
            last_sound: None,
            last_command_key: None,
            last_low_power: None,
            next_ack_is_first: true,
        }
    }
}

#[derive(Resource, Default)]
struct KillCredits(Vec<Entity>);

impl Default for DoubleClickState {
    fn default() -> Self {
        Self {
            last_click_time: -1000.0,
            last_unit: None,
            last_unit_type: None,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Team {
    Player(usize),
    Neutral,
}

impl Team {
    #[allow(dead_code)]
    fn faction_id(self) -> &'static str {
        match self {
            Team::Player(_) => "player",
            Team::Neutral => "neutral",
        }
    }

    fn index(self) -> usize {
        match self {
            Team::Player(index) => index,
            Team::Neutral => usize::MAX,
        }
    }

    fn economy_index(self) -> Option<usize> {
        match self {
            Team::Player(index) => Some(index),
            Team::Neutral => None,
        }
    }

    fn label(self) -> String {
        match self {
            Team::Player(index) => format!("玩家{}", index + 1),
            Team::Neutral => "中立".to_string(),
        }
    }

    fn from_playable_index(index: usize) -> Option<Self> {
        Some(Team::Player(index))
    }
}

fn player_teams(count: usize) -> impl Iterator<Item = Team> {
    (0..count).map(Team::Player)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerVisibilityMode {
    PerPlayer,
    AllPlayers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerControlMode {
    Player,
    Spectator,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct VisiblePlayer {
    team: Team,
    visibility: PlayerVisibilityMode,
    control: PlayerControlMode,
}

impl VisiblePlayer {
    fn per_player(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::PerPlayer,
            control: PlayerControlMode::Player,
        }
    }

    fn all_players(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::AllPlayers,
            control: PlayerControlMode::Spectator,
        }
    }

    #[cfg(test)]
    fn spectator_per_player(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::PerPlayer,
            control: PlayerControlMode::Spectator,
        }
    }

    fn all_players_visible(self) -> bool {
        self.visibility == PlayerVisibilityMode::AllPlayers
    }

    fn is_spectator(self) -> bool {
        self.control == PlayerControlMode::Spectator
    }
}

impl Default for VisiblePlayer {
    fn default() -> Self {
        Self {
            team: Team::Player(0),
            visibility: PlayerVisibilityMode::PerPlayer,
            control: PlayerControlMode::Player,
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct TeamRelations {
    allied: Vec<Vec<bool>>,
}

impl Default for TeamRelations {
    fn default() -> Self {
        Self { allied: Vec::new() }
    }
}

impl TeamRelations {
    fn ensure_player_count(&mut self, count: usize) {
        if self.allied.len() < count {
            let old_len = self.allied.len();
            for row in &mut self.allied {
                row.resize(count, false);
            }
            self.allied.resize_with(count, || vec![false; count]);
            for index in old_len..count {
                self.allied[index][index] = true;
            }
        }
    }

    fn set_allied(&mut self, a: Team, b: Team, allied: bool) {
        let (Some(a), Some(b)) = (a.economy_index(), b.economy_index()) else {
            return;
        };
        self.ensure_player_count(a.max(b) + 1);
        self.allied[a][b] = allied;
        self.allied[b][a] = allied;
    }

    fn are_allied(&self, a: Team, b: Team) -> bool {
        if a == b {
            return true;
        }
        let (Some(a), Some(b)) = (a.economy_index(), b.economy_index()) else {
            return false;
        };
        self.allied
            .get(a)
            .and_then(|row| row.get(b))
            .copied()
            .unwrap_or(false)
    }

    fn are_enemies(&self, a: Team, b: Team) -> bool {
        a.economy_index().is_some() && b.economy_index().is_some() && !self.are_allied(a, b)
    }
}

fn player_color(slot: usize) -> Color {
    let [r, g, b] = player_color_rgb(slot);
    Color::srgb(r, g, b)
}

fn player_color_with_alpha(slot: usize, alpha: f32) -> Color {
    let [r, g, b] = player_color_rgb(slot);
    Color::srgba(r, g, b, alpha)
}

fn player_color_rgb(slot: usize) -> [f32; 3] {
    PLAYER_COLOR_PALETTE[slot % PLAYER_COLOR_PALETTE.len()]
}

#[derive(Component, Clone, Copy)]
struct Selectable {
    radius: f32,
}

#[derive(Component)]
struct Selected;

#[derive(Component, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

impl Health {
    fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    fn ratio(self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }
}

#[derive(Component, Clone, Copy)]
struct Unit {
    id: &'static str,
    speed: f32,
    can_crush: bool,
    can_be_crushed: bool,
}

#[derive(Component, Clone, Copy)]
struct Structure {
    id: &'static str,
}

#[derive(Component, Clone, Copy)]
struct IncomeSource {
    ore: i32,
    crystal: i32,
    interval: f32,
    remaining: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceKind {
    Ore,
    Crystal,
}

impl ResourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Ore => "Ore",
            Self::Crystal => "Crystal",
        }
    }

    fn collect_seconds(self) -> f32 {
        match self {
            Self::Ore => 1.0,
            Self::Crystal => 2.0,
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Ore => Color::srgb(0.18, 0.52, 1.0),
            Self::Crystal => Color::srgb(0.95, 0.18, 0.16),
        }
    }
}

#[derive(Component, Clone, Copy)]
struct ResourceNode {
    kind: ResourceKind,
    amount: i32,
}

#[derive(Component, Clone, Copy)]
struct ResourceCargo {
    capacity: i32,
    ore: i32,
    crystal: i32,
}

impl ResourceCargo {
    fn total(self) -> i32 {
        self.ore + self.crystal
    }

    fn is_full(self) -> bool {
        self.total() >= self.capacity
    }

    fn has_any(self) -> bool {
        self.total() > 0
    }

    fn add_one(&mut self, kind: ResourceKind) -> bool {
        if self.is_full() {
            return false;
        }
        match kind {
            ResourceKind::Ore => self.ore += 1,
            ResourceKind::Crystal => self.crystal += 1,
        }
        true
    }

    fn clear(&mut self) -> (i32, i32) {
        let carried = (self.ore, self.crystal);
        self.ore = 0;
        self.crystal = 0;
        carried
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupplyCrateEffect {
    Resources,
    Repair,
    Veterancy,
}

impl SupplyCrateEffect {
    fn label(self) -> &'static str {
        match self {
            Self::Resources => "资源补给",
            Self::Repair => "维修补给",
            Self::Veterancy => "老兵补给",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Resources => Color::srgb(0.24, 0.7, 1.0),
            Self::Repair => Color::srgb(0.25, 0.95, 0.48),
            Self::Veterancy => Color::srgb(1.0, 0.85, 0.22),
        }
    }
}

#[derive(Component, Clone, Copy)]
struct SupplyCrate {
    effect: SupplyCrateEffect,
    pickup_radius: f32,
    resource_ore: i32,
    resource_crystal: i32,
    repair_radius: f32,
    repair_amount: f32,
}

#[derive(Component, Clone, Copy)]
struct Veterancy {
    rank: u8,
    experience_points: u32,
    base_health: f32,
    base_damage: f32,
    base_range: f32,
    base_vision: f32,
}

#[derive(Component, Clone, Copy)]
struct Mine {
    damage: f32,
    trigger_radius: f32,
    blast_radius: f32,
    arming_remaining: f32,
    source: Option<Entity>,
}

#[derive(Component, Clone, Copy)]
struct MineLayer {
    damage: f32,
    deploy_interval: f32,
    deploy_radius: f32,
    spacing: f32,
    limit: usize,
    cooldown: f32,
    deploy_index: usize,
}

#[derive(Component, Clone, Copy)]
struct Garrison {
    capacity: usize,
    damage_per_unit: f32,
    count: usize,
}

#[derive(Component, Clone, Copy)]
struct MoveOrder {
    target: Vec3,
}

#[derive(Component, Clone, Copy)]
struct FollowOrder {
    target: Entity,
    allow_enemy: bool,
    offset: Vec3,
}

#[derive(Clone)]
enum UnitQueuedOrder {
    Move(Vec3),
    Attack(Entity),
    Capture(Entity),
    Garrison(Entity),
    Harvest { target: Entity, state: HarvestState },
    Repair(Entity),
    Construct(Entity),
    Follow { target: Entity, offset: Vec3 },
    AttackMove(Vec3),
    Patrol { origin: Vec3, destination: Vec3 },
    ForceFollow { target: Entity, offset: Vec3 },
}

#[derive(Clone, Copy)]
struct OrderTargetChoices {
    supply_crate_position: Option<Vec3>,
    resource_target: Option<Entity>,
    resource_dropoff_target: Option<Entity>,
    enemy_target: Option<Entity>,
    repair_target: Option<Entity>,
    construct_target: Option<Entity>,
    garrison_target: Option<Entity>,
    follow_target: Option<Entity>,
}

impl OrderTargetChoices {
    fn force_follow_target(self) -> Option<Entity> {
        self.enemy_target
            .or(self.resource_target)
            .or(self.resource_dropoff_target)
            .or(self.repair_target)
            .or(self.construct_target)
            .or(self.garrison_target)
            .or(self.follow_target)
    }
}

#[derive(Clone, Copy)]
struct UnitOrderContext {
    force_move: bool,
    enemy_target_capturable: bool,
    attack_move: bool,
    patrol: bool,
    origin: Vec3,
    point: Vec3,
    offset: Vec3,
}

#[derive(Component)]
struct OrderQueue {
    orders: VecDeque<UnitQueuedOrder>,
}

#[derive(Component, Clone, Copy)]
struct HoldPosition {
    enabled: bool,
}

#[derive(Component, Clone, Copy)]
struct AttackOrder {
    target: Entity,
}

#[derive(Component, Clone, Copy)]
struct CaptureOrder {
    target: Entity,
    elapsed: f32,
}

#[derive(Component, Clone, Copy)]
struct GarrisonOrder {
    target: Entity,
}

#[derive(Component, Clone, Copy)]
struct HarvestOrder {
    resource: Option<Entity>,
    state: HarvestState,
    collect_remaining: f32,
}

#[derive(Component, Clone, Copy)]
struct RepairOrder {
    target: Entity,
}

#[derive(Component, Clone, Copy)]
struct ConstructOrder {
    target: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HarvestState {
    MovingToResource,
    Collecting,
    MovingToDropoff,
}

#[derive(Component, Clone, Copy)]
struct AttackMoveOrder {
    destination: Vec3,
}

#[derive(Component, Clone, Copy)]
struct PatrolOrder {
    origin: Vec3,
    destination: Vec3,
    moving_to_destination: bool,
}

type ActiveUnitOrderFilter = Or<(
    With<MoveOrder>,
    With<FollowOrder>,
    With<AttackOrder>,
    With<CaptureOrder>,
    With<GarrisonOrder>,
    With<HarvestOrder>,
    With<RepairOrder>,
    With<ConstructOrder>,
    With<AttackMoveOrder>,
    With<PatrolOrder>,
)>;

type IdleUnitOrderFilter = (
    Without<MoveOrder>,
    Without<FollowOrder>,
    Without<AttackOrder>,
    Without<CaptureOrder>,
    Without<GarrisonOrder>,
    Without<HarvestOrder>,
    Without<RepairOrder>,
    Without<ConstructOrder>,
    Without<AttackMoveOrder>,
    Without<PatrolOrder>,
);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MovementDomain {
    Terrain,
    Air,
}

impl MovementDomain {
    fn from_registry(domain: registry::MoveDomain) -> Self {
        match domain {
            registry::MoveDomain::Terrain => Self::Terrain,
            registry::MoveDomain::Air => Self::Air,
        }
    }
}

#[derive(Component, Clone, Copy)]
struct Weapon {
    range: f32,
    damage: f32,
    cooldown: f32,
    splash_radius: f32,
    splash_damage_multiplier: f32,
    structure_damage_multiplier: f32,
    cooldown_left: f32,
    can_attack_air: bool,
    can_attack_ground: bool,
}

#[derive(Component, Clone, Copy)]
struct ShotPulse {
    from: Vec3,
    to: Vec3,
    ttl: f32,
    team: Team,
}

#[derive(Resource, Default)]
struct NextSpawnId(u32);

impl Weapon {
    fn new(
        range: f32,
        damage: f32,
        cooldown: f32,
        splash_radius: f32,
        splash_damage_multiplier: f32,
        structure_damage_multiplier: f32,
        can_attack_air: bool,
        can_attack_ground: bool,
    ) -> Self {
        Self {
            range,
            damage,
            cooldown,
            splash_radius,
            splash_damage_multiplier,
            structure_damage_multiplier,
            can_attack_air,
            can_attack_ground,
            cooldown_left: 0.0,
        }
    }
}

#[derive(Resource)]
struct Economies {
    players: Vec<TeamEconomy>,
}

impl Default for Economies {
    fn default() -> Self {
        Self {
            players: Vec::new(),
        }
    }
}

impl Economies {
    fn get(&self, team: Team) -> &TeamEconomy {
        let Some(index) = team.economy_index() else {
            panic!("neutral team has no economy");
        };
        self.players
            .get(index)
            .unwrap_or_else(|| panic!("player slot {} has no economy", index + 1))
    }

    fn get_mut(&mut self, team: Team) -> &mut TeamEconomy {
        let Some(index) = team.economy_index() else {
            panic!("neutral team has no economy");
        };
        if self.players.len() <= index {
            self.players.resize_with(index + 1, || {
                TeamEconomy::new(
                    BEVY_PLAYTEST_STARTING_RESOURCES.ore,
                    BEVY_PLAYTEST_STARTING_RESOURCES.crystal,
                )
            });
        }
        &mut self.players[index]
    }

    fn apply_starting_resources(&mut self, resources: StartingResources) {
        for economy in &mut self.players {
            economy.ore = resources.ore;
            economy.crystal = resources.crystal;
        }
    }
}

#[derive(Clone)]
struct TeamEconomy {
    ore: i32,
    crystal: i32,
    power_used: i32,
    power_capacity: i32,
    power_sabotage_remaining: f32,
    production_veterancy_ranks: [u8; PRODUCTION_VETERANCY_PRODUCER_COUNT],
}

impl TeamEconomy {
    fn new(ore: i32, crystal: i32) -> Self {
        Self {
            ore,
            crystal,
            power_used: 0,
            power_capacity: 0,
            power_sabotage_remaining: 0.0,
            production_veterancy_ranks: [0; PRODUCTION_VETERANCY_PRODUCER_COUNT],
        }
    }

    fn can_afford(&self, cost: registry::Cost) -> bool {
        self.ore >= cost.ore && self.crystal >= cost.crystal
    }

    fn spend(&mut self, cost: registry::Cost) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        self.ore -= cost.ore;
        self.crystal -= cost.crystal;
        true
    }

    fn refund(&mut self, cost: registry::Cost) {
        self.ore += cost.ore;
        self.crystal += cost.crystal;
    }

    fn low_power(&self) -> bool {
        self.power_used > self.power_capacity
    }

    fn production_veterancy_rank(&self, producer_id: &str) -> u8 {
        production_veterancy_slot(producer_id)
            .map(|idx| self.production_veterancy_ranks[idx])
            .unwrap_or(0)
    }

    fn grant_production_veterancy_rank(&mut self, producer_id: &str, rank: u8) {
        let Some(idx) = production_veterancy_slot(producer_id) else {
            return;
        };
        self.production_veterancy_ranks[idx] =
            self.production_veterancy_ranks[idx].max(rank.min(VETERANCY_MAX_RANK));
    }
}

fn production_veterancy_slot(producer_id: &str) -> Option<usize> {
    match producer_id {
        "Barracks" => Some(0),
        "VehicleFactory" => Some(1),
        "AircraftFactory" => Some(2),
        _ => None,
    }
}

fn production_speed_multiplier(economy: &TeamEconomy) -> f32 {
    if economy.low_power() {
        LOW_POWER_PRODUCTION_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum BuildAction {
    None,
    Train(&'static str),
    Build(&'static str),
    DeployMcv,
    SellStructure,
    RepairStructure,
    ToggleDeployMode,
    SetRallyPoint,
    HoldPosition,
    AttackMove,
    Patrol,
    GuardArea,
    StopSelected,
    ScatterSelected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CommandHotkey {
    display: &'static str,
    key_code: KeyCode,
}

impl CommandHotkey {
    const fn new(display: &'static str, key_code: KeyCode) -> Self {
        Self { display, key_code }
    }
}

impl BuildAction {
    fn audio_command_key(self) -> Option<&'static str> {
        match self {
            Self::DeployMcv => Some(COMMAND_KEY_DEPLOY_MCV),
            Self::ToggleDeployMode => Some(COMMAND_KEY_TOGGLE_DEPLOY),
            Self::HoldPosition => Some(COMMAND_KEY_HOLD_POSITION),
            Self::GuardArea => Some(COMMAND_KEY_GUARD_AREA),
            Self::StopSelected => Some(COMMAND_KEY_CANCEL),
            Self::ScatterSelected => Some(COMMAND_KEY_SCATTER),
            Self::None
            | Self::Train(_)
            | Self::Build(_)
            | Self::SellStructure
            | Self::RepairStructure
            | Self::SetRallyPoint
            | Self::AttackMove
            | Self::Patrol => None,
        }
    }
}

#[derive(Clone, Copy)]
struct BuildJob {
    team: Team,
    action: BuildAction,
    producer_entity: Entity,
    producer_id: &'static str,
    timer: f32,
    origin: Vec3,
}

#[derive(Resource, Default)]
struct BuildQueue(Vec<BuildJob>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueueButtonState {
    count: usize,
    full: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnqueueBuildActionResult {
    Enqueued,
    NotEnoughResources,
    QueueFull,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructurePlacementValidity {
    Valid,
    CollidesWithObject,
    NotEnoughResources,
    OutOfMap,
    MissingTech,
    OutOfBaseRadius,
}

#[derive(Resource)]
struct AiDirector {
    production_timer: Vec<f32>,
    production_cursor: Vec<usize>,
    attack_timer: Vec<f32>,
    build_timer: Vec<f32>,
    construction_timer: Vec<f32>,
    capture_timer: Vec<f32>,
    saboteur_timer: Vec<f32>,
    support_timer: Vec<f32>,
    repair_timer: Vec<f32>,
}

impl AiDirector {
    fn ensure_team(&mut self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        if self.production_timer.len() <= index {
            self.production_timer.resize(index + 1, 2.5);
            self.production_cursor.resize(index + 1, 0);
            self.attack_timer
                .resize(index + 1, AI_OPENING_ATTACK_GRACE_SECONDS);
            self.build_timer.resize(index + 1, 8.0);
            self.construction_timer.resize(index + 1, 0.0);
            self.capture_timer.resize(index + 1, 3.0);
            self.saboteur_timer.resize(index + 1, 4.0);
            self.support_timer.resize(index + 1, 6.0);
            self.repair_timer.resize(index + 1, 0.0);
        }
        Some(index)
    }
}

impl Default for AiDirector {
    fn default() -> Self {
        Self {
            production_timer: Vec::new(),
            production_cursor: Vec::new(),
            attack_timer: Vec::new(),
            build_timer: Vec::new(),
            construction_timer: Vec::new(),
            capture_timer: Vec::new(),
            saboteur_timer: Vec::new(),
            support_timer: Vec::new(),
            repair_timer: Vec::new(),
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct AiDifficultySettings {
    players: Vec<AiDifficulty>,
}

impl AiDifficultySettings {
    fn difficulty(&self, team: Team) -> AiDifficulty {
        team.economy_index()
            .and_then(|index| self.players.get(index).copied())
            .unwrap_or(AiDifficulty::Normal)
    }

    fn set_difficulty(&mut self, team: Team, difficulty: AiDifficulty) {
        if let Some(index) = team.economy_index() {
            if self.players.len() <= index {
                self.players.resize(index + 1, AiDifficulty::Normal);
            }
            self.players[index] = difficulty;
        }
    }

    fn default_ai_difficulty(&self, player_team: Team) -> AiDifficulty {
        active_ai_teams(Some(player_team), None)
            .next()
            .map(|team| self.difficulty(team))
            .unwrap_or(AiDifficulty::Normal)
    }
}

impl Default for AiDifficultySettings {
    fn default() -> Self {
        let _available_difficulties = AiDifficulty::ALL;
        Self {
            players: Vec::new(),
        }
    }
}

#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct SelectionText;

#[derive(Component)]
struct BattleLogRoot {
    font: Handle<Font>,
}

#[derive(Component, Clone, Copy)]
struct BattleLogEntryButton(usize);

#[derive(Component)]
struct ObjectiveTrackerText;

#[derive(Component)]
struct ProductionQueueText;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ObjectiveTrackerState {
    max_enemy_anchors_seen: u32,
}

#[derive(Component, Clone, Copy)]
struct ProductionQueueSlot(usize);

#[derive(Component, Clone, Copy)]
struct ProductionQueueSlotLabel(usize);

#[derive(Component, Clone, Copy, Default)]
struct ProductionQueueSlotTarget {
    producer_entity: Option<Entity>,
    local_index: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct VisualFaction(SkirmishFaction);

#[derive(Component)]
struct MinimapRoot;

#[derive(Component)]
struct MinimapContent;

#[derive(Component)]
struct MinimapStatusText;

#[derive(Component)]
struct MinimapMarker;

#[derive(SystemParam)]
struct OrderResources<'w> {
    map_bounds: Res<'w, MapBounds>,
    relations: Res<'w, TeamRelations>,
    command_mode: ResMut<'w, CommandMode>,
    support_cooldowns: ResMut<'w, SupportCooldowns>,
    battle_log: ResMut<'w, BattleLog>,
    audio_feedback: ResMut<'w, AudioFeedback>,
}

#[derive(SystemParam)]
struct CommandActionResources<'w> {
    build_queue: ResMut<'w, BuildQueue>,
    command_mode: ResMut<'w, CommandMode>,
    economies: ResMut<'w, Economies>,
    next_id: ResMut<'w, NextSpawnId>,
    player_factions: Res<'w, PlayerFactions>,
    audio_feedback: ResMut<'w, AudioFeedback>,
    battle_log: ResMut<'w, BattleLog>,
}

#[derive(SystemParam)]
struct AiDirectorResources<'w> {
    map_bounds: Res<'w, MapBounds>,
    economies: ResMut<'w, Economies>,
    next_id: ResMut<'w, NextSpawnId>,
    director: ResMut<'w, AiDirector>,
    ai_settings: Res<'w, AiDifficultySettings>,
    player_factions: Res<'w, PlayerFactions>,
    active_teams: Option<Res<'w, ActiveTeams>>,
    relations: Res<'w, TeamRelations>,
    support_cooldowns: ResMut<'w, SupportCooldowns>,
    battle_log: ResMut<'w, BattleLog>,
    audio_feedback: ResMut<'w, AudioFeedback>,
}

#[derive(SystemParam)]
struct StructurePlacementInputResources<'w> {
    visible_player: Res<'w, VisiblePlayer>,
    player_factions: Res<'w, PlayerFactions>,
    asset_server: Res<'w, AssetServer>,
    map_bounds: Res<'w, MapBounds>,
    next_id: ResMut<'w, NextSpawnId>,
    economies: ResMut<'w, Economies>,
    command_mode: ResMut<'w, CommandMode>,
    placement_feedback: ResMut<'w, StructurePlacementFeedback>,
    audio_feedback: ResMut<'w, AudioFeedback>,
    battle_log: ResMut<'w, BattleLog>,
}

#[derive(SystemParam)]
struct StructurePlacementPreviewParams<'w, 's> {
    command_mode: Res<'w, CommandMode>,
    visible_player: Res<'w, VisiblePlayer>,
    player_factions: Res<'w, PlayerFactions>,
    economies: Res<'w, Economies>,
    map_bounds: Res<'w, MapBounds>,
    window_q: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
    structures: Query<'w, 's, StructurePrereqItem<'static>>,
    occupiers: Query<
        'w,
        's,
        PlacementOccupierItem<'static>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
}

#[derive(Component)]
struct ButtonLabel;

#[derive(Component, Clone, Copy)]
struct CommandSlot(usize);

#[derive(Component, Clone, Copy)]
struct CommandSlotLabel(usize);

#[derive(Component, Clone, Copy)]
struct CommandSlotAvailability {
    enabled: bool,
}

impl Default for CommandSlotAvailability {
    fn default() -> Self {
        Self { enabled: false }
    }
}

fn apply_match_setup_settings(
    setup_settings: Res<MatchSetupSettings>,
    mut selected_map: ResMut<SelectedSkirmishMap>,
    mut economies: ResMut<Economies>,
    mut visible_player: ResMut<VisiblePlayer>,
    mut ai_difficulties: ResMut<AiDifficultySettings>,
    mut team_relations: ResMut<TeamRelations>,
    mut active_teams: ResMut<ActiveTeams>,
    mut player_factions: ResMut<PlayerFactions>,
    mut player_color_slots: ResMut<PlayerColorSlots>,
    mut camera_state: ResMut<RtsCamera>,
) {
    selected_map.godot_path = setup_settings.map_path;
    for team in player_teams(setup_settings.active_teams.len()) {
        let _ = economies.get_mut(team);
    }
    economies.apply_starting_resources(setup_settings.starting_resources);
    *visible_player = setup_settings.visible_player;
    *ai_difficulties = setup_settings.ai_difficulties.clone();
    *team_relations = setup_settings.team_relations.clone();
    team_relations.ensure_player_count(setup_settings.active_teams.len());
    *active_teams = ActiveTeams(setup_settings.active_teams.clone());
    *player_factions = PlayerFactions(setup_settings.player_factions.clone());
    *player_color_slots = PlayerColorSlots(setup_settings.player_color_slots.clone());
    let map = selected_map.definition();
    *camera_state = RtsCamera::focused_on(team_start_camera_focus_for_spawn_slot(
        map,
        setup_settings.player_spawn_slot(setup_settings.visible_player.team),
        setup_settings.player_faction(setup_settings.visible_player.team),
        setup_settings.startup_loadout,
    ));
}

fn begin_match_from_setup(
    mut match_flow: ResMut<MatchFlow>,
    mut match_state: ResMut<MatchState>,
    mut camera_state: ResMut<RtsCamera>,
    selected_map: Res<SelectedSkirmishMap>,
    setup_settings: Res<MatchSetupSettings>,
    mut command_mode: ResMut<CommandMode>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut unit_groups: ResMut<UnitGroups>,
    mut camera_bookmarks: ResMut<CameraBookmarks>,
    mut camera_mouse_rotation: ResMut<CameraMouseRotation>,
    mut match_menu: ResMut<MatchMenuState>,
    mut briefing: ResMut<MatchBriefingState>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut objective_tracker: ResMut<ObjectiveTrackerState>,
) {
    *match_state = MatchState::default();
    match_flow.active = true;
    let start_position = team_start_camera_focus_for_spawn_slot(
        selected_map.definition(),
        setup_settings.player_spawn_slot(setup_settings.visible_player.team),
        setup_settings.player_faction(setup_settings.visible_player.team),
        setup_settings.startup_loadout,
    );
    *camera_state = RtsCamera::focused_on(start_position);
    *command_mode = CommandMode::default();
    *selection_drag = SelectionDragState::default();
    *unit_groups = UnitGroups::default();
    *camera_bookmarks = CameraBookmarks::default();
    *camera_mouse_rotation = CameraMouseRotation::default();
    match_menu.visible = false;
    *briefing = MatchBriefingState::default();
    briefing.show();
    battle_log.entries.clear();
    *audio_feedback = AudioFeedback::default();
    *objective_tracker = ObjectiveTrackerState::default();
}

fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selection: Res<SkirmishMenuSelection>,
) {
    let font = asset_server.load("fonts/wqy-microhei-ui.ttf");

    commands.spawn((
        Name::new("Skirmish Menu Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::MainMenu),
    ));

    commands
        .spawn((
            Name::new("Skirmish Setup Menu"),
            DespawnOnExit(AppScreen::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                row_gap: px(12),
                padding: UiRect::new(px(12), px(12), px(14), px(16)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.015, 0.019, 0.024)),
        ))
        .with_children(|root| {
            root.spawn(menu_top_bar_node()).with_children(|bar| {
                bar.spawn((
                    Text::new("BEVY OPEN RTS"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.82, 0.9, 0.9)),
                ));
                bar.spawn((
                    Text::new("遭遇战作战室"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.98, 0.78, 0.42)),
                ));
                bar.spawn((
                    Text::new(main_menu_brief_status_text(*selection)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.58, 0.86, 0.66)),
                    MainMenuBriefStatusText,
                ));
            });

            root.spawn(menu_scroll_area_node())
                .with_children(|content| {
                    content
                        .spawn(menu_preview_column_node())
                        .with_children(|column| {
                            column.spawn(menu_panel_node(400.0)).with_children(|panel| {
                                panel.spawn(menu_panel_title("战区情报", font.clone()));
                                spawn_skirmish_map_preview(panel, *selection);
                                panel.spawn((
                                    Text::new(main_menu_faction_info_text(*selection)),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.76, 0.84, 0.82)),
                                    Node {
                                        width: Val::Percent(100.0),
                                        ..default()
                                    },
                                    MainMenuFactionInfoText,
                                ));
                            });

                            column.spawn(menu_panel_node(400.0)).with_children(|panel| {
                                panel.spawn(menu_panel_title("地图选择", font.clone()));
                                panel.spawn(menu_button_row_node()).with_children(|row| {
                                    for index in 0..SKIRMISH_MAPS.len() {
                                        let action = MainMenuAction::SelectMap(index);
                                        row.spawn(menu_button(action, 174.0)).with_children(
                                            |button| {
                                                button.spawn(menu_action_button_label(
                                                    action,
                                                    *selection,
                                                    font.clone(),
                                                    13.0,
                                                ));
                                            },
                                        );
                                    }
                                    let action = MainMenuAction::SelectMap(random_map_index());
                                    row.spawn(menu_button(action, 174.0))
                                        .with_children(|button| {
                                            button.spawn(menu_action_button_label(
                                                action,
                                                *selection,
                                                font.clone(),
                                                13.0,
                                            ));
                                        });
                                });
                            });
                        });

                    content
                        .spawn(menu_options_grid_node())
                        .with_children(|column| {
                            column.spawn(menu_panel_node(760.0)).with_children(|panel| {
                                panel.spawn(menu_panel_title("作战规则", font.clone()));
                                panel.spawn(menu_rule_grid_node()).with_children(|grid| {
                                    grid.spawn(menu_section_node()).with_children(|section| {
                                        section.spawn(menu_section_title(
                                            "模式  9/0/A/M",
                                            font.clone(),
                                        ));
                                        section.spawn(menu_button_row_node()).with_children(
                                            |row| {
                                                for mode in SkirmishMatchMode::ALL {
                                                    let width = match mode {
                                                        SkirmishMatchMode::OneVsOne => 112.0,
                                                        SkirmishMatchMode::FreeForAll => 140.0,
                                                        SkirmishMatchMode::AiVsAi => 132.0,
                                                        SkirmishMatchMode::AlliedTwoVsOne => 132.0,
                                                    };
                                                    let action =
                                                        MainMenuAction::SelectMatchMode(mode);
                                                    row.spawn(menu_button(action, width))
                                                        .with_children(|button| {
                                                            button.spawn(menu_action_button_label(
                                                                action,
                                                                *selection,
                                                                font.clone(),
                                                                13.0,
                                                            ));
                                                        });
                                                }
                                            },
                                        );
                                    });

                                    grid.spawn(menu_section_node()).with_children(|section| {
                                        section.spawn(menu_section_title(
                                            "AI难度  F1-F4",
                                            font.clone(),
                                        ));
                                        section.spawn(menu_button_row_node()).with_children(
                                            |row| {
                                                for difficulty in AiDifficulty::ALL {
                                                    let action = MainMenuAction::SelectAiDifficulty(
                                                        difficulty,
                                                    );
                                                    row.spawn(menu_button(action, 124.0))
                                                        .with_children(|button| {
                                                            button.spawn(menu_action_button_label(
                                                                action,
                                                                *selection,
                                                                font.clone(),
                                                                12.0,
                                                            ));
                                                        });
                                                }
                                            },
                                        );
                                    });

                                    grid.spawn(menu_section_node()).with_children(|section| {
                                        section.spawn(menu_section_title(
                                            "初始资源  5-8",
                                            font.clone(),
                                        ));
                                        section.spawn(menu_button_row_node()).with_children(
                                            |row| {
                                                for index in
                                                    0..GODOT_STARTING_RESOURCE_OPTIONS.len()
                                                {
                                                    let action =
                                                        MainMenuAction::SelectStartingResources(
                                                            index,
                                                        );
                                                    row.spawn(menu_button(action, 124.0))
                                                        .with_children(|button| {
                                                            button.spawn(menu_action_button_label(
                                                                action,
                                                                *selection,
                                                                font.clone(),
                                                                12.0,
                                                            ));
                                                        });
                                                }
                                            },
                                        );
                                    });
                                });
                            });

                            column.spawn(menu_panel_node(760.0)).with_children(|panel| {
                                panel.spawn(menu_panel_title("玩家槽位", font.clone()));
                                panel
                                    .spawn((
                                        menu_lobby_list_node(),
                                        MainMenuLobbyListRoot { font: font.clone() },
                                    ))
                                    .with_children(|list| {
                                        for slot in 0..selection.selected_map_player_slots() {
                                            spawn_menu_lobby_slot_row(
                                                list,
                                                slot,
                                                font.clone(),
                                                *selection,
                                            );
                                        }
                                    });
                            });
                        });
                });

            root.spawn(menu_bottom_bar_node()).with_children(|bar| {
                bar.spawn(menu_button(MainMenuAction::StartMatch, 270.0))
                    .with_children(|button| {
                        button.spawn(menu_action_button_label(
                            MainMenuAction::StartMatch,
                            *selection,
                            font.clone(),
                            18.0,
                        ));
                    });
            });
            root.spawn((
                Text::new(main_menu_summary_text(*selection)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(1.0),
                    ..default()
                },
                TextColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-10_000),
                    top: px(-10_000),
                    width: px(1),
                    height: px(1),
                    ..default()
                },
                MainMenuSummaryText,
            ));
        });
}

fn restore_main_menu_selection_from_match_setup(
    setup_settings: Res<MatchSetupSettings>,
    mut selection: ResMut<SkirmishMenuSelection>,
) {
    *selection = SkirmishMenuSelection::from_match_setup(setup_settings.clone());
}

fn menu_action_button_label(
    action: MainMenuAction,
    selection: SkirmishMenuSelection,
    font: Handle<Font>,
    font_size: f32,
) -> impl Bundle {
    (
        menu_button_label(
            main_menu_button_label_text(action, selection),
            font,
            font_size,
        ),
        MainMenuButtonLabel { action },
    )
}

fn main_menu_button_label_text(action: MainMenuAction, selection: SkirmishMenuSelection) -> String {
    match action {
        MainMenuAction::SelectMap(index) if index == random_map_index() => {
            format!("R {RANDOM_MAP_LABEL}")
        }
        MainMenuAction::SelectMap(index) => SKIRMISH_MAPS
            .get(index)
            .map(|map| format!("{} {}", index + 1, map.name))
            .unwrap_or_else(|| "地图".to_string()),
        MainMenuAction::SelectStartingResources(index) => GODOT_STARTING_RESOURCE_OPTIONS
            .get(index)
            .map(|option| format!("{} {}", index + 5, starting_resource_option_label(option)))
            .unwrap_or_else(|| "资源".to_string()),
        MainMenuAction::SelectMatchMode(mode) => {
            format!("{} {}", skirmish_match_mode_key(mode), mode.label())
        }
        MainMenuAction::SelectAiDifficulty(difficulty) => format!(
            "{} {}",
            skirmish_ai_difficulty_key(difficulty),
            difficulty.short_label()
        ),
        MainMenuAction::SelectLobbySlot(_) => "我方".to_string(),
        MainMenuAction::CycleLobbySlotController(slot) => format!(
            "{}{}+",
            lobby_slot_key_prefix(skirmish_lobby_slot_controller_key(slot)),
            selection
                .lobby_controllers
                .get(slot)
                .copied()
                .unwrap_or(SkirmishPlayerController::None)
                .short_label()
        ),
        MainMenuAction::CycleLobbySlotFaction(slot) => format!(
            "{}{}+",
            lobby_slot_key_prefix(skirmish_lobby_slot_faction_key(slot)),
            selection
                .lobby_factions
                .get(slot)
                .copied()
                .unwrap_or(SkirmishFaction::Alliance)
                .label()
        ),
        MainMenuAction::CycleLobbySlotTeamId(slot) => format!(
            "{}T{}+",
            lobby_slot_key_prefix(skirmish_lobby_slot_team_key(slot)),
            selection.lobby_team_ids.get(slot).copied().unwrap_or(0) % SKIRMISH_TEAM_OPTION_COUNT
                + 1
        ),
        MainMenuAction::CycleLobbySlotColor(slot) => format!(
            "{}C{}+",
            lobby_slot_key_prefix(skirmish_lobby_slot_color_key(slot)),
            selection
                .lobby_color_slots
                .get(slot)
                .copied()
                .unwrap_or(slot)
                % PLAYER_COLOR_PALETTE.len()
                + 1
        ),
        MainMenuAction::StartMatch => "开始对战  Enter".to_string(),
    }
}

fn lobby_slot_key_prefix(key: &'static str) -> String {
    if key.is_empty() {
        String::new()
    } else {
        format!("{key} ")
    }
}

fn skirmish_lobby_slot_controller_key(slot: usize) -> &'static str {
    match slot {
        0 => "Q",
        1 => "W",
        2 => "E",
        _ => "",
    }
}

fn skirmish_lobby_slot_faction_key(slot: usize) -> &'static str {
    match slot {
        0 => "Z",
        1 => "X",
        2 => "V",
        _ => "",
    }
}

fn skirmish_lobby_slot_team_key(slot: usize) -> &'static str {
    match slot {
        0 => "J",
        1 => "K",
        2 => "L",
        _ => "",
    }
}

fn skirmish_lobby_slot_color_key(slot: usize) -> &'static str {
    match slot {
        0 => "U",
        1 => "I",
        2 => "O",
        _ => "",
    }
}

fn skirmish_match_mode_key(mode: SkirmishMatchMode) -> &'static str {
    match mode {
        SkirmishMatchMode::OneVsOne => "9",
        SkirmishMatchMode::FreeForAll => "0",
        SkirmishMatchMode::AiVsAi => "A",
        SkirmishMatchMode::AlliedTwoVsOne => "M",
    }
}

fn skirmish_ai_difficulty_key(difficulty: AiDifficulty) -> &'static str {
    match difficulty {
        AiDifficulty::Beginner => "F1",
        AiDifficulty::Easy => "F2",
        AiDifficulty::Normal => "F3",
        AiDifficulty::Hard => "F4",
    }
}

fn starting_resource_option_label(option: &StartingResourceOption) -> &'static str {
    match option.key {
        "STARTING_RESOURCES_LOW" => "低 4/2",
        "STARTING_RESOURCES_STANDARD" => "标准 8/4",
        "STARTING_RESOURCES_HIGH" => "高 16/8",
        "STARTING_RESOURCES_RICH" => "富矿 32/16",
        _ => "资源",
    }
}

fn menu_top_bar_node() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            max_width: px(1280),
            height: px(66),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::new(px(20), px(20), px(0), px(0)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.32, 0.36, 0.34)),
        BackgroundColor(Color::srgba(0.045, 0.055, 0.058, 0.96)),
    )
}

fn menu_bottom_bar_node() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            max_width: px(1280),
            height: px(92),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(16),
            padding: UiRect::new(px(18), px(18), px(10), px(10)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.34, 0.32)),
        BackgroundColor(Color::srgba(0.035, 0.046, 0.048, 0.95)),
    )
}

fn menu_scroll_area_node() -> impl Bundle {
    (
        MainMenuScrollArea,
        ScrollPosition::default(),
        Node {
            width: Val::Percent(100.0),
            max_width: px(1248),
            min_height: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(12),
            row_gap: px(14),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    )
}

fn menu_preview_column_node() -> impl Bundle {
    Node {
        width: px(400),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(14),
        ..default()
    }
}

fn menu_options_grid_node() -> impl Bundle {
    Node {
        width: px(760),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::FlexStart,
        row_gap: px(14),
        ..default()
    }
}

fn menu_panel_node(width: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(12),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.31, 0.36, 0.34)),
        BackgroundColor(Color::srgba(0.033, 0.044, 0.045, 0.95)),
    )
}

fn menu_panel_title(label: &'static str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.72, 0.38)),
    )
}

fn menu_rule_grid_node() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::FlexStart,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: px(10),
        row_gap: px(12),
        ..default()
    }
}

fn menu_lobby_list_node() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(6),
        ..default()
    }
}

fn menu_section_node() -> impl Bundle {
    Node {
        width: px(232),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(8),
        ..default()
    }
}

fn menu_button_row_node() -> impl Bundle {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::FlexStart,
        justify_content: JustifyContent::FlexStart,
        column_gap: px(8),
        row_gap: px(8),
        ..default()
    }
}

fn menu_section_title(label: &'static str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.62, 0.74, 0.74)),
    )
}

fn spawn_menu_lobby_slot_row(
    parent: &mut ChildSpawnerCommands<'_>,
    slot: usize,
    font: Handle<Font>,
    selection: SkirmishMenuSelection,
) {
    let controller = selection
        .lobby_controllers
        .get(slot)
        .copied()
        .unwrap_or(SkirmishPlayerController::None);
    let faction = selection
        .lobby_factions
        .get(slot)
        .copied()
        .unwrap_or(SkirmishFaction::Alliance);
    let team_id =
        selection.lobby_team_ids.get(slot).copied().unwrap_or(0) % SKIRMISH_TEAM_OPTION_COUNT + 1;
    let color_slot = selection
        .lobby_color_slots
        .get(slot)
        .copied()
        .unwrap_or(slot)
        % PLAYER_COLOR_PALETTE.len()
        + 1;
    let active = controller.is_active();
    let status = if active {
        format!(
            "{} | {} | T{} | C{}",
            controller.short_label(),
            faction.label(),
            team_id,
            color_slot
        )
    } else {
        "关闭".to_string()
    };
    parent
        .spawn(menu_lobby_slot_row_node(slot, selection))
        .with_children(|row| {
            row.spawn(menu_lobby_slot_label_node(48.0))
                .with_children(|cell| {
                    cell.spawn((
                        Text::new(format!("{:02}", slot + 1)),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.72, 0.38)),
                    ));
                });

            row.spawn(menu_lobby_slot_label_node(96.0))
                .with_children(|cell| {
                    cell.spawn((
                        Text::new(if controller.is_human() {
                            "我方槽位"
                        } else if active {
                            "电脑槽位"
                        } else {
                            "空位"
                        }),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.88, 0.74)),
                    ));
                });
            row.spawn(menu_lobby_slot_label_node(176.0))
                .with_children(|cell| {
                    cell.spawn((
                        Text::new(status),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.8, 0.78)),
                    ));
                });
            for action in [
                MainMenuAction::SelectLobbySlot(slot),
                MainMenuAction::CycleLobbySlotController(slot),
                MainMenuAction::CycleLobbySlotFaction(slot),
                MainMenuAction::CycleLobbySlotTeamId(slot),
                MainMenuAction::CycleLobbySlotColor(slot),
            ] {
                row.spawn(menu_button(action, 62.0))
                    .with_children(|button| {
                        button.spawn(menu_action_button_label(
                            action,
                            selection,
                            font.clone(),
                            10.0,
                        ));
                    });
            }
        });
}

fn menu_lobby_slot_label_node(width: f32) -> impl Bundle {
    Node {
        width: px(width),
        min_height: px(30),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexStart,
        ..default()
    }
}

fn menu_lobby_slot_row_node(slot: usize, selection: SkirmishMenuSelection) -> impl Bundle {
    let focused = selection.focus_lobby_slot() == Some(slot);
    let active = selection
        .lobby_controllers
        .get(slot)
        .copied()
        .is_some_and(SkirmishPlayerController::is_active);
    let border = if focused {
        Color::srgb(0.95, 0.72, 0.38)
    } else if active {
        Color::srgb(0.34, 0.44, 0.42)
    } else {
        Color::srgb(0.19, 0.23, 0.23)
    };
    let background = if active {
        Color::srgba(0.04, 0.055, 0.056, 0.96)
    } else {
        Color::srgba(0.028, 0.034, 0.034, 0.88)
    };

    (
        MainMenuLobbySlotRow,
        Node {
            width: Val::Percent(100.0),
            min_height: px(40),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::new(px(10), px(10), px(4), px(4)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
    )
}

fn menu_button(action: MainMenuAction, width: f32) -> impl Bundle {
    (
        Button,
        MainMenuButton { action },
        Node {
            width: px(width),
            min_height: px(38),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::new(px(8), px(8), px(0), px(0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.34, 0.33)),
        BackgroundColor(Color::srgba(0.046, 0.058, 0.06, 0.94)),
    )
}

fn menu_button_label(label: impl Into<String>, font: Handle<Font>, font_size: f32) -> impl Bundle {
    (
        Text::new(label.into()),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

fn main_menu_scroll(
    mut wheel_events: MessageReader<MouseWheel>,
    mut scroll_q: Query<&mut ScrollPosition, With<MainMenuScrollArea>>,
) {
    let mut delta = 0.0;
    for event in wheel_events.read() {
        let scroll_lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y * 0.05,
        };
        delta -= scroll_lines * 48.0;
    }
    if delta == 0.0 {
        return;
    }
    for mut scroll in &mut scroll_q {
        scroll.y = (scroll.y + delta).max(0.0);
    }
}

fn skirmish_map_preview_root() -> impl Bundle {
    (
        Name::new("Skirmish Map Preview"),
        SkirmishMapPreviewRoot,
        Node {
            width: px(SKIRMISH_MAP_PREVIEW_SIZE.x),
            height: px(SKIRMISH_MAP_PREVIEW_SIZE.y),
            min_width: px(SKIRMISH_MAP_PREVIEW_SIZE.x),
            min_height: px(SKIRMISH_MAP_PREVIEW_SIZE.y),
            align_self: AlignSelf::Center,
            position_type: PositionType::Relative,
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.26, 0.36, 0.34)),
        BackgroundColor(Color::srgba(0.028, 0.044, 0.042, 0.96)),
    )
}

fn spawn_skirmish_map_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    selection: SkirmishMenuSelection,
) {
    parent
        .spawn(skirmish_map_preview_root())
        .with_children(|preview| {
            spawn_skirmish_map_preview_elements(preview, selection);
        });
}

fn spawn_skirmish_map_preview_elements(
    parent: &mut ChildSpawnerCommands<'_>,
    selection: SkirmishMenuSelection,
) {
    let map = selection.map();
    let rect = skirmish_map_preview_rect(map, SKIRMISH_MAP_PREVIEW_SIZE);
    parent.spawn(skirmish_map_preview_frame_node(rect));
    spawn_skirmish_map_preview_grid(parent, rect);

    for resource in map.resources {
        let kind = match resource.kind {
            ResourceKind::Ore => SkirmishMapPreviewMarkerKind::Ore,
            ResourceKind::Crystal => SkirmishMapPreviewMarkerKind::Crystal,
        };
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            resource.position,
            kind,
            7.0,
        ));
    }
    for tech in map.neutral_tech {
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            tech.position,
            SkirmishMapPreviewMarkerKind::NeutralTech,
            10.0,
        ));
    }
    for crate_spec in map.supply_crates {
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            crate_spec.position,
            SkirmishMapPreviewMarkerKind::SupplyCrate,
            8.5,
        ));
    }
    for (slot, spawn_point) in map.spawn_points.iter().copied().enumerate() {
        parent.spawn(skirmish_map_preview_spawn_marker_node(
            map,
            spawn_point,
            skirmish_spawn_slot_color(selection, slot),
        ));
    }
}

fn skirmish_map_preview_frame_node(rect: SkirmishMapPreviewRect) -> impl Bundle {
    (
        Name::new("Skirmish Map Preview Frame"),
        SkirmishMapPreviewElement,
        Node {
            position_type: PositionType::Absolute,
            left: px(rect.left),
            top: px(rect.top),
            width: px(rect.width),
            height: px(rect.height),
            border: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.48, 0.62, 0.54)),
        BackgroundColor(Color::srgb(0.035, 0.055, 0.052)),
    )
}

fn spawn_skirmish_map_preview_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    rect: SkirmishMapPreviewRect,
) {
    for index in 1..SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS {
        let ratio = index as f32 / SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS as f32;
        let x = rect.left + rect.width * ratio;
        let y = rect.top + rect.height * ratio;
        parent.spawn(skirmish_map_preview_grid_line_node(
            x,
            rect.top,
            1.0,
            rect.height,
        ));
        parent.spawn(skirmish_map_preview_grid_line_node(
            rect.left, y, rect.width, 1.0,
        ));
    }
}

fn skirmish_map_preview_grid_line_node(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> impl Bundle {
    (
        Name::new("Skirmish Map Preview Grid"),
        SkirmishMapPreviewElement,
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(height),
            ..default()
        },
        BackgroundColor(Color::srgba(0.17, 0.26, 0.23, 0.65)),
    )
}

fn skirmish_map_preview_marker_node(
    map: &SkirmishMapDef,
    point: (f32, f32),
    kind: SkirmishMapPreviewMarkerKind,
    size: f32,
) -> impl Bundle {
    let preview = skirmish_map_preview_point(map, point, SKIRMISH_MAP_PREVIEW_SIZE);
    let color = skirmish_map_preview_marker_color(kind, 1.0);
    (
        Name::new("Skirmish Map Preview Marker"),
        SkirmishMapPreviewElement,
        SkirmishMapPreviewMarker { kind },
        Node {
            position_type: PositionType::Absolute,
            left: px(preview.x - size * 0.5),
            top: px(preview.y - size * 0.5),
            width: px(size),
            height: px(size),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(
                if matches!(kind, SkirmishMapPreviewMarkerKind::NeutralTech) {
                    1.0
                } else {
                    999.0
                },
            )),
            ..default()
        },
        BorderColor::all(Color::srgb(0.02, 0.025, 0.02)),
        BackgroundColor(color),
    )
}

fn skirmish_map_preview_spawn_marker_node(
    map: &SkirmishMapDef,
    point: (f32, f32),
    color: Color,
) -> impl Bundle {
    let preview = skirmish_map_preview_point(map, point, SKIRMISH_MAP_PREVIEW_SIZE);
    let size = 13.0;
    (
        Name::new("Skirmish Map Preview Spawn"),
        SkirmishMapPreviewElement,
        SkirmishMapPreviewMarker {
            kind: SkirmishMapPreviewMarkerKind::Spawn,
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(preview.x - size * 0.5),
            top: px(preview.y - size * 0.5),
            width: px(size),
            height: px(size),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(Color::srgb(0.02, 0.025, 0.02)),
        BackgroundColor(color),
    )
}

fn skirmish_spawn_slot_color(selection: SkirmishMenuSelection, slot: usize) -> Color {
    Team::from_playable_index(slot)
        .and_then(|team| selection.player_color_slot(team))
        .map(player_color)
        .unwrap_or_else(|| player_color(slot))
}

fn skirmish_map_preview_marker_color(kind: SkirmishMapPreviewMarkerKind, alpha: f32) -> Color {
    match kind {
        SkirmishMapPreviewMarkerKind::Spawn => Color::srgba(1.0, 1.0, 1.0, alpha),
        SkirmishMapPreviewMarkerKind::Ore => Color::srgba(0.25, 0.66, 1.0, alpha),
        SkirmishMapPreviewMarkerKind::Crystal => Color::srgba(1.0, 0.45, 0.24, alpha),
        SkirmishMapPreviewMarkerKind::NeutralTech => Color::srgba(1.0, 0.86, 0.28, alpha),
        SkirmishMapPreviewMarkerKind::SupplyCrate => Color::srgba(0.42, 1.0, 0.52, alpha),
    }
}

fn skirmish_map_preview_rect(map: &SkirmishMapDef, preview_size: Vec2) -> SkirmishMapPreviewRect {
    let available_width = (preview_size.x - SKIRMISH_MAP_PREVIEW_PADDING * 2.0).max(1.0);
    let available_height = (preview_size.y - SKIRMISH_MAP_PREVIEW_PADDING * 2.0).max(1.0);
    let map_aspect = map.size.0 / map.size.1.max(1.0);
    let mut width = available_width;
    let mut height = available_height;
    if width / height > map_aspect {
        width = height * map_aspect;
    } else {
        height = width / map_aspect;
    }
    SkirmishMapPreviewRect {
        left: SKIRMISH_MAP_PREVIEW_PADDING + (available_width - width) * 0.5,
        top: SKIRMISH_MAP_PREVIEW_PADDING + (available_height - height) * 0.5,
        width,
        height,
    }
}

fn skirmish_map_preview_point(map: &SkirmishMapDef, point: (f32, f32), preview_size: Vec2) -> Vec2 {
    let rect = skirmish_map_preview_rect(map, preview_size);
    Vec2::new(
        rect.left + rect.width * point.0 / map.size.0.max(1.0),
        rect.top + rect.height * point.1 / map.size.1.max(1.0),
    )
}

fn update_skirmish_map_preview(
    mut commands: Commands,
    selection: Res<SkirmishMenuSelection>,
    root_q: Query<Entity, With<SkirmishMapPreviewRoot>>,
    elements: Query<Entity, With<SkirmishMapPreviewElement>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok(root) = root_q.single() else {
        return;
    };
    for entity in &elements {
        commands.entity(entity).try_despawn();
    }
    let Ok(mut root_commands) = commands.get_entity(root) else {
        return;
    };
    root_commands.with_children(|parent| {
        spawn_skirmish_map_preview_elements(parent, *selection);
    });
}

fn update_main_menu_lobby_slots(
    mut commands: Commands,
    selection: Res<SkirmishMenuSelection>,
    root_q: Query<(Entity, &MainMenuLobbyListRoot)>,
    rows: Query<Entity, With<MainMenuLobbySlotRow>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok((root, list_root)) = root_q.single() else {
        return;
    };
    for entity in &rows {
        commands.entity(entity).try_despawn();
    }
    let Ok(mut root_commands) = commands.get_entity(root) else {
        return;
    };
    root_commands.with_children(|parent| {
        for slot in 0..selection.selected_map_player_slots() {
            spawn_menu_lobby_slot_row(parent, slot, list_root.font.clone(), *selection);
        }
    });
}

fn main_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<SkirmishMenuSelection>,
    mut setup_settings: ResMut<MatchSetupSettings>,
    mut random_map_cursor: ResMut<RandomMapCursor>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(
        &Interaction,
        &MainMenuButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .into_iter()
    .enumerate()
    {
        if index < SKIRMISH_MAPS.len() && keyboard.just_pressed(key) {
            selection.map_index = index;
        }
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        selection.map_index = random_map_index();
    }
    for (index, key) in [
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ]
    .into_iter()
    .enumerate()
    {
        if index < GODOT_STARTING_RESOURCE_OPTIONS.len() && keyboard.just_pressed(key) {
            selection.starting_resource_index = index;
        }
    }
    for (slot, key) in [(0, KeyCode::KeyH), (1, KeyCode::KeyD), (2, KeyCode::KeyC)] {
        if keyboard.just_pressed(key) {
            selection.select_lobby_slot(slot);
        }
    }
    for (mode, key) in [
        (SkirmishMatchMode::OneVsOne, KeyCode::Digit9),
        (SkirmishMatchMode::FreeForAll, KeyCode::Digit0),
        (SkirmishMatchMode::AiVsAi, KeyCode::KeyA),
        (SkirmishMatchMode::AlliedTwoVsOne, KeyCode::KeyM),
    ] {
        if keyboard.just_pressed(key) {
            selection.set_match_mode(mode);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyJ), (1, KeyCode::KeyK), (2, KeyCode::KeyL)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_team_id(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyZ), (1, KeyCode::KeyX), (2, KeyCode::KeyV)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_faction(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyQ), (1, KeyCode::KeyW), (2, KeyCode::KeyE)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_controller(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyU), (1, KeyCode::KeyI), (2, KeyCode::KeyO)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_color(slot);
        }
    }
    for (difficulty, key) in [
        (AiDifficulty::Beginner, KeyCode::F1),
        (AiDifficulty::Easy, KeyCode::F2),
        (AiDifficulty::Normal, KeyCode::F3),
        (AiDifficulty::Hard, KeyCode::F4),
    ] {
        if keyboard.just_pressed(key) {
            selection.set_ai_difficulty(difficulty);
        }
    }

    let mut start_requested = keyboard.just_pressed(KeyCode::Enter);
    let selection_snapshot = *selection;
    for (interaction, button, mut background, mut border) in &mut buttons {
        let clicked = *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match button.action {
                MainMenuAction::SelectMap(index)
                    if index < SKIRMISH_MAPS.len() || is_random_map_index(index) =>
                {
                    selection.map_index = index;
                }
                MainMenuAction::SelectStartingResources(index)
                    if index < GODOT_STARTING_RESOURCE_OPTIONS.len() =>
                {
                    selection.starting_resource_index = index;
                }
                MainMenuAction::SelectMatchMode(mode) => {
                    selection.set_match_mode(mode);
                }
                MainMenuAction::SelectAiDifficulty(difficulty) => {
                    selection.set_ai_difficulty(difficulty);
                }
                MainMenuAction::SelectLobbySlot(slot) => {
                    selection.select_lobby_slot(slot);
                }
                MainMenuAction::CycleLobbySlotController(slot) => {
                    selection.cycle_lobby_slot_controller(slot);
                }
                MainMenuAction::CycleLobbySlotFaction(slot) => {
                    selection.cycle_lobby_slot_faction(slot);
                }
                MainMenuAction::CycleLobbySlotTeamId(slot) => {
                    selection.cycle_lobby_slot_team_id(slot);
                }
                MainMenuAction::CycleLobbySlotColor(slot) => {
                    selection.cycle_lobby_slot_color(slot);
                }
                MainMenuAction::StartMatch => {
                    start_requested = true;
                }
                MainMenuAction::SelectMap(_) | MainMenuAction::SelectStartingResources(_) => {}
            }
        }

        let (bg, border_color) =
            main_menu_button_visual(button.action, *interaction, selection_snapshot);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }

    if start_requested {
        start_shared_match_from_menu_selection(
            *selection,
            &mut setup_settings,
            &mut random_map_cursor,
            &mut next_state,
        );
    }
}

fn main_menu_button_visual(
    action: MainMenuAction,
    interaction: Interaction,
    selection: SkirmishMenuSelection,
) -> (Color, Color) {
    if matches!(action, MainMenuAction::StartMatch) {
        if !selection.can_start() {
            return match interaction {
                Interaction::Pressed => (
                    Color::srgba(0.052, 0.056, 0.056, 0.9),
                    Color::srgb(0.18, 0.22, 0.22),
                ),
                Interaction::Hovered => (
                    Color::srgba(0.06, 0.066, 0.066, 0.92),
                    Color::srgb(0.24, 0.28, 0.28),
                ),
                Interaction::None => (
                    Color::srgba(0.038, 0.044, 0.044, 0.86),
                    Color::srgb(0.16, 0.19, 0.19),
                ),
            };
        }
        return match interaction {
            Interaction::Pressed => (
                Color::srgba(0.11, 0.36, 0.2, 0.98),
                Color::srgb(0.74, 0.96, 0.62),
            ),
            Interaction::Hovered => (
                Color::srgba(0.08, 0.28, 0.18, 0.98),
                Color::srgb(0.58, 0.86, 0.5),
            ),
            Interaction::None => (
                Color::srgba(0.052, 0.18, 0.13, 0.96),
                Color::srgb(0.38, 0.64, 0.38),
            ),
        };
    }

    let selected = action.is_selected(selection);
    match (selected, interaction) {
        (true, Interaction::Pressed) => (
            Color::srgba(0.32, 0.22, 0.08, 0.98),
            Color::srgb(1.0, 0.78, 0.36),
        ),
        (true, _) => (
            Color::srgba(0.22, 0.16, 0.065, 0.98),
            Color::srgb(0.88, 0.62, 0.28),
        ),
        (false, Interaction::Pressed) => (
            Color::srgba(0.08, 0.1, 0.1, 0.96),
            Color::srgb(0.42, 0.5, 0.48),
        ),
        (false, Interaction::Hovered) => (
            Color::srgba(0.062, 0.078, 0.078, 0.96),
            Color::srgb(0.42, 0.52, 0.5),
        ),
        (false, Interaction::None) => (
            Color::srgba(0.046, 0.058, 0.06, 0.94),
            Color::srgb(0.28, 0.34, 0.33),
        ),
    }
}

fn update_main_menu_summary(
    selection: Res<SkirmishMenuSelection>,
    mut summary_q: Query<&mut Text, With<MainMenuSummaryText>>,
    mut brief_q: Query<
        &mut Text,
        (
            With<MainMenuBriefStatusText>,
            Without<MainMenuSummaryText>,
            Without<MainMenuFactionInfoText>,
        ),
    >,
    mut faction_info_q: Query<
        &mut Text,
        (
            With<MainMenuFactionInfoText>,
            Without<MainMenuSummaryText>,
            Without<MainMenuBriefStatusText>,
        ),
    >,
    mut button_label_q: Query<
        (&MainMenuButtonLabel, &mut Text),
        (
            Without<MainMenuSummaryText>,
            Without<MainMenuBriefStatusText>,
            Without<MainMenuFactionInfoText>,
        ),
    >,
) {
    if !selection.is_changed() {
        return;
    }
    if let Ok(mut text) = summary_q.single_mut() {
        **text = main_menu_summary_text(*selection);
    }
    if let Ok(mut text) = brief_q.single_mut() {
        **text = main_menu_brief_status_text(*selection);
    }
    if let Ok(mut text) = faction_info_q.single_mut() {
        **text = main_menu_faction_info_text(*selection);
    }
    for (label, mut text) in &mut button_label_q {
        **text = main_menu_button_label_text(label.action, *selection);
    }
}

fn main_menu_brief_status_text(selection: SkirmishMenuSelection) -> String {
    let resources = selection.starting_resources();
    format!(
        "{}  |  资源 {}/{}",
        selection.start_status().summary_label(),
        resources.ore,
        resources.crystal,
    )
}

fn main_menu_summary_text(selection: SkirmishMenuSelection) -> String {
    let map = selection.map();
    let resources = selection.starting_resources();
    let focus_label = if selection.human_team().is_none() {
        "观战焦点"
    } else {
        "我方出生槽"
    };
    format!(
        "地图: {}  |  模式: {}  |  {}: {}  |  AI: {}\n控制: {}  |  队伍: {}\n种族: {}  |  颜色: {}\n参战玩家: {}/{}  |  需要出生点: {}  |  地图出生点: {}  |  资源: {}/{}  |  {}\n开始: Enter/点击开始对战",
        selection.map_label(),
        selection.match_mode.label(),
        focus_label,
        selection.focus_team().label(),
        selection.ai_difficulty.short_label(),
        skirmish_player_controller_text(selection),
        skirmish_team_setup_text(selection),
        skirmish_player_faction_text(selection),
        skirmish_player_color_text(selection),
        selection.active_team_count(),
        selection.selected_map_player_slots(),
        selection.required_player_slots(),
        map.players,
        resources.ore,
        resources.crystal,
        selection.start_status().summary_label()
    )
}

fn skirmish_player_controller_text(selection: SkirmishMenuSelection) -> String {
    player_teams(selection.active_teams().len())
        .filter_map(|team| {
            selection
                .player_controller(team)
                .map(|controller| format!("{}={}", team.label(), controller.short_label()))
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn skirmish_team_setup_text(selection: SkirmishMenuSelection) -> String {
    let active_teams = selection.active_teams();
    player_teams(active_teams.len())
        .filter_map(|team| {
            let index = team.economy_index()?;
            active_teams.get(index).copied().unwrap_or(false).then(|| {
                format!(
                    "{}=T{}",
                    team.label(),
                    selection.team_id(team).unwrap_or(0) + 1
                )
            })
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn skirmish_player_faction_text(selection: SkirmishMenuSelection) -> String {
    let active_teams = selection.active_teams();
    player_teams(active_teams.len())
        .filter_map(|team| {
            let index = team.economy_index()?;
            active_teams.get(index).copied().unwrap_or(false).then(|| {
                format!(
                    "{}={}",
                    team.label(),
                    selection
                        .player_faction(team)
                        .unwrap_or_else(|| SkirmishFaction::from_team(team))
                        .label()
                )
            })
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn skirmish_player_color_text(selection: SkirmishMenuSelection) -> String {
    let active_teams = selection.active_teams();
    player_teams(active_teams.len())
        .filter_map(|team| {
            let index = team.economy_index()?;
            active_teams.get(index).copied().unwrap_or(false).then(|| {
                format!(
                    "{}=C{}",
                    team.label(),
                    selection.player_color_slot(team).unwrap_or(index) + 1
                )
            })
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn main_menu_faction_info_text(selection: SkirmishMenuSelection) -> String {
    let faction = selection.focus_faction();
    format!(
        "对手: {}  |  {}  |  {}",
        skirmish_opponents_text(selection),
        skirmish_faction_roster_summary(faction),
        skirmish_faction_playstyle_summary(faction)
    )
}

fn skirmish_opponents_text(selection: SkirmishMenuSelection) -> String {
    let focus_team = selection.focus_team();
    let active_teams = selection.active_teams();
    let relations = selection.team_relations();
    match selection.match_mode {
        SkirmishMatchMode::OneVsOne | SkirmishMatchMode::AiVsAi => player_teams(active_teams.len())
            .find(|team| *team != focus_team && relations.are_enemies(focus_team, *team))
            .map(|team| team.label().to_string())
            .unwrap_or_else(|| "无".to_string()),
        SkirmishMatchMode::FreeForAll => player_teams(active_teams.len())
            .filter(|team| *team != focus_team && relations.are_enemies(focus_team, *team))
            .map(Team::label)
            .collect::<Vec<_>>()
            .join("/"),
        SkirmishMatchMode::AlliedTwoVsOne => match (
            allied_skirmish_ally(focus_team, active_teams.len()),
            allied_skirmish_enemy(focus_team, active_teams.len()),
        ) {
            (Some(ally), Some(enemy)) => {
                format!("{}（盟友: {}）", enemy.label(), ally.label())
            }
            _ => "无".to_string(),
        },
    }
}

fn skirmish_faction_roster_summary(faction: SkirmishFaction) -> String {
    faction_roster_summary_for_id(faction.registry_id())
}

fn faction_roster_summary_for_id(faction_id: &str) -> String {
    let Some(faction) = registry::faction(faction_id) else {
        return "资料缺失".to_string();
    };
    format!(
        "建筑 {}  步兵 {}  载具 {}  空军 {}",
        faction.structures.len(),
        faction_product_count(faction, "Barracks"),
        faction_product_count(faction, "VehicleFactory"),
        faction_product_count(faction, "AircraftFactory")
    )
}

fn faction_product_count(faction: &registry::FactionDef, producer: &str) -> usize {
    faction.production_for(producer).map_or(0, <[&str]>::len)
}

fn faction_playstyle_summary(faction: SkirmishFaction) -> &'static str {
    match faction {
        SkirmishFaction::Alliance => "人族: 全科技混合军，防御和兵种最完整，适合稳步推进",
        SkirmishFaction::Demon => "魔族: 火力突击和攻城压制，单位线更集中，适合快速正面进攻",
        SkirmishFaction::Chaos => "混沌族: 护盾、无人机、干扰和高阶防御，适合控场消耗",
    }
}

fn skirmish_faction_playstyle_summary(faction: SkirmishFaction) -> &'static str {
    faction_playstyle_summary(faction)
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next_id: ResMut<NextSpawnId>,
    selected_map: Res<SelectedSkirmishMap>,
    setup_settings: Res<MatchSetupSettings>,
    camera_state: Res<RtsCamera>,
) {
    let skirmish_map = selected_map.definition();
    let catalog_consistent = skirmish_map.is_catalog_consistent();
    debug_assert!(catalog_consistent);
    let map_bounds = MapBounds::from_map(skirmish_map);
    commands.insert_resource(map_bounds);

    commands.spawn((
        Camera3d::default(),
        camera_transform_from_state(&camera_state),
        camera_projection_from_state(&camera_state),
        MainCamera,
        MatchScopedEntity,
    ));

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 14_000.0,
            ..default()
        },
        Transform::from_xyz(-4.0, 12.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
        MatchScopedEntity,
    ));

    commands.spawn((
        Name::new(format!("{} Terrain", skirmish_map.name)),
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(skirmish_map.size.0, skirmish_map.size.1),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.2),
            perceptual_roughness: 0.92,
            ..default()
        })),
        MatchScopedEntity,
    ));

    for x in [-17.0, -8.0, 6.0, 15.0] {
        spawn_prop(
            &mut commands,
            &asset_server,
            "models/kenney-spacekit/rock_crystalsLargeA.glb",
            Vec3::new(x, 0.0, -2.0 + x.sin() * 8.0),
            0.9,
        );
    }

    for team in player_teams(setup_settings.active_teams.len())
        .filter(|team| setup_settings.team_active(*team))
    {
        let spawn_slot = setup_settings.player_spawn_slot(team);
        let Some(spawn_point) = skirmish_map.spawn_points.get(spawn_slot).copied() else {
            continue;
        };
        let visible_team = setup_settings.visible_player.team;
        setup_team(
            &mut commands,
            &asset_server,
            &mut next_id,
            team,
            setup_settings.player_faction(team),
            visible_team,
            map_local_to_world(skirmish_map, spawn_point),
            setup_settings.startup_loadout,
        );
    }
    setup_resource_nodes(&mut commands, &asset_server, skirmish_map);
    setup_supply_crates(&mut commands, &asset_server, skirmish_map);
    setup_neutral_tech(
        &mut commands,
        &asset_server,
        &mut next_id,
        skirmish_map,
        setup_settings.visible_player.team,
    );
    setup_ui(&mut commands, &asset_server);
}

#[cfg(test)]
fn team_startup(team: Team) -> &'static TeamStartup {
    team_startup_for_loadout(team, StartupLoadoutMode::PlaytestExpanded)
}

#[allow(dead_code)]
fn team_startup_for_loadout(team: Team, loadout: StartupLoadoutMode) -> &'static TeamStartup {
    faction_startup_for_loadout(SkirmishFaction::from_team(team), loadout)
}

fn faction_startup_for_loadout(
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> &'static TeamStartup {
    if loadout == StartupLoadoutMode::GodotSkirmish {
        return &GODOT_SKIRMISH_STARTUP;
    }
    match faction {
        SkirmishFaction::Alliance => &HUMAN_STARTUP,
        SkirmishFaction::Demon => &DEMON_STARTUP,
        SkirmishFaction::Chaos => &CHAOS_STARTUP,
    }
}

#[allow(dead_code)]
fn team_ai_profile(team: Team) -> &'static TeamAiProfile {
    faction_ai_profile(SkirmishFaction::from_team(team))
}

fn faction_ai_profile(faction: SkirmishFaction) -> &'static TeamAiProfile {
    match faction {
        SkirmishFaction::Alliance => &HUMAN_AI_PROFILE,
        SkirmishFaction::Demon => &DEMON_AI_PROFILE,
        SkirmishFaction::Chaos => &CHAOS_AI_PROFILE,
    }
}

#[allow(dead_code)]
fn team_ai_profile_for_difficulty(team: Team, difficulty: AiDifficulty) -> TeamAiProfile {
    faction_ai_profile_for_difficulty(SkirmishFaction::from_team(team), difficulty)
}

fn faction_ai_profile_for_difficulty(
    faction: SkirmishFaction,
    difficulty: AiDifficulty,
) -> TeamAiProfile {
    let mut profile = *faction_ai_profile(faction);
    match difficulty {
        AiDifficulty::Beginner => {
            profile.production_priority = BEGINNER_AI_PRODUCTION_PRIORITY;
            profile.defense_priority = BEGINNER_AI_DEFENSE_PRIORITY;
            profile.defense_limits = BEGINNER_AI_DEFENSE_LIMITS;
            profile.expected_command_centers = 1;
            profile.expected_workers = 1;
            profile.expected_refineries = 1;
            profile.expected_ore_harvesters = 0;
            profile.expected_battlegroups = 0;
            profile.expected_units_in_battlegroup = 0;
            profile.active_offense_enabled = false;
            profile.capture_enabled = false;
            profile.saboteur_enabled = false;
            profile.support_powers_enabled = false;
            profile.production_interval = 7.5;
            profile.build_interval = 14.0;
            profile.attack_interval = 12.0;
            profile.capture_interval = 12.0;
            profile.saboteur_interval = 12.0;
            profile.support_interval = 12.0;
            profile.defense_limit_bonus = 0;
        }
        AiDifficulty::Easy => {
            profile.expected_command_centers = 1;
            profile.expected_workers = 2;
            profile.expected_refineries = 1;
            profile.expected_ore_harvesters = 1;
            profile.expected_battlegroups = 1;
            profile.expected_units_in_battlegroup = 3;
            profile.capture_enabled = false;
            profile.saboteur_enabled = false;
            profile.support_powers_enabled = false;
            profile.production_interval = 5.5;
            profile.attack_interval = 9.0;
            profile.build_interval = 13.0;
            profile.capture_interval = AI_CAPTURE_INTERVAL_SECONDS + 2.0;
            profile.saboteur_interval = AI_SABOTEUR_INTERVAL_SECONDS + 3.0;
            profile.support_interval = 5.5;
        }
        AiDifficulty::Normal => {}
        AiDifficulty::Hard => {
            profile.expected_command_centers = 2;
            profile.expected_workers = 4;
            profile.expected_refineries = 2;
            profile.expected_ore_harvesters = 3;
            profile.expected_battlegroups = 3;
            profile.expected_units_in_battlegroup = 5;
            profile.production_interval = 3.0;
            profile.attack_interval = 4.5;
            profile.build_interval = 8.0;
            profile.capture_interval = (AI_CAPTURE_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.saboteur_interval = (AI_SABOTEUR_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.support_interval = 2.5;
            profile.defense_limit_bonus = 1;
        }
    }
    if matches!(difficulty, AiDifficulty::Beginner) {
        debug_assert!(!ai_profile_requests_offensive_combat_units(&profile));
    }
    profile
}

fn ai_profile_requests_offensive_combat_units(profile: &TeamAiProfile) -> bool {
    profile.production_priority.iter().any(|id| {
        registry::entity(id).is_some_and(|def| {
            def.weapon.is_some() && def.speed > 0.0 && !matches!(def.id, "Worker" | "OreHarvester")
        })
    })
}

fn setup_team(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    base: Vec3,
    loadout: StartupLoadoutMode,
) {
    let startup = faction_startup_for_loadout(faction, loadout);
    for spawn in startup.structures {
        spawn_structure_for_faction(
            commands,
            asset_server,
            next_id,
            spawn.id,
            team,
            visible_team,
            base + Vec3::new(spawn.offset.0, 0.0, spawn.offset.1),
            faction,
        );
    }
    for spawn in startup.units {
        spawn_unit_for_faction(
            commands,
            asset_server,
            next_id,
            spawn.id,
            team,
            base + Vec3::new(spawn.offset.0, 0.0, spawn.offset.1),
            0,
            faction,
            visible_team,
        );
    }
}

fn setup_neutral_tech(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    map: &SkirmishMapDef,
    visible_team: Team,
) {
    for spawn in map.neutral_tech {
        spawn_structure(
            commands,
            asset_server,
            next_id,
            spawn.id,
            Team::Neutral,
            visible_team,
            map_local_to_world(map, spawn.position),
        );
    }
}

fn setup_resource_nodes(commands: &mut Commands, asset_server: &AssetServer, map: &SkirmishMapDef) {
    for spec in map.resources {
        spawn_resource_node(
            commands,
            asset_server,
            spec.kind,
            spec.amount,
            map_local_to_world(map, spec.position),
        );
    }
}

fn spawn_resource_node(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kind: ResourceKind,
    amount: i32,
    position: Vec3,
) -> Entity {
    let scale = match kind {
        ResourceKind::Ore => 0.5,
        ResourceKind::Crystal => 0.38,
    };
    let radius = match kind {
        ResourceKind::Ore => 0.68,
        ResourceKind::Crystal => 0.55,
    };
    let entity_id = commands
        .spawn((
            Name::new(format!("{} node", kind.label())),
            Transform::from_translation(position),
            ResourceNode { kind, amount },
            Team::Neutral,
            Selectable { radius },
            VisibilityState { visible: false },
            Visibility::Hidden,
            MatchScopedEntity,
        ))
        .id();
    commands.entity(entity_id).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(
                asset_server.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset("models/kenney-spacekit/rock_crystalsLargeA.glb"),
                ),
            ),
            Transform::from_translation(Vec3::Y * 0.03).with_scale(Vec3::splat(scale)),
        ));
    });
    entity_id
}

fn setup_supply_crates(commands: &mut Commands, asset_server: &AssetServer, map: &SkirmishMapDef) {
    for spec in map.supply_crates {
        spawn_supply_crate(
            commands,
            asset_server,
            spec.effect,
            map_local_to_world(map, spec.position),
        );
    }
}

fn spawn_supply_crate(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: SupplyCrateEffect,
    position: Vec3,
) -> Entity {
    let entity_id = commands
        .spawn((
            Name::new(effect.label()),
            Transform::from_translation(position + Vec3::Y * 0.12),
            SupplyCrate {
                effect,
                pickup_radius: SUPPLY_CRATE_PICKUP_RADIUS,
                resource_ore: SUPPLY_CRATE_RESOURCE_ORE,
                resource_crystal: SUPPLY_CRATE_RESOURCE_CRYSTAL,
                repair_radius: SUPPLY_CRATE_REPAIR_RADIUS,
                repair_amount: SUPPLY_CRATE_REPAIR_AMOUNT,
            },
            Team::Neutral,
            Selectable {
                radius: SUPPLY_CRATE_PICKUP_RADIUS,
            },
            VisibilityState { visible: false },
            Visibility::Hidden,
            MatchScopedEntity,
        ))
        .id();
    commands.entity(entity_id).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/monorail_trainBox.glb"),
            )),
            Transform::from_scale(Vec3::splat(0.42)),
        ));
    });
    entity_id
}

fn spawn_prop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    model: &'static str,
    position: Vec3,
    scale: f32,
) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model))),
        Transform::from_translation(position).with_scale(Vec3::splat(scale)),
        MatchScopedEntity,
    ));
}

#[allow(dead_code)]
fn spawn_unit(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    visible_team: Team,
) -> Entity {
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        initial_veterancy_rank,
        default_visual_faction(team),
        visible_team,
    )
}

fn spawn_unit_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    faction: SkirmishFaction,
    visible_team: Team,
) -> Entity {
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        initial_veterancy_rank,
        Some(faction),
        visible_team,
    )
}

fn spawn_unit_with_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    visual_faction: Option<SkirmishFaction>,
    visible_team: Team,
) -> Entity {
    let Some(def) = registry::entity(id) else {
        return commands.spawn_empty().id();
    };
    next_id.0 += 1;
    let position = position + Vec3::Y * def.height;
    let unit_speed = if def.mine_trigger_radius > 0.0 {
        0.0
    } else {
        def.speed
    };
    let can_gain_veterancy = def
        .weapon
        .is_some_and(|weapon| unit_speed > 0.0 && weapon.damage > 0.0);
    let initial_veterancy_rank = if can_gain_veterancy {
        initial_veterancy_rank.min(VETERANCY_MAX_RANK)
    } else {
        0
    };
    let veterancy_idx = initial_veterancy_rank as usize;
    let base_vision = unit_vision_radius(def);
    let initial_health = (def.health * VETERANCY_HP_MULTIPLIER_BY_RANK[veterancy_idx]).ceil();
    let initial_vision = base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[veterancy_idx];
    let initial_visible = initial_visibility_state(team, visible_team);
    let entity_id = commands
        .spawn((
            Name::new(format!("{} {}", team.label(), def.label)),
            Transform::from_translation(position).with_scale(Vec3::splat(def.scale)),
            Unit {
                id,
                speed: unit_speed,
                can_crush: def.can_crush,
                can_be_crushed: def.can_be_crushed,
            },
            HoldPosition { enabled: false },
            team,
            Selectable { radius: def.radius },
            Health::new(initial_health),
            VisionRadius(initial_vision),
            initial_visible,
            MovementDomain::from_registry(def.domain),
            initial_visibility(team, visible_team),
            MatchScopedEntity,
        ))
        .id();
    if let Some(faction) = visual_faction {
        commands
            .entity(entity_id)
            .try_insert(VisualFaction(faction));
    }
    spawn_entity_models(commands, asset_server, entity_id, visual_faction, def);
    if let Some(weapon) = def.weapon {
        let weapon_damage =
            (weapon.damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[veterancy_idx] * 10.0).round()
                / 10.0;
        let weapon_range = weapon.range + VETERANCY_RANGE_BONUS_BY_RANK[veterancy_idx];
        commands.entity(entity_id).try_insert(Weapon::new(
            weapon_range,
            weapon_damage,
            weapon.cooldown,
            weapon.splash_radius,
            weapon.splash_damage_multiplier,
            weapon.structure_damage_multiplier,
            weapon.can_attack_air,
            weapon.can_attack_ground,
        ));
        if can_gain_veterancy {
            commands.entity(entity_id).try_insert(Veterancy {
                rank: initial_veterancy_rank,
                experience_points: VETERANCY_KILLS_BY_RANK[veterancy_idx],
                base_health: def.health,
                base_damage: weapon.damage,
                base_range: weapon.range,
                base_vision,
            });
        }
    }
    if def.resource_capacity > 0 {
        commands.entity(entity_id).try_insert(ResourceCargo {
            capacity: def.resource_capacity,
            ore: 0,
            crystal: 0,
        });
    }
    if def.mine_damage > 0.0 && def.mine_trigger_radius > 0.0 && def.mine_blast_radius > 0.0 {
        commands.entity(entity_id).try_insert(Mine {
            damage: def.mine_damage,
            trigger_radius: def.mine_trigger_radius,
            blast_radius: def.mine_blast_radius,
            arming_remaining: def.mine_arming_delay,
            source: None,
        });
    }
    if def.mine_deploy_interval > 0.0 && def.mine_limit > 0 {
        commands.entity(entity_id).try_insert(MineLayer {
            damage: def.mine_damage,
            deploy_interval: def.mine_deploy_interval,
            deploy_radius: def.mine_deploy_radius,
            spacing: def.mine_spacing,
            limit: def.mine_limit,
            cooldown: 0.2,
            deploy_index: 0,
        });
    }
    attach_support_effects(commands, entity_id, def);
    entity_id
}

fn spawn_structure(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        0.0,
        default_visual_faction(team),
    )
}

fn spawn_structure_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    faction: SkirmishFaction,
) -> Entity {
    spawn_structure_with_rotation_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        0.0,
        faction,
    )
}

#[allow(dead_code)]
fn spawn_structure_with_rotation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        rotation_y_radians,
        default_visual_faction(team),
    )
}

fn spawn_structure_with_rotation_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
    faction: SkirmishFaction,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        rotation_y_radians,
        Some(faction),
    )
}

fn spawn_structure_for_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
    visual_faction: Option<SkirmishFaction>,
) -> Entity {
    let Some(def) = registry::entity(id) else {
        return commands.spawn_empty().id();
    };
    next_id.0 += 1;
    let initial_visible = initial_visibility_state(team, visible_team);
    let entity_id = commands
        .spawn((
            Name::new(format!("{} {}", team.label(), def.label)),
            Transform::from_translation(position)
                .with_rotation(Quat::from_rotation_y(rotation_y_radians))
                .with_scale(Vec3::splat(def.scale)),
            Structure { id },
            team,
            Selectable { radius: def.radius },
            Health::new(def.health),
            VisionRadius(structure_vision_radius(def)),
            initial_visible,
            MovementDomain::from_registry(def.domain),
            initial_visibility(team, visible_team),
            MatchScopedEntity,
        ))
        .id();
    if let Some(faction) = visual_faction {
        commands
            .entity(entity_id)
            .try_insert(VisualFaction(faction));
    }
    spawn_entity_models(commands, asset_server, entity_id, visual_faction, def);
    if let Some(weapon) = def.weapon {
        commands.entity(entity_id).try_insert(Weapon::new(
            weapon.range,
            weapon.damage,
            weapon.cooldown,
            weapon.splash_radius,
            weapon.splash_damage_multiplier,
            weapon.structure_damage_multiplier,
            weapon.can_attack_air,
            weapon.can_attack_ground,
        ));
    }
    if is_rally_point_structure(id) {
        commands.entity(entity_id).try_insert(RallyPoint {
            target: None,
            target_unit: None,
        });
    }
    attach_support_effects(commands, entity_id, def);
    entity_id
}

#[allow(dead_code)]
fn spawn_structure_under_construction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_harvester_origin: Option<Vec3>,
    rotation_y_radians: f32,
    visible_team: Team,
) -> Entity {
    spawn_structure_under_construction_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        free_harvester_origin,
        rotation_y_radians,
        visible_team,
        default_visual_faction(team),
    )
}

fn spawn_structure_under_construction_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_harvester_origin: Option<Vec3>,
    rotation_y_radians: f32,
    visible_team: Team,
    faction: SkirmishFaction,
) -> Entity {
    spawn_structure_under_construction_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        free_harvester_origin,
        rotation_y_radians,
        visible_team,
        Some(faction),
    )
}

fn spawn_structure_under_construction_with_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_harvester_origin: Option<Vec3>,
    rotation_y_radians: f32,
    visible_team: Team,
    visual_faction: Option<SkirmishFaction>,
) -> Entity {
    let Some(def) = registry::entity(id) else {
        return commands.spawn_empty().id();
    };
    let entity = spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        rotation_y_radians,
        visual_faction,
    );
    commands.entity(entity).try_insert((
        UnderConstruction {
            remaining: 1.0,
            total: 1.0,
            cost: def.cost,
            free_harvester_origin,
        },
        Health {
            current: 1.0,
            max: def.health,
        },
    ));
    entity
}

fn progress_under_construction_structures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_id: ResMut<NextSpawnId>,
    map_bounds: Res<MapBounds>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    mut structures: Query<(
        Entity,
        &Structure,
        &Team,
        &Transform,
        Option<&VisualFaction>,
        &mut Health,
        &mut UnderConstruction,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, structure, team, transform, visual_faction, mut health, construction) in
        &mut structures
    {
        let progress = structure_construction_progress(*construction);
        health.current = structure_construction_health(health.max, progress);
        if construction.remaining > 0.0 {
            continue;
        }

        health.current = health.max;
        commands.entity(entity).try_remove::<UnderConstruction>();
        if let Some(origin) = construction.free_harvester_origin {
            let spawn_seed = next_id.0 + 17;
            spawn_refinery_free_harvester(
                &mut commands,
                &asset_server,
                &mut next_id,
                structure.id,
                *team,
                player_team,
                transform.translation,
                origin,
                spawn_seed,
                *map_bounds,
                visual_faction.copied().map(|faction| faction.0),
            );
        }
        if *team == player_team {
            let label = registry::entity(structure.id)
                .map(|def| def.label)
                .unwrap_or(structure.id);
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::ProductionReady);
            record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::ConstructionComplete);
            record_production_ready_battle_log(
                *team,
                player_team,
                true,
                label,
                transform.translation,
                &mut battle_log,
            );
        }
    }
}

fn structure_construction_progress(construction: UnderConstruction) -> f32 {
    if construction.total <= 0.0 {
        return 1.0;
    }
    ((construction.total - construction.remaining) / construction.total).clamp(0.0, 1.0)
}

fn structure_construction_health(max_health: f32, progress: f32) -> f32 {
    if max_health <= 1.0 {
        return max_health;
    }
    (1.0 + (max_health - 1.0) * progress.clamp(0.0, 1.0)).clamp(1.0, max_health)
}

fn apply_structure_construction_progress(
    construction: &mut UnderConstruction,
    health: &mut Health,
    delta_seconds: f32,
) {
    construction.remaining = (construction.remaining
        - STRUCTURE_CONSTRUCTION_PROGRESS_PER_SECOND * delta_seconds)
        .max(0.0);
    let progress = structure_construction_progress(*construction);
    health.current = structure_construction_health(health.max, progress);
}

fn structure_is_constructed(under_construction: Option<&UnderConstruction>) -> bool {
    under_construction.is_none()
}

fn active_structure_power_delta(
    structure: &Structure,
    under_construction: Option<&UnderConstruction>,
) -> Option<i32> {
    if !structure_is_constructed(under_construction) {
        return None;
    }
    registry::entity(structure.id).map(|def| def.power_delta)
}

fn is_rally_point_structure(id: &str) -> bool {
    matches!(
        id,
        "CommandCenter" | "Barracks" | "VehicleFactory" | "AircraftFactory"
    )
}

fn attach_support_effects(commands: &mut Commands, entity_id: Entity, def: &registry::EntityDef) {
    if def.support_shield_radius > 0.0 && def.support_shield_duration > 0.0 {
        commands
            .entity(entity_id)
            .try_insert(MobileShieldProjector {
                refresh_remaining: 0.0,
                radius: def.support_shield_radius,
                duration: def.support_shield_duration,
                damage_scale: def.support_shield_damage_multiplier,
            });
    }
    if def.id == "ShieldTrooper" {
        commands.entity(entity_id).try_insert(PassiveSupportShield {
            damage_scale: SHIELD_TROOPER_PASSIVE_DAMAGE_SCALE,
        });
    }
    if let Some(mode) = passive_repair_aura_mode(def) {
        commands.entity(entity_id).try_insert(RepairAura {
            rate: def.repair_rate,
            radius: def.repair_radius,
            mode,
        });
    }
    if def.healing_rate > 0.0 && def.healing_radius > 0.0 {
        commands.entity(entity_id).try_insert(HealingAura {
            rate: def.healing_rate,
            radius: def.healing_radius,
        });
    }
    if def.income_interval > 0.0 && (def.resource_income_ore > 0 || def.resource_income_crystal > 0)
    {
        commands.entity(entity_id).try_insert(IncomeSource {
            ore: def.resource_income_ore,
            crystal: def.resource_income_crystal,
            interval: def.income_interval,
            remaining: def.income_interval,
        });
    }
    if def.garrison_capacity > 0 && def.garrison_attack_damage_per_unit > 0.0 {
        commands.entity(entity_id).try_insert(Garrison {
            capacity: def.garrison_capacity,
            damage_per_unit: def.garrison_attack_damage_per_unit,
            count: 0,
        });
    }
}

fn passive_repair_aura_mode(def: &registry::EntityDef) -> Option<RepairAuraMode> {
    if def.repair_rate <= 0.0 || def.repair_radius <= 0.0 {
        return None;
    }
    match def.id {
        "TechRepairDepot" => Some(RepairAuraMode::AllEligible),
        "RepairPad" => Some(RepairAuraMode::NearestEligible),
        _ => None,
    }
}

fn can_receive_repair_aura(
    unit: Option<&Unit>,
    structure: Option<&Structure>,
    domain: &MovementDomain,
) -> bool {
    let Some(unit) = unit else {
        return false;
    };
    structure.is_none()
        && *domain == MovementDomain::Terrain
        && unit.speed > 0.0
        && !is_infantry_unit(unit)
}

#[derive(Clone, Copy)]
struct RepairCapability {
    rate: f32,
    radius: f32,
}

fn repair_capability(unit: &Unit) -> Option<RepairCapability> {
    let def = registry::entity(unit.id)?;
    (def.repair_rate > 0.0).then_some(RepairCapability {
        rate: def.repair_rate,
        radius: def.repair_radius,
    })
}

fn repair_order_range(capability: RepairCapability, source_radius: f32, target_radius: f32) -> f32 {
    if capability.radius > 0.0 {
        capability.radius + target_radius
    } else {
        source_radius + target_radius + REPAIR_ADHERENCE_MARGIN_M + REPAIR_ENTRY_MARGIN_M
    }
}

fn can_repair_order_target(
    unit: Option<&Unit>,
    structure: Option<&Structure>,
    under_construction: Option<&UnderConstruction>,
    health: &Health,
) -> bool {
    health.current > 0.0
        && health.current < health.max
        && (unit.is_some() || structure.is_some())
        && structure.is_none_or(|_| structure_is_constructed(under_construction))
}

fn powered_repair_offline(
    team: &Team,
    structure: Option<&Structure>,
    economies: &Economies,
) -> bool {
    let Some(structure) = structure else {
        return false;
    };
    if matches!(team, Team::Neutral) || !economies.get(*team).low_power() {
        return false;
    }
    registry::entity(structure.id)
        .is_some_and(|def| def.power_delta < 0 && def.repair_rate > 0.0 && def.repair_radius > 0.0)
}

fn can_receive_healing_aura(unit: Option<&Unit>) -> bool {
    unit.is_some_and(is_infantry_unit)
}

fn support_damage_scale(
    shield: Option<&SupportShield>,
    passive_shield: Option<&PassiveSupportShield>,
) -> f32 {
    shield
        .map(|shield| shield.damage_scale)
        .or_else(|| passive_shield.map(|shield| shield.damage_scale))
        .unwrap_or(1.0)
}

const HUMAN_SUPPORT_RATE_MULTIPLIER: f32 = 1.15;
const DEMON_STRUCTURE_WEAPON_DAMAGE_MULTIPLIER: f32 = 1.12;
const CHAOS_INCOMING_WEAPON_DAMAGE_SCALE: f32 = 0.9;

fn faction_support_rate_multiplier(faction: Option<SkirmishFaction>) -> f32 {
    match faction {
        Some(SkirmishFaction::Alliance) => HUMAN_SUPPORT_RATE_MULTIPLIER,
        Some(SkirmishFaction::Demon | SkirmishFaction::Chaos) | None => 1.0,
    }
}

fn faction_weapon_damage_multiplier(
    attacker_faction: Option<SkirmishFaction>,
    target_team: Team,
    target_is_structure: bool,
) -> f32 {
    if attacker_faction == Some(SkirmishFaction::Demon)
        && target_is_structure
        && target_team != Team::Neutral
    {
        DEMON_STRUCTURE_WEAPON_DAMAGE_MULTIPLIER
    } else {
        1.0
    }
}

fn faction_incoming_weapon_damage_scale(target_faction: Option<SkirmishFaction>) -> f32 {
    match target_faction {
        Some(SkirmishFaction::Chaos) => CHAOS_INCOMING_WEAPON_DAMAGE_SCALE,
        Some(SkirmishFaction::Alliance | SkirmishFaction::Demon) | None => 1.0,
    }
}

fn applied_weapon_damage(
    base_damage: f32,
    attacker_faction: Option<SkirmishFaction>,
    target_team: Team,
    target_faction: Option<SkirmishFaction>,
    target_is_structure: bool,
    shield: Option<&SupportShield>,
    passive_shield: Option<&PassiveSupportShield>,
) -> f32 {
    base_damage
        * faction_weapon_damage_multiplier(attacker_faction, target_team, target_is_structure)
        * faction_incoming_weapon_damage_scale(target_faction)
        * support_damage_scale(shield, passive_shield)
}

fn is_infantry_unit(unit: &Unit) -> bool {
    matches!(
        unit.id,
        "LightRifleInfantry"
            | "RocketInfantry"
            | "FieldMedic"
            | "ShieldTrooper"
            | "FlakRocketTeam"
            | "FlakRocketTeamMk2"
            | "HeavyMachinegunTrooper"
            | "ShockTrooper"
            | "GrenadierTrooper"
            | "MortarTeam"
            | "CryoSprayer"
            | "SniperScout"
            | "RailSniperTeam"
            | "PhaseSaboteur"
            | "SaboteurInfiltrator"
            | "PulseRifleCommando"
            | "TacticalOfficer"
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProceduralEntityModel {
    LandMine,
    TeslaFenceSegment,
}

impl ProceduralEntityModel {
    fn for_entity_id(id: &str) -> Option<Self> {
        match id {
            "LandMine" => Some(Self::LandMine),
            "TeslaFenceSegment" => Some(Self::TeslaFenceSegment),
            _ => None,
        }
    }

    #[cfg(test)]
    fn part_count(self) -> usize {
        match self {
            Self::LandMine => 2,
            Self::TeslaFenceSegment => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FactionIdentityMarker {
    Human,
    Demon,
    Chaos,
}

impl FactionIdentityMarker {
    fn for_faction(faction: SkirmishFaction) -> Self {
        match faction {
            SkirmishFaction::Alliance => Self::Human,
            SkirmishFaction::Demon => Self::Demon,
            SkirmishFaction::Chaos => Self::Chaos,
        }
    }

    #[allow(dead_code)]
    fn for_team(team: Team) -> Option<Self> {
        team.economy_index()
            .map(|_| Self::for_faction(SkirmishFaction::from_team(team)))
    }

    #[cfg(test)]
    fn part_count(self) -> usize {
        match self {
            Self::Human => 2,
            Self::Demon => 3,
            Self::Chaos => 2,
        }
    }
}

fn default_visual_faction(team: Team) -> Option<SkirmishFaction> {
    team.economy_index()
        .map(|_| SkirmishFaction::from_team(team))
}

fn spawn_entity_models(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    if def.render_parts.is_empty() {
        if let Some(model) = ProceduralEntityModel::for_entity_id(def.id) {
            spawn_procedural_entity_model(commands, root, model);
        } else {
            let fallback = DEFAULT_MODEL_FALLBACK;
            commands.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(fallback))),
                Transform::IDENTITY,
            ));
        }
    } else {
        for part in def.render_parts {
            let rotation = Quat::from_xyzw(
                part.rotation[0],
                part.rotation[1],
                part.rotation[2],
                part.rotation[3],
            );
            commands.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(part.model))),
                Transform::from_translation(Vec3::new(
                    part.translation[0],
                    part.translation[1],
                    part.translation[2],
                ))
                .with_rotation(rotation)
                .with_scale(Vec3::new(
                    part.scale[0],
                    part.scale[1],
                    part.scale[2],
                )),
            ));
        }
    }
    spawn_faction_identity_marker(commands, root, visual_faction, def);
}

fn spawn_faction_identity_marker(
    commands: &mut Commands,
    root: Entity,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    let Some(marker) = visual_faction.map(FactionIdentityMarker::for_faction) else {
        return;
    };
    let inv_scale = 1.0 / def.scale.max(0.1);
    let world_size = match def.role {
        registry::EntityRole::Structure => (def.radius * 0.26).clamp(0.28, 0.68),
        registry::EntityRole::Unit => (def.radius * 0.22).clamp(0.12, 0.26),
    };
    let local_size = world_size * inv_scale;
    let local_offset = Vec3::new(0.0, 0.12 * inv_scale, -def.radius * 0.72 * inv_scale);
    commands.queue(move |world: &mut World| {
        if world.get_entity(root).is_err() {
            return;
        }
        spawn_faction_identity_marker_model(world, root, marker, local_offset, local_size);
    });
}

fn spawn_faction_identity_marker_model(
    world: &mut World,
    root: Entity,
    marker: FactionIdentityMarker,
    local_offset: Vec3,
    size: f32,
) {
    match marker {
        FactionIdentityMarker::Human => {
            let Some(plate_mesh) =
                add_procedural_mesh(world, Cuboid::new(size * 1.35, size * 0.09, size * 0.46))
            else {
                return;
            };
            let Some(mast_mesh) =
                add_procedural_mesh(world, Cuboid::new(size * 0.14, size * 0.72, size * 0.14))
            else {
                return;
            };
            let Some(blue_material) = add_procedural_material(
                world,
                Color::srgb(0.16, 0.44, 0.98),
                0.35,
                0.32,
                LinearRgba::rgb(0.02, 0.07, 0.22),
            ) else {
                return;
            };
            let Some(white_material) = add_procedural_material(
                world,
                Color::srgb(0.86, 0.94, 1.0),
                0.25,
                0.22,
                LinearRgba::rgb(0.06, 0.08, 0.12),
            ) else {
                return;
            };
            spawn_procedural_mesh_child(
                world,
                root,
                "Human Faction Command Plate",
                plate_mesh,
                blue_material,
                Transform::from_translation(local_offset),
            );
            spawn_procedural_mesh_child(
                world,
                root,
                "Human Faction Signal Mast",
                mast_mesh,
                white_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.36),
            );
        }
        FactionIdentityMarker::Demon => {
            let Some(spike_mesh) = add_procedural_mesh(
                world,
                ConicalFrustum {
                    radius_top: 0.0,
                    radius_bottom: size * 0.22,
                    height: size * 0.95,
                }
                .mesh()
                .resolution(18),
            ) else {
                return;
            };
            let Some(spike_material) = add_procedural_material(
                world,
                Color::srgb(0.92, 0.12, 0.055),
                0.25,
                0.42,
                LinearRgba::rgb(0.55, 0.035, 0.01),
            ) else {
                return;
            };
            for (name, x) in [
                ("Demon Faction Left Spike", -0.42),
                ("Demon Faction Center Spike", 0.0),
                ("Demon Faction Right Spike", 0.42),
            ] {
                spawn_procedural_mesh_child(
                    world,
                    root,
                    name,
                    spike_mesh.clone(),
                    spike_material.clone(),
                    Transform::from_translation(
                        local_offset + Vec3::new(x * size, size * 0.44, 0.0),
                    ),
                );
            }
        }
        FactionIdentityMarker::Chaos => {
            let Some(ring_mesh) = add_procedural_mesh(
                world,
                Torus::new(size * 0.055, size * 0.31)
                    .mesh()
                    .minor_resolution(8)
                    .major_resolution(28),
            ) else {
                return;
            };
            let Some(core_mesh) = add_procedural_mesh(
                world,
                Cylinder::new(size * 0.16, size * 0.14)
                    .mesh()
                    .resolution(20),
            ) else {
                return;
            };
            let Some(ring_material) = add_procedural_material(
                world,
                Color::srgb(0.55, 0.22, 0.95),
                0.0,
                0.2,
                LinearRgba::rgb(0.45, 0.09, 1.2),
            ) else {
                return;
            };
            let Some(core_material) = add_procedural_material(
                world,
                Color::srgb(0.08, 0.9, 1.0),
                0.0,
                0.18,
                LinearRgba::rgb(0.05, 0.85, 1.2),
            ) else {
                return;
            };
            spawn_procedural_mesh_child(
                world,
                root,
                "Chaos Faction Energy Ring",
                ring_mesh,
                ring_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.18)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );
            spawn_procedural_mesh_child(
                world,
                root,
                "Chaos Faction Core",
                core_mesh,
                core_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.18),
            );
        }
    }
}

fn spawn_procedural_entity_model(
    commands: &mut Commands,
    root: Entity,
    model: ProceduralEntityModel,
) {
    commands.queue(move |world: &mut World| {
        if world.get_entity(root).is_err() {
            return;
        }

        match model {
            ProceduralEntityModel::LandMine => spawn_land_mine_procedural_model(world, root),
            ProceduralEntityModel::TeslaFenceSegment => {
                spawn_tesla_fence_segment_procedural_model(world, root)
            }
        }
    });
}

fn add_procedural_mesh(world: &mut World, mesh: impl Into<Mesh>) -> Option<Handle<Mesh>> {
    let mut meshes = world.get_resource_mut::<Assets<Mesh>>()?;
    Some(meshes.add(mesh))
}

fn add_procedural_material(
    world: &mut World,
    base_color: Color,
    metallic: f32,
    perceptual_roughness: f32,
    emissive: LinearRgba,
) -> Option<Handle<StandardMaterial>> {
    let mut materials = world.get_resource_mut::<Assets<StandardMaterial>>()?;
    Some(materials.add(StandardMaterial {
        base_color,
        metallic,
        perceptual_roughness,
        emissive,
        ..default()
    }))
}

fn spawn_procedural_mesh_child(
    world: &mut World,
    root: Entity,
    name: &'static str,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) {
    world.spawn((
        Name::new(name),
        ChildOf(root),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}

fn spawn_land_mine_procedural_model(world: &mut World, root: Entity) {
    let Some(body_mesh) = add_procedural_mesh(
        world,
        ConicalFrustum {
            radius_top: 0.34,
            radius_bottom: 0.38,
            height: 0.12,
        }
        .mesh()
        .resolution(32),
    ) else {
        return;
    };
    let Some(ring_mesh) = add_procedural_mesh(
        world,
        Torus::new(0.03, 0.31)
            .mesh()
            .minor_resolution(8)
            .major_resolution(32),
    ) else {
        return;
    };

    let Some(dark_material) = add_procedural_material(
        world,
        Color::srgb(0.055, 0.058, 0.065),
        0.7,
        0.4,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(team_material) = add_procedural_material(
        world,
        Color::srgb(0.99, 0.81, 0.48),
        0.55,
        0.35,
        LinearRgba::BLACK,
    ) else {
        return;
    };

    spawn_procedural_mesh_child(
        world,
        root,
        "LandMine Body",
        body_mesh,
        dark_material,
        Transform::from_xyz(0.0, 0.06, 0.0),
    );
    spawn_procedural_mesh_child(
        world,
        root,
        "LandMine TeamRing",
        ring_mesh,
        team_material,
        Transform::from_xyz(0.0, 0.14, 0.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
}

fn spawn_tesla_fence_segment_procedural_model(world: &mut World, root: Entity) {
    let Some(base_mesh) = add_procedural_mesh(world, Cuboid::new(1.55, 0.16, 0.5)) else {
        return;
    };
    let Some(post_mesh) = add_procedural_mesh(
        world,
        ConicalFrustum {
            radius_top: 0.12,
            radius_bottom: 0.16,
            height: 0.95,
        }
        .mesh()
        .resolution(24),
    ) else {
        return;
    };
    let Some(arc_mesh) = add_procedural_mesh(world, Cuboid::new(1.18, 0.06, 0.06)) else {
        return;
    };
    let Some(cap_mesh) =
        add_procedural_mesh(world, Cylinder::new(0.16, 0.08).mesh().resolution(24))
    else {
        return;
    };

    let Some(body_material) = add_procedural_material(
        world,
        Color::srgb(0.99, 0.81, 0.48),
        0.55,
        0.38,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(dark_material) = add_procedural_material(
        world,
        Color::srgb(0.08, 0.10, 0.11),
        0.65,
        0.35,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(arc_material) = add_procedural_material(
        world,
        Color::srgb(0.08, 0.78, 1.0),
        0.0,
        0.25,
        LinearRgba::rgb(0.09, 1.33, 1.8),
    ) else {
        return;
    };

    spawn_procedural_mesh_child(
        world,
        root,
        "TeslaFenceSegment Base",
        base_mesh,
        dark_material,
        Transform::from_xyz(0.0, 0.08, 0.0),
    );
    for (name, x) in [
        ("TeslaFenceSegment LeftPost", -0.62),
        ("TeslaFenceSegment RightPost", 0.62),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            post_mesh.clone(),
            body_material.clone(),
            Transform::from_xyz(x, 0.58, 0.0),
        );
    }
    for (name, z) in [
        ("TeslaFenceSegment ArcBeamFront", -0.14),
        ("TeslaFenceSegment ArcBeamBack", 0.14),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            arc_mesh.clone(),
            arc_material.clone(),
            Transform::from_xyz(0.0, 0.72, z),
        );
    }
    for (name, x) in [
        ("TeslaFenceSegment LeftCap", -0.62),
        ("TeslaFenceSegment RightCap", 0.62),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            cap_mesh.clone(),
            arc_material.clone(),
            Transform::from_xyz(x, 1.08, 0.0),
        );
    }
}

fn unit_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        def.radius * 5.0 + 3.5
    } else {
        FOG_REVEAL_RADIUS
    }
}

fn structure_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        (def.radius * 4.5 + 2.5).clamp(1.5, FOG_REVEAL_RADIUS)
    } else {
        0.0
    }
}

fn initial_visibility_state(team: Team, visible_team: Team) -> VisibilityState {
    VisibilityState {
        visible: team == visible_team,
    }
}

fn initial_visibility(team: Team, visible_team: Team) -> Visibility {
    if team == visible_team {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn setup_match_end_overlay(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            MatchEndOverlay,
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(520),
                        min_height: px(370),
                        padding: UiRect::all(px(20)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        align_items: AlignItems::Stretch,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(MATCH_END_BG_COLOR),
                    BorderColor::all(Color::srgba(0.18, 0.18, 0.22, 0.95)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("结算"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(MATCH_END_TITLE_FONT_SIZE),
                            ..default()
                        },
                        TextColor(MATCH_END_TITLE_COLOR),
                        MatchEndTitle,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(MATCH_END_TEXT_FONT_SIZE),
                            ..default()
                        },
                        TextColor(Color::srgba(0.87, 0.9, 0.95, 0.95)),
                        MatchEndReason,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(MATCH_END_TEXT_FONT_SIZE),
                            ..default()
                        },
                        TextColor(Color::srgba(0.9, 0.96, 0.97, 0.95)),
                        MatchEndStats,
                    ));
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(10),
                            row_gap: px(10),
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(px(8)),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn(match_end_button(MatchEndAction::Restart))
                                .with_children(|button| {
                                    button.spawn(match_end_button_label("重开对局", font.clone()));
                                });
                            row.spawn(match_end_button(MatchEndAction::ReturnToSetup))
                                .with_children(|button| {
                                    button.spawn(match_end_button_label("返回设置", font.clone()));
                                });
                            row.spawn(match_end_button(MatchEndAction::ExitToMenu))
                                .with_children(|button| {
                                    button.spawn(match_end_button_label("退出菜单", font.clone()));
                                });
                        });
                });
        });
}

fn match_end_button(action: MatchEndAction) -> impl Bundle {
    (
        Button,
        MatchEndButton { action },
        Node {
            width: px(145),
            height: px(42),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.36, 0.42)),
        BackgroundColor(Color::srgba(0.055, 0.072, 0.088, 0.94)),
    )
}

fn match_end_button_label(label: &'static str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

fn setup_match_menu_overlay(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(45),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.42)),
            MatchMenuOverlay,
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(430),
                        min_height: px(320),
                        padding: UiRect::all(px(22)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        align_items: AlignItems::Stretch,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.96)),
                    BorderColor::all(Color::srgba(0.34, 0.44, 0.52, 0.96)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("对局菜单"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.93, 0.97, 1.0)),
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.86, 0.9)),
                        MatchMenuStatusText,
                    ));
                    panel
                        .spawn(match_menu_button(MatchMenuAction::Resume))
                        .with_children(|button| {
                            button.spawn(match_menu_button_label("继续战斗", font.clone()));
                        });
                    panel.spawn(match_menu_speed_row(font.clone()));
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(10),
                            row_gap: px(10),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn(match_menu_button(MatchMenuAction::PreviousPerspective))
                                .with_children(|button| {
                                    button.spawn(match_menu_button_label("上一视角", font.clone()));
                                });
                            row.spawn(match_menu_button(MatchMenuAction::NextPerspective))
                                .with_children(|button| {
                                    button.spawn(match_menu_button_label("下一视角", font.clone()));
                                });
                        });
                    panel
                        .spawn(match_menu_button(MatchMenuAction::Restart))
                        .with_children(|button| {
                            button.spawn(match_menu_button_label("重开对局", font.clone()));
                        });
                    panel
                        .spawn(match_menu_button(MatchMenuAction::ReturnToSetup))
                        .with_children(|button| {
                            button.spawn(match_menu_button_label("返回设置", font.clone()));
                        });
                });
        });
}

fn match_menu_speed_row(font: Handle<Font>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            row_gap: px(8),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Text::new("游戏速度"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.86, 0.9)),
                Node {
                    width: px(78),
                    ..default()
                },
            ),
            match_menu_speed_button(MatchSpeedPreset::ALL[0], font.clone()),
            match_menu_speed_button(MatchSpeedPreset::ALL[1], font.clone()),
            match_menu_speed_button(MatchSpeedPreset::ALL[2], font.clone()),
            match_menu_speed_button(MatchSpeedPreset::ALL[3], font.clone()),
            match_menu_speed_button(MatchSpeedPreset::ALL[4], font),
        ],
    )
}

fn match_menu_speed_button(preset: MatchSpeedPreset, font: Handle<Font>) -> impl Bundle {
    (
        match_menu_button(MatchMenuAction::SetSpeed(preset)),
        children![match_menu_button_label(preset.label(), font)],
    )
}

fn match_menu_button(action: MatchMenuAction) -> impl Bundle {
    (
        Button,
        MatchMenuButton { action },
        Node {
            flex_grow: 1.0,
            height: px(46),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.36, 0.42)),
        BackgroundColor(Color::srgba(0.055, 0.072, 0.088, 0.94)),
    )
}

fn match_menu_button_label(label: &'static str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

fn setup_match_briefing(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Button,
            MatchBriefingButton {
                action: MatchBriefingAction::Show,
            },
            MatchBriefingReopenButton,
            Visibility::Hidden,
            GlobalZIndex(34),
            Node {
                position_type: PositionType::Absolute,
                left: px(14),
                top: px(76),
                width: px(92),
                height: px(32),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.46, 0.48)),
            BackgroundColor(Color::srgba(0.035, 0.055, 0.065, 0.94)),
            MatchScopedEntity,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("目标"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.96, 0.96)),
            ));
        });

    commands
        .spawn((
            MatchBriefingPanel,
            Visibility::Hidden,
            GlobalZIndex(35),
            Node {
                position_type: PositionType::Absolute,
                left: px(14),
                top: px(112),
                width: px(430),
                padding: UiRect::axes(px(12), px(10)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.42, 0.78, 0.76, 1.0)),
            BackgroundColor(Color::srgba(0.035, 0.055, 0.065, 0.94)),
            MatchScopedEntity,
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("战斗简报"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.68, 0.93, 1.0)),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    header
                        .spawn((
                            Button,
                            MatchBriefingButton {
                                action: MatchBriefingAction::Dismiss,
                            },
                            Node {
                                width: px(28),
                                height: px(24),
                                border: UiRect::all(px(1)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.28, 0.36, 0.42)),
                            BackgroundColor(Color::srgba(0.055, 0.072, 0.088, 0.94)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("X"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.92, 0.96, 0.98)),
                            ));
                        });
                });

            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.88, 0.88)),
                Node {
                    max_width: px(396),
                    ..default()
                },
                MatchBriefingText,
            ));
        });
}

fn match_briefing_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut briefing: ResMut<MatchBriefingState>,
    mut buttons: Query<(
        &Interaction,
        &MatchBriefingButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (interaction, button, mut background, mut border) in &mut buttons {
        if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            match button.action {
                MatchBriefingAction::Show => briefing.show(),
                MatchBriefingAction::Dismiss => briefing.dismiss(),
            }
        }

        let (bg, border_color) = match_briefing_button_visual(*interaction);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

fn match_briefing_button_visual(interaction: Interaction) -> (Color, Color) {
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.16, 0.28, 0.32, 0.98),
            Color::srgb(0.7, 0.9, 0.92),
        ),
        Interaction::Hovered => (
            Color::srgba(0.1, 0.18, 0.2, 0.96),
            Color::srgb(0.5, 0.75, 0.76),
        ),
        Interaction::None => (
            Color::srgba(0.035, 0.055, 0.065, 0.94),
            Color::srgb(0.28, 0.46, 0.48),
        ),
    }
}

fn update_match_briefing_overlay(
    time: Res<Time>,
    mut briefing: ResMut<MatchBriefingState>,
    setup_settings: Res<MatchSetupSettings>,
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    active_teams: Res<ActiveTeams>,
    mut panels: Query<
        &mut Visibility,
        (With<MatchBriefingPanel>, Without<MatchBriefingReopenButton>),
    >,
    mut reopen_buttons: Query<
        &mut Visibility,
        (With<MatchBriefingReopenButton>, Without<MatchBriefingPanel>),
    >,
    mut briefing_text: Query<&mut Text, With<MatchBriefingText>>,
) {
    if briefing.visible && briefing.auto_hide_seconds > 0.0 {
        briefing.elapsed_seconds += time.delta_secs();
        if briefing.elapsed_seconds >= briefing.auto_hide_seconds {
            briefing.dismiss();
        }
    }

    let panel_visibility = if briefing.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let reopen_visibility = if briefing.visible {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    for mut visibility in &mut panels {
        *visibility = panel_visibility;
    }
    for mut visibility in &mut reopen_buttons {
        *visibility = reopen_visibility;
    }

    if let Ok(mut text) = briefing_text.single_mut() {
        **text = match_briefing_text(
            &setup_settings,
            visible_player.team,
            &relations,
            &active_teams,
        );
    }
}

fn match_briefing_text(
    settings: &MatchSetupSettings,
    visible_team: Team,
    relations: &TeamRelations,
    active_teams: &ActiveTeams,
) -> String {
    let (enemies, allies) = match_briefing_player_counts(visible_team, relations, active_teams);
    format!(
        "目标：摧毁所有敌方指挥中心，并保住至少一个我方指挥中心\n\
         敌人: {enemies}\n\
         盟友: {allies}\n\
         Ore: {} / Crystal: {}\n\
         推荐开局\n\
         - 派工人采集附近水晶，并尽快补充矿车\n\
         - 在雷达、防御和高级生产耗电前先补电力\n\
         - 用兵营做廉价克制，或用战车工厂施加装甲压力\n\
         - 侦察敌方科技、占领中立建筑，并在后期武器到来前打击扩张",
        settings.starting_resources.ore, settings.starting_resources.crystal
    )
}

fn match_briefing_player_counts(
    visible_team: Team,
    relations: &TeamRelations,
    active_teams: &ActiveTeams,
) -> (u32, u32) {
    let mut enemies = 0u32;
    let mut allies = 0u32;
    for team in player_teams(active_teams.0.len()) {
        let Some(index) = team.economy_index() else {
            continue;
        };
        if !active_teams.0.get(index).copied().unwrap_or(false) || team == visible_team {
            continue;
        }
        if relations.are_enemies(visible_team, team) {
            enemies += 1;
        } else {
            allies += 1;
        }
    }
    (enemies, allies)
}

fn setup_support_cooldowns(mut support_cooldowns: ResMut<SupportCooldowns>) {
    *support_cooldowns = SupportCooldowns::default();
}

fn update_support_cooldowns(
    time: Res<Time>,
    economies: Res<Economies>,
    active_teams: Option<Res<ActiveTeams>>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut support_cooldowns: ResMut<SupportCooldowns>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    structures: Query<StructurePrereqItem<'_>>,
) {
    let delta = time.delta_secs();
    let player_team = visible_player_team(visible_player.as_deref());
    let team_count = active_teams
        .as_deref()
        .map(|active| active.0.len())
        .unwrap_or(economies.players.len());
    for team in player_teams(team_count) {
        if !team_is_active(team, active_teams.as_deref()) {
            continue;
        }
        let Some(team_index) = support_cooldowns.ensure_team(team) else {
            continue;
        };
        for power in SupportPowerKind::ALL {
            let idx = power.idx();
            let def = power.definition();
            let requirements_met = support_requirements_met(team, def.requirements, &structures);
            if !requirements_met {
                support_cooldowns.initial_charge_started[team_index][idx] = false;
            } else if def.initial_cooldown > 0.0
                && !support_cooldowns.initial_charge_started[team_index][idx]
                && support_cooldowns.remaining[team_index][idx] <= 0.0
            {
                support_cooldowns.initial_charge_started[team_index][idx] = true;
                support_cooldowns.remaining[team_index][idx] = def.initial_cooldown;
                record_support_power_charging_feedback(
                    &mut audio_feedback,
                    &mut battle_log,
                    team,
                    player_team,
                    power,
                    def.initial_cooldown,
                );
                continue;
            }
            let before = support_cooldowns.remaining[team_index][idx];
            support_cooldowns.remaining[team_index][idx] = (before - delta).max(0.0);
            let became_ready = before > 0.0 && support_cooldowns.remaining[team_index][idx] == 0.0;
            if became_ready
                && support_power_available_for_audio(team, power, &economies, &structures)
            {
                record_support_power_ready_audio_feedback(
                    &mut audio_feedback,
                    team,
                    player_team,
                    power,
                );
                record_support_power_ready_battle_log(&mut battle_log, team, player_team, power);
            }
        }
    }
}

fn update_support_effects(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    player_factions: Option<Res<PlayerFactions>>,
    map_bounds: Res<MapBounds>,
    mut warnings: Query<(Entity, &mut SupportWarning)>,
    mut reveals: Query<(Entity, &mut TemporarySupportReveal)>,
    mut chrono_relays: Query<(Entity, &mut ChronoRelay)>,
    mut support_params: ParamSet<(
        Query<(Entity, &mut EmpDisabled)>,
        Query<(Entity, &mut SupportShield)>,
        Query<(
            Entity,
            &Transform,
            &Team,
            &Selectable,
            &Health,
            &mut MobileShieldProjector,
            Option<&EmpDisabled>,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&Unit>,
            Option<&Structure>,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            Option<&Unit>,
            Option<&Structure>,
            &MovementDomain,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
            &mut Health,
            &Selectable,
            Option<&FogMemoryVisible>,
        )>,
    )>,
    mut pending_strikes: Query<(Entity, &mut PendingOrbitalStrike, &Transform)>,
    mut pending_paradrops: Query<(Entity, &mut PendingParadrop)>,
    mut next_spawn_id: ResMut<NextSpawnId>,
    mut match_state: ResMut<MatchState>,
    mut battle_log: ResMut<BattleLog>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, mut warning) in &mut warnings {
        warning.remaining -= time.delta_secs();
        if warning.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        warning.radius = (warning.radius + time.delta_secs() * 0.16).min(10.0);
        if warning.remaining <= 0.2 {
            warning.radius = (warning.radius * 0.84).max(0.15);
        }
    }

    for (entity, mut reveal) in &mut reveals {
        reveal.remaining -= time.delta_secs();
        if reveal.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
    }

    {
        let mut emp_disables = support_params.p0();
        for (entity, mut disabled) in &mut emp_disables {
            disabled.remaining -= time.delta_secs();
            if disabled.remaining <= 0.0 {
                commands.entity(entity).try_remove::<EmpDisabled>();
                continue;
            }
        }
    }

    for (entity, mut chrono) in &mut chrono_relays {
        chrono.remaining -= time.delta_secs();
        if chrono.remaining <= 0.0 {
            commands.entity(entity).try_remove::<ChronoRelay>();
            continue;
        }
    }

    {
        let mut support_shields = support_params.p1();
        for (entity, mut shield) in &mut support_shields {
            shield.remaining -= time.delta_secs();
            if shield.remaining <= 0.0 {
                commands.entity(entity).try_remove::<SupportShield>();
                continue;
            }
        }
    }

    let projector_refreshes: Vec<(Team, Vec3, f32, f32, f32)> = {
        let mut mobile_shield_projectors = support_params.p2();
        mobile_shield_projectors
            .iter_mut()
            .filter_map(
                |(
                    _projector_entity,
                    projector_transform,
                    owner,
                    _selectable,
                    projector_health,
                    mut projector,
                    emp,
                )| {
                    if projector_health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0)
                    {
                        return None;
                    }
                    projector.refresh_remaining -= time.delta_secs();
                    if projector.refresh_remaining > 0.0 {
                        return None;
                    }
                    projector.refresh_remaining = 0.2;
                    Some((
                        *owner,
                        projector_transform.translation,
                        projector.radius,
                        projector.duration,
                        projector.damage_scale,
                    ))
                },
            )
            .collect()
    };
    if !projector_refreshes.is_empty() {
        let shield_targets = support_params.p3();
        for (owner, projector_position, projector_radius, duration, damage_scale) in
            projector_refreshes
        {
            for (
                target_entity,
                target_team,
                target_transform,
                target_selectable,
                target_health,
                target_unit,
                target_structure,
            ) in &shield_targets
            {
                if target_unit.is_none() && target_structure.is_none() {
                    continue;
                }
                if !relations.are_allied(owner, *target_team) || target_health.current <= 0.0 {
                    continue;
                }
                if xz_distance(target_transform.translation, projector_position)
                    > projector_radius + target_selectable.radius
                {
                    continue;
                }
                queue_apply_support_shield(&mut commands, target_entity, duration, damage_scale);
            }
        }
    }

    let occupiable_spawn_points: Vec<(Vec3, f32)> = {
        let health_q = support_params.p4();
        health_q
            .iter()
            .filter_map(|(_, _, transform, _, _, _, _, _, health, selectable, _)| {
                (health.current > 0.0).then_some((transform.translation, selectable.radius))
            })
            .collect()
    };

    let mut impacts: Vec<(Vec3, f32, f32, Team)> = Vec::new();
    let mut strike_entities: Vec<Entity> = Vec::new();
    for (entity, mut strike, transform) in &mut pending_strikes {
        strike.remaining -= time.delta_secs();
        if strike.remaining > 0.0 {
            continue;
        }
        impacts.push((
            transform.translation,
            strike.radius,
            strike.damage,
            strike.team,
        ));
        strike_entities.push(entity);
        let pulse_height = 0.85 + strike.impact_scale * 0.28;
        let pulse_ttl = 0.12 + strike.impact_scale * 0.06;
        commands.spawn((
            ShotPulse {
                from: transform.translation + Vec3::new(0.0, pulse_height, 0.0),
                to: transform.translation + Vec3::new(0.0, 0.2, 0.0),
                ttl: pulse_ttl,
                team: strike.team,
            },
            MatchScopedEntity,
        ));
    }

    for entity in strike_entities {
        commands.entity(entity).try_despawn();
    }

    let mut paradrops: Vec<(Vec3, Team, SkirmishFaction, &'static [&'static str])> = Vec::new();
    let mut paradrop_entities: Vec<Entity> = Vec::new();
    for (entity, mut paradrop) in &mut pending_paradrops {
        paradrop.remaining -= time.delta_secs();
        if paradrop.remaining > 0.0 {
            continue;
        }
        paradrops.push((
            paradrop.target,
            paradrop.team,
            slot_faction_from_option(player_factions.as_deref(), paradrop.team),
            paradrop.unit_paths,
        ));
        paradrop_entities.push(entity);
    }
    for entity in paradrop_entities {
        commands.entity(entity).try_despawn();
    }

    for (impact_pos, impact_radius, impact_damage, team) in impacts {
        let mut health_q = support_params.p4();
        for (
            target_entity,
            target_team,
            target_transform,
            _unit,
            structure,
            _domain,
            shield,
            passive_shield,
            mut target_health,
            selectable,
            fog_memory,
        ) in &mut health_q
        {
            if target_health.current <= 0.0 {
                continue;
            }
            if !relations.are_enemies(team, *target_team) {
                continue;
            }
            if xz_distance(target_transform.translation, impact_pos) > impact_radius {
                continue;
            }
            let damage = impact_damage * support_damage_scale(shield, passive_shield);
            target_health.current -= damage;
            if relations.are_allied(*target_team, player_team) && damage > 0.0 {
                push_under_attack_log(
                    &mut battle_log,
                    target_transform.translation,
                    structure.is_some(),
                );
            }
            if target_health.current <= 0.0 {
                if relations.are_allied(*target_team, player_team) {
                    if structure.is_some() {
                        match_state.structures_lost += 1;
                    } else {
                        match_state.units_lost += 1;
                    }
                } else if structure.is_some() {
                    match_state.enemy_structures_destroyed += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                spawn_destruction_effects(
                    &mut commands,
                    &asset_server,
                    target_transform.translation,
                    selectable.radius,
                    structure.is_some(),
                    *target_team,
                    structure.is_some() && fog_memory.is_some(),
                );
                commands.entity(target_entity).try_despawn();
            }
        }
    }

    for (target, team, faction, unit_paths) in paradrops {
        spawn_paradrop_units(
            &mut commands,
            &asset_server,
            &mut next_spawn_id,
            target,
            team,
            faction,
            player_team,
            unit_paths,
            &occupiable_spawn_points,
            *map_bounds,
        );
        commands.spawn((
            ShotPulse {
                from: target + Vec3::new(0.0, 1.2, 0.0),
                to: target + Vec3::new(0.0, 0.2, 0.0),
                ttl: 0.18,
                team,
            },
            MatchScopedEntity,
        ));
    }
}

fn update_repair_and_healing_auras(
    time: Res<Time>,
    economies: Res<Economies>,
    relations: Res<TeamRelations>,
    player_factions: Res<PlayerFactions>,
    support_aura_sources: Query<
        (
            &Team,
            &Transform,
            Option<&RepairAura>,
            Option<&HealingAura>,
            Option<&Structure>,
            Option<&UnderConstruction>,
        ),
        Or<(With<RepairAura>, With<HealingAura>)>,
    >,
    mut health_q: Query<(
        Entity,
        &Team,
        &Transform,
        Option<&Unit>,
        Option<&Structure>,
        &MovementDomain,
        &mut Health,
        &Selectable,
    )>,
) {
    let mut repair_sources: Vec<(Team, Vec3, f32, f32, RepairAuraMode)> = Vec::new();
    let mut healing_sources: Vec<(Team, Vec3, f32, f32)> = Vec::new();
    for (team, transform, repair_aura, healing_aura, structure, under_construction) in
        &support_aura_sources
    {
        if !structure_is_constructed(under_construction) {
            continue;
        }
        if let Some(aura) = repair_aura {
            if powered_repair_offline(team, structure, &economies) {
                continue;
            }
            let support_rate =
                aura.rate * faction_support_rate_multiplier(player_factions.faction(*team));
            repair_sources.push((
                *team,
                transform.translation,
                aura.radius,
                support_rate,
                aura.mode,
            ));
        }
        if let Some(aura) = healing_aura {
            let support_rate =
                aura.rate * faction_support_rate_multiplier(player_factions.faction(*team));
            healing_sources.push((*team, transform.translation, aura.radius, support_rate));
        }
    }

    if !repair_sources.is_empty() {
        let repair_targets = health_q
            .iter_mut()
            .filter_map(
                |(
                    entity,
                    target_team,
                    target_transform,
                    target_unit,
                    target_structure,
                    target_domain,
                    target_health,
                    target_selectable,
                )| {
                    (target_health.current > 0.0
                        && target_health.current < target_health.max
                        && can_receive_repair_aura(target_unit, target_structure, target_domain))
                    .then_some((
                        entity,
                        *target_team,
                        target_transform.translation,
                        target_selectable.radius,
                    ))
                },
            )
            .collect::<Vec<_>>();
        let mut repair_events: Vec<(Entity, f32)> = Vec::new();
        for (source_team, source_position, source_radius, source_rate, mode) in &repair_sources {
            match mode {
                RepairAuraMode::AllEligible => {
                    for (target_entity, target_team, target_position, target_radius) in
                        &repair_targets
                    {
                        if !relations.are_allied(*source_team, *target_team) {
                            continue;
                        }
                        if xz_distance(*source_position, *target_position)
                            > *source_radius + *target_radius
                        {
                            continue;
                        }
                        repair_events.push((*target_entity, *source_rate * time.delta_secs()));
                    }
                }
                RepairAuraMode::NearestEligible => {
                    let mut best = None;
                    let mut best_distance = f32::MAX;
                    for (target_entity, target_team, target_position, target_radius) in
                        &repair_targets
                    {
                        if !relations.are_allied(*source_team, *target_team) {
                            continue;
                        }
                        let distance = xz_distance(*source_position, *target_position);
                        if distance <= *source_radius + *target_radius && distance < best_distance {
                            best = Some(*target_entity);
                            best_distance = distance;
                        }
                    }
                    if let Some(target_entity) = best {
                        repair_events.push((target_entity, *source_rate * time.delta_secs()));
                    }
                }
            }
        }
        for (
            entity,
            _target_team,
            _target_transform,
            _target_unit,
            _target_structure,
            _target_domain,
            mut target_health,
            _target_selectable,
        ) in &mut health_q
        {
            let repaired = repair_events
                .iter()
                .filter_map(|(target, amount)| (*target == entity).then_some(*amount))
                .sum::<f32>();
            if repaired > 0.0 {
                target_health.current = (target_health.current + repaired).min(target_health.max);
            }
        }
    }

    if !healing_sources.is_empty() {
        for (
            _target_entity,
            target_team,
            target_transform,
            target_unit,
            _target_structure,
            _target_domain,
            mut target_health,
            target_selectable,
        ) in &mut health_q
        {
            if target_health.current <= 0.0 || target_health.current >= target_health.max {
                continue;
            }
            if !can_receive_healing_aura(target_unit) {
                continue;
            }
            let mut healed = 0.0;
            for (source_team, source_position, source_radius, source_rate) in &healing_sources {
                if !relations.are_allied(*source_team, *target_team) {
                    continue;
                }
                if xz_distance(*source_position, target_transform.translation)
                    > *source_radius + target_selectable.radius
                {
                    continue;
                }
                healed += *source_rate * time.delta_secs();
            }
            if healed > 0.0 {
                target_health.current = (target_health.current + healed).min(target_health.max);
            }
        }
    }
}

fn spawn_paradrop_units(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_spawn_id: &mut NextSpawnId,
    target: Vec3,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    unit_paths: &'static [&'static str],
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) {
    let count = unit_paths.len();
    for (i, unit_path) in unit_paths.iter().enumerate() {
        let Some(def) = registry::entity(unit_path) else {
            continue;
        };
        let offset = formation_offset(i, count);
        let spawn_position = find_paradrop_spawn_position(
            target,
            offset,
            def.radius,
            occupiable_spawn_points,
            bounds,
        );
        spawn_unit_for_faction(
            commands,
            asset_server,
            next_spawn_id,
            unit_path,
            team,
            spawn_position,
            0,
            faction,
            visible_team,
        );
    }
}

fn find_paradrop_spawn_position(
    target: Vec3,
    preferred_offset: Vec3,
    unit_radius: f32,
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) -> Vec3 {
    let preferred = (target + preferred_offset).with_y(0.0);
    if is_spawn_position_free(preferred, unit_radius, occupiable_spawn_points, bounds) {
        return preferred;
    }

    let preferred_direction = {
        let dir = preferred_offset.xz();
        if dir.length_squared() < 1e-4 {
            Vec2::new(0.0, 1.0)
        } else {
            dir.normalize()
        }
    };

    let ring_step = 0.5;
    let max_rings = 18;
    for ring in 1..=max_rings {
        let search_radius = ring as f32 * ring_step;
        let samples = 12 + ring * 2;
        let angular_offset = preferred_direction.angle_to(Vec2::Y);
        for sample in 0..samples {
            let angle = angular_offset + sample as f32 * (std::f32::consts::TAU / samples as f32);
            let candidate = Vec3::new(
                target.x + preferred_offset.x + angle.cos() * search_radius,
                0.0,
                target.z + preferred_offset.z + angle.sin() * search_radius,
            );
            let clamped = bounds.clamp_ground_point(candidate, unit_radius);
            if is_spawn_position_free(clamped, unit_radius, occupiable_spawn_points, bounds) {
                return clamped;
            }
        }
    }

    bounds.clamp_ground_point(preferred, unit_radius)
}

fn is_spawn_position_free(
    candidate: Vec3,
    unit_radius: f32,
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) -> bool {
    let inner_bounds = MapBounds {
        half_width: (bounds.half_width - unit_radius).max(0.0),
        half_depth: (bounds.half_depth - unit_radius).max(0.0),
    };
    if !inner_bounds.contains_ground_point(candidate) {
        return false;
    }

    for (position, radius) in occupiable_spawn_points {
        if xz_distance(candidate, *position) <= unit_radius + *radius + 0.05 {
            return false;
        }
    }
    true
}

fn setup_ui(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load("fonts/wqy-microhei-ui.ttf");

    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(14),
            ..default()
        },
        StatsText,
        MatchScopedEntity,
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.84, 0.9, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: px(42),
            left: px(14),
            ..default()
        },
        SelectionText,
        MatchScopedEntity,
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.95, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(18),
            max_width: px(390),
            ..default()
        },
        ObjectiveTrackerText,
        MatchScopedEntity,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(BATTLE_LOG_TOP_PX),
            right: px(BATTLE_LOG_RIGHT_PX),
            width: px(BATTLE_LOG_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            align_items: AlignItems::FlexEnd,
            ..default()
        },
        BattleLogRoot { font: font.clone() },
        MatchScopedEntity,
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.9, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: px(14),
            bottom: px(238),
            max_width: px(560),
            ..default()
        },
        ProductionQueueText,
        MatchScopedEntity,
    ));

    let queue_font = font.clone();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(14),
                bottom: px(142),
                width: px(790),
                height: px(88),
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            for index in 0..PRODUCTION_QUEUE_HUD_SLOT_COUNT {
                parent
                    .spawn(production_queue_slot(index))
                    .with_children(|slot| {
                        slot.spawn(production_queue_slot_label(index, queue_font.clone()));
                    });
            }
        });

    setup_minimap(commands, font.clone());
    setup_selection_drag_box(commands);
    setup_match_end_overlay(commands, font.clone());
    setup_match_menu_overlay(commands, font.clone());
    setup_match_briefing(commands, font.clone());

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(14),
                right: px(14),
                bottom: px(14),
                height: px(118),
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(8),
                row_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            for index in 0..COMMAND_SLOT_COUNT {
                parent.spawn(command_button(index)).with_children(|button| {
                    button.spawn(command_button_label(index, font.clone()));
                });
            }
        });
}

fn setup_selection_drag_box(commands: &mut Commands) {
    commands.spawn((
        SelectionDragBox,
        Visibility::Hidden,
        GlobalZIndex(30),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: px(0),
            height: px(0),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.62, 0.86, 1.0, 0.96)),
        BackgroundColor(Color::srgba(0.18, 0.46, 0.72, 0.16)),
        MatchScopedEntity,
    ));
}

fn setup_minimap(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                right: px(MINIMAP_RIGHT_PX),
                bottom: px(MINIMAP_BOTTOM_PX),
                width: px(MINIMAP_SIZE_PX),
                height: px(MINIMAP_SIZE_PX),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.24, 0.31, 0.36)),
            BackgroundColor(Color::srgba(0.025, 0.035, 0.04, 0.88)),
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                MinimapContent,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: px(MINIMAP_SIZE_PX),
                    height: px(MINIMAP_SIZE_PX),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.055, 0.058, 0.78)),
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.86, 0.88)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(10),
                    right: px(10),
                    top: px(56),
                    ..default()
                },
                MinimapStatusText,
            ));
        });
}

fn update_match_clock(mut match_state: ResMut<MatchState>, time: Res<Time>) {
    if match_state.is_running() {
        match_state.start_time_sec += time.delta_secs();
    }
}

fn match_end_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    match_state: Res<MatchState>,
    mut match_menu: ResMut<MatchMenuState>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(
        &Interaction,
        &MatchEndButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let match_finished = !match_state.is_running();
    for (interaction, button, mut background, mut border) in &mut buttons {
        let clicked = match_finished
            && *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match_menu.visible = false;
            match button.action {
                MatchEndAction::Restart => {
                    next_state.set(AppScreen::RestartingMatch);
                }
                MatchEndAction::ReturnToSetup | MatchEndAction::ExitToMenu => {
                    next_state.set(AppScreen::MainMenu);
                }
            }
        }

        let (bg, border_color) = match_end_button_visual(*interaction, match_finished);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

fn match_end_button_visual(interaction: Interaction, enabled: bool) -> (Color, Color) {
    if !enabled {
        return (
            Color::srgba(0.035, 0.045, 0.055, 0.54),
            Color::srgb(0.18, 0.22, 0.26),
        );
    }
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.16, 0.28, 0.36, 0.98),
            Color::srgb(0.66, 0.86, 0.96),
        ),
        Interaction::Hovered => (
            Color::srgba(0.1, 0.18, 0.24, 0.96),
            Color::srgb(0.46, 0.68, 0.78),
        ),
        Interaction::None => (
            Color::srgba(0.055, 0.072, 0.088, 0.94),
            Color::srgb(0.28, 0.36, 0.42),
        ),
    }
}

fn update_match_end_overlay(
    match_state: Res<MatchState>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    mut overlay_q: Query<(&mut Visibility, &Children), With<MatchEndOverlay>>,
    mut title_text_q: Query<
        &mut Text,
        (
            With<MatchEndTitle>,
            Without<MatchEndReason>,
            Without<MatchEndStats>,
        ),
    >,
    mut reason_text_q: Query<
        &mut Text,
        (
            With<MatchEndReason>,
            Without<MatchEndTitle>,
            Without<MatchEndStats>,
        ),
    >,
    mut stats_text_q: Query<
        &mut Text,
        (
            With<MatchEndStats>,
            Without<MatchEndTitle>,
            Without<MatchEndReason>,
        ),
    >,
) {
    if overlay_q.is_empty() {
        return;
    }

    let Ok((mut overlay_visibility, _children)) = overlay_q.single_mut() else {
        return;
    };
    if match_state.is_running() {
        *overlay_visibility = Visibility::Hidden;
        return;
    }
    *overlay_visibility = Visibility::Visible;

    if let Ok(mut title_text) = title_text_q.single_mut() {
        **title_text = "对局结算".to_string();
    }
    if let Ok(mut reason_text) = reason_text_q.single_mut() {
        **reason_text = match_state.result_reason.to_string();
    }
    if let Ok(mut stats_text) = stats_text_q.single_mut() {
        let minutes = (match_state.start_time_sec.max(0.0) / 60.0).floor() as u32;
        let seconds = (match_state.start_time_sec.max(0.0) as u32) % 60;
        let visible_economy = economies.get(visible_player.team);
        let human_losses = format!(
            "己方损失: 单位 {}  建筑 {}",
            match_state.units_lost, match_state.structures_lost
        );
        let enemy_losses = format!(
            "敌方击杀: 单位 {}  建筑 {}",
            match_state.enemy_units_destroyed, match_state.enemy_structures_destroyed
        );
        let resources = format!(
            "{}资源: Ore {}  Crystal {}",
            visible_player.team.label(),
            visible_economy.ore,
            visible_economy.crystal
        );
        **stats_text = format!(
            "剩余阵营: {}  剩余锚点: {}  用时: {:02}:{:02}\n{enemy_losses}\n{human_losses}\n{resources}",
            match_state.remaining_teams, match_state.remaining_anchors, minutes, seconds
        );
    }
}

fn evaluate_match_end(
    mut match_state: ResMut<MatchState>,
    mut match_flow: ResMut<MatchFlow>,
    mut audio_feedback: ResMut<AudioFeedback>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    structures: Query<(&Structure, &Team, &Health)>,
    units: Query<(&Unit, &Team, &Health)>,
) {
    let controlled_team = controlled_player_team(visible_player.as_deref());
    let mut active_anchor_team = Vec::new();
    let mut active_anchor_count = 0u32;
    for (structure, team, health) in &structures {
        if is_structure_elimination_anchor(structure) && health.current > 0.0 {
            record_active_elimination_anchor(
                *team,
                &mut active_anchor_team,
                &mut active_anchor_count,
            );
        }
    }

    for (unit, team, health) in &units {
        if is_worker_elimination_anchor(unit) && health.current > 0.0 {
            record_active_elimination_anchor(
                *team,
                &mut active_anchor_team,
                &mut active_anchor_count,
            );
        }
    }

    let active_teams = active_anchor_team.iter().filter(|active| **active).count() as u32;
    match_state.remaining_teams = active_teams;
    match_state.remaining_anchors = active_anchor_count;

    let Some(player_team) = controlled_team else {
        if active_teams <= 1 {
            finalize_match(
                &mut match_state,
                &mut match_flow,
                MatchPhase::MatchFinished,
                "战斗结束",
            );
        }
        return;
    };

    let player_side_active = active_anchor_team.iter().enumerate().any(|(idx, active)| {
        *active
            && Team::from_playable_index(idx)
                .is_some_and(|team| relations.are_allied(player_team, team))
    });
    let has_enemy = active_anchor_team.iter().enumerate().any(|(idx, active)| {
        *active
            && Team::from_playable_index(idx)
                .is_some_and(|team| relations.are_enemies(player_team, team))
    });

    if !player_side_active {
        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Defeat);
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::HumanDefeat,
            "失利：己方锚点被全部摧毁",
        );
        return;
    }

    if !has_enemy {
        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Victory);
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::HumanVictory,
            "胜利：敌方锚点全部丢失",
        );
        return;
    }

    if active_teams <= 1 {
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::MatchFinished,
            "战斗结束",
        );
    }
}

fn is_worker_elimination_anchor(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| {
        def.is_worker || def.is_harvester || matches!(def.id, "MobileConstructionVehicle")
    })
}

fn is_structure_elimination_anchor(structure: &Structure) -> bool {
    structure.id == "CommandCenter"
}

fn record_active_elimination_anchor(
    team: Team,
    active_anchor_team: &mut Vec<bool>,
    active_anchor_count: &mut u32,
) {
    let Some(idx) = team.economy_index() else {
        return;
    };
    if active_anchor_team.len() <= idx {
        active_anchor_team.resize(idx + 1, false);
    }
    active_anchor_team[idx] = true;
    *active_anchor_count += 1;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ObjectiveTrackerSnapshot {
    enemy_teams: u32,
    remaining_anchors: u32,
    total_anchors: u32,
    structures: u32,
    workers: u32,
    completion_percent: u32,
}

fn objective_tracker_snapshot(
    visible_team: Team,
    relations: &TeamRelations,
    structures: &Query<'_, '_, (&Structure, &Team, &Health)>,
    units: &Query<'_, '_, (&Unit, &Team, &Health)>,
    state: &mut ObjectiveTrackerState,
) -> ObjectiveTrackerSnapshot {
    let mut active_enemy_teams = Vec::new();
    let mut structure_count = 0u32;
    let mut worker_count = 0u32;

    for (structure, team, health) in structures {
        if health.current <= 0.0
            || !is_structure_elimination_anchor(structure)
            || !relations.are_enemies(visible_team, *team)
        {
            continue;
        }
        structure_count += 1;
        if let Some(idx) = team.economy_index() {
            if active_enemy_teams.len() <= idx {
                active_enemy_teams.resize(idx + 1, false);
            }
            active_enemy_teams[idx] = true;
        }
    }

    for (unit, team, health) in units {
        if health.current <= 0.0
            || !is_worker_elimination_anchor(unit)
            || !relations.are_enemies(visible_team, *team)
        {
            continue;
        }
        worker_count += 1;
        if let Some(idx) = team.economy_index() {
            if active_enemy_teams.len() <= idx {
                active_enemy_teams.resize(idx + 1, false);
            }
            active_enemy_teams[idx] = true;
        }
    }

    let remaining_anchors = structure_count + worker_count;
    state.max_enemy_anchors_seen = state.max_enemy_anchors_seen.max(remaining_anchors);
    let total_anchors = state.max_enemy_anchors_seen;
    let objective_complete = total_anchors > 0 && remaining_anchors == 0;

    ObjectiveTrackerSnapshot {
        enemy_teams: active_enemy_teams.iter().filter(|active| **active).count() as u32,
        remaining_anchors,
        total_anchors,
        structures: structure_count,
        workers: worker_count,
        completion_percent: objective_completion_percent(
            remaining_anchors,
            total_anchors,
            objective_complete,
        ),
    }
}

fn objective_completion_percent(remaining_anchors: u32, total_anchors: u32, complete: bool) -> u32 {
    if complete {
        return 100;
    }
    if total_anchors == 0 {
        return 0;
    }
    let destroyed = total_anchors.saturating_sub(remaining_anchors);
    ((destroyed as f32 / total_anchors as f32) * 100.0).round() as u32
}

fn objective_tracker_text(snapshot: ObjectiveTrackerSnapshot) -> String {
    let title = if snapshot.total_anchors > 0 && snapshot.remaining_anchors == 0 {
        "目标完成"
    } else {
        "目标: 消灭敌方锚点"
    };
    format!(
        "{title}\n敌方队伍: {}  锚点: {}/{}\n结构: {}  工人: {}  进度: {}%",
        snapshot.enemy_teams,
        snapshot.remaining_anchors,
        snapshot.total_anchors,
        snapshot.structures,
        snapshot.workers,
        snapshot.completion_percent,
    )
}

fn update_visibility(
    mut commands: Commands,
    visible_player: Res<VisiblePlayer>,
    mut visibility_params: ParamSet<(
        Query<(&Team, &Transform, &VisionRadius, &VisibilityState)>,
        Query<(
            Entity,
            &Team,
            &Transform,
            Option<&Structure>,
            Option<&FogMemoryVisible>,
            &mut VisibilityState,
            &mut Visibility,
        )>,
        Query<(Entity, &Transform, &FogMemoryStructureRemnant)>,
    )>,
) {
    let visible_team = visible_player.team;
    let all_players_visible = visible_player.all_players_visible();
    let mut revealers = Vec::new();
    if !all_players_visible {
        {
            let visible_revealers = visibility_params.p0();
            for (team, transform, vision_radius, visibility_state) in &visible_revealers {
                if *team == visible_team && visibility_state.visible {
                    revealers.push((transform.translation, vision_radius.0));
                }
            }
        }
    }

    {
        let mut tracked_entities = visibility_params.p1();
        for (
            entity,
            team,
            transform,
            structure,
            fog_memory,
            mut visibility_state,
            mut visibility,
        ) in &mut tracked_entities
        {
            let should_be_visible = if all_players_visible || *team == visible_team {
                true
            } else {
                revealers.iter().any(|(source, source_radius)| {
                    xz_distance(transform.translation, *source) <= *source_radius + FOG_COMPENSATION
                })
            };
            let was_visible = visibility_state.visible;
            let is_known_structure = structure.is_some() && (fog_memory.is_some() || was_visible);

            visibility_state.visible = should_be_visible;
            *visibility = if should_be_visible {
                if fog_memory.is_some() {
                    commands.entity(entity).try_remove::<FogMemoryVisible>();
                }
                Visibility::Visible
            } else {
                commands.entity(entity).try_remove::<Selected>();
                if structure.is_some() && was_visible {
                    commands.entity(entity).try_insert(FogMemoryVisible);
                }
                if is_known_structure {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            };
        }
    }

    let remnant_entities = visibility_params.p2();
    for (entity, transform, _remnant) in &remnant_entities {
        let is_rescouted = all_players_visible
            || revealers.iter().any(|(source, source_radius)| {
                xz_distance(transform.translation, *source) <= *source_radius + FOG_COMPENSATION
            });
        if is_rescouted {
            commands.entity(entity).try_despawn();
        }
    }
}

fn update_click_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: Query<(Entity, &mut ClickMarker)>,
) {
    for (entity, mut marker) in &mut markers {
        marker.ttl -= time.delta_secs();
        if marker.ttl <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        let ratio = (marker.ttl / CLICK_MARKER_TTL_SECONDS).clamp(0.0, 1.0);
        marker.radius =
            CLICK_MARKER_RADIUS_END + (CLICK_MARKER_RADIUS_START - CLICK_MARKER_RADIUS_END) * ratio;
    }
}

fn update_combat_wreckage(
    mut commands: Commands,
    time: Res<Time>,
    mut wreckage: Query<(Entity, &mut CombatWreckage)>,
) {
    for (entity, mut wreckage) in &mut wreckage {
        if combat_wreckage_expired(&mut wreckage, time.delta_secs()) {
            commands.entity(entity).try_despawn();
        }
    }
}

fn update_structure_destruction_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut StructureDestructionVfx)>,
) {
    for (entity, mut effect) in &mut effects {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

fn update_veterancy_promotion_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut VeterancyPromotionEffect)>,
) {
    for (entity, mut effect) in &mut effects {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

fn cleanup_dead_entities(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut match_state: ResMut<MatchState>,
    dead_entities: Query<(
        Entity,
        &Transform,
        &Team,
        &Selectable,
        &Health,
        Option<&Structure>,
        Option<&Unit>,
        Option<&FogMemoryVisible>,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, transform, team, selectable, health, structure, unit, fog_memory) in &dead_entities
    {
        if health.current > 0.0 || (structure.is_none() && unit.is_none()) {
            continue;
        }
        let is_structure = structure.is_some();
        let remembered_fog_structure = is_structure && fog_memory.is_some();
        if relations.are_allied(*team, player_team) {
            if is_structure {
                match_state.structures_lost += 1;
            } else {
                match_state.units_lost += 1;
            }
        } else if is_structure {
            match_state.enemy_structures_destroyed += 1;
        } else {
            match_state.enemy_units_destroyed += 1;
        }
        if let Some(asset_server) = asset_server.as_deref() {
            spawn_destruction_effects(
                &mut commands,
                asset_server,
                transform.translation,
                selectable.radius,
                is_structure,
                *team,
                remembered_fog_structure,
            );
        } else if remembered_fog_structure {
            spawn_fog_memory_structure_remnant(
                &mut commands,
                None,
                transform.translation,
                selectable.radius,
            );
        } else if is_structure {
            spawn_structure_destruction_vfx(
                &mut commands,
                transform.translation,
                selectable.radius,
                *team,
            );
        }
        commands.entity(entity).try_despawn();
    }
}

fn update_battle_log(
    mut commands: Commands,
    time: Res<Time>,
    mut battle_log: ResMut<BattleLog>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    root_q: Query<(Entity, &BattleLogRoot, Option<&Children>)>,
) {
    let delta = time.delta_secs();
    battle_log.under_attack_cooldown = (battle_log.under_attack_cooldown - delta).max(0.0);
    for entry in &mut battle_log.entries {
        entry.remaining -= delta;
        if entry.minimap_ping_active {
            entry.minimap_ping_remaining = (entry.minimap_ping_remaining - delta).max(0.0);
            entry.minimap_ping_active = entry.minimap_ping_remaining > 0.0;
        }
    }
    battle_log.entries.retain(|entry| entry.remaining > 0.0);
    if let Some(focus) = battle_log
        .entries
        .iter()
        .rev()
        .find_map(|entry| entry.focus)
    {
        latest_battle_event.focus = Some(focus);
    }

    if let Ok((root, root_data, children)) = root_q.single() {
        if let Some(children) = children {
            for child in children {
                commands.entity(*child).try_despawn();
            }
        }
        let visible_entries = battle_log
            .entries
            .iter()
            .enumerate()
            .rev()
            .collect::<Vec<_>>();
        let Ok(mut root_commands) = commands.get_entity(root) else {
            return;
        };
        root_commands.with_children(|parent| {
            for (index, entry) in visible_entries {
                spawn_battle_log_entry(parent, root_data.font.clone(), index, entry);
            }
        });
    }
}

fn spawn_battle_log_entry(
    parent: &mut ChildSpawnerCommands<'_>,
    font: Handle<Font>,
    index: usize,
    entry: &BattleLogEntry,
) {
    let text = battle_log_entry_text(font, entry);
    if entry.focus.is_some() {
        parent
            .spawn((
                Button,
                BattleLogEntryButton(index),
                Node {
                    max_width: px(BATTLE_LOG_WIDTH_PX),
                    padding: UiRect::axes(px(6), px(2)),
                    border: UiRect::all(px(1)),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                BorderColor::all(Color::srgba(0.82, 0.9, 0.72, 0.18)),
                BackgroundColor(battle_log_entry_button_color(Interaction::None)),
            ))
            .with_children(|button| {
                button.spawn(text);
            });
    } else {
        parent
            .spawn(Node {
                max_width: px(BATTLE_LOG_WIDTH_PX),
                padding: UiRect::axes(px(6), px(2)),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            })
            .with_children(|node| {
                node.spawn(text);
            });
    }
}

fn battle_log_entry_text(font: Handle<Font>, entry: &BattleLogEntry) -> impl Bundle {
    (
        Text::new(format!("> {}", entry.message)),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(if entry.focus.is_some() {
            Color::srgb(1.0, 0.92, 0.6)
        } else {
            Color::srgb(0.78, 0.84, 0.78)
        }),
    )
}

fn battle_log_entry_button_color(interaction: Interaction) -> Color {
    match interaction {
        Interaction::Pressed => Color::srgba(0.18, 0.24, 0.16, 0.62),
        Interaction::Hovered => Color::srgba(0.13, 0.17, 0.12, 0.5),
        Interaction::None => Color::srgba(0.0, 0.0, 0.0, 0.0),
    }
}

fn push_battle_log(battle_log: &mut BattleLog, message: impl Into<String>, focus: Option<Vec3>) {
    push_battle_log_with_kind(battle_log, message, focus, BattleEventPingKind::Generic);
}

fn push_battle_log_with_kind(
    battle_log: &mut BattleLog,
    message: impl Into<String>,
    focus: Option<Vec3>,
    ping_kind: BattleEventPingKind,
) {
    battle_log.entries.push_back(BattleLogEntry {
        message: message.into(),
        remaining: BATTLE_LOG_ENTRY_TTL_SECONDS,
        focus,
        ping_kind,
        minimap_ping_active: focus.is_some(),
        minimap_ping_remaining: if focus.is_some() {
            BATTLE_EVENT_PING_LIFETIME_SECONDS
        } else {
            0.0
        },
    });
    while battle_log.entries.len() > BATTLE_LOG_MAX_ENTRIES {
        let _ = battle_log.entries.pop_front();
    }
}

fn push_under_attack_log(
    battle_log: &mut BattleLog,
    focus: Vec3,
    target_is_structure: bool,
) -> bool {
    if battle_log.under_attack_cooldown > 0.0 {
        return false;
    }
    let message = if target_is_structure {
        "基地遭到攻击"
    } else {
        "单位遭到攻击"
    };
    push_battle_log(battle_log, message, Some(focus));
    battle_log.under_attack_cooldown = BATTLE_LOG_UNDER_ATTACK_COOLDOWN_SECONDS;
    true
}

fn update_minimap(
    mut commands: Commands,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    player_colors: Res<PlayerColorSlots>,
    camera_state: Res<RtsCamera>,
    map_bounds: Res<MapBounds>,
    mut battle_log: ResMut<BattleLog>,
    content_q: Query<Entity, With<MinimapContent>>,
    mut root_q: Query<&mut BackgroundColor, With<MinimapRoot>>,
    mut status_text_q: Query<&mut Text, With<MinimapStatusText>>,
    marker_q: Query<Entity, With<MinimapMarker>>,
    world_q: Query<(
        &Transform,
        &Team,
        &Selectable,
        &VisibilityState,
        Option<&Unit>,
        Option<&Structure>,
        Option<&Health>,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
    )>,
) {
    for marker in &marker_q {
        commands.entity(marker).try_despawn();
    }

    let Ok(content) = content_q.single() else {
        return;
    };
    let visible_team = visible_player.team;
    let radar_state = radar_state_for_team(visible_team, &economies, &world_q);
    if let Ok(mut root_color) = root_q.single_mut() {
        *root_color = BackgroundColor(if radar_state == MinimapRadarState::Online {
            Color::srgba(0.025, 0.048, 0.052, 0.9)
        } else {
            Color::srgba(0.025, 0.03, 0.034, 0.9)
        });
    }
    if let Ok(mut text) = status_text_q.single_mut() {
        **text = radar_state.status_text().to_string();
    }
    if radar_state != MinimapRadarState::Online {
        for entry in &mut battle_log.entries {
            entry.minimap_ping_active = false;
        }
        return;
    }

    let Ok(mut content_commands) = commands.get_entity(content) else {
        return;
    };
    content_commands.with_children(|parent| {
        for (transform, team, _selectable, visibility, unit, structure, health, resource, supply) in
            &world_q
        {
            if health.is_some_and(|health| health.current <= 0.0) {
                continue;
            }
            if *team != visible_team && !visibility.visible {
                continue;
            }

            let (size, color) = minimap_entity_marker_style(
                *team,
                unit,
                structure,
                resource,
                supply,
                &player_colors,
            );
            parent.spawn(minimap_marker_bundle(
                transform.translation,
                size,
                color,
                *map_bounds,
            ));
        }

        parent.spawn(minimap_camera_marker_bundle(
            camera_state.focus,
            *map_bounds,
        ));

        for entry in battle_log
            .entries
            .iter()
            .filter(|entry| entry.minimap_ping_active && entry.focus.is_some())
        {
            let focus = entry.focus.unwrap();
            let progress = minimap_ping_progress(entry);
            let size = minimap_ping_size_at_progress(entry.ping_kind, progress);
            let color = minimap_ping_color_at_progress(entry.ping_kind, progress);
            parent.spawn(minimap_ping_bundle(focus, size, color, *map_bounds));
        }
    });
}

fn minimap_entity_marker_style(
    team: Team,
    unit: Option<&Unit>,
    structure: Option<&Structure>,
    resource: Option<&ResourceNode>,
    supply: Option<&SupplyCrate>,
    player_colors: &PlayerColorSlots,
) -> (f32, Color) {
    if resource.is_some() {
        return (
            MINIMAP_RESOURCE_MARKER_PX,
            Color::srgba(0.38, 0.74, 0.96, 0.92),
        );
    }
    if supply.is_some() {
        return (
            MINIMAP_RESOURCE_MARKER_PX + 1.0,
            Color::srgba(0.95, 0.84, 0.34, 0.94),
        );
    }
    let size = if structure.is_some() {
        MINIMAP_STRUCTURE_MARKER_PX
    } else if unit.is_some() {
        MINIMAP_ENTITY_MARKER_PX
    } else {
        MINIMAP_RESOURCE_MARKER_PX
    };
    (size, player_colors.minimap_color(team))
}

fn minimap_marker_bundle(world: Vec3, size: f32, color: Color, bounds: MapBounds) -> impl Bundle {
    let local = minimap_local_position_in_bounds(world, bounds);
    (
        MinimapMarker,
        Node {
            position_type: PositionType::Absolute,
            left: px(local.x - size * 0.5),
            top: px(local.y - size * 0.5),
            width: px(size),
            height: px(size),
            ..default()
        },
        BackgroundColor(color),
    )
}

fn minimap_camera_marker_bundle(world: Vec3, bounds: MapBounds) -> impl Bundle {
    let size = 11.0;
    let local = minimap_local_position_in_bounds(world, bounds);
    (
        MinimapMarker,
        Node {
            position_type: PositionType::Absolute,
            left: px(local.x - size * 0.5),
            top: px(local.y - size * 0.5),
            width: px(size),
            height: px(size),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.92, 0.96, 1.0, 0.95)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
    )
}

fn minimap_ping_bundle(world: Vec3, size: f32, color: Color, bounds: MapBounds) -> impl Bundle {
    let local = minimap_local_position_in_bounds(world, bounds);
    (
        MinimapMarker,
        Node {
            position_type: PositionType::Absolute,
            left: px(local.x - size * 0.5),
            top: px(local.y - size * 0.5),
            width: px(size),
            height: px(size),
            border: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(color),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
    )
}

fn minimap_ping_size(kind: BattleEventPingKind) -> f32 {
    match kind {
        BattleEventPingKind::Generic => 18.0,
        BattleEventPingKind::SupportPower => 22.0,
        BattleEventPingKind::EnemySupportPower => 24.0,
        BattleEventPingKind::EnemySuperweapon => 31.0,
    }
}

fn minimap_ping_progress(entry: &BattleLogEntry) -> f32 {
    1.0 - (entry.minimap_ping_remaining / BATTLE_EVENT_PING_LIFETIME_SECONDS).clamp(0.0, 1.0)
}

fn minimap_ping_size_at_progress(kind: BattleEventPingKind, progress: f32) -> f32 {
    let min = match kind {
        BattleEventPingKind::Generic | BattleEventPingKind::SupportPower => 5.0,
        BattleEventPingKind::EnemySupportPower => 6.0,
        BattleEventPingKind::EnemySuperweapon => 7.0,
    };
    min.lerp(minimap_ping_size(kind), progress.clamp(0.0, 1.0))
}

fn minimap_ping_color_at_progress(kind: BattleEventPingKind, progress: f32) -> Color {
    let alpha_scale = 1.0 - progress.clamp(0.0, 1.0);
    match kind {
        BattleEventPingKind::Generic => Color::srgba(1.0, 0.92, 0.32, 0.9 * alpha_scale),
        BattleEventPingKind::SupportPower => Color::srgba(0.35, 0.82, 1.0, 0.95 * alpha_scale),
        BattleEventPingKind::EnemySupportPower => Color::srgba(1.0, 0.44, 0.18, 0.96 * alpha_scale),
        BattleEventPingKind::EnemySuperweapon => Color::srgba(1.0, 0.12, 0.08, alpha_scale),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MinimapRadarState {
    Online,
    MissingRadar,
    LowPower,
}

impl MinimapRadarState {
    fn status_text(self) -> &'static str {
        match self {
            Self::Online => "",
            Self::MissingRadar => "雷达离线\n建造 Radar Uplink",
            Self::LowPower => "雷达离线\n电力不足",
        }
    }
}

fn minimap_radar_state(has_radar: bool, low_power: bool) -> MinimapRadarState {
    if !has_radar {
        MinimapRadarState::MissingRadar
    } else if low_power {
        MinimapRadarState::LowPower
    } else {
        MinimapRadarState::Online
    }
}

fn radar_state_for_team(
    radar_team: Team,
    economies: &Economies,
    world_q: &Query<(
        &Transform,
        &Team,
        &Selectable,
        &VisibilityState,
        Option<&Unit>,
        Option<&Structure>,
        Option<&Health>,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
    )>,
) -> MinimapRadarState {
    let has_radar = world_q
        .iter()
        .any(|(_, team, _, _, _, structure, health, _, _)| {
            *team == radar_team
                && structure.is_some_and(|structure| structure.id == "RadarUplink")
                && health.is_none_or(|health| health.current > 0.0)
        });
    minimap_radar_state(has_radar, economies.get(radar_team).low_power())
}

#[cfg(feature = "audio")]
fn play_pending_audio_feedback(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut feedback: ResMut<AudioFeedback>,
) {
    if let Some(sound) = feedback.pending_sound.take() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(sound.audio_path())),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(sound.volume())),
            MatchScopedEntity,
        ));
        feedback.last_sound = Some(sound);
    }
    if let Some(voice) = feedback.pending_voice.take() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(voice.audio_path())),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.92)),
            MatchScopedEntity,
        ));
        feedback.last_voice = Some(voice);
    }
}

#[cfg(not(feature = "audio"))]
fn play_pending_audio_feedback(mut feedback: ResMut<AudioFeedback>) {
    if let Some(sound) = feedback.pending_sound.take() {
        feedback.last_sound = Some(sound);
    }
    if let Some(voice) = feedback.pending_voice.take() {
        feedback.last_voice = Some(voice);
    }
}

impl SoundEffectKind {
    #[allow(dead_code)]
    fn audio_path(self) -> &'static str {
        match self {
            Self::Select => "sfx/ui_select.wav",
            Self::Command => "sfx/command_confirm.wav",
            Self::ProductionStart => "sfx/production_start.wav",
            Self::ProductionReady => "sfx/production_ready.wav",
            Self::ConstructionStarted => "sfx/construction_started.wav",
            Self::ConstructionCanceled => "sfx/construction_canceled.wav",
            Self::Error => "sfx/error.wav",
            Self::LowPower => "sfx/low_power_warning.wav",
            Self::RepairStarted => "sfx/repair_started.wav",
            Self::StructureCaptured => "sfx/structure_captured.wav",
            Self::StructureLost => "sfx/structure_lost.wav",
            Self::StructureSold => "sfx/structure_sold.wav",
            Self::SupplyCrate => "sfx/supply_crate.wav",
            Self::UnitPromoted => "sfx/unit_promoted.wav",
            Self::SupportPowerReady => "sfx/support_power_ready.wav",
            Self::SupportPowerFire => "sfx/support_power_fire.wav",
            Self::SuperweaponWarning => "sfx/superweapon_warning.wav",
            Self::WeaponHit => "sfx/weapon_hit.wav",
            Self::Explosion => "sfx/explosion_small.wav",
        }
    }

    #[allow(dead_code)]
    fn volume(self) -> f32 {
        match self {
            Self::Select => 0.72,
            Self::Command => 0.66,
            Self::ProductionStart => 0.5,
            Self::ProductionReady => 0.56,
            Self::ConstructionStarted => 0.56,
            Self::ConstructionCanceled => 0.56,
            Self::Error => 0.63,
            Self::LowPower => 0.7,
            Self::RepairStarted => 0.56,
            Self::StructureCaptured => 0.63,
            Self::StructureLost => 0.72,
            Self::StructureSold => 0.63,
            Self::SupplyCrate => 0.56,
            Self::UnitPromoted => 0.63,
            Self::SupportPowerReady => 0.63,
            Self::SupportPowerFire => 0.7,
            Self::SuperweaponWarning => 0.82,
            Self::WeaponHit => 0.45,
            Self::Explosion => 0.63,
        }
    }
}

impl UnitVoiceEvent {
    #[allow(dead_code)]
    fn audio_path(self) -> &'static str {
        match self {
            Self::Hello => "voice/english/ttsmaker-com-2704-jackson-us/sir.ogg",
            Self::Ack1 => "voice/english/ttsmaker-com-2704-jackson-us/yes_sir.ogg",
            Self::Ack2 => "voice/english/ttsmaker-com-2704-jackson-us/acknowledged.ogg",
            Self::Training => "voice/english/ttsmaker-com-148-alayna-us/training.ogg",
            Self::UnitReady => "voice/english/ttsmaker-com-148-alayna-us/unit_ready.ogg",
            Self::ConstructionComplete => {
                "voice/english/ttsmaker-com-148-alayna-us/construction_complete.ogg"
            }
            Self::NotEnoughResources => {
                "voice/english/ttsmaker-com-148-alayna-us/not_enough_resources.ogg"
            }
            Self::SupportPowerReady => "voice/english/ttsmaker-com-148-alayna-us/unit_ready.ogg",
            Self::SupportPowerFired => {
                "voice/english/ttsmaker-com-2704-jackson-us/acknowledged.ogg"
            }
            Self::EnemySupportPowerFired => {
                "voice/english/ttsmaker-com-148-alayna-us/unit_under_attack.ogg"
            }
            Self::EnemySuperweaponReady | Self::EnemySuperweaponLaunched => {
                "voice/english/ttsmaker-com-148-alayna-us/your_base_is_under_attack.ogg"
            }
            Self::Victory => "voice/english/ttsmaker-com-148-alayna-us/you_are_victorious.ogg",
            Self::Defeat => "voice/english/ttsmaker-com-148-alayna-us/you_have_lost.ogg",
            Self::BaseUnderAttack => {
                "voice/english/ttsmaker-com-148-alayna-us/your_base_is_under_attack.ogg"
            }
            Self::UnitUnderAttack => {
                "voice/english/ttsmaker-com-148-alayna-us/unit_under_attack.ogg"
            }
            Self::UnitLost => "voice/english/ttsmaker-com-148-alayna-us/unit_lost.ogg",
        }
    }
}

fn record_sound_audio_feedback(feedback: &mut AudioFeedback, sound: SoundEffectKind) {
    feedback.pending_sound = Some(sound);
}

fn record_voice_audio_feedback(feedback: &mut AudioFeedback, voice: UnitVoiceEvent) {
    feedback.pending_voice = Some(voice);
}

fn record_low_power_audio_feedback(feedback: &mut AudioFeedback, is_low_power: bool) -> bool {
    let became_low_power = feedback
        .last_low_power
        .is_some_and(|was_low_power| !was_low_power && is_low_power);
    if became_low_power {
        record_sound_audio_feedback(feedback, SoundEffectKind::LowPower);
    }
    feedback.last_low_power = Some(is_low_power);
    became_low_power
}

fn record_low_power_battle_log(battle_log: &mut BattleLog) {
    push_battle_log(battle_log, "低电力: 生产减速/防御停火/雷达离线", None);
}

fn record_insufficient_funds_battle_log(team: Team, player_team: Team, battle_log: &mut BattleLog) {
    if team == player_team {
        push_battle_log(battle_log, "资源不足", None);
    }
}

fn record_structure_placement_failure_battle_log(
    team: Team,
    player_team: Team,
    validity: StructurePlacementValidity,
    focus: Vec3,
    battle_log: &mut BattleLog,
) {
    if team == player_team
        && let Some(message) = structure_placement_feedback_text(validity)
    {
        push_battle_log(battle_log, message, Some(focus));
    }
}

fn structure_placement_feedback_text(validity: StructurePlacementValidity) -> Option<&'static str> {
    match validity {
        StructurePlacementValidity::Valid => None,
        StructurePlacementValidity::CollidesWithObject => Some("无法摆放: 与单位/建筑/资源重叠"),
        StructurePlacementValidity::NotEnoughResources => Some("无法摆放: 资源不足"),
        StructurePlacementValidity::OutOfMap => Some("无法摆放: 超出地图边界"),
        StructurePlacementValidity::MissingTech => Some("无法摆放: 缺少建造前置"),
        StructurePlacementValidity::OutOfBaseRadius => Some("无法摆放: 离基地太远"),
    }
}

fn record_support_power_audio_feedback(
    feedback: &mut AudioFeedback,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerFire);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::SupportPowerFired);
    } else if power.is_superweapon() {
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySuperweaponLaunched);
    } else {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerFire);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySupportPowerFired);
    }
}

fn record_build_action_audio_feedback(
    feedback: &mut AudioFeedback,
    team: Team,
    player_team: Team,
    action: BuildAction,
) {
    if team != player_team {
        return;
    }
    match action {
        BuildAction::Train(_) => {
            record_sound_audio_feedback(feedback, SoundEffectKind::ProductionStart);
            record_voice_audio_feedback(feedback, UnitVoiceEvent::Training);
        }
        BuildAction::Build(_) => {
            record_sound_audio_feedback(feedback, SoundEffectKind::ConstructionStarted);
        }
        BuildAction::RepairStructure => {
            record_sound_audio_feedback(feedback, SoundEffectKind::RepairStarted);
        }
        BuildAction::SellStructure => {
            record_sound_audio_feedback(feedback, SoundEffectKind::StructureSold);
        }
        BuildAction::None
        | BuildAction::DeployMcv
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected => {}
    }
}

fn record_support_power_ready_audio_feedback(
    feedback: &mut AudioFeedback,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerReady);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::SupportPowerReady);
    } else if power.is_superweapon() {
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySuperweaponReady);
    }
}

fn record_support_power_charging_feedback(
    feedback: &mut AudioFeedback,
    battle_log: &mut BattleLog,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
    charge_seconds: f32,
) {
    if !power.is_superweapon() {
        return;
    }
    let charge_seconds = charge_seconds.ceil() as i32;
    if team == player_team {
        push_battle_log(
            battle_log,
            format!("超级武器充能: {} {charge_seconds}s", power.label()),
            None,
        );
    } else {
        push_battle_log(
            battle_log,
            format!("敌方超级武器充能: {} {charge_seconds}s", power.label()),
            None,
        );
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
    }
}

fn record_support_power_ready_battle_log(
    battle_log: &mut BattleLog,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        push_battle_log(battle_log, format!("支援就绪: {}", power.label()), None);
    } else if power.is_superweapon() {
        push_battle_log(
            battle_log,
            format!("敌方超级武器就绪: {}", power.label()),
            None,
        );
    }
}

fn monitor_low_power_audio_feedback(
    economies: Res<Economies>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    if record_low_power_audio_feedback(&mut feedback, economies.get(player_team).low_power()) {
        record_low_power_battle_log(&mut battle_log);
    }
}

fn record_selection_audio_feedback(
    feedback: &mut AudioFeedback,
    selected_owned: bool,
    selected_owned_voice_unit: bool,
) {
    if selected_owned {
        feedback.pending_sound = Some(SoundEffectKind::Select);
    }
    if selected_owned_voice_unit {
        feedback.pending_voice = Some(UnitVoiceEvent::Hello);
    }
}

fn record_command_audio_feedback(
    feedback: &mut AudioFeedback,
    has_owned_voice_unit: bool,
    command_key: Option<&'static str>,
) {
    if !has_owned_voice_unit {
        return;
    }
    feedback.pending_sound = Some(SoundEffectKind::Command);
    let event = if feedback.next_ack_is_first {
        UnitVoiceEvent::Ack1
    } else {
        UnitVoiceEvent::Ack2
    };
    feedback.pending_voice = Some(event);
    feedback.next_ack_is_first = !feedback.next_ack_is_first;
    feedback.last_command_key = command_key;
}

fn is_voice_unit(unit: &Unit) -> bool {
    unit.speed > 0.0
}

fn selected_query_has_owned_voice_unit(
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    team: Team,
) -> bool {
    selected_units
        .iter()
        .any(|(_, unit, unit_team, ..)| *unit_team == team && is_voice_unit(unit))
}

fn combat_wreckage_expired(wreckage: &mut CombatWreckage, delta_secs: f32) -> bool {
    wreckage.remaining -= delta_secs;
    wreckage.remaining <= 0.0
}

fn spawn_destruction_effects(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
    is_structure: bool,
    team: Team,
    remembered_fog_structure: bool,
) {
    if is_structure && remembered_fog_structure {
        spawn_fog_memory_structure_remnant(commands, Some(asset_server), position, radius);
        return;
    }
    spawn_combat_wreckage(commands, asset_server, position, radius);
    if is_structure {
        spawn_structure_destruction_vfx(commands, position, radius, team);
    }
}

fn spawn_fog_memory_structure_remnant(
    commands: &mut Commands,
    asset_server: Option<&AssetServer>,
    position: Vec3,
    radius: f32,
) {
    let radius = radius.max(0.45);
    let parent = commands
        .spawn((
            Name::new("Fog memory structure remnant"),
            Transform::from_translation(Vec3::new(position.x, 0.02, position.z)),
            Visibility::Visible,
            FogMemoryStructureRemnant { radius },
            MatchScopedEntity,
        ))
        .id();
    let Some(asset_server) = asset_server else {
        return;
    };
    commands.entity(parent).with_children(|remnant| {
        remnant.spawn((
            Name::new("Fog memory scorch mark"),
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/craterLarge.glb"),
            )),
            Transform::from_scale(Vec3::splat((radius * 0.45).clamp(0.28, 0.8))),
            ScorchMark,
        ));
        for (index, (asset, offset, scale)) in [
            (
                "models/kenney-spacekit/rocks_smallA.glb",
                Vec3::new(0.42, 0.03, 0.18),
                0.22,
            ),
            (
                "models/kenney-spacekit/rocks_smallB.glb",
                Vec3::new(-0.34, 0.02, 0.28),
                0.2,
            ),
            (
                "models/kenney-spacekit/meteor_half.glb",
                Vec3::new(0.12, 0.04, -0.36),
                0.16,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let spread = radius * 0.62;
            remnant.spawn((
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset))),
                Transform::from_translation(offset * spread)
                    .with_rotation(Quat::from_rotation_y(index as f32 * 1.9))
                    .with_scale(Vec3::splat(scale * radius)),
            ));
        }
    });
}

fn spawn_combat_wreckage(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
) {
    let radius = radius.max(0.45);
    let parent = commands
        .spawn((
            Name::new("Combat wreckage"),
            Transform::from_translation(Vec3::new(position.x, 0.02, position.z)),
            Visibility::Visible,
            CombatWreckage {
                remaining: COMBAT_WRECKAGE_LIFETIME_SECONDS,
            },
            MatchScopedEntity,
        ))
        .id();
    commands.entity(parent).with_children(|wreckage| {
        wreckage.spawn((
            Name::new("Scorch mark"),
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/craterLarge.glb"),
            )),
            Transform::from_scale(Vec3::splat((radius * 0.45).clamp(0.28, 0.8))),
            ScorchMark,
        ));
        for (index, (asset, offset, scale)) in [
            (
                "models/kenney-spacekit/rocks_smallA.glb",
                Vec3::new(0.42, 0.03, 0.18),
                0.22,
            ),
            (
                "models/kenney-spacekit/rocks_smallB.glb",
                Vec3::new(-0.34, 0.02, 0.28),
                0.2,
            ),
            (
                "models/kenney-spacekit/meteor_half.glb",
                Vec3::new(0.12, 0.04, -0.36),
                0.16,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let spread = radius * 0.62;
            wreckage.spawn((
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset))),
                Transform::from_translation(offset * spread)
                    .with_rotation(Quat::from_rotation_y(index as f32 * 1.9))
                    .with_scale(Vec3::splat(scale * radius)),
            ));
        }
    });
}

fn spawn_structure_destruction_vfx(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    team: Team,
) {
    let radius = radius.max(0.7);
    for (name, kind, ttl, y_offset, scale) in [
        (
            "ExplosionFireball",
            StructureDestructionVfxKind::ExplosionFireball,
            STRUCTURE_FIREBALL_LIFETIME_SECONDS,
            0.45,
            1.15,
        ),
        (
            "SmokeColumn",
            StructureDestructionVfxKind::SmokeColumn,
            STRUCTURE_SMOKE_COLUMN_LIFETIME_SECONDS,
            0.25,
            1.45,
        ),
    ] {
        commands.spawn((
            Name::new(name),
            Transform::from_translation(Vec3::new(position.x, y_offset, position.z)),
            StructureDestructionVfx {
                kind,
                remaining: ttl,
                total: ttl,
                radius: radius * scale,
                team,
            },
            MatchScopedEntity,
        ));
    }
}

fn command_button(index: usize) -> impl Bundle {
    (
        Button,
        BuildAction::None,
        CommandSlot(index),
        CommandSlotAvailability::default(),
        Node {
            width: px(146),
            height: px(46),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.34, 0.39)),
        BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.78)),
    )
}

fn command_button_label(index: usize, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.94, 0.96)),
        CommandSlotLabel(index),
        ButtonLabel,
    )
}

fn production_queue_slot(index: usize) -> impl Bundle {
    (
        Button,
        ProductionQueueSlot(index),
        ProductionQueueSlotTarget::default(),
        Visibility::Hidden,
        Node {
            width: px(92),
            height: px(40),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.22, 0.3, 0.35)),
        BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.9)),
    )
}

fn production_queue_slot_label(index: usize, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.94, 0.96)),
        ProductionQueueSlotLabel(index),
        ButtonLabel,
    )
}

fn camera_control(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut wheel_events: MessageReader<MouseWheel>,
    map_bounds: Res<MapBounds>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut camera_state: ResMut<RtsCamera>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    clamp_camera_view_safely(&mut camera_state, *map_bounds);
    if mouse_motion_events.read().next().is_some() {
        camera_state.edge_pan_active = true;
    }
    let cursor = window.cursor_position();
    let pan = camera_screen_pan_vector(
        &keyboard,
        cursor,
        window_size_vec(window),
        camera_state.edge_pan_active,
        cursor.is_some_and(|_| cursor_is_over_hud(window)),
    );
    if pan.length_squared() > 0.0 {
        let yaw = camera_state.yaw;
        let distance = camera_state.distance;
        let delta_seconds = time.delta_secs();
        camera_state.focus += camera_pan_delta(yaw, distance, pan, delta_seconds);
        clamp_camera_focus_safely(&mut camera_state, *map_bounds);
    }
    for event in wheel_events.read() {
        let scroll = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y * 0.05,
        };
        camera_state.distance = camera_zoom_distance_after_scroll(camera_state.distance, scroll);
    }
    let Ok((mut transform, mut projection)) = camera_q.single_mut() else {
        return;
    };
    *transform = camera_transform_from_state(&camera_state);
    *projection = camera_projection_from_state(&camera_state);
}

fn camera_zoom_distance_after_scroll(current_distance: f32, scroll_lines: f32) -> f32 {
    safe_camera_distance(current_distance - scroll_lines)
}

fn safe_camera_distance(distance: f32) -> f32 {
    distance.clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE)
}

fn safe_camera_focus(focus: Vec3, bounds: MapBounds) -> Vec3 {
    bounds.clamp_ground_point(focus, CAMERA_BOUNDS_MARGIN)
}

fn set_camera_focus_safely(camera: &mut RtsCamera, focus: Vec3, bounds: MapBounds) {
    camera.focus = safe_camera_focus(focus, bounds);
}

fn clamp_camera_focus_safely(camera: &mut RtsCamera, bounds: MapBounds) {
    camera.focus = safe_camera_focus(camera.focus, bounds);
}

fn clamp_camera_distance_safely(camera: &mut RtsCamera) {
    camera.distance = safe_camera_distance(camera.distance);
}

fn clamp_camera_view_safely(camera: &mut RtsCamera, bounds: MapBounds) {
    clamp_camera_focus_safely(camera, bounds);
    clamp_camera_distance_safely(camera);
}

fn cursor_edge_pan_vector(
    cursor: Option<Vec2>,
    window_size: Vec2,
    edge_pan_active: bool,
    over_hud: bool,
) -> Vec2 {
    if !edge_pan_active || over_hud {
        return Vec2::ZERO;
    }
    let Some(cursor) = cursor else {
        return Vec2::ZERO;
    };
    let mut pan = Vec2::ZERO;
    if cursor.x < EDGE_PAN_PX {
        pan.x -= 1.0;
    } else if cursor.x > window_size.x - EDGE_PAN_PX {
        pan.x += 1.0;
    }
    if cursor.y < EDGE_PAN_PX {
        pan.y += 1.0;
    } else if cursor.y > window_size.y - EDGE_PAN_PX {
        pan.y -= 1.0;
    }
    pan
}

fn window_size_vec(window: &Window) -> Vec2 {
    Vec2::new(window.width(), window.height())
}

fn camera_screen_pan_vector(
    keyboard: &ButtonInput<KeyCode>,
    cursor: Option<Vec2>,
    window_size: Vec2,
    edge_pan_active: bool,
    over_hud: bool,
) -> Vec2 {
    let mut pan = camera_keyboard_pan_vector(keyboard);
    let edge_pan = cursor_edge_pan_vector(cursor, window_size, edge_pan_active, over_hud);
    if edge_pan.x != 0.0 {
        pan.x = edge_pan.x;
    }
    if edge_pan.y != 0.0 {
        pan.y = edge_pan.y;
    }
    pan
}

fn camera_keyboard_pan_vector(keyboard: &ButtonInput<KeyCode>) -> Vec2 {
    camera_pan_from_key_states(
        keyboard.pressed(KeyCode::KeyA),
        keyboard.pressed(KeyCode::KeyD),
        keyboard.pressed(KeyCode::KeyW),
        keyboard.pressed(KeyCode::KeyS),
    )
}

fn camera_pan_from_key_states(left: bool, right: bool, up: bool, down: bool) -> Vec2 {
    Vec2::new(
        (right as i32 - left as i32) as f32,
        (up as i32 - down as i32) as f32,
    )
}

fn camera_pan_delta(yaw: f32, distance: f32, pan: Vec2, delta_seconds: f32) -> Vec3 {
    if pan.length_squared() <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);
    let screen_move = pan.normalize()
        * Vec2::new(
            CAMERA_PAN_SPEED_MULTIPLIER,
            CAMERA_PAN_SPEED_MULTIPLIER * 2.0,
        )
        * distance
        * delta_seconds;
    right * screen_move.x + forward * screen_move.y
}

fn structure_placement_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut placement: StructurePlacementInputResources,
    structures: Query<StructurePrereqItem<'_>>,
    occupiers: Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) {
    if controlled_player_team(Some(&*placement.visible_player)).is_none() {
        placement.command_mode.pending_structure_placement = None;
        *placement.placement_feedback = StructurePlacementFeedback::default();
        return;
    }
    if placement.command_mode.pending_structure_placement.is_none() {
        *placement.placement_feedback = StructurePlacementFeedback::default();
        return;
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        rotate_pending_structure_placement(&mut placement.command_mode);
    }
    if mouse.just_pressed(MouseButton::Right) {
        placement.command_mode.pending_structure_placement = None;
        *placement.placement_feedback = StructurePlacementFeedback::default();
        return;
    }
    let pointer = window_q.single().ok().and_then(|window| {
        (!cursor_is_over_hud(window))
            .then(|| pointer_ground(window, &camera_q))
            .flatten()
    });
    let team = placement.visible_player.team;
    let faction = placement.player_factions.slot_faction(team);
    let map_bounds = *placement.map_bounds;
    let mut placement_request = None;
    if let Some(pending) = placement.command_mode.pending_structure_placement.as_mut() {
        update_pending_structure_placement_pointer(pending, &mouse, pointer);
        placement.placement_feedback.validity = pending.position.or(pointer).map(|point| {
            structure_placement_validity_for_faction(
                team,
                faction,
                pending.id,
                point,
                map_bounds,
                &placement.economies,
                &structures,
                &occupiers,
            )
        });
        if mouse.just_released(MouseButton::Left) {
            if let Some(point) = pending.position.or(pointer) {
                placement_request = Some((pending.id, point, pending.rotation_y_radians()));
            }
            finish_pending_structure_drag(pending);
        }
    }
    let Some((id, point, rotation_y_radians)) = placement_request else {
        return;
    };
    let player_team = placement.visible_player.team;
    match place_structure_at_for_faction(
        &mut commands,
        &placement.asset_server,
        &mut placement.next_id,
        team,
        faction,
        player_team,
        id,
        point,
        rotation_y_radians,
        map_bounds,
        &mut placement.economies,
        &structures,
        &occupiers,
    ) {
        Ok((_entity, label)) => {
            placement.command_mode.pending_structure_placement = None;
            *placement.placement_feedback = StructurePlacementFeedback::default();
            if team == player_team {
                record_sound_audio_feedback(
                    &mut placement.audio_feedback,
                    SoundEffectKind::ConstructionStarted,
                );
                push_battle_log(
                    &mut placement.battle_log,
                    format!("开始施工: {label}"),
                    Some(point),
                );
            }
        }
        Err(StructurePlacementValidity::NotEnoughResources) => {
            if team == player_team {
                record_sound_audio_feedback(&mut placement.audio_feedback, SoundEffectKind::Error);
                record_voice_audio_feedback(
                    &mut placement.audio_feedback,
                    UnitVoiceEvent::NotEnoughResources,
                );
                record_structure_placement_failure_battle_log(
                    team,
                    player_team,
                    StructurePlacementValidity::NotEnoughResources,
                    point,
                    &mut placement.battle_log,
                );
            }
        }
        Err(validity) => {
            if team == player_team {
                record_sound_audio_feedback(&mut placement.audio_feedback, SoundEffectKind::Error);
                record_structure_placement_failure_battle_log(
                    team,
                    player_team,
                    validity,
                    point,
                    &mut placement.battle_log,
                );
            }
        }
    }
}

#[allow(dead_code)]
fn place_structure_at(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    visible_team: Team,
    id: &'static str,
    point: Vec3,
    rotation_y_radians: f32,
    bounds: MapBounds,
    economies: &mut Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) -> Result<(Entity, &'static str), StructurePlacementValidity> {
    place_structure_at_for_faction(
        commands,
        asset_server,
        next_id,
        team,
        SkirmishFaction::from_team(team),
        visible_team,
        id,
        point,
        rotation_y_radians,
        bounds,
        economies,
        structures,
        occupiers,
    )
}

fn place_structure_at_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    id: &'static str,
    point: Vec3,
    rotation_y_radians: f32,
    bounds: MapBounds,
    economies: &mut Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) -> Result<(Entity, &'static str), StructurePlacementValidity> {
    let def = registry::entity(id).ok_or(StructurePlacementValidity::MissingTech)?;
    let validity = structure_placement_validity_for_faction(
        team, faction, id, point, bounds, economies, structures, occupiers,
    );
    if validity != StructurePlacementValidity::Valid {
        return Err(validity);
    }
    if !economies.get_mut(team).spend(def.cost) {
        return Err(StructurePlacementValidity::NotEnoughResources);
    }
    let free_harvester_origin = if id == "Refinery" {
        nearest_base_construction_anchor(team, point, def.radius, structures)
    } else {
        None
    };
    let entity = spawn_structure_under_construction_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        point,
        free_harvester_origin,
        rotation_y_radians,
        visible_team,
        faction,
    );
    Ok((entity, def.label))
}

fn rotate_pending_structure_placement(command_mode: &mut CommandMode) -> bool {
    let Some(pending) = command_mode.pending_structure_placement.as_mut() else {
        return false;
    };
    pending.rotation_y_radians = normalize_structure_rotation_y(
        pending.rotation_y_radians + STRUCTURE_PLACEMENT_ROTATION_STEP_RADIANS,
    );
    true
}

fn update_pending_structure_placement_pointer(
    pending: &mut PendingStructurePlacement,
    mouse: &ButtonInput<MouseButton>,
    pointer: Option<Vec3>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(point) = pointer {
            begin_pending_structure_drag(pending, point);
        }
    }
    if pending.drag_rotation_origin.is_some() {
        if let Some(point) = pointer {
            rotate_pending_structure_drag_towards(pending, point);
        }
    } else if let Some(point) = pointer {
        pending.position = Some(point);
    }
}

fn begin_pending_structure_drag(pending: &mut PendingStructurePlacement, point: Vec3) {
    pending.position = Some(point);
    pending.drag_rotation_origin = Some(point);
}

fn finish_pending_structure_drag(pending: &mut PendingStructurePlacement) {
    pending.drag_rotation_origin = None;
}

fn rotate_pending_structure_drag_towards(
    pending: &mut PendingStructurePlacement,
    target: Vec3,
) -> bool {
    let Some(origin) = pending.drag_rotation_origin else {
        return false;
    };
    let Some(rotation) = structure_drag_rotation_y(origin, target) else {
        return false;
    };
    pending.rotation_y_radians = rotation;
    true
}

fn structure_drag_rotation_y(origin: Vec3, target: Vec3) -> Option<f32> {
    let delta = Vec2::new(target.x - origin.x, target.z - origin.z);
    if delta.length() < STRUCTURE_PLACEMENT_ROTATION_DEAD_ZONE_M {
        return None;
    }
    Some(normalize_structure_rotation_y(delta.x.atan2(delta.y)))
}

fn normalize_structure_rotation_y(rotation_y_radians: f32) -> f32 {
    let normalized = rotation_y_radians.rem_euclid(std::f32::consts::TAU);
    if normalized < 0.0001 || (std::f32::consts::TAU - normalized) < 0.0001 {
        0.0
    } else {
        normalized
    }
}

#[allow(dead_code)]
fn structure_placement_validity(
    team: Team,
    id: &'static str,
    point: Vec3,
    bounds: MapBounds,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) -> StructurePlacementValidity {
    structure_placement_validity_for_faction(
        team,
        SkirmishFaction::from_team(team),
        id,
        point,
        bounds,
        economies,
        structures,
        occupiers,
    )
}

fn structure_placement_validity_for_faction(
    team: Team,
    faction: SkirmishFaction,
    id: &'static str,
    point: Vec3,
    bounds: MapBounds,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) -> StructurePlacementValidity {
    let Some(def) = registry::entity(id) else {
        return StructurePlacementValidity::MissingTech;
    };
    if !map_contains_ground_point_in_bounds(point, bounds) {
        return StructurePlacementValidity::OutOfMap;
    }
    let Some(faction) = faction_def(faction) else {
        return StructurePlacementValidity::MissingTech;
    };
    if !faction.can_construct(id) || !requirements_met(def, team, structures) {
        return StructurePlacementValidity::MissingTech;
    }
    if !economies.get(team).can_afford(def.cost) {
        return StructurePlacementValidity::NotEnoughResources;
    }
    if nearest_base_construction_anchor(team, point, def.radius, structures).is_none() {
        return StructurePlacementValidity::OutOfBaseRadius;
    }
    if structure_placement_collides(point, def.radius, occupiers) {
        return StructurePlacementValidity::CollidesWithObject;
    }
    StructurePlacementValidity::Valid
}

fn nearest_base_construction_anchor(
    team: Team,
    point: Vec3,
    structure_radius: f32,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<Vec3> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (structure, structure_team, transform, under_construction) in structures {
        if *structure_team != team || !structure_is_constructed(under_construction) {
            continue;
        }
        let Some(def) = registry::entity(structure.id) else {
            continue;
        };
        let distance = xz_distance(transform.translation, point);
        let allowed = def.radius + structure_radius + BASE_CONSTRUCTION_RADIUS_M;
        if distance <= allowed && distance < best_distance {
            best = Some(transform.translation);
            best_distance = distance;
        }
    }
    best
}

fn structure_placement_collides(
    point: Vec3,
    radius: f32,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) -> bool {
    occupiers
        .iter()
        .any(|(_entity, transform, selectable, health, resource_node)| {
            if health.is_some_and(|health| health.current <= 0.0)
                || resource_node.is_some_and(|resource| resource.amount <= 0)
            {
                return false;
            }
            xz_distance(transform.translation, point) <= selectable.radius + radius
        })
}

fn minimap_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    mut camera_state: ResMut<RtsCamera>,
    mut order_resources: OrderResources,
    world_q: Query<(
        &Transform,
        &Team,
        &Selectable,
        &VisibilityState,
        Option<&Unit>,
        Option<&Structure>,
        Option<&Health>,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
    )>,
    mut selected_params: ParamSet<(
        Query<SelectedOrderUnitItem<'_>, SelectedOrderUnitFilter>,
        Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
    )>,
    selectable_q: Query<SelectableOrderTargetItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
) {
    if order_resources
        .command_mode
        .pending_structure_placement
        .is_some()
    {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) && !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(local) = cursor_minimap_local(window) else {
        return;
    };
    disarm_support_power_on_left_click(&mut order_resources.command_mode, &mouse);

    let visible_team = visible_player.team;
    if radar_state_for_team(visible_team, &economies, &world_q) != MinimapRadarState::Online {
        return;
    }

    let Some(target) =
        minimap_world_position_from_local_in_bounds(local, *order_resources.map_bounds)
    else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        set_camera_focus_safely(&mut camera_state, target, *order_resources.map_bounds);
    }
    let Some(controlled_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(power) = order_resources.command_mode.support_power {
            let support_targets = support_power_target_snapshots(&selectable_q);
            if activate_support_power(
                &mut commands,
                target,
                power,
                controlled_team,
                controlled_team,
                &economies,
                &mut order_resources.support_cooldowns,
                &mut order_resources.battle_log,
                &order_resources.relations,
                &structures,
                &support_targets,
            ) {
                record_support_power_audio_feedback(
                    &mut order_resources.audio_feedback,
                    controlled_team,
                    controlled_team,
                    power,
                );
            }
            order_resources.command_mode.support_power = None;
            return;
        }

        if order_resources.command_mode.rally_point {
            let mut set_any = false;
            for (team, mut rally_point) in &mut selected_params.p1() {
                if *team == controlled_team
                    && apply_rally_point_command_in_bounds(
                        &mut rally_point,
                        target,
                        None,
                        *order_resources.map_bounds,
                    )
                {
                    set_any = true;
                }
            }
            if set_any {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    ClickMarker {
                        ttl: CLICK_MARKER_TTL_SECONDS,
                        radius: CLICK_MARKER_RADIUS_START,
                    },
                    MatchScopedEntity,
                ));
            }
            order_resources.command_mode.rally_point = false;
            return;
        }

        let queue_mode =
            keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        let attack_move = order_resources.command_mode.attack_move;
        let patrol = order_resources.command_mode.patrol;
        let mut issued_any = false;
        let has_owned_voice_unit;
        {
            let selected_units = selected_params.p0();
            let selected = selected_units
                .iter()
                .filter(|(_, _, _, team, ..)| **team == controlled_team)
                .collect::<Vec<_>>();
            has_owned_voice_unit = selected.iter().any(|selection| is_voice_unit(selection.2));
            let count = selected.len().max(1);
            for (index, (entity, transform, unit, _unit_team, orders, _cargo)) in
                selected.into_iter().enumerate()
            {
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
                    queue,
                ) = orders;
                let offset = formation_offset(index, count);
                let Some(desired) = desired_order_for_selected_unit(
                    unit,
                    OrderTargetChoices {
                        supply_crate_position: None,
                        resource_target: None,
                        resource_dropoff_target: None,
                        enemy_target: None,
                        repair_target: None,
                        construct_target: None,
                        garrison_target: None,
                        follow_target: None,
                    },
                    UnitOrderContext {
                        force_move: false,
                        enemy_target_capturable: false,
                        attack_move,
                        patrol,
                        origin: transform.translation,
                        point: target,
                        offset,
                    },
                ) else {
                    continue;
                };
                issued_any = true;
                let has_active = has_active_orders_in_query(
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
                );
                issue_or_queue_unit_order(
                    &mut commands,
                    entity,
                    desired,
                    queue_mode,
                    true,
                    has_active,
                    queue,
                );
                commands
                    .entity(entity)
                    .try_insert(HoldPosition { enabled: false });
            }
        }
        let set_rally_any = if should_set_terrain_rally_points(queue_mode, attack_move, patrol) {
            apply_selected_terrain_rally_points(
                controlled_team,
                target,
                *order_resources.map_bounds,
                &mut selected_params.p1(),
            )
        } else {
            false
        };
        order_resources.command_mode.attack_move = false;
        order_resources.command_mode.patrol = false;
        if issued_any {
            record_command_audio_feedback(
                &mut order_resources.audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_MINIMAP_MOVE),
            );
        }
        if issued_any || set_rally_any {
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                ClickMarker {
                    ttl: CLICK_MARKER_TTL_SECONDS,
                    radius: CLICK_MARKER_RADIUS_START,
                },
                MatchScopedEntity,
            ));
        }
    }
}

fn rotate_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    time: Res<Time>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut camera_state: ResMut<RtsCamera>,
    mut mouse_rotation: ResMut<CameraMouseRotation>,
) {
    if mouse.just_pressed(MouseButton::Middle) {
        let now = time.elapsed_secs();
        if camera_middle_click_is_reset(mouse_rotation.last_middle_press_time, now) {
            reset_camera_rotation(&mut camera_state, &mut mouse_rotation);
            mouse_rotation.last_middle_press_time = Some(now);
            return;
        }
        begin_camera_mouse_rotation(&mut mouse_rotation, camera_state.yaw, now);
    }

    if mouse.just_released(MouseButton::Middle) {
        mouse_rotation.active = false;
    }

    if mouse_rotation.active {
        let delta_x = mouse_motion_events
            .read()
            .map(|event| event.delta.x)
            .sum::<f32>();
        if delta_x.abs() > f32::EPSILON {
            apply_camera_mouse_rotation(&mut camera_state, &mut mouse_rotation, delta_x);
        }
        return;
    }
    for _ in mouse_motion_events.read() {}

    let movement_active = if let Ok(window) = window_q.single() {
        let cursor = window.cursor_position();
        camera_screen_pan_vector(
            &keyboard,
            cursor,
            window_size_vec(window),
            camera_state.edge_pan_active,
            cursor.is_some_and(|_| cursor_is_over_hud(window)),
        )
        .length_squared()
            > 0.0
    } else {
        camera_keyboard_pan_vector(&keyboard).length_squared() > 0.0
    };
    let rotate = camera_arrow_rotation_axis(&keyboard, movement_active);
    if rotate == 0.0 {
        return;
    }

    camera_state.yaw += rotate * CAMERA_ROTATE_SPEED * time.delta_secs();
}

fn camera_arrow_rotation_axis(keyboard: &ButtonInput<KeyCode>, movement_active: bool) -> f32 {
    camera_arrow_rotation_from_key_states(
        keyboard.pressed(KeyCode::KeyQ),
        keyboard.pressed(KeyCode::KeyE),
        movement_active,
    )
}

fn camera_arrow_rotation_from_key_states(
    counterclockwise: bool,
    clockwise: bool,
    movement_active: bool,
) -> f32 {
    if movement_active {
        return 0.0;
    }
    (clockwise as i32 - counterclockwise as i32) as f32
}

fn begin_camera_mouse_rotation(rotation: &mut CameraMouseRotation, yaw: f32, now: f32) {
    rotation.active = true;
    rotation.start_yaw = yaw;
    rotation.accumulated_x = 0.0;
    rotation.last_middle_press_time = Some(now);
}

fn apply_camera_mouse_rotation(
    camera: &mut RtsCamera,
    rotation: &mut CameraMouseRotation,
    delta_x: f32,
) {
    rotation.accumulated_x += delta_x;
    camera.yaw = rotation.start_yaw + rotation.accumulated_x * CAMERA_MOUSE_ROTATION_SPEED;
}

fn reset_camera_rotation(camera: &mut RtsCamera, rotation: &mut CameraMouseRotation) {
    camera.yaw = CAMERA_DEFAULT_YAW;
    rotation.active = false;
    rotation.accumulated_x = 0.0;
}

fn camera_middle_click_is_reset(last_click_time: Option<f32>, current_time: f32) -> bool {
    last_click_time.is_some_and(|last_click_time| {
        let delta = current_time - last_click_time;
        (DOUBLE_CLICK_MIN_SECONDS..=DOUBLE_CLICK_MAX_SECONDS).contains(&delta)
    })
}

fn focus_latest_battle_event(
    keyboard: Res<ButtonInput<KeyCode>>,
    latest_battle_event: Res<LatestBattleEvent>,
    map_bounds: Res<MapBounds>,
    mut camera_state: ResMut<RtsCamera>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        if let Some(focus) = latest_battle_event.focus {
            set_camera_focus_safely(&mut camera_state, focus, *map_bounds);
        }
    }
}

fn battle_log_entry_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    battle_log: Res<BattleLog>,
    map_bounds: Res<MapBounds>,
    mut camera_state: ResMut<RtsCamera>,
    mut buttons: Query<
        (
            &Interaction,
            &BattleLogEntryButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button, mut background, mut border) in &mut buttons {
        if *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left)
            && let Some(focus) = battle_log
                .entries
                .get(button.0)
                .and_then(|entry| entry.focus)
        {
            set_camera_focus_safely(&mut camera_state, focus, *map_bounds);
        }
        *background = BackgroundColor(battle_log_entry_button_color(*interaction));
        *border = BorderColor::all(match *interaction {
            Interaction::Pressed => Color::srgba(0.94, 0.98, 0.72, 0.56),
            Interaction::Hovered => Color::srgba(0.86, 0.94, 0.68, 0.4),
            Interaction::None => Color::srgba(0.82, 0.9, 0.72, 0.18),
        });
    }
}

fn select_entities(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    visible_player: Res<VisiblePlayer>,
    mut command_mode: ResMut<CommandMode>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut drag_state: ResMut<SelectionDragState>,
    mut double_click_state: ResMut<DoubleClickState>,
    mut audio_feedback: ResMut<AudioFeedback>,
    selectable_q: Query<(
        Entity,
        &Transform,
        &Selectable,
        &Team,
        &VisibilityState,
        Option<&Unit>,
        Option<&ResourceNode>,
        Option<&Selected>,
    )>,
) {
    if command_mode.pending_structure_placement.is_some() {
        return;
    }
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        for (entity, _, _, _, _, _, _, _) in &selectable_q {
            commands.entity(entity).try_remove::<Selected>();
        }
        drag_state.active = false;
        drag_state.dragging = false;
        double_click_state.last_unit = None;
        double_click_state.last_unit_type = None;
        return;
    };
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    };

    disarm_support_power_on_left_click(&mut command_mode, &mouse);

    if mouse.just_pressed(MouseButton::Left) {
        drag_state.active = true;
        drag_state.dragging = false;
        drag_state.start = cursor;
        drag_state.started_in_hud = cursor_is_over_hud(window);
        if selection_drag_should_interrupt(&drag_state, cursor, window_size(window)) {
            cancel_selection_drag(&mut drag_state);
        }
        return;
    }
    if !drag_state.active {
        return;
    }
    let additive = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse.pressed(MouseButton::Left) {
        if selection_drag_should_interrupt(&drag_state, cursor, window_size(window)) {
            cancel_selection_drag(&mut drag_state);
            return;
        }
        if !drag_state.started_in_hud
            && (cursor - drag_state.start).length() >= DRAG_SELECT_THRESHOLD
        {
            drag_state.dragging = true;
        }
        return;
    }

    if drag_state.started_in_hud {
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    }

    if drag_state.dragging {
        let Some(screen_polygon) = screen_polygon_for_drag(drag_state.start, cursor) else {
            if !additive {
                for (entity, _, _, _, _, _, _, _) in &selectable_q {
                    commands.entity(entity).try_remove::<Selected>();
                }
            }
            drag_state.active = false;
            drag_state.dragging = false;
            return;
        };
        let Ok((camera, camera_transform)) = camera_q.single() else {
            drag_state.active = false;
            drag_state.dragging = false;
            return;
        };
        if !additive {
            for (entity, _, _, _, _, _, _, _) in &selectable_q {
                commands.entity(entity).try_remove::<Selected>();
            }
        }
        let mut selected_owned = false;
        let mut selected_owned_voice_unit = false;
        for (entity, transform, _, team, visibility, unit, resource_node, _) in &selectable_q {
            if !visibility.visible {
                continue;
            }
            if *team != visible_team || resource_node.is_some() {
                continue;
            }
            let Ok(screen_position) =
                camera.world_to_viewport(camera_transform, transform.translation)
            else {
                continue;
            };
            if point_in_polygon(screen_position, &screen_polygon) {
                commands.entity(entity).try_insert(Selected);
                selected_owned = true;
                selected_owned_voice_unit |= unit.is_some_and(is_voice_unit);
            } else if !additive {
                commands.entity(entity).try_remove::<Selected>();
            }
        }
        record_selection_audio_feedback(
            &mut audio_feedback,
            selected_owned,
            selected_owned_voice_unit,
        );
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    }

    let Some(point) = pointer_ground(window, &camera_q) else {
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    };

    if !additive {
        for (entity, _, _, _, _, _, _, _) in &selectable_q {
            commands.entity(entity).try_remove::<Selected>();
        }
    }

    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, team, visibility, unit, resource_node, selected) in
        &selectable_q
    {
        if !visibility.visible {
            continue;
        }
        if *team != visible_team && resource_node.is_none() {
            continue;
        }
        if resource_node.is_some_and(|resource| resource.amount <= 0) {
            continue;
        }
        let ground_distance = xz_distance(transform.translation, point);
        let screen_distance = camera
            .world_to_viewport(camera_transform, transform.translation)
            .ok()
            .map(|screen_position| screen_position.distance(cursor));
        let ground_pick = ground_distance <= selectable.radius + 0.35;
        let screen_pick = screen_distance.is_some_and(|distance| {
            distance <= single_click_selection_screen_radius(selectable.radius)
        });
        let distance = screen_distance.unwrap_or(ground_distance * 64.0);
        if (ground_pick || screen_pick) && distance < nearest_distance {
            nearest = Some((
                entity,
                unit.map(|unit| unit.id),
                unit.is_some_and(is_voice_unit),
                selected.is_some(),
            ));
            nearest_distance = distance;
        }
    }

    if let Some((entity, target_unit, target_voice_unit, target_selected)) = nearest {
        let current_time = time.elapsed_secs();
        if double_click_state.last_unit == Some(entity)
            && double_click_state.last_unit_type == target_unit
            && (current_time - double_click_state.last_click_time) >= DOUBLE_CLICK_MIN_SECONDS
            && (current_time - double_click_state.last_click_time) <= DOUBLE_CLICK_MAX_SECONDS
            && let Some(target_id) = target_unit
        {
            for (entity, _, _, _, _, _, _, _) in &selectable_q {
                commands.entity(entity).try_remove::<Selected>();
            }
            let Ok((camera, camera_transform)) = camera_q.single() else {
                drag_state.active = false;
                drag_state.dragging = false;
                return;
            };
            let mut selected_owned = false;
            let mut selected_owned_voice_unit = false;
            for (entity, transform, _, team, visibility, same_unit, resource_node, _) in
                &selectable_q
            {
                if !visibility.visible {
                    continue;
                }
                if *team != visible_team || resource_node.is_some() {
                    continue;
                }
                if let Some(candidate_unit) = same_unit {
                    if candidate_unit.id == target_id
                        && point_is_on_screen(
                            window,
                            camera,
                            camera_transform,
                            transform.translation,
                        )
                    {
                        commands.entity(entity).try_insert(Selected);
                        selected_owned = true;
                        selected_owned_voice_unit |= is_voice_unit(candidate_unit);
                    }
                }
            }
            record_selection_audio_feedback(
                &mut audio_feedback,
                selected_owned,
                selected_owned_voice_unit,
            );
        } else {
            if single_click_selection_action(additive, target_selected)
                == SingleClickSelectionAction::ToggleDeselect
            {
                commands.entity(entity).try_remove::<Selected>();
                double_click_state.last_click_time = time.elapsed_secs();
                double_click_state.last_unit = None;
                double_click_state.last_unit_type = None;
                drag_state.active = false;
                drag_state.dragging = false;
                return;
            }
            commands.entity(entity).try_insert(Selected);
            record_selection_audio_feedback(&mut audio_feedback, true, target_voice_unit);
        }
        double_click_state.last_click_time = current_time;
        double_click_state.last_unit = Some(entity);
        double_click_state.last_unit_type = target_unit;
    } else {
        double_click_state.last_click_time = time.elapsed_secs();
        double_click_state.last_unit = None;
        double_click_state.last_unit_type = None;
    }

    drag_state.active = false;
    drag_state.dragging = false;
}

fn single_click_selection_screen_radius(selectable_radius: f32) -> f32 {
    (SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PX
        + selectable_radius.max(0.0) * SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PER_METER_PX)
        .clamp(24.0, 72.0)
}

fn cancel_selection_drag(drag_state: &mut SelectionDragState) {
    drag_state.active = false;
    drag_state.dragging = false;
}

fn disarm_support_power_on_left_click(
    command_mode: &mut CommandMode,
    mouse: &ButtonInput<MouseButton>,
) -> bool {
    if mouse.just_pressed(MouseButton::Left) && command_mode.support_power.is_some() {
        command_mode.support_power = None;
        return true;
    }
    false
}

fn selection_drag_should_interrupt(
    drag_state: &SelectionDragState,
    cursor: Vec2,
    window_size: Vec2,
) -> bool {
    drag_state.active
        && !drag_state.started_in_hud
        && selection_drag_hits_screen_margin(cursor, window_size)
}

fn selection_drag_hits_screen_margin(cursor: Vec2, window_size: Vec2) -> bool {
    cursor.x <= SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.x >= window_size.x - SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.y <= SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.y >= window_size.y - SELECTION_DRAG_INTERRUPT_MARGIN_PX
}

fn window_size(window: &Window) -> Vec2 {
    Vec2::new(window.width(), window.height())
}

fn update_selection_drag_box(
    window_q: Query<&Window, With<PrimaryWindow>>,
    drag_state: Res<SelectionDragState>,
    mut drag_box_q: Query<(&mut Visibility, &mut Node), With<SelectionDragBox>>,
) {
    let Ok((mut visibility, mut node)) = drag_box_q.single_mut() else {
        return;
    };
    let Some(rect) = active_selection_drag_box_rect(&window_q, &drag_state) else {
        *visibility = Visibility::Hidden;
        return;
    };

    node.left = px(rect.left);
    node.top = px(rect.top);
    node.width = px(rect.width);
    node.height = px(rect.height);
    *visibility = Visibility::Visible;
}

fn active_selection_drag_box_rect(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    drag_state: &SelectionDragState,
) -> Option<ScreenRect> {
    if !drag_state.active || !drag_state.dragging || drag_state.started_in_hud {
        return None;
    }
    let window = window_q.single().ok()?;
    selection_drag_box_rect(drag_state.start, window.cursor_position()?)
}

fn selection_drag_box_rect(start: Vec2, end: Vec2) -> Option<ScreenRect> {
    let min = start.min(end);
    let max = start.max(end);
    let width = max.x - min.x;
    let height = max.y - min.y;
    if width < 1.0 || height < 1.0 {
        return None;
    }
    Some(ScreenRect {
        left: min.x,
        top: min.y,
        width,
        height,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SingleClickSelectionAction {
    Select,
    ToggleDeselect,
}

fn single_click_selection_action(
    additive: bool,
    already_selected: bool,
) -> SingleClickSelectionAction {
    if additive && already_selected {
        SingleClickSelectionAction::ToggleDeselect
    } else {
        SingleClickSelectionAction::Select
    }
}

fn issue_orders(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut order_resources: OrderResources,
    mut selected_params: ParamSet<(
        Query<SelectedOrderUnitItem<'_>, SelectedOrderUnitFilter>,
        Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
    )>,
    selectable_q: Query<SelectableOrderTargetItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
    structure_targets: Query<(Entity, &Structure, &Team, Option<&UnderConstruction>), With<Health>>,
    garrison_targets: Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&Garrison>,
            Option<&UnderConstruction>,
        ),
        With<Structure>,
    >,
    resource_targets: Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    supply_crate_targets: Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &SupplyCrate,
    )>,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    if order_resources
        .command_mode
        .pending_structure_placement
        .is_some()
    {
        return;
    }
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor_blocks_world_order_controls(window, cursor) {
        return;
    }
    let Some(raw_point) = pointer_ground(window, &camera_q) else {
        return;
    };
    let Some(point) = validated_terrain_target_in_bounds(raw_point, *order_resources.map_bounds)
    else {
        return;
    };

    if order_resources.command_mode.support_power.is_some() {
        let power = order_resources.command_mode.support_power.unwrap();
        let support_targets = support_power_target_snapshots(&selectable_q);
        if activate_support_power(
            &mut commands,
            point,
            power,
            visible_team,
            visible_team,
            &economies,
            &mut order_resources.support_cooldowns,
            &mut order_resources.battle_log,
            &order_resources.relations,
            &structures,
            &support_targets,
        ) {
            record_support_power_audio_feedback(
                &mut order_resources.audio_feedback,
                visible_team,
                visible_team,
                power,
            );
        }
        order_resources.command_mode.support_power = None;
        return;
    }

    if order_resources.command_mode.rally_point {
        let rally_unit_target = rally_target_at(point, visible_team, &selectable_q);
        let set_any = apply_selected_rally_points(
            visible_team,
            point,
            rally_unit_target,
            *order_resources.map_bounds,
            &mut selected_params.p1(),
        );
        if set_any {
            commands.spawn((
                Transform::from_translation(point + Vec3::Y * 0.04),
                ClickMarker {
                    ttl: CLICK_MARKER_TTL_SECONDS,
                    radius: CLICK_MARKER_RADIUS_START,
                },
                MatchScopedEntity,
            ));
        }
        order_resources.command_mode.rally_point = false;
        return;
    }

    let queue_mode = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let force_move = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);

    let enemy_target =
        nearest_enemy_order_target(point, cursor, &camera_q, visible_team, &selectable_q);

    let selected_units = selected_params.p0();
    let selected: Vec<_> = selected_units
        .iter()
        .filter(|(_, _, _, team, ..)| **team == visible_team)
        .collect();
    let has_owned_voice_unit = selected.iter().any(|selection| is_voice_unit(selection.2));
    let has_selected_resource_collector = selected
        .iter()
        .any(|(_, _, unit, ..)| can_unit_collect_resources(unit));

    let garrison_target = nearest_garrison_target(
        point,
        visible_team,
        &order_resources.relations,
        &garrison_targets,
    );
    let resource_target = nearest_resource_order_target(
        point,
        cursor,
        &camera_q,
        &resource_targets,
        has_selected_resource_collector,
    );
    let resource_dropoff_target =
        nearest_resource_dropoff_order_target(point, visible_team, &selectable_q);
    let supply_crate_target = nearest_supply_crate_target(point, &supply_crate_targets);
    let construct_target = nearest_construct_order_target(point, visible_team, &selectable_q);
    let repair_target = nearest_repair_order_target(point, visible_team, &selectable_q);
    let follow_target = nearest_follow_order_target(
        point,
        visible_team,
        &order_resources.relations,
        &selectable_q,
    );
    let terrain_target_only = enemy_target.is_none()
        && garrison_target.is_none()
        && resource_target.is_none()
        && resource_dropoff_target.is_none()
        && supply_crate_target.is_none()
        && construct_target.is_none()
        && repair_target.is_none()
        && follow_target.is_none();
    if cursor_is_over_top_status_hud(cursor) && terrain_target_only {
        return;
    }

    let mut issued_any = false;
    let count = selected.len().max(1);
    for (i, (entity, transform, unit, _unit_team, orders, _cargo)) in
        selected.into_iter().enumerate()
    {
        let (
            move_order,
            follow_order,
            attack_order,
            capture_order,
            garrison_order,
            harvest_order,
            repair_order,
            construct_order,
            attack_move,
            patrol_order,
            queue,
        ) = orders;
        let offset = formation_offset(i, count);
        let Some(desired) = desired_order_for_selected_unit(
            unit,
            OrderTargetChoices {
                supply_crate_position: supply_crate_target
                    .map(|(_, target_position)| target_position),
                resource_target,
                resource_dropoff_target,
                enemy_target,
                repair_target,
                construct_target,
                garrison_target,
                follow_target: follow_target.filter(|target| *target != entity),
            },
            UnitOrderContext {
                force_move,
                enemy_target_capturable: enemy_target.is_some_and(|target| {
                    can_unit_capture_target(
                        unit,
                        target,
                        visible_team,
                        &order_resources.relations,
                        &structure_targets,
                    )
                }),
                attack_move: order_resources.command_mode.attack_move,
                patrol: order_resources.command_mode.patrol,
                origin: transform.translation,
                point,
                offset,
            },
        ) else {
            continue;
        };
        issued_any = true;
        let has_active = has_active_orders_in_query(
            move_order,
            follow_order,
            attack_order,
            capture_order,
            garrison_order,
            harvest_order,
            repair_order,
            construct_order,
            attack_move,
            patrol_order,
        );
        issue_or_queue_unit_order(
            &mut commands,
            entity,
            desired,
            queue_mode,
            false,
            has_active,
            queue,
        );
        commands
            .entity(entity)
            .try_insert(HoldPosition { enabled: false });
    }

    let should_set_plain_rally = should_set_terrain_rally_points(
        queue_mode,
        order_resources.command_mode.attack_move,
        order_resources.command_mode.patrol,
    );
    let set_rally_any = if should_set_plain_rally {
        if let Some(rally_unit_target) = rally_target_at(point, visible_team, &selectable_q) {
            apply_selected_rally_points(
                visible_team,
                point,
                Some(rally_unit_target),
                *order_resources.map_bounds,
                &mut selected_params.p1(),
            )
        } else if terrain_target_only {
            apply_selected_terrain_rally_points(
                visible_team,
                point,
                *order_resources.map_bounds,
                &mut selected_params.p1(),
            )
        } else {
            false
        }
    } else {
        false
    };

    order_resources.command_mode.attack_move = false;
    order_resources.command_mode.patrol = false;
    if issued_any {
        record_command_audio_feedback(
            &mut order_resources.audio_feedback,
            has_owned_voice_unit,
            None,
        );
    }
    if issued_any || set_rally_any {
        commands.spawn((
            Transform::from_translation(point + Vec3::Y * 0.03),
            ClickMarker {
                ttl: CLICK_MARKER_TTL_SECONDS,
                radius: CLICK_MARKER_RADIUS_START,
            },
            MatchScopedEntity,
        ));
    }
}

fn support_power_target_snapshots(
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Vec<SupportPowerTargetSnapshot> {
    selectable_q
        .iter()
        .filter_map(
            |(
                entity,
                transform,
                _selectable,
                target_team,
                _visibility,
                _resource_node,
                _supply_crate,
                health,
                unit,
                structure,
                _under_construction,
            )| {
                let health = health?;
                (unit.is_some() || structure.is_some()).then_some(SupportPowerTargetSnapshot {
                    entity,
                    team: *target_team,
                    position: transform.translation,
                    health: *health,
                    mobile: unit.is_some_and(|unit| unit.speed > 0.0),
                })
            },
        )
        .collect::<Vec<_>>()
}

fn should_set_terrain_rally_points(queue_mode: bool, attack_move: bool, patrol: bool) -> bool {
    !queue_mode && !attack_move && !patrol
}

fn apply_selected_terrain_rally_points(
    visible_team: Team,
    target: Vec3,
    bounds: MapBounds,
    rally_points: &mut Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
) -> bool {
    apply_selected_rally_points(visible_team, target, None, bounds, rally_points)
}

fn apply_selected_rally_points(
    visible_team: Team,
    target: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
    bounds: MapBounds,
    rally_points: &mut Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
) -> bool {
    let mut set_any = false;
    for (team, mut rally_point) in rally_points {
        if *team == visible_team
            && apply_rally_point_command_in_bounds(
                &mut rally_point,
                target,
                rally_unit_target,
                bounds,
            )
        {
            set_any = true;
        }
    }
    set_any
}

fn desired_order_for_selected_unit(
    unit: &Unit,
    choices: OrderTargetChoices,
    context: UnitOrderContext,
) -> Option<UnitQueuedOrder> {
    if context.force_move
        && let Some(target) = choices.force_follow_target()
    {
        return (unit.speed > 0.0).then_some(UnitQueuedOrder::ForceFollow {
            target,
            offset: context.offset,
        });
    }
    if let Some(target_position) = choices.supply_crate_position {
        return (unit.speed > 0.0).then_some(UnitQueuedOrder::Move(target_position));
    }
    if let Some(target) = choices.enemy_target {
        if context.enemy_target_capturable && can_unit_capture(unit) {
            return Some(UnitQueuedOrder::Capture(target));
        }
        return Some(UnitQueuedOrder::Attack(target));
    }
    if let Some(target) = choices.repair_target
        && repair_capability(unit).is_some()
    {
        return Some(UnitQueuedOrder::Repair(target));
    }
    if let Some(target) = choices.construct_target
        && can_unit_construct_structures(unit)
    {
        return Some(UnitQueuedOrder::Construct(target));
    }
    if let Some(target) = choices.resource_target
        && can_unit_collect_resources(unit)
    {
        return (unit.speed > 0.0).then_some(UnitQueuedOrder::Harvest {
            target,
            state: HarvestState::MovingToResource,
        });
    }
    if let Some(target) = choices.resource_dropoff_target
        && can_unit_collect_resources(unit)
    {
        return Some(UnitQueuedOrder::Harvest {
            target,
            state: HarvestState::MovingToDropoff,
        });
    }
    if let Some(target) = choices.garrison_target
        && can_unit_garrison(unit)
    {
        return Some(UnitQueuedOrder::Garrison(target));
    }
    if let Some(target) = choices.follow_target {
        return (unit.speed > 0.0).then_some(UnitQueuedOrder::Follow {
            target,
            offset: context.offset,
        });
    }
    if unit.speed <= 0.0 {
        return None;
    }
    let destination = context.point + context.offset;
    Some(if context.attack_move {
        UnitQueuedOrder::AttackMove(destination)
    } else if context.patrol {
        UnitQueuedOrder::Patrol {
            origin: context.origin,
            destination,
        }
    } else {
        UnitQueuedOrder::Move(destination)
    })
}

#[cfg(test)]
fn apply_rally_point_command(
    rally_point: &mut RallyPoint,
    point: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
) -> bool {
    apply_rally_point_command_in_bounds(rally_point, point, rally_unit_target, MapBounds::default())
}

fn apply_rally_point_command_in_bounds(
    rally_point: &mut RallyPoint,
    point: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
    bounds: MapBounds,
) -> bool {
    let target = if let Some((_, position)) = rally_unit_target {
        position
    } else {
        let Some(target) = validated_terrain_target_in_bounds(point, bounds) else {
            return false;
        };
        target
    };
    rally_point.target = Some(target);
    rally_point.target_unit = rally_unit_target.map(|(entity, _)| entity);
    true
}

fn update_rally_point_targets(
    mut rally_points: Query<&mut RallyPoint>,
    targets: Query<
        (&Transform, Option<&Health>, Option<&ResourceNode>),
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) {
    for mut rally_point in &mut rally_points {
        let target_state = rally_point.target_unit.and_then(|target| {
            targets
                .get(target)
                .ok()
                .map(|(transform, health, resource)| {
                    (
                        transform.translation,
                        health.is_none_or(|health| health.current > 0.0)
                            && resource.is_none_or(|resource| resource.amount > 0),
                    )
                })
        });
        refresh_rally_point_target(&mut rally_point, target_state);
    }
}

fn refresh_rally_point_target(
    rally_point: &mut RallyPoint,
    target_state: Option<(Vec3, bool)>,
) -> bool {
    if rally_point.target_unit.is_none() {
        return false;
    }
    if let Some((position, alive)) = target_state
        && alive
    {
        rally_point.target = Some(position);
        return true;
    }
    rally_point.target = None;
    rally_point.target_unit = None;
    true
}

fn can_unit_capture_target(
    unit: &Unit,
    target: Entity,
    team: Team,
    relations: &TeamRelations,
    structures: &Query<(Entity, &Structure, &Team, Option<&UnderConstruction>), With<Health>>,
) -> bool {
    if capture_time_for_unit(unit) <= 0.0 {
        return false;
    }
    let Ok((_entity, _structure, target_team, under_construction)) = structures.get(target) else {
        return false;
    };
    structure_is_constructed(under_construction)
        && can_capture_structure_team(team, *target_team, relations)
}

fn can_capture_structure_team(
    capturer_team: Team,
    target_team: Team,
    relations: &TeamRelations,
) -> bool {
    target_team == Team::Neutral || relations.are_enemies(capturer_team, target_team)
}

fn rally_target_at(
    point: Vec3,
    owner_team: Team,
    selectable_q: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &Team,
        &VisibilityState,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
        Option<&Health>,
        Option<&Unit>,
        Option<&Structure>,
        Option<&UnderConstruction>,
    )>,
) -> Option<(Entity, Vec3)> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        team,
        visibility,
        resource_node,
        supply_crate,
        health,
        unit,
        structure,
        _under_construction,
    ) in selectable_q
    {
        if !visibility.visible || supply_crate.is_some() {
            continue;
        }
        let alive = health.is_none_or(|health| health.current > 0.0)
            && resource_node.is_none_or(|resource| resource.amount > 0);
        if !alive {
            continue;
        }
        let targetable_resource = resource_node.is_some();
        let targetable_owned_unit = *team == owner_team && unit.is_some();
        let targetable_owned_structure = *team == owner_team && structure.is_some();
        if !targetable_resource && !targetable_owned_unit && !targetable_owned_structure {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < best_distance {
            best = Some((entity, transform.translation));
            best_distance = distance;
        }
    }
    best
}

fn nearest_repair_order_target(
    point: Vec3,
    team: Team,
    selectable_q: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &Team,
        &VisibilityState,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
        Option<&Health>,
        Option<&Unit>,
        Option<&Structure>,
        Option<&UnderConstruction>,
    )>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        target_team,
        visibility,
        resource_node,
        supply_crate,
        health,
        unit,
        structure,
        under_construction,
    ) in selectable_q
    {
        let Some(health) = health else {
            continue;
        };
        if !visibility.visible
            || *target_team != team
            || resource_node.is_some()
            || supply_crate.is_some()
            || !can_repair_order_target(unit, structure, under_construction, health)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < best_distance {
            best = Some(entity);
            best_distance = distance;
        }
    }
    best
}

fn nearest_construct_order_target(
    point: Vec3,
    team: Team,
    selectable_q: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &Team,
        &VisibilityState,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
        Option<&Health>,
        Option<&Unit>,
        Option<&Structure>,
        Option<&UnderConstruction>,
    )>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        target_team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        structure,
        under_construction,
    ) in selectable_q
    {
        if !visibility.visible
            || *target_team != team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
            || structure.is_none()
            || under_construction.is_none()
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < best_distance {
            best = Some(entity);
            best_distance = distance;
        }
    }
    best
}

fn nearest_follow_order_target(
    point: Vec3,
    team: Team,
    relations: &TeamRelations,
    selectable_q: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &Team,
        &VisibilityState,
        Option<&ResourceNode>,
        Option<&SupplyCrate>,
        Option<&Health>,
        Option<&Unit>,
        Option<&Structure>,
        Option<&UnderConstruction>,
    )>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        target_team,
        visibility,
        resource_node,
        supply_crate,
        health,
        unit,
        structure,
        _under_construction,
    ) in selectable_q
    {
        if !visibility.visible
            || resource_node.is_some()
            || supply_crate.is_some()
            || !relations.are_allied(team, *target_team)
            || (unit.is_none() && structure.is_none())
            || health.is_none_or(|health| health.current <= 0.0)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < best_distance {
            best = Some(entity);
            best_distance = distance;
        }
    }
    best
}

fn nearest_garrison_target(
    point: Vec3,
    team: Team,
    relations: &TeamRelations,
    structures: &Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&Garrison>,
            Option<&UnderConstruction>,
        ),
        With<Structure>,
    >,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (
        entity,
        structure,
        structure_team,
        transform,
        selectable,
        health,
        garrison,
        under_construction,
    ) in structures
    {
        let Some(garrison) = garrison else {
            continue;
        };
        if !can_garrison_structure_target(
            team,
            structure,
            *structure_team,
            health,
            garrison,
            under_construction,
            relations,
        ) {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

fn can_garrison_structure_target(
    unit_team: Team,
    structure: &Structure,
    structure_team: Team,
    health: &Health,
    garrison: &Garrison,
    under_construction: Option<&UnderConstruction>,
    relations: &TeamRelations,
) -> bool {
    structure.id == "TechBunker"
        && health.current > 0.0
        && structure_is_constructed(under_construction)
        && relations.are_allied(structure_team, unit_team)
        && garrison.count < garrison.capacity
}

fn nearest_enemy_order_target(
    point: Vec3,
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    visible_team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Option<Entity> {
    enemy_target_at_cursor(cursor, camera_q, visible_team, selectable_q)
        .or_else(|| nearest_enemy_target_with_snap_radius(point, visible_team, selectable_q, 0.45))
}

fn enemy_target_at_cursor(
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    visible_team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Option<Entity> {
    let (camera, camera_transform) = camera_q.single().ok()?;
    let mut nearest = None;
    let mut nearest_screen_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        _structure,
        _under_construction,
    ) in selectable_q
    {
        if !visibility.visible
            || *team == visible_team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
        {
            continue;
        }
        let Some((screen_distance, pick_radius)) = selectable_cursor_pick_distance(
            cursor,
            camera,
            camera_transform,
            transform,
            selectable,
            ENEMY_ORDER_SCREEN_PICK_MIN_RADIUS_PX,
            ENEMY_ORDER_SCREEN_PICK_MAX_RADIUS_PX,
        ) else {
            continue;
        };
        if screen_distance <= pick_radius && screen_distance < nearest_screen_distance {
            nearest = Some(entity);
            nearest_screen_distance = screen_distance;
        }
    }
    nearest
}

fn nearest_enemy_target_with_snap_radius(
    point: Vec3,
    visible_team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
    snap_radius: f32,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        _structure,
        _under_construction,
    ) in selectable_q
    {
        if !visibility.visible
            || *team == visible_team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + snap_radius && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

fn nearest_resource_order_target(
    point: Vec3,
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    resources: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    favor_resource_collectors: bool,
) -> Option<Entity> {
    let ground_snap_radius = if favor_resource_collectors {
        RESOURCE_ORDER_COLLECTOR_GROUND_SNAP_RADIUS_M
    } else {
        RESOURCE_ORDER_GROUND_SNAP_RADIUS_M
    };
    let screen_pick_max_radius = if favor_resource_collectors {
        RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX
    } else {
        RESOURCE_ORDER_SCREEN_PICK_MAX_RADIUS_PX
    };
    resource_target_at_cursor(cursor, camera_q, resources, screen_pick_max_radius)
        .or_else(|| nearest_resource_target_with_snap_radius(point, resources, ground_snap_radius))
}

fn resource_target_at_cursor(
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    resources: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    max_pick_radius: f32,
) -> Option<Entity> {
    let (camera, camera_transform) = camera_q.single().ok()?;
    let mut nearest = None;
    let mut nearest_screen_distance = f32::MAX;
    for (entity, transform, selectable, visibility, resource) in resources {
        if !visibility.visible || resource.amount <= 0 {
            continue;
        }
        let Some((screen_distance, pick_radius)) = selectable_cursor_pick_distance(
            cursor,
            camera,
            camera_transform,
            transform,
            selectable,
            RESOURCE_ORDER_SCREEN_PICK_MIN_RADIUS_PX,
            max_pick_radius,
        ) else {
            continue;
        };
        if screen_distance <= pick_radius && screen_distance < nearest_screen_distance {
            nearest = Some(entity);
            nearest_screen_distance = screen_distance;
        }
    }
    nearest
}

fn selectable_cursor_pick_distance(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    transform: &Transform,
    selectable: &Selectable,
    min_pick_radius: f32,
    max_pick_radius: f32,
) -> Option<(f32, f32)> {
    let base = transform.translation;
    let top = base + Vec3::Y * selectable.radius * 1.35;
    let base_screen = camera.world_to_viewport(camera_transform, base).ok()?;
    let top_screen = camera.world_to_viewport(camera_transform, top).ok();
    let screen_distance = top_screen.map_or(cursor.distance(base_screen), |top_screen| {
        cursor
            .distance(base_screen)
            .min(cursor.distance(top_screen))
    });
    let projected_radius = [
        Vec3::X * selectable.radius,
        Vec3::Z * selectable.radius,
        Vec3::new(selectable.radius, 0.0, selectable.radius),
    ]
    .into_iter()
    .filter_map(|offset| {
        camera
            .world_to_viewport(camera_transform, base + offset)
            .ok()
            .map(|edge| edge.distance(base_screen))
    })
    .fold(0.0, f32::max);
    let pick_radius = projected_radius.clamp(min_pick_radius, max_pick_radius.max(min_pick_radius));
    Some((screen_distance, pick_radius))
}

fn nearest_resource_target_with_snap_radius(
    point: Vec3,
    resources: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    snap_radius: f32,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, visibility, resource) in resources {
        if !visibility.visible || resource.amount <= 0 {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + snap_radius && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

fn nearest_resource_dropoff_order_target(
    point: Vec3,
    team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        target_team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        structure,
        under_construction,
    ) in selectable_q
    {
        let Some(structure) = structure else {
            continue;
        };
        if !visibility.visible
            || *target_team != team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
            || !structure_is_constructed(under_construction)
            || !is_resource_dropoff_structure(structure)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

fn nearest_supply_crate_target(
    point: Vec3,
    crates: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &SupplyCrate,
    )>,
) -> Option<(Entity, Vec3)> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, visibility, _crate) in crates {
        if !visibility.visible {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < nearest_distance {
            nearest = Some((entity, transform.translation));
            nearest_distance = distance;
        }
    }
    nearest
}

fn capture_time_for_unit(unit: &Unit) -> f32 {
    registry::entity(unit.id).map_or(0.0, |def| def.capture_time)
}

fn can_unit_capture(unit: &Unit) -> bool {
    capture_time_for_unit(unit) > 0.0
}

fn apply_infiltration_on_capture(
    capturer_def: &registry::EntityDef,
    target_def: &registry::EntityDef,
    capturer_team: Team,
    victim_team: Team,
    economies: &mut Economies,
) {
    if target_def.is_infiltration_resource_target {
        apply_resource_infiltration(capturer_def, capturer_team, victim_team, economies);
    }

    if target_def.is_infiltration_power_sabotage_target
        && capturer_def.infiltration_power_sabotage_duration > 0.0
        && victim_team.economy_index().is_some()
        && victim_team != capturer_team
    {
        let economy = economies.get_mut(victim_team);
        economy.power_sabotage_remaining = economy
            .power_sabotage_remaining
            .max(capturer_def.infiltration_power_sabotage_duration);
    }

    if let Some(producer_id) = target_def.infiltration_production_veterancy_producer
        && capturer_def.infiltration_production_veterancy_rank > 0
        && capturer_team.economy_index().is_some()
    {
        economies
            .get_mut(capturer_team)
            .grant_production_veterancy_rank(
                producer_id,
                capturer_def.infiltration_production_veterancy_rank,
            );
    }
}

fn apply_resource_infiltration(
    capturer_def: &registry::EntityDef,
    capturer_team: Team,
    victim_team: Team,
    economies: &mut Economies,
) {
    if capturer_def.infiltration_resource_steal_ratio <= 0.0
        || capturer_def.infiltration_resource_steal_cap <= 0
        || capturer_team.economy_index().is_none()
        || victim_team.economy_index().is_none()
        || victim_team == capturer_team
    {
        return;
    }

    let victim = economies.get(victim_team);
    let ore = infiltration_steal_amount(
        victim.ore,
        capturer_def.infiltration_resource_steal_ratio,
        capturer_def.infiltration_resource_steal_cap,
    );
    let crystal = infiltration_steal_amount(
        victim.crystal,
        capturer_def.infiltration_resource_steal_ratio,
        capturer_def.infiltration_resource_steal_cap,
    );
    if ore <= 0 && crystal <= 0 {
        return;
    }

    {
        let victim = economies.get_mut(victim_team);
        victim.ore -= ore;
        victim.crystal -= crystal;
    }
    let capturer = economies.get_mut(capturer_team);
    capturer.ore += ore;
    capturer.crystal += crystal;
}

fn infiltration_steal_amount(available: i32, ratio: f32, cap: i32) -> i32 {
    if available <= 0 || ratio <= 0.0 || cap <= 0 {
        return 0;
    }
    cap.min(available)
        .min(INFILTRATION_RESOURCE_STEAL_MIN.max(((available as f32) * ratio).ceil() as i32))
}

fn can_unit_garrison(unit: &Unit) -> bool {
    is_infantry_unit(unit)
}

fn can_unit_construct_structures(unit: &Unit) -> bool {
    matches!(unit.id, "Worker" | "MobileConstructionVehicle")
}

fn can_unit_collect_resources(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| def.resource_capacity > 0)
}

fn unit_has_movement_trait(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| def.speed > 0.0)
}

fn can_unit_guard_area(unit: &Unit) -> bool {
    unit_has_movement_trait(unit)
        && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
}

fn unit_supports_hold_position(unit: &Unit) -> bool {
    can_unit_guard_area(unit)
}

fn unit_supports_attack_move(unit: &Unit) -> bool {
    unit.speed > 0.0 && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
}

fn unit_supports_patrol(unit: &Unit) -> bool {
    unit.speed > 0.0
}

fn activate_support_power(
    mut commands: &mut Commands,
    target: Vec3,
    power: SupportPowerKind,
    team: Team,
    player_team: Team,
    economies: &Economies,
    support_cooldowns: &mut SupportCooldowns,
    battle_log: &mut BattleLog,
    relations: &TeamRelations,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &[SupportPowerTargetSnapshot],
) -> bool {
    let def = power.definition();
    if !support_cooldowns.ready(team, power) {
        return false;
    }
    if def.requires_power && economies.get(team).low_power() {
        return false;
    }
    if !support_requirements_met(team, def.requirements, structures) {
        return false;
    }

    match power {
        SupportPowerKind::RadarSweep => {
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.01),
                TemporarySupportReveal {
                    remaining: def.duration,
                    radius: def.radius,
                },
                team,
                VisibilityState { visible: true },
                VisionRadius(def.radius),
                MatchScopedEntity,
            ));
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                SupportWarning {
                    remaining: def.duration,
                    radius: def.radius,
                    color: Color::srgba(0.32, 0.88, 0.42, 0.45),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::OrbitalStrike => {
            let delay = def.impact_delay;
            let warning_color = Color::srgba(1.0, 0.72, 0.22, 0.55);
            if delay <= 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: 0.0,
                        radius: def.radius,
                        damage: def.damage,
                        impact_scale: def.damage_scale,
                        team,
                    },
                    SupportWarning {
                        remaining: 0.15,
                        radius: def.radius * 0.55,
                        color: warning_color,
                    },
                    MatchScopedEntity,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: delay,
                        radius: def.radius,
                        damage: def.damage,
                        impact_scale: def.damage_scale,
                        team,
                    },
                    SupportWarning {
                        remaining: delay,
                        radius: def.radius,
                        color: warning_color,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::EmpPulse => {
            for target_snapshot in targets {
                if target_snapshot.health.current <= 0.0 {
                    continue;
                }
                if !target_snapshot.mobile {
                    continue;
                }
                if relations.are_enemies(team, target_snapshot.team)
                    && xz_distance(target_snapshot.position, target) <= def.radius
                {
                    queue_apply_emp_disabled(&mut commands, target_snapshot.entity, def.duration);
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                SupportWarning {
                    remaining: 0.75,
                    radius: def.radius,
                    color: Color::srgba(0.8, 0.45, 1.0, 0.55),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::ChronoRelay => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                {
                    continue;
                }
                if !target_snapshot.mobile {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    queue_apply_chrono_relay(
                        &mut commands,
                        target_snapshot.entity,
                        def.duration,
                        def.damage_scale,
                    );
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.45),
                SupportWarning {
                    remaining: 0.2,
                    radius: def.radius,
                    color: Color::srgba(0.36, 0.93, 0.98, 0.45),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::ShieldOverdrive => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    queue_apply_support_shield(
                        &mut commands,
                        target_snapshot.entity,
                        def.duration,
                        def.damage_scale,
                    );
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.45),
                SupportWarning {
                    remaining: 0.2,
                    radius: def.radius,
                    color: Color::srgba(0.6, 0.85, 0.55, 0.42),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::NaniteRepairSwarm => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                    || target_snapshot.health.max <= 0.0
                {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    let healed_health = (target_snapshot.health.current + def.healing)
                        .min(target_snapshot.health.max);
                    if healed_health <= target_snapshot.health.current {
                        continue;
                    }
                    commands.entity(target_snapshot.entity).try_insert(Health {
                        current: healed_health,
                        max: target_snapshot.health.max,
                    });
                    commands.spawn((
                        ShotPulse {
                            from: target_snapshot.position + Vec3::new(0.0, 0.45, 0.0),
                            to: target_snapshot.position + Vec3::new(0.0, 0.12, 0.0),
                            ttl: 0.14,
                            team,
                        },
                        MatchScopedEntity,
                    ));
                }
            }
        }
        SupportPowerKind::WeatherStorm => {
            let secondary_radius = def.radius * 0.55;
            let impact_points = [
                (Vec3::ZERO, def.radius),
                (Vec3::new(1.6, 0.0, -1.2), secondary_radius),
                (Vec3::new(-1.4, 0.0, 1.1), secondary_radius),
            ];
            if def.impact_delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: def.impact_delay,
                        radius: def.radius,
                        color: Color::srgba(0.4, 0.9, 1.0, 0.5),
                    },
                    MatchScopedEntity,
                ));
            }
            for (idx, (offset, radius)) in impact_points.into_iter().enumerate() {
                commands.spawn((
                    Transform::from_translation(target + offset + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: def.impact_delay,
                        radius,
                        damage: if idx == 0 { def.damage } else { 0.0 },
                        impact_scale: def.damage_scale,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::StrategicMissile => {
            let secondary_radius = def.radius * 0.45;
            let impact_points = [
                (Vec3::ZERO, def.radius),
                (Vec3::new(0.9, 0.0, 0.9), secondary_radius),
                (Vec3::new(-0.8, 0.0, -0.7), secondary_radius),
            ];
            if def.impact_delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: def.impact_delay,
                        radius: def.radius,
                        color: Color::srgba(1.0, 0.16, 0.08, 0.58),
                    },
                    MatchScopedEntity,
                ));
                commands.spawn((
                    ShotPulse {
                        from: target + Vec3::new(0.0, 8.5, 0.0),
                        to: target + Vec3::new(0.0, 0.4, 0.0),
                        ttl: def.impact_delay,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
            for (idx, (offset, radius)) in impact_points.into_iter().enumerate() {
                commands.spawn((
                    Transform::from_translation(target + offset + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: def.impact_delay,
                        radius,
                        damage: if idx == 0 { def.damage } else { 0.0 },
                        impact_scale: def.damage_scale,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::Paradrop => {
            let delay = def.impact_delay;
            if delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: delay,
                        radius: def.radius,
                        color: Color::srgba(0.25, 0.8, 1.0, 0.48),
                    },
                    MatchScopedEntity,
                ));
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                PendingParadrop {
                    remaining: delay,
                    team,
                    target,
                    unit_paths: def.unit_paths,
                },
                MatchScopedEntity,
            ));
        }
    }
    let (message, ping_kind) = if team == player_team {
        (
            format!("支援已使用: {}", power.label()),
            BattleEventPingKind::SupportPower,
        )
    } else if matches!(
        power,
        SupportPowerKind::StrategicMissile | SupportPowerKind::WeatherStorm
    ) {
        (
            format!("敌方超级武器: {}", power.label()),
            BattleEventPingKind::EnemySuperweapon,
        )
    } else {
        (
            format!("敌方支援: {}", power.label()),
            BattleEventPingKind::EnemySupportPower,
        )
    };
    push_battle_log_with_kind(battle_log, message, Some(target), ping_kind);
    support_cooldowns.set(team, power, def.cooldown);
    true
}

fn match_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    command_mode: Res<CommandMode>,
    mut match_menu: ResMut<MatchMenuState>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if match_menu.visible {
        match_menu.visible = false;
    } else if !command_mode.has_pending_interaction() {
        match_menu.visible = true;
    }
}

fn match_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut match_menu: ResMut<MatchMenuState>,
    mut match_speed: ResMut<MatchSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut visible_player: ResMut<VisiblePlayer>,
    active_teams: Res<ActiveTeams>,
    selected_map: Res<SelectedSkirmishMap>,
    setup_settings: Res<MatchSetupSettings>,
    mut camera_state: ResMut<RtsCamera>,
    mut buttons: Query<(
        &Interaction,
        &MatchMenuButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (interaction, button, mut background, mut border) in &mut buttons {
        let enabled = match_menu_action_enabled(button.action, &visible_player, &active_teams);
        let clicked = match_menu.visible
            && enabled
            && *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match button.action {
                MatchMenuAction::Resume => {
                    match_menu.visible = false;
                }
                MatchMenuAction::SetSpeed(preset) => {
                    match_speed.preset = preset;
                    virtual_time.set_relative_speed(preset.scale());
                }
                MatchMenuAction::PreviousPerspective => {
                    if cycle_spectator_visible_player(&mut visible_player, &active_teams, -1) {
                        *camera_state = RtsCamera::focused_on(team_start_camera_focus_for_faction(
                            selected_map.definition(),
                            visible_player.team,
                            setup_settings.player_faction(visible_player.team),
                            setup_settings.startup_loadout,
                        ));
                    }
                }
                MatchMenuAction::NextPerspective => {
                    if cycle_spectator_visible_player(&mut visible_player, &active_teams, 1) {
                        *camera_state = RtsCamera::focused_on(team_start_camera_focus_for_faction(
                            selected_map.definition(),
                            visible_player.team,
                            setup_settings.player_faction(visible_player.team),
                            setup_settings.startup_loadout,
                        ));
                    }
                }
                MatchMenuAction::Restart => {
                    match_menu.visible = false;
                    next_state.set(AppScreen::RestartingMatch);
                }
                MatchMenuAction::ReturnToSetup => {
                    match_menu.visible = false;
                    next_state.set(AppScreen::MainMenu);
                }
            }
        }

        let selected = matches!(
            button.action,
            MatchMenuAction::SetSpeed(preset) if preset == match_speed.preset
        );
        let (bg, border_color) = match_menu_button_visual(*interaction, enabled, selected);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

fn match_menu_action_enabled(
    action: MatchMenuAction,
    visible_player: &VisiblePlayer,
    active_teams: &ActiveTeams,
) -> bool {
    match action {
        MatchMenuAction::PreviousPerspective | MatchMenuAction::NextPerspective => {
            spectator_perspective_switch_enabled(visible_player, active_teams)
        }
        MatchMenuAction::Resume
        | MatchMenuAction::SetSpeed(_)
        | MatchMenuAction::Restart
        | MatchMenuAction::ReturnToSetup => true,
    }
}

fn match_menu_button_visual(
    interaction: Interaction,
    enabled: bool,
    selected: bool,
) -> (Color, Color) {
    if !enabled {
        return (
            Color::srgba(0.035, 0.045, 0.055, 0.54),
            Color::srgb(0.18, 0.22, 0.26),
        );
    }
    if selected {
        return match interaction {
            Interaction::Pressed => (
                Color::srgba(0.18, 0.36, 0.34, 0.98),
                Color::srgb(0.72, 0.94, 0.82),
            ),
            Interaction::Hovered => (
                Color::srgba(0.12, 0.28, 0.27, 0.96),
                Color::srgb(0.56, 0.78, 0.7),
            ),
            Interaction::None => (
                Color::srgba(0.08, 0.22, 0.21, 0.94),
                Color::srgb(0.42, 0.62, 0.56),
            ),
        };
    }
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.16, 0.28, 0.36, 0.98),
            Color::srgb(0.66, 0.86, 0.96),
        ),
        Interaction::Hovered => (
            Color::srgba(0.1, 0.18, 0.24, 0.96),
            Color::srgb(0.46, 0.68, 0.78),
        ),
        Interaction::None => (
            Color::srgba(0.055, 0.072, 0.088, 0.94),
            Color::srgb(0.28, 0.36, 0.42),
        ),
    }
}

fn update_match_menu_overlay(
    match_menu: Res<MatchMenuState>,
    match_speed: Res<MatchSpeed>,
    selected_map: Res<SelectedSkirmishMap>,
    match_state: Res<MatchState>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    mut overlay_q: Query<&mut Visibility, With<MatchMenuOverlay>>,
    mut status_q: Query<&mut Text, With<MatchMenuStatusText>>,
) {
    for mut visibility in &mut overlay_q {
        *visibility = if match_menu.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if !match_menu.visible {
        return;
    }

    let minutes = (match_state.start_time_sec.max(0.0) / 60.0).floor() as u32;
    let seconds = (match_state.start_time_sec.max(0.0) as u32) % 60;
    let economy = economies.get(visible_player.team);
    let perspective_label = if visible_player.is_spectator() {
        "观战视角"
    } else {
        "阵营"
    };
    for mut text in &mut status_q {
        **text = format!(
            "地图: {}\n{}: {}  用时: {:02}:{:02}\n速度: {}\nOre {}  Crystal {}  电力 {}/{}",
            selected_map.definition().name,
            perspective_label,
            visible_player.team.label(),
            minutes,
            seconds,
            match_speed.preset.label(),
            economy.ore,
            economy.crystal,
            economy.power_capacity,
            economy.power_used
        );
    }
}

fn update_command_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    structures: Query<StructurePrereqItem<'_>>,
    mut command_mode: ResMut<CommandMode>,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        clear_targeting_modes(&mut command_mode);
        return;
    };
    if keyboard.just_pressed(KeyCode::KeyM) {
        toggle_attack_move_mode(&mut command_mode);
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        toggle_patrol_mode(&mut command_mode);
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        clear_targeting_modes(&mut command_mode);
    }

    if support_hotkey_modifier_pressed(&keyboard) {
        return;
    }
    for power in SupportPowerKind::ALL {
        if keyboard.just_pressed(power.hotkey())
            && player_support_power_available(
                visible_team,
                power,
                &economies,
                &support_cooldowns,
                &structures,
            )
        {
            toggle_support_power_mode(&mut command_mode, power);
            return;
        }
    }
}

fn support_hotkey_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::AltLeft)
        || keyboard.pressed(KeyCode::AltRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
}

fn player_support_power_available(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    support_cooldowns: &SupportCooldowns,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    support_cooldowns.ready(team, power)
        && (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

fn command_queue_controls(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut command_mode: ResMut<CommandMode>,
    mut audio_feedback: ResMut<AudioFeedback>,
    selected_units: Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    let selected: Vec<_> = selected_units.iter().collect();
    if selected.is_empty() {
        return;
    }
    let has_owned_voice_unit = selected
        .iter()
        .any(|(_, unit, team, ..)| **team == visible_team && is_voice_unit(unit));

    if keyboard.just_pressed(KeyCode::KeyS) {
        if stop_selected_entities(
            &mut commands,
            selected
                .iter()
                .filter_map(|(entity, _, team, _, _, orders)| {
                    (**team == visible_team && has_active_order_state(*orders)).then_some(*entity)
                }),
        ) {
            clear_targeting_modes(&mut command_mode);
            record_command_audio_feedback(
                &mut audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_CANCEL),
            );
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        if toggle_selected_hold_position(
            &mut commands,
            visible_team,
            selected
                .iter()
                .map(|(entity, unit, team, _, hold, ..)| (*entity, *unit, *team, *hold)),
        ) {
            clear_targeting_modes(&mut command_mode);
            record_command_audio_feedback(
                &mut audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_HOLD_POSITION),
            );
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyG) {
        if guard_selected_area(
            &mut commands,
            visible_team,
            selected
                .iter()
                .map(|(entity, unit, team, ..)| (*entity, *unit, *team)),
        ) {
            clear_targeting_modes(&mut command_mode);
            record_command_audio_feedback(
                &mut audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_GUARD_AREA),
            );
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyX) {
        let scatter_units = selected
            .iter()
            .filter(|(_, unit, team, ..)| **team == visible_team && unit.speed > 0.0)
            .map(|(entity, _, _, transform, ..)| (*entity, transform.translation))
            .collect::<Vec<_>>();
        if scatter_selected_positions(&mut commands, &scatter_units) {
            clear_targeting_modes(&mut command_mode);
            record_command_audio_feedback(
                &mut audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_SCATTER),
            );
        }
    }
}

fn clear_targeting_modes(command_mode: &mut CommandMode) {
    command_mode.attack_move = false;
    command_mode.patrol = false;
    command_mode.rally_point = false;
    command_mode.support_power = None;
    command_mode.pending_structure_placement = None;
}

fn begin_attack_move_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.attack_move = true;
    true
}

fn toggle_attack_move_mode(command_mode: &mut CommandMode) -> bool {
    if command_mode.attack_move {
        clear_targeting_modes(command_mode);
        return false;
    }
    begin_attack_move_mode(command_mode, true)
}

fn begin_patrol_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.patrol = true;
    true
}

fn toggle_patrol_mode(command_mode: &mut CommandMode) -> bool {
    if command_mode.patrol {
        clear_targeting_modes(command_mode);
        return false;
    }
    begin_patrol_mode(command_mode, true)
}

fn begin_rally_point_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.rally_point = true;
    true
}

fn toggle_support_power_mode(command_mode: &mut CommandMode, power: SupportPowerKind) -> bool {
    let enabled = command_mode.support_power != Some(power);
    clear_targeting_modes(command_mode);
    if enabled {
        command_mode.support_power = Some(power);
    }
    enabled
}

#[allow(dead_code)]
fn begin_structure_placement_mode(
    team: Team,
    id: &'static str,
    command_mode: &mut CommandMode,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    begin_structure_placement_mode_for_faction(
        team,
        SkirmishFaction::from_team(team),
        id,
        command_mode,
        structures,
    )
}

fn begin_structure_placement_mode_for_faction(
    team: Team,
    faction: SkirmishFaction,
    id: &'static str,
    command_mode: &mut CommandMode,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let Some(faction) = faction_def(faction) else {
        return false;
    };
    let Some(def) = registry::entity(id) else {
        return false;
    };
    if !faction.can_construct(id) || !requirements_met(def, team, structures) {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.pending_structure_placement = Some(PendingStructurePlacement::new(id));
    true
}

fn toggle_selected_hold_position<'a>(
    commands: &mut Commands,
    team: Team,
    selected_units: impl IntoIterator<Item = (Entity, &'a Unit, &'a Team, &'a HoldPosition)>,
) -> bool {
    let hold_units = selected_units
        .into_iter()
        .filter(|(_, unit, unit_team, _)| **unit_team == team && unit_supports_hold_position(unit))
        .collect::<Vec<_>>();
    if hold_units.is_empty() {
        return false;
    }
    let all_holding = hold_units.iter().all(|(_, _, _, hold)| hold.enabled);
    let new_state = !all_holding;
    for (entity, _, _, _) in hold_units {
        commands
            .entity(entity)
            .try_insert(HoldPosition { enabled: new_state });
        if new_state {
            clear_order_state(commands, entity);
            commands.entity(entity).try_remove::<OrderQueue>();
        }
    }
    true
}

fn guard_selected_area<'a>(
    commands: &mut Commands,
    team: Team,
    selected_units: impl IntoIterator<Item = (Entity, &'a Unit, &'a Team)>,
) -> bool {
    let mut guarded_any = false;
    for (entity, unit, unit_team) in selected_units {
        if *unit_team != team || !can_unit_guard_area(unit) {
            continue;
        }
        clear_order_state(commands, entity);
        commands
            .entity(entity)
            .try_remove::<OrderQueue>()
            .try_insert(HoldPosition { enabled: false });
        guarded_any = true;
    }
    guarded_any
}

fn stop_selected_units(
    commands: &mut Commands,
    team: Team,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) -> bool {
    stop_selected_entities(
        commands,
        selected_units
            .iter()
            .filter_map(|(entity, _, unit_team, _, _, orders)| {
                (*unit_team == team && has_active_order_state(orders)).then_some(entity)
            }),
    )
}

fn selected_under_construction_stop_target<'a>(
    team: Team,
    selected_team_unit_count: usize,
    selected_structures: impl IntoIterator<
        Item = (Entity, &'a Team, &'a Health, Option<&'a UnderConstruction>),
    >,
) -> Option<(Entity, registry::Cost)> {
    if selected_team_unit_count > 0 {
        return None;
    }
    let mut selected_structure_count = 0usize;
    let mut target = None;
    for (entity, structure_team, health, under_construction) in selected_structures {
        if *structure_team != team || health.current <= 0.0 {
            continue;
        }
        selected_structure_count += 1;
        if let Some(under_construction) = under_construction {
            target = Some((entity, under_construction.cost));
        }
    }
    (selected_structure_count == 1).then_some(target).flatten()
}

fn cancel_selected_under_construction_structure<'a>(
    commands: &mut Commands,
    team: Team,
    selected_team_unit_count: usize,
    selected_structures: impl IntoIterator<
        Item = (Entity, &'a Team, &'a Health, Option<&'a UnderConstruction>),
    >,
    economies: &mut Economies,
    build_queue: &mut BuildQueue,
) -> bool {
    let Some((entity, cost)) = selected_under_construction_stop_target(
        team,
        selected_team_unit_count,
        selected_structures,
    ) else {
        return false;
    };
    let refund = construction_cancel_refund(cost);
    {
        let economy = economies.get_mut(team);
        economy.ore += refund.0;
        economy.crystal += refund.1;
    }
    cancel_jobs_for_producer(build_queue, economies, entity);
    commands.entity(entity).try_despawn();
    true
}

fn stop_selected_entities(
    commands: &mut Commands,
    entities: impl IntoIterator<Item = Entity>,
) -> bool {
    let mut stopped_any = false;
    for entity in entities {
        clear_order_state(commands, entity);
        commands.entity(entity).try_remove::<OrderQueue>();
        stopped_any = true;
    }
    stopped_any
}

fn scatter_selected_units(
    commands: &mut Commands,
    team: Team,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) -> bool {
    let units = selected_units
        .iter()
        .filter(|(_, unit, unit_team, ..)| **unit_team == team && unit_supports_patrol(unit))
        .map(|(entity, _, _, transform, ..)| (entity, transform.translation))
        .collect::<Vec<_>>();
    scatter_selected_positions(commands, &units)
}

fn scatter_selected_positions(commands: &mut Commands, units: &[(Entity, Vec3)]) -> bool {
    if units.is_empty() {
        return false;
    }
    let positions = units
        .iter()
        .map(|(_, position)| *position)
        .collect::<Vec<_>>();
    let targets = scatter_target_positions(&positions);
    for ((entity, _), target) in units.iter().zip(targets) {
        clear_order_state(commands, *entity);
        commands
            .entity(*entity)
            .try_remove::<OrderQueue>()
            .try_insert(HoldPosition { enabled: false })
            .try_insert(MoveOrder { target });
    }
    true
}

fn scatter_target_positions(positions: &[Vec3]) -> Vec<Vec3> {
    if positions.is_empty() {
        return Vec::new();
    }
    let selected_len = positions.len() as f32;
    let pivot = positions.iter().copied().sum::<Vec3>() / selected_len;
    positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let direction = *position - pivot;
            let direction = if direction.length_squared() > 0.0001 {
                direction.normalize()
            } else {
                let angle = index as f32 / selected_len * core::f32::consts::TAU;
                Vec3::new(angle.cos(), 0.0, angle.sin())
            };
            *position + direction * SCATTER_DISTANCE
        })
        .collect()
}

fn progress_queued_orders(
    mut commands: Commands,
    mut units: Query<
        (
            Entity,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
            &mut OrderQueue,
        ),
        With<Unit>,
    >,
) {
    for (
        entity,
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
        mut queue,
    ) in &mut units
    {
        if move_order.is_some()
            || follow_order.is_some()
            || attack_order.is_some()
            || capture_order.is_some()
            || garrison_order.is_some()
            || harvest_order.is_some()
            || repair_order.is_some()
            || construct_order.is_some()
            || attack_move_order.is_some()
            || patrol_order.is_some()
        {
            continue;
        }
        if let Some(order) = queue.orders.pop_front() {
            issue_unit_order(&mut commands, entity, order);
        }
        if queue.orders.is_empty() {
            commands.entity(entity).try_remove::<OrderQueue>();
        }
    }
}

fn clear_emp_disabled_orders(
    mut commands: Commands,
    units: Query<
        Entity,
        (
            With<EmpDisabled>,
            Or<(
                With<MoveOrder>,
                With<FollowOrder>,
                With<AttackOrder>,
                With<CaptureOrder>,
                With<GarrisonOrder>,
                With<HarvestOrder>,
                With<RepairOrder>,
                With<ConstructOrder>,
                With<AttackMoveOrder>,
                With<PatrolOrder>,
                With<OrderQueue>,
            )>,
        ),
    >,
) {
    for entity in &units {
        clear_order_state(&mut commands, entity);
        commands.entity(entity).try_remove::<OrderQueue>();
    }
}

fn issue_unit_order(commands: &mut Commands, entity: Entity, order: UnitQueuedOrder) {
    clear_order_state(commands, entity);

    match order {
        UnitQueuedOrder::Move(target) => {
            commands.entity(entity).try_insert(MoveOrder { target });
        }
        UnitQueuedOrder::Attack(target) => {
            commands.entity(entity).try_insert(AttackOrder { target });
        }
        UnitQueuedOrder::Capture(target) => {
            commands.entity(entity).try_insert(CaptureOrder {
                target,
                elapsed: 0.0,
            });
        }
        UnitQueuedOrder::Garrison(target) => {
            commands.entity(entity).try_insert(GarrisonOrder { target });
        }
        UnitQueuedOrder::Harvest { target, state } => {
            commands.entity(entity).try_insert(HarvestOrder {
                resource: Some(target),
                state,
                collect_remaining: 0.0,
            });
        }
        UnitQueuedOrder::Repair(target) => {
            commands.entity(entity).try_insert(RepairOrder { target });
        }
        UnitQueuedOrder::Construct(target) => {
            commands
                .entity(entity)
                .try_insert(ConstructOrder { target });
        }
        UnitQueuedOrder::Follow { target, offset } => {
            commands.entity(entity).try_insert(FollowOrder {
                target,
                allow_enemy: false,
                offset,
            });
        }
        UnitQueuedOrder::AttackMove(destination) => {
            commands
                .entity(entity)
                .try_insert(AttackMoveOrder { destination });
        }
        UnitQueuedOrder::Patrol {
            origin,
            destination,
        } => {
            commands.entity(entity).try_insert(PatrolOrder {
                origin,
                destination,
                moving_to_destination: true,
            });
        }
        UnitQueuedOrder::ForceFollow { target, offset } => {
            commands.entity(entity).try_insert(FollowOrder {
                target,
                allow_enemy: true,
                offset,
            });
        }
    }
}

fn issue_or_queue_unit_order(
    commands: &mut Commands,
    entity: Entity,
    order: UnitQueuedOrder,
    queue_mode: bool,
    allow_queue: bool,
    has_active: bool,
    queue: Option<&OrderQueue>,
) {
    if should_queue_selected_order(queue_mode, allow_queue, has_active, queue) {
        let mut queued = VecDeque::from(
            queue
                .map(|order_queue| order_queue.orders.clone())
                .unwrap_or_default(),
        );
        queued.push_back(order);
        commands
            .entity(entity)
            .try_insert(OrderQueue { orders: queued });
    } else {
        issue_unit_order(commands, entity, order);
        commands.entity(entity).try_remove::<OrderQueue>();
    }
}

fn should_queue_selected_order(
    queue_mode: bool,
    allow_queue: bool,
    has_active: bool,
    queue: Option<&OrderQueue>,
) -> bool {
    allow_queue
        && queue_mode
        && (has_active || queue.is_some_and(|order_queue| !order_queue.orders.is_empty()))
}

fn clear_order_state(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .try_remove::<MoveOrder>()
        .try_remove::<FollowOrder>()
        .try_remove::<AttackOrder>()
        .try_remove::<CaptureOrder>()
        .try_remove::<GarrisonOrder>()
        .try_remove::<HarvestOrder>()
        .try_remove::<RepairOrder>()
        .try_remove::<ConstructOrder>()
        .try_remove::<AttackMoveOrder>()
        .try_remove::<PatrolOrder>();
}

fn clear_non_attack_order_state(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .try_remove::<MoveOrder>()
        .try_remove::<FollowOrder>()
        .try_remove::<CaptureOrder>()
        .try_remove::<GarrisonOrder>()
        .try_remove::<HarvestOrder>()
        .try_remove::<RepairOrder>()
        .try_remove::<ConstructOrder>()
        .try_remove::<AttackMoveOrder>()
        .try_remove::<PatrolOrder>();
}

fn has_active_orders_in_query(
    move_order: Option<&MoveOrder>,
    follow_order: Option<&FollowOrder>,
    attack_order: Option<&AttackOrder>,
    capture_order: Option<&CaptureOrder>,
    garrison_order: Option<&GarrisonOrder>,
    harvest_order: Option<&HarvestOrder>,
    repair_order: Option<&RepairOrder>,
    construct_order: Option<&ConstructOrder>,
    attack_move_order: Option<&AttackMoveOrder>,
    patrol_order: Option<&PatrolOrder>,
) -> bool {
    move_order.is_some()
        || follow_order.is_some()
        || attack_order.is_some()
        || capture_order.is_some()
        || garrison_order.is_some()
        || harvest_order.is_some()
        || repair_order.is_some()
        || construct_order.is_some()
        || attack_move_order.is_some()
        || patrol_order.is_some()
}

fn has_active_orders_or_queue(
    move_order: Option<&MoveOrder>,
    follow_order: Option<&FollowOrder>,
    attack_order: Option<&AttackOrder>,
    capture_order: Option<&CaptureOrder>,
    garrison_order: Option<&GarrisonOrder>,
    harvest_order: Option<&HarvestOrder>,
    repair_order: Option<&RepairOrder>,
    construct_order: Option<&ConstructOrder>,
    attack_move_order: Option<&AttackMoveOrder>,
    patrol_order: Option<&PatrolOrder>,
    queue: Option<&OrderQueue>,
) -> bool {
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
    ) || queue.is_some_and(|queue| !queue.orders.is_empty())
}

fn has_active_order_state(order_state: CommandOrderStateItem<'_>) -> bool {
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
        queue,
    ) = order_state;
    has_active_orders_or_queue(
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
        queue,
    )
}

fn refresh_command_panel(
    build_queue: Res<BuildQueue>,
    visible_player: Res<VisiblePlayer>,
    player_factions: Res<PlayerFactions>,
    selected_units: Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: Query<StructureEntityItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
    mut slot_q: Query<(
        &CommandSlot,
        &mut BuildAction,
        &mut CommandSlotAvailability,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut label_q: Query<(&CommandSlotLabel, &mut Text, &mut TextColor)>,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        for (slot, mut action, mut availability, interaction, mut background, mut border) in
            &mut slot_q
        {
            let _ = slot;
            *action = BuildAction::None;
            availability.enabled = false;
            let (bg, border_color) = command_button_colors(BuildAction::None, false, *interaction);
            *background = BackgroundColor(bg);
            *border = BorderColor::all(border_color);
        }
        for (_, mut text, mut text_color) in &mut label_q {
            **text = String::new();
            *text_color = command_button_text_color(BuildAction::None, false);
        }
        return;
    };
    let faction = player_factions.slot_faction(visible_team);
    let actions = current_command_actions_for_faction(
        visible_team,
        faction,
        &selected_units,
        &selected_structures,
        &structures,
    );
    for (slot, mut action, mut availability, interaction, mut background, mut border) in &mut slot_q
    {
        let next_action = actions.get(slot.0).copied().unwrap_or(BuildAction::None);
        let enabled = command_action_enabled_for_panel(
            visible_team,
            faction,
            next_action,
            &selected_units,
            &selected_structures,
            &producer_structures,
            &structures,
            &build_queue,
        );
        *action = next_action;
        availability.enabled = enabled;
        let (bg, border_color) = command_button_colors(next_action, enabled, *interaction);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
    for (slot, mut text, mut text_color) in &mut label_q {
        let action = actions.get(slot.0).copied();
        let enabled = action.is_some_and(|action| {
            command_action_enabled_for_panel(
                visible_team,
                faction,
                action,
                &selected_units,
                &selected_structures,
                &producer_structures,
                &structures,
                &build_queue,
            )
        });
        let queue_state = action.and_then(|action| {
            command_queue_button_state_for_action(
                visible_team,
                faction,
                action,
                &selected_structures,
                &producer_structures,
                &build_queue,
            )
        });
        **text = command_label_with_queue(slot.0, action, queue_state);
        *text_color = command_button_text_color(action.unwrap_or(BuildAction::None), enabled);
    }
}

#[allow(dead_code)]
fn current_command_actions(
    team: Team,
    selected_units: &Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Vec<BuildAction> {
    current_command_actions_for_faction(
        team,
        SkirmishFaction::from_team(team),
        selected_units,
        selected_structures,
        structures,
    )
}

fn current_command_actions_for_faction(
    team: Team,
    faction: SkirmishFaction,
    selected_units: &Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    _structures: &Query<StructurePrereqItem<'_>>,
) -> Vec<BuildAction> {
    let Some(faction) = faction_def(faction) else {
        return Vec::new();
    };
    let selected_team_unit_count = selected_units
        .iter()
        .filter(|(_, unit_team, ..)| **unit_team == team)
        .count();
    let selected_builder_unit_count = selected_units
        .iter()
        .filter(|(unit, unit_team, ..)| **unit_team == team && can_unit_construct_structures(unit))
        .count();
    let selected_team_structures = selected_structures
        .iter()
        .filter(|(_, _, structure_team, health, _, _)| {
            **structure_team == team && health.current > 0.0
        })
        .map(|(_, structure, _, _, _, under_construction)| (structure.id, under_construction))
        .collect::<Vec<_>>();
    let has_selected_team_structure = !selected_team_structures.is_empty();
    let has_single_selected_under_construction_structure = selected_team_unit_count == 0
        && selected_team_structures.len() == 1
        && selected_team_structures[0].1.is_some();
    let selected_production_structure = if selected_team_unit_count == 0
        && !selected_team_structures.is_empty()
        && selected_team_structures
            .iter()
            .all(|(_, under_construction)| structure_is_constructed(*under_construction))
    {
        let candidate = selected_team_structures[0].0;
        selected_team_structures
            .iter()
            .all(|(id, _)| *id == candidate)
            .then_some(candidate)
            .filter(|id| faction.production_for(id).is_some())
    } else {
        None
    };
    let show_worker_construction_menu = selected_team_unit_count == 1
        && selected_builder_unit_count == 1
        && selected_team_structures.is_empty();
    let has_repairable_structure = selected_structures.iter().any(
        |(_, _, structure_team, health, repair, under_construction)| {
            *structure_team == team
                && repair.is_none()
                && structure_is_constructed(under_construction)
                && health.current > 0.0
                && health.current < health.max
        },
    );
    let mut actions = Vec::new();

    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit_supports_hold_position(unit))
    {
        push_action_unique(&mut actions, BuildAction::HoldPosition);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit_supports_attack_move(unit))
    {
        push_action_unique(&mut actions, BuildAction::AttackMove);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit_supports_patrol(unit))
    {
        push_action_unique(&mut actions, BuildAction::Patrol);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit.id == "MobileConstructionVehicle")
    {
        push_action_unique(&mut actions, BuildAction::DeployMcv);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit.id == "SiegeDrillTank")
    {
        push_action_unique(&mut actions, BuildAction::ToggleDeployMode);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && can_unit_guard_area(unit))
    {
        push_action_unique(&mut actions, BuildAction::GuardArea);
    }
    if selected_units
        .iter()
        .any(|(unit, unit_team, ..)| *unit_team == team && unit_supports_patrol(unit))
    {
        push_action_unique(&mut actions, BuildAction::ScatterSelected);
    }
    if selected_team_unit_count > 0 || has_single_selected_under_construction_structure {
        push_action_unique(&mut actions, BuildAction::StopSelected);
    }

    if let Some(producer_id) = selected_production_structure {
        if let Some(products) = faction.production_for(producer_id) {
            for product in products {
                if registry::entity(product).is_some() {
                    push_action_unique(&mut actions, BuildAction::Train(product));
                }
            }
        }
        push_action_unique(&mut actions, BuildAction::SetRallyPoint);
        push_action_unique(&mut actions, BuildAction::SellStructure);
        if has_repairable_structure {
            push_action_unique(&mut actions, BuildAction::RepairStructure);
        }
    } else {
        if has_selected_team_structure {
            push_action_unique(&mut actions, BuildAction::SellStructure);
        }
        if show_worker_construction_menu {
            for structure in faction.structures {
                if registry::entity(structure).is_some() {
                    push_action_unique(&mut actions, BuildAction::Build(structure));
                }
            }
        }
    }
    actions.truncate(COMMAND_SLOT_COUNT);
    actions
}

fn command_action_enabled_for_panel(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_units: &Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    build_queue: &BuildQueue,
) -> bool {
    match action {
        BuildAction::None => false,
        BuildAction::Train(product_id) => {
            let Some(def) = registry::entity(product_id) else {
                return false;
            };
            if !requirements_met(def, team, structures) {
                return false;
            }
            command_queue_producers_for_action(
                team,
                faction,
                action,
                selected_structures,
                producer_structures,
            )
            .iter()
            .any(|producer| build_queue_has_capacity(build_queue, *producer))
        }
        BuildAction::Build(id) => {
            let Some(def) = registry::entity(id) else {
                return false;
            };
            faction_def(faction).is_some_and(|faction| faction.can_construct(id))
                && requirements_met(def, team, structures)
        }
        BuildAction::StopSelected => {
            selected_units.iter().any(
                |(
                    _unit,
                    unit_team,
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
                    queue,
                )| {
                    *unit_team == team
                        && has_active_orders_or_queue(
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
                            queue,
                        )
                },
            ) || selected_under_construction_stop_target(
                team,
                selected_units
                    .iter()
                    .filter(|(_, unit_team, ..)| **unit_team == team)
                    .count(),
                selected_structures.iter().map(
                    |(entity, _, structure_team, health, _, under_construction)| {
                        (entity, structure_team, health, under_construction)
                    },
                ),
            )
            .is_some()
        }
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::ScatterSelected => true,
    }
}

fn command_button_colors(
    action: BuildAction,
    enabled: bool,
    interaction: Interaction,
) -> (Color, Color) {
    if action == BuildAction::None {
        return (
            Color::srgba(0.035, 0.045, 0.055, 0.54),
            Color::srgb(0.18, 0.22, 0.26),
        );
    }
    if !enabled {
        return (
            Color::srgba(0.04, 0.048, 0.055, 0.66),
            Color::srgb(0.18, 0.21, 0.24),
        );
    }
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.18, 0.3, 0.42, 0.96),
            Color::srgb(0.46, 0.58, 0.66),
        ),
        Interaction::Hovered => (
            Color::srgba(0.11, 0.15, 0.19, 0.94),
            Color::srgb(0.36, 0.46, 0.52),
        ),
        Interaction::None => (
            Color::srgba(0.06, 0.08, 0.1, 0.88),
            Color::srgb(0.28, 0.34, 0.39),
        ),
    }
}

fn command_button_text_color(action: BuildAction, enabled: bool) -> TextColor {
    if action == BuildAction::None {
        TextColor(Color::srgba(0.48, 0.54, 0.58, 0.55))
    } else if enabled {
        TextColor(Color::srgb(0.9, 0.94, 0.96))
    } else {
        TextColor(Color::srgba(0.62, 0.68, 0.72, 0.68))
    }
}

fn push_action_unique(actions: &mut Vec<BuildAction>, action: BuildAction) {
    if !actions.contains(&action) {
        actions.push(action);
    }
}

#[cfg(test)]
fn command_label(index: usize, action: Option<BuildAction>) -> String {
    command_label_with_queue(index, action, None)
}

fn command_grid_hotkey(index: usize) -> Option<CommandHotkey> {
    COMMAND_SLOT_HOTKEYS.get(index).copied()
}

fn command_action_hotkey(index: usize, action: BuildAction) -> Option<CommandHotkey> {
    match action {
        BuildAction::None => None,
        BuildAction::GuardArea => Some(CommandHotkey::new("G", KeyCode::KeyG)),
        BuildAction::StopSelected => Some(CommandHotkey::new("S", KeyCode::KeyS)),
        BuildAction::ScatterSelected => Some(CommandHotkey::new("X", KeyCode::KeyX)),
        _ => command_grid_hotkey(index),
    }
}

fn command_action_display_key(index: usize, action: BuildAction) -> &'static str {
    command_action_hotkey(index, action)
        .map(|hotkey| hotkey.display)
        .unwrap_or(" ")
}

fn command_label_with_queue(
    index: usize,
    action: Option<BuildAction>,
    queue_state: Option<QueueButtonState>,
) -> String {
    let Some(action) = action else {
        return String::new();
    };
    let key = command_action_display_key(index, action);
    match action {
        BuildAction::Train(id) | BuildAction::Build(id) => {
            let Some(def) = registry::entity(id) else {
                return String::new();
            };
            let cost = def.cost;
            let prefix = if matches!(action, BuildAction::Build(_)) {
                "B"
            } else {
                "T"
            };
            let queue_badge = queue_state
                .filter(|state| state.count > 0 || state.full)
                .map(queue_button_badge_text)
                .unwrap_or_default();
            format!(
                "{key} {prefix} {} {}/{}{queue_badge}",
                compact_label(def.label),
                cost.ore,
                cost.crystal
            )
        }
        BuildAction::SellStructure => format!("{key} 出售建筑"),
        BuildAction::RepairStructure => format!("{key} 维修建筑"),
        BuildAction::DeployMcv => format!("{key} 展开基地"),
        BuildAction::ToggleDeployMode => format!("{key} 切换部署"),
        BuildAction::SetRallyPoint => format!("{key} 设置集结"),
        BuildAction::HoldPosition => format!("{key} 坚守"),
        BuildAction::AttackMove => format!("{key} 攻击移动"),
        BuildAction::Patrol => format!("{key} 巡逻"),
        BuildAction::GuardArea => format!("{key} 守卫区域"),
        BuildAction::StopSelected => format!("{key} 停止"),
        BuildAction::ScatterSelected => format!("{key} 散开"),
        BuildAction::None => String::new(),
    }
}

fn compact_label(label: &str) -> String {
    let mut value = label.to_string();
    for (from, to) in [
        ("Command Center", "Cmd Center"),
        ("Factory", "Fact."),
        ("Defense", "Def."),
        ("Infantry", "Inf."),
        ("Trooper", "Trp."),
        ("Vehicle", "Veh."),
    ] {
        value = value.replace(from, to);
    }
    if value.chars().count() > 18 {
        value.chars().take(17).collect::<String>() + "."
    } else {
        value
    }
}

fn selection_hotkeys(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    map_bounds: Res<MapBounds>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    selected_q: Query<(Entity, &Team, Option<&Structure>), With<Selected>>,
    selectable_q: Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    army_selectable_q: Query<
        (Entity, &Transform, &Team, &Unit, &VisibilityState),
        With<Selectable>,
    >,
    production_structure_q: Query<ProductionHotkeyStructureItem<'_>, With<Selectable>>,
    unit_q: Query<
        (
            Entity,
            &Team,
            &Unit,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
            &VisibilityState,
        ),
        With<Unit>,
    >,
    mut unit_groups: ResMut<UnitGroups>,
    mut bookmarks: ResMut<CameraBookmarks>,
    mut camera_state: ResMut<RtsCamera>,
    mut battle_log: ResMut<BattleLog>,
) {
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let alt = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);
    handle_camera_bookmark_hotkeys(
        &keyboard,
        &mut bookmarks,
        &mut camera_state,
        *map_bounds,
        alt,
        ctrl,
    );
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };

    let selected_visible: Vec<Entity> = selected_q
        .iter()
        .filter_map(|(entity, team, _structure)| (*team == visible_team).then_some(entity))
        .collect();

    if !alt {
        for (index, key) in GROUP_SLOT_KEYS.iter().enumerate() {
            if !keyboard.just_pressed(*key) {
                continue;
            }
            if ctrl {
                if shift {
                    let mut target = valid_group_entities(
                        &selectable_q,
                        visible_team,
                        &unit_groups.slots[index],
                    );
                    for entity in selected_visible.iter().copied() {
                        if !target.contains(&entity) {
                            target.push(entity);
                        }
                    }
                    if !target.is_empty() {
                        record_control_group_assigned_battle_log(
                            &mut battle_log,
                            index,
                            target.len(),
                            selected_entities_focus(&selectable_q, visible_team, &target),
                        );
                    }
                    unit_groups.slots[index] = target;
                } else {
                    let previous = unit_groups.slots[index].clone();
                    unit_groups.slots[index] = selected_visible.clone();
                    if selected_visible.is_empty() {
                        if !previous.is_empty() {
                            record_control_group_cleared_battle_log(&mut battle_log, index);
                        }
                    } else {
                        record_control_group_assigned_battle_log(
                            &mut battle_log,
                            index,
                            selected_visible.len(),
                            selected_entities_focus(&selectable_q, visible_team, &selected_visible),
                        );
                    }
                }
                unit_groups.last_accessed = None;
                continue;
            }

            let group =
                valid_group_entities(&selectable_q, visible_team, &unit_groups.slots[index]);
            unit_groups.slots[index] = group.clone();
            let should_focus = unit_groups.last_accessed == Some(index)
                && is_exact_current_selection(&selected_visible, &group);
            apply_selected_from_ids(&mut commands, &selectable_q, &group, shift, visible_team);
            unit_groups.last_accessed = if group.is_empty() { None } else { Some(index) };
            if should_focus {
                focus_entities(
                    &mut camera_state,
                    &selectable_q,
                    visible_team,
                    &group,
                    *map_bounds,
                );
            }
        }
    }

    if let Some(select_all) = production_structure_hotkey_select_all(
        alt,
        ctrl,
        shift,
        keyboard.just_pressed(KeyCode::KeyC),
    ) {
        select_production_structures_for_hotkey(
            &mut commands,
            &selectable_q,
            &production_structure_q,
            &selected_q,
            visible_team,
            select_all,
            &["CommandCenter"],
            &mut camera_state,
            *map_bounds,
        );
        return;
    }
    if let Some(select_all) = production_structure_hotkey_select_all(
        alt,
        ctrl,
        shift,
        keyboard.just_pressed(KeyCode::KeyB),
    ) {
        select_production_structures_for_hotkey(
            &mut commands,
            &selectable_q,
            &production_structure_q,
            &selected_q,
            visible_team,
            select_all,
            &["Barracks"],
            &mut camera_state,
            *map_bounds,
        );
        return;
    }
    if let Some(select_all) = production_structure_hotkey_select_all(
        alt,
        ctrl,
        shift,
        keyboard.just_pressed(KeyCode::KeyV),
    ) {
        select_production_structures_for_hotkey(
            &mut commands,
            &selectable_q,
            &production_structure_q,
            &selected_q,
            visible_team,
            select_all,
            &["VehicleFactory"],
            &mut camera_state,
            *map_bounds,
        );
        return;
    }
    if let Some(select_all) = production_structure_hotkey_select_all(
        alt,
        ctrl,
        shift,
        keyboard.just_pressed(KeyCode::KeyF),
    ) {
        select_production_structures_for_hotkey(
            &mut commands,
            &selectable_q,
            &production_structure_q,
            &selected_q,
            visible_team,
            select_all,
            &["AircraftFactory"],
            &mut camera_state,
            *map_bounds,
        );
        return;
    }

    if alt && keyboard.just_pressed(KeyCode::KeyA) {
        if ctrl {
            let ids = army_selectable_q
                .iter()
                .filter_map(|(entity, _, team, unit, visibility)| {
                    is_visible_army_selection_candidate(*team, visible_team, unit, visibility)
                        .then_some(entity)
                })
                .collect::<Vec<_>>();
            apply_selected_from_ids(&mut commands, &selectable_q, &ids, false, visible_team);
            return;
        }

        let Some(window) = window_q.single().ok() else {
            return;
        };
        let Ok((camera, camera_transform)) = camera_q.single() else {
            return;
        };
        let ids = army_selectable_q
            .iter()
            .filter_map(|(entity, transform, team, unit, visibility)| {
                (is_visible_army_selection_candidate(*team, visible_team, unit, visibility)
                    && point_is_on_screen(window, camera, camera_transform, transform.translation))
                .then_some(entity)
            })
            .collect::<Vec<_>>();
        apply_selected_from_ids(&mut commands, &selectable_q, &ids, false, visible_team);
        return;
    }

    if alt && ctrl && keyboard.just_pressed(KeyCode::KeyI) {
        let ids = unit_q
            .iter()
            .filter_map(
                |(
                    entity,
                    team,
                    unit,
                    order_queue,
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
                    visibility,
                )| {
                    if *team != visible_team || !visibility.visible || !is_unit_harvester(unit) {
                        return None;
                    }
                    is_unit_idle(
                        order_queue,
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
                    )
                    .then_some(entity)
                },
            )
            .collect::<Vec<_>>();
        apply_selected_from_ids(&mut commands, &selectable_q, &ids, false, visible_team);
        return;
    }
    if alt && keyboard.just_pressed(KeyCode::KeyI) {
        let ids = unit_q
            .iter()
            .filter_map(
                |(
                    entity,
                    team,
                    unit,
                    order_queue,
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
                    visibility,
                )| {
                    if *team != visible_team
                        || !visibility.visible
                        || !is_builder_worker_selection_unit(unit)
                    {
                        return None;
                    }
                    if is_unit_idle(
                        order_queue,
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
                    ) {
                        Some(entity)
                    } else {
                        None
                    }
                },
            )
            .collect::<Vec<_>>();
        apply_selected_from_ids(&mut commands, &selectable_q, &ids, false, visible_team);
    }
}

fn handle_camera_bookmark_hotkeys(
    keyboard: &ButtonInput<KeyCode>,
    bookmarks: &mut CameraBookmarks,
    camera_state: &mut RtsCamera,
    map_bounds: MapBounds,
    alt: bool,
    ctrl: bool,
) {
    for (index, key) in CAMERA_BOOKMARK_KEYS.iter().enumerate() {
        if !keyboard.just_pressed(*key) {
            continue;
        }
        if alt && ctrl {
            bookmarks.slots[index] = Some(CameraBookmark::capture(camera_state));
            continue;
        }
        if alt && let Some(bookmark) = bookmarks.slots[index] {
            bookmark.restore_safely(camera_state, map_bounds);
        }
    }
}

fn production_structure_hotkey_select_all(
    alt: bool,
    ctrl: bool,
    shift: bool,
    just_pressed: bool,
) -> Option<bool> {
    (alt && !ctrl && just_pressed).then_some(shift)
}

fn is_production_structure_hotkey_candidate(
    structure: &Structure,
    health: &Health,
    visibility: &VisibilityState,
    under_construction: Option<&UnderConstruction>,
    structure_ids: &[&str],
) -> bool {
    structure_ids.contains(&structure.id)
        && health.current > 0.0
        && visibility.visible
        && structure_is_constructed(under_construction)
}

fn is_army_selection_unit(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some() && !is_economy_worker_selection_unit(unit)
}

fn is_visible_army_selection_candidate(
    team: Team,
    visible_team: Team,
    unit: &Unit,
    visibility: &VisibilityState,
) -> bool {
    team == visible_team && visibility.visible && is_army_selection_unit(unit)
}

fn is_builder_worker_selection_unit(unit: &Unit) -> bool {
    matches!(unit.id, "Worker" | "MobileConstructionVehicle")
}

fn is_economy_worker_selection_unit(unit: &Unit) -> bool {
    matches!(
        unit.id,
        "Worker" | "OreHarvester" | "MobileConstructionVehicle"
    )
}

fn is_unit_harvester(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| def.is_harvester)
}

fn is_exact_current_selection(current: &[Entity], target: &[Entity]) -> bool {
    if target.is_empty() || current.len() != target.len() {
        return false;
    }

    current.iter().all(|entity| target.contains(entity))
}

fn valid_group_entities(
    selectable_q: &Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    team: Team,
    entities: &[Entity],
) -> Vec<Entity> {
    entities
        .iter()
        .filter_map(|target| {
            selectable_q
                .iter()
                .any(|(entity, _, entity_team, _, _)| entity == *target && *entity_team == team)
                .then_some(*target)
        })
        .collect()
}

fn focus_entities(
    camera_state: &mut RtsCamera,
    selectable_q: &Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    team: Team,
    entities: &[Entity],
    bounds: MapBounds,
) {
    let mut focus = Vec3::ZERO;
    let mut count = 0usize;

    for (entity, transform, entity_team, _, _) in selectable_q.iter() {
        if *entity_team != team {
            continue;
        }
        if !entities.contains(&entity) {
            continue;
        }
        focus += transform.translation;
        count += 1;
    }

    if count > 0 {
        set_camera_focus_safely(camera_state, focus / count as f32, bounds);
    }
}

fn selected_entities_focus(
    selectable_q: &Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    team: Team,
    entities: &[Entity],
) -> Option<Vec3> {
    let mut focus = Vec3::ZERO;
    let mut count = 0usize;
    for (entity, transform, entity_team, _, _) in selectable_q.iter() {
        if *entity_team == team && entities.contains(&entity) {
            focus += transform.translation;
            count += 1;
        }
    }
    (count > 0).then_some(focus / count as f32)
}

fn record_control_group_assigned_battle_log(
    battle_log: &mut BattleLog,
    index: usize,
    count: usize,
    focus: Option<Vec3>,
) {
    if count == 0 {
        return;
    }
    push_battle_log(
        battle_log,
        format!("编组 {} 已设置: {} 个单位", index + 1, count),
        focus,
    );
}

fn record_control_group_cleared_battle_log(battle_log: &mut BattleLog, index: usize) {
    push_battle_log(battle_log, format!("编组 {} 已清空", index + 1), None);
}

fn select_production_structures_for_hotkey(
    commands: &mut Commands,
    selectable_q: &Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    production_structure_q: &Query<ProductionHotkeyStructureItem<'_>, With<Selectable>>,
    selected_q: &Query<(Entity, &Team, Option<&Structure>), With<Selected>>,
    team: Team,
    select_all: bool,
    structure_ids: &[&str],
    camera_state: &mut RtsCamera,
    bounds: MapBounds,
) {
    let mut structures = production_structure_q
        .iter()
        .filter_map(
            |(entity, entity_team, structure, health, visibility, under_construction)| {
                if *entity_team != team {
                    return None;
                }
                is_production_structure_hotkey_candidate(
                    structure,
                    health,
                    visibility,
                    under_construction,
                    structure_ids,
                )
                .then_some(entity)
            },
        )
        .collect::<Vec<_>>();
    structures.sort_by_key(|entity| entity.index());

    if structures.is_empty() {
        return;
    }

    let selected_structure = selected_q
        .iter()
        .filter_map(|(entity, selected_team, structure)| {
            if *selected_team != team {
                return None;
            }
            let structure = structure?;
            (structure_ids.contains(&structure.id) && structures.contains(&entity))
                .then_some(entity)
        })
        .collect::<Vec<_>>();

    let selected_entities = if select_all {
        structures.clone()
    } else if let Some(current) = selected_structure
        .first()
        .copied()
        .filter(|_| selected_structure.len() == 1)
    {
        let start_index = structures
            .iter()
            .position(|entity| *entity == current)
            .unwrap_or(0);
        let next_index = if start_index + 1 >= structures.len() {
            0
        } else {
            start_index + 1
        };
        vec![structures[next_index]]
    } else {
        vec![structures[0]]
    };

    apply_selected_from_ids(commands, selectable_q, &selected_entities, false, team);
    let focus_selection = if select_all {
        structures
            .first()
            .map_or(&selected_entities[..], |first| std::slice::from_ref(first))
    } else {
        &selected_entities[..]
    };
    focus_entities(camera_state, selectable_q, team, focus_selection, bounds);
}

fn is_unit_idle(
    order_queue: Option<&OrderQueue>,
    move_order: Option<&MoveOrder>,
    follow_order: Option<&FollowOrder>,
    attack_order: Option<&AttackOrder>,
    capture_order: Option<&CaptureOrder>,
    garrison_order: Option<&GarrisonOrder>,
    harvest_order: Option<&HarvestOrder>,
    repair_order: Option<&RepairOrder>,
    construct_order: Option<&ConstructOrder>,
    attack_move_order: Option<&AttackMoveOrder>,
    patrol_order: Option<&PatrolOrder>,
) -> bool {
    if let Some(queue) = order_queue
        && !queue.orders.is_empty()
    {
        return false;
    }
    move_order.is_none()
        && follow_order.is_none()
        && attack_order.is_none()
        && capture_order.is_none()
        && garrison_order.is_none()
        && harvest_order.is_none()
        && repair_order.is_none()
        && construct_order.is_none()
        && attack_move_order.is_none()
        && patrol_order.is_none()
}

fn apply_selected_from_ids(
    commands: &mut Commands,
    selectable_q: &Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    target: &[Entity],
    additive: bool,
    team: Team,
) {
    if !additive {
        for (entity, _, entity_team, _, _) in selectable_q.iter() {
            if *entity_team == team {
                commands.entity(entity).try_remove::<Selected>();
            }
        }
    }
    for (entity, _, entity_team, ..) in selectable_q.iter() {
        if *entity_team != team {
            continue;
        }
        if target.contains(&entity) {
            commands.entity(entity).try_insert(Selected);
        }
    }
}

fn command_shortcuts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut action_resources: CommandActionResources,
    slot_q: Query<(&CommandSlot, &BuildAction, Option<&CommandSlotAvailability>)>,
    selected_units: Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    selected_sell_structures: Query<SelectedSellStructureItem<'_>, With<Selected>>,
    selected_repair_structures: Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    selected_structures: Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: Query<StructureEntityItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    if keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::AltLeft)
        || keyboard.pressed(KeyCode::AltRight)
    {
        return;
    }

    for index in 0..COMMAND_SLOT_COUNT {
        let Some((_, action, availability)) = slot_q.iter().find(|(slot, ..)| slot.0 == index)
        else {
            continue;
        };
        let Some(hotkey) = command_action_hotkey(index, *action) else {
            continue;
        };
        if !keyboard.just_pressed(hotkey.key_code) {
            continue;
        }
        if availability.is_some_and(|availability| !availability.enabled) {
            return;
        }

        let _ = execute_command_action(
            &mut commands,
            &asset_server,
            &mut action_resources.next_id,
            visible_team,
            action_resources.player_factions.slot_faction(visible_team),
            *action,
            &mut action_resources.command_mode,
            &mut action_resources.economies,
            &selected_units,
            &selected_sell_structures,
            &selected_repair_structures,
            &selected_structures,
            &producer_structures,
            &structures,
            &mut action_resources.build_queue,
            &mut action_resources.audio_feedback,
            &mut action_resources.battle_log,
            production_batch_modifier_pressed(&keyboard),
        );
        return;
    }
}

fn command_buttons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut action_resources: CommandActionResources,
    selected_units: Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    selected_sell_structures: Query<SelectedSellStructureItem<'_>, With<Selected>>,
    selected_repair_structures: Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    selected_structures: Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: Query<StructureEntityItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
    mut interaction_q: Query<
        (
            &Interaction,
            &BuildAction,
            &CommandSlotAvailability,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    for (interaction, action, availability, mut background, mut border) in &mut interaction_q {
        match *interaction {
            Interaction::Pressed => {
                if *action != BuildAction::None && availability.enabled {
                    if mouse.just_pressed(MouseButton::Right) {
                        if cancel_latest_queued_product(
                            visible_team,
                            action_resources.player_factions.slot_faction(visible_team),
                            *action,
                            &selected_structures,
                            &producer_structures,
                            &mut action_resources.build_queue,
                            &mut action_resources.economies,
                        ) {
                            record_sound_audio_feedback(
                                &mut action_resources.audio_feedback,
                                SoundEffectKind::ConstructionCanceled,
                            );
                        }
                    } else if mouse.just_pressed(MouseButton::Left) {
                        let _ = execute_command_action(
                            &mut commands,
                            &asset_server,
                            &mut action_resources.next_id,
                            visible_team,
                            action_resources.player_factions.slot_faction(visible_team),
                            *action,
                            &mut action_resources.command_mode,
                            &mut action_resources.economies,
                            &selected_units,
                            &selected_sell_structures,
                            &selected_repair_structures,
                            &selected_structures,
                            &producer_structures,
                            &structures,
                            &mut action_resources.build_queue,
                            &mut action_resources.audio_feedback,
                            &mut action_resources.battle_log,
                            production_batch_modifier_pressed(&keyboard),
                        );
                    }
                }
            }
            Interaction::Hovered | Interaction::None => {}
        }
        let effective_interaction = if availability.enabled {
            *interaction
        } else {
            Interaction::None
        };
        let (bg, border_color) =
            command_button_colors(*action, availability.enabled, effective_interaction);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

fn production_batch_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)
}

fn production_queue_slot_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    visible_player: Res<VisiblePlayer>,
    mut build_queue: ResMut<BuildQueue>,
    mut economies: ResMut<Economies>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut interaction_q: Query<
        (
            &Interaction,
            &ProductionQueueSlotTarget,
            &mut BackgroundColor,
        ),
        Changed<Interaction>,
    >,
) {
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    for (interaction, target, mut color) in &mut interaction_q {
        match *interaction {
            Interaction::Pressed => {
                if let Some(producer_entity) = target.producer_entity
                    && mouse.just_pressed(MouseButton::Left)
                    && cancel_queued_job_at_local_index(
                        visible_team,
                        producer_entity,
                        target.local_index,
                        &mut build_queue,
                        &mut economies,
                    )
                {
                    record_sound_audio_feedback(
                        &mut audio_feedback,
                        SoundEffectKind::ConstructionCanceled,
                    );
                }
                *color = BackgroundColor(Color::srgba(0.17, 0.28, 0.34, 0.96));
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.1, 0.16, 0.19, 0.96));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.9));
            }
        }
    }
}

fn execute_command_action(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    command_mode: &mut CommandMode,
    economies: &mut Economies,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    selected_sell_structures: &Query<SelectedSellStructureItem<'_>, With<Selected>>,
    selected_repair_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    build_queue: &mut BuildQueue,
    audio_feedback: &mut AudioFeedback,
    battle_log: &mut BattleLog,
    batch_to_limit: bool,
) -> bool {
    let canceling_construction = action == BuildAction::SellStructure
        && selected_sell_structures.iter().any(
            |(_, _, structure_team, health, under_construction)| {
                *structure_team == team && health.current > 0.0 && under_construction.is_some()
            },
        )
        || action == BuildAction::StopSelected
            && selected_under_construction_stop_target(
                team,
                selected_units
                    .iter()
                    .filter(|(_, _, unit_team, ..)| **unit_team == team)
                    .count(),
                selected_sell_structures.iter().map(
                    |(entity, _, structure_team, health, under_construction)| {
                        (entity, structure_team, health, under_construction)
                    },
                ),
            )
            .is_some();
    let handled = match action {
        BuildAction::DeployMcv => deploy_selected_mcv(
            commands,
            asset_server,
            next_id,
            team,
            faction,
            selected_units,
        ),
        BuildAction::SellStructure => sell_selected_structures(
            commands,
            team,
            selected_sell_structures,
            economies,
            build_queue,
        ),
        BuildAction::RepairStructure => {
            repair_selected_structures(commands, team, selected_repair_structures, economies)
        }
        BuildAction::ToggleDeployMode => {
            request_selected_deploy_toggle(commands, team, selected_units)
        }
        BuildAction::SetRallyPoint => begin_rally_point_mode(command_mode, true),
        BuildAction::HoldPosition => {
            let handled = toggle_selected_hold_position(
                commands,
                team,
                selected_units
                    .iter()
                    .map(|(entity, unit, unit_team, _, hold, ..)| (entity, unit, unit_team, hold)),
            );
            if handled {
                clear_targeting_modes(command_mode);
            }
            handled
        }
        BuildAction::AttackMove => begin_attack_move_mode(
            command_mode,
            selected_units.iter().any(|(_, unit, unit_team, ..)| {
                *unit_team == team && unit_supports_attack_move(unit)
            }),
        ),
        BuildAction::Patrol => begin_patrol_mode(
            command_mode,
            selected_units
                .iter()
                .any(|(_, unit, unit_team, ..)| *unit_team == team && unit_supports_patrol(unit)),
        ),
        BuildAction::GuardArea => {
            clear_targeting_modes(command_mode);
            guard_selected_area(
                commands,
                team,
                selected_units
                    .iter()
                    .map(|(entity, unit, unit_team, ..)| (entity, unit, unit_team)),
            )
        }
        BuildAction::StopSelected => {
            clear_targeting_modes(command_mode);
            cancel_selected_under_construction_structure(
                commands,
                team,
                selected_units
                    .iter()
                    .filter(|(_, _, unit_team, ..)| **unit_team == team)
                    .count(),
                selected_sell_structures.iter().map(
                    |(entity, _, structure_team, health, under_construction)| {
                        (entity, structure_team, health, under_construction)
                    },
                ),
                economies,
                build_queue,
            ) || stop_selected_units(commands, team, selected_units)
        }
        BuildAction::ScatterSelected => {
            clear_targeting_modes(command_mode);
            scatter_selected_units(commands, team, selected_units)
        }
        BuildAction::Train(_) => {
            match enqueue_build_action_for_faction(
                team,
                faction,
                action,
                selected_structures,
                producer_structures,
                structures,
                economies,
                build_queue,
                batch_to_limit,
            ) {
                EnqueueBuildActionResult::Enqueued => true,
                EnqueueBuildActionResult::NotEnoughResources => {
                    record_sound_audio_feedback(audio_feedback, SoundEffectKind::Error);
                    record_voice_audio_feedback(audio_feedback, UnitVoiceEvent::NotEnoughResources);
                    record_insufficient_funds_battle_log(team, team, battle_log);
                    false
                }
                EnqueueBuildActionResult::QueueFull => {
                    record_sound_audio_feedback(audio_feedback, SoundEffectKind::Error);
                    false
                }
                EnqueueBuildActionResult::Unavailable => false,
            }
        }
        BuildAction::Build(id) => {
            begin_structure_placement_mode_for_faction(team, faction, id, command_mode, structures)
        }
        BuildAction::None => false,
    };
    if handled
        && !canceling_construction
        && let Some(command_key) = action.audio_command_key()
    {
        record_command_audio_feedback(
            audio_feedback,
            selected_query_has_owned_voice_unit(selected_units, team),
            Some(command_key),
        );
    }
    if handled && canceling_construction {
        record_sound_audio_feedback(audio_feedback, SoundEffectKind::ConstructionCanceled);
    } else if handled && !matches!(action, BuildAction::Build(_)) {
        record_build_action_audio_feedback(audio_feedback, team, team, action);
    }
    handled
}

fn request_selected_deploy_toggle(
    commands: &mut Commands,
    team: Team,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) -> bool {
    let mut requested_any = false;
    for (entity, unit, unit_team, ..) in selected_units {
        if *unit_team == team && unit.id == "SiegeDrillTank" {
            commands.entity(entity).try_insert(DeployModeToggleRequest);
            requested_any = true;
        }
    }
    requested_any
}

fn deploy_selected_mcv(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) -> bool {
    for (entity, unit, unit_team, transform, ..) in selected_units {
        if *unit_team != team || unit.id != "MobileConstructionVehicle" {
            continue;
        }
        let deploy_position = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
        spawn_structure_for_faction(
            commands,
            asset_server,
            next_id,
            "CommandCenter",
            team,
            team,
            deploy_position,
            faction,
        );
        commands.entity(entity).try_despawn();
        return true;
    }
    false
}

fn sell_selected_structures(
    commands: &mut Commands,
    team: Team,
    selected_structures: &Query<SelectedSellStructureItem<'_>, With<Selected>>,
    economies: &mut Economies,
    build_queue: &mut BuildQueue,
) -> bool {
    let mut sold_any = false;
    for (entity, structure, structure_team, health, under_construction) in selected_structures {
        if *structure_team != team || health.current <= 0.0 {
            continue;
        }
        let Some(def) = registry::entity(structure.id) else {
            continue;
        };
        let refund = if let Some(construction) = under_construction {
            construction_cancel_refund(construction.cost)
        } else {
            structure_sell_refund(def, health)
        };
        let economy = economies.get_mut(team);
        economy.ore += refund.0;
        economy.crystal += refund.1;
        cancel_jobs_for_producer(build_queue, economies, entity);
        commands.entity(entity).try_despawn();
        sold_any = true;
    }
    sold_any
}

fn structure_sell_refund(def: &registry::EntityDef, health: &Health) -> (i32, i32) {
    let health_ratio = if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        1.0
    };
    (
        (def.cost.ore as f32 * STRUCTURE_SELL_REFUND_RATIO * health_ratio).ceil() as i32,
        (def.cost.crystal as f32 * STRUCTURE_SELL_REFUND_RATIO * health_ratio).ceil() as i32,
    )
}

fn construction_cancel_refund(cost: registry::Cost) -> (i32, i32) {
    (cost.ore, cost.crystal)
}

fn cancel_jobs_for_producer(
    build_queue: &mut BuildQueue,
    economies: &mut Economies,
    producer_entity: Entity,
) {
    build_queue.0.retain(|job| {
        let should_cancel = job.producer_entity == producer_entity;
        if should_cancel {
            refund_build_job_cost(job, economies);
        }
        !should_cancel
    });
}

fn cancel_latest_queued_product(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    build_queue: &mut BuildQueue,
    economies: &mut Economies,
) -> bool {
    let product_id = build_target_product(action);
    if product_id.is_empty() {
        return false;
    }
    let producer_entities = cancellation_producers_for_action(
        team,
        faction,
        action,
        selected_structures,
        producer_structures,
    );
    cancel_latest_queued_product_for_producers(
        team,
        product_id,
        &producer_entities,
        build_queue,
        economies,
    )
}

fn cancellation_producers_for_action(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
) -> Vec<Entity> {
    match action {
        BuildAction::Train(product_id) => {
            let Some(faction) = faction_def(faction) else {
                return Vec::new();
            };
            let selected = selected_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && faction.can_produce(structure.id, product_id))
                        .then_some(entity)
                    },
                )
                .collect::<Vec<_>>();
            if !selected.is_empty() {
                return selected;
            }
            producer_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && faction.can_produce(structure.id, product_id))
                        .then_some(entity)
                    },
                )
                .collect()
        }
        BuildAction::Build(_) => {
            let selected = selected_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && structure.id == "CommandCenter")
                            .then_some(entity)
                    },
                )
                .collect::<Vec<_>>();
            if !selected.is_empty() {
                return selected;
            }
            producer_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && structure.id == "CommandCenter")
                            .then_some(entity)
                    },
                )
                .collect()
        }
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => Vec::new(),
    }
}

fn cancel_latest_queued_product_for_producers(
    team: Team,
    product_id: &'static str,
    producer_entities: &[Entity],
    build_queue: &mut BuildQueue,
    economies: &mut Economies,
) -> bool {
    let Some(index) = build_queue.0.iter().rposition(|job| {
        job.team == team
            && build_target_product(job.action) == product_id
            && producer_entities.contains(&job.producer_entity)
    }) else {
        return false;
    };
    let canceled_job = build_queue.0.remove(index);
    refund_build_job_cost(&canceled_job, economies);
    true
}

fn cancel_queued_job_at_local_index(
    team: Team,
    producer_entity: Entity,
    local_index: usize,
    build_queue: &mut BuildQueue,
    economies: &mut Economies,
) -> bool {
    let Some((index, _)) = build_queue
        .0
        .iter()
        .enumerate()
        .filter(|(_, job)| job.team == team && job.producer_entity == producer_entity)
        .nth(local_index)
    else {
        return false;
    };
    let canceled_job = build_queue.0.remove(index);
    refund_build_job_cost(&canceled_job, economies);
    true
}

fn command_queue_button_state_for_action(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    build_queue: &BuildQueue,
) -> Option<QueueButtonState> {
    if matches!(action, BuildAction::Build(_)) {
        return None;
    }
    let product_id = build_target_product(action);
    if product_id.is_empty() {
        return None;
    }
    let producer_entities = command_queue_producers_for_action(
        team,
        faction,
        action,
        selected_structures,
        producer_structures,
    );
    queue_button_state_for_product(team, product_id, &producer_entities, build_queue)
}

fn command_queue_producers_for_action(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
) -> Vec<Entity> {
    match action {
        BuildAction::Train(product_id) => {
            let Some(faction) = faction_def(faction) else {
                return Vec::new();
            };
            let selected = selected_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && faction.can_produce(structure.id, product_id))
                        .then_some(entity)
                    },
                )
                .collect::<Vec<_>>();
            if !selected.is_empty() {
                return selected;
            }
            producer_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && faction.can_produce(structure.id, product_id))
                        .then_some(entity)
                    },
                )
                .collect()
        }
        BuildAction::Build(_) => {
            let selected = selected_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && structure.id == "CommandCenter")
                            .then_some(entity)
                    },
                )
                .collect::<Vec<_>>();
            if !selected.is_empty() {
                return selected;
            }
            producer_structures
                .iter()
                .filter_map(
                    |(entity, structure, structure_team, _, under_construction)| {
                        (*structure_team == team
                            && structure_is_constructed(under_construction)
                            && structure.id == "CommandCenter")
                            .then_some(entity)
                    },
                )
                .collect()
        }
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => Vec::new(),
    }
}

fn queue_button_state_for_product(
    team: Team,
    product_id: &'static str,
    producer_entities: &[Entity],
    build_queue: &BuildQueue,
) -> Option<QueueButtonState> {
    if producer_entities.is_empty() {
        return None;
    }
    let count = build_queue
        .0
        .iter()
        .filter(|job| {
            job.team == team
                && build_target_product(job.action) == product_id
                && producer_entities.contains(&job.producer_entity)
        })
        .count();
    let full = producer_entities
        .iter()
        .all(|producer_entity| !build_queue_has_capacity(build_queue, *producer_entity));
    Some(QueueButtonState { count, full })
}

fn queue_button_badge_text(state: QueueButtonState) -> String {
    if state.full {
        " [满]".to_string()
    } else {
        format!(" [x{}]", state.count)
    }
}

fn repair_selected_structures(
    commands: &mut Commands,
    team: Team,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    economies: &mut Economies,
) -> bool {
    let mut started_any = false;
    for (entity, structure, structure_team, health, repair, under_construction) in
        selected_structures
    {
        if *structure_team != team
            || repair.is_some()
            || !structure_is_constructed(under_construction)
            || health.current <= 0.0
            || health.current >= health.max
        {
            continue;
        }
        let Some(def) = registry::entity(structure.id) else {
            continue;
        };
        let cost = structure_repair_cost(def, health);
        if !economies.get(team).can_afford(cost) {
            continue;
        }
        if !economies.get_mut(team).spend(cost) {
            continue;
        }
        commands.entity(entity).try_insert(ManualStructureRepair {
            points_remaining: missing_structure_hitpoints(health),
        });
        started_any = true;
    }
    started_any
}

fn repair_ai_damaged_structures(
    commands: &mut Commands,
    team: Team,
    structures: &Query<AiRepairStructureItem<'_>, With<Structure>>,
    economies: &mut Economies,
) -> usize {
    let mut candidates = Vec::new();
    for (entity, structure, structure_team, health, repair, under_construction) in structures {
        if *structure_team != team
            || repair.is_some()
            || !structure_is_constructed(under_construction)
            || health.current <= 0.0
            || health.current >= health.max
            || health.max <= 0.0
        {
            continue;
        }
        let missing_ratio = (missing_structure_hitpoints(health) / health.max).clamp(0.0, 1.0);
        if missing_ratio < AI_REPAIR_MIN_MISSING_HITPOINT_RATIO {
            continue;
        }
        candidates.push((health.ratio(), entity, structure.id, *health));
    }
    candidates.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut started = 0usize;
    for (_ratio, entity, structure_id, health) in candidates {
        if started >= AI_REPAIR_MAX_STARTS_PER_REFRESH {
            break;
        }
        let Some(def) = registry::entity(structure_id) else {
            continue;
        };
        let cost = structure_repair_cost(def, &health);
        if !economies.get(team).can_afford(cost) {
            continue;
        }
        if !economies.get_mut(team).spend(cost) {
            continue;
        }
        commands.entity(entity).try_insert(ManualStructureRepair {
            points_remaining: missing_structure_hitpoints(&health),
        });
        started += 1;
    }
    started
}

fn structure_repair_cost(def: &registry::EntityDef, health: &Health) -> registry::Cost {
    let hp_ratio = if health.max > 0.0 {
        (missing_structure_hitpoints(health) / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    registry::Cost {
        ore: (def.cost.ore as f32 * STRUCTURE_MANUAL_REPAIR_COST_RATIO * hp_ratio).ceil() as i32,
        crystal: (def.cost.crystal as f32 * STRUCTURE_MANUAL_REPAIR_COST_RATIO * hp_ratio).ceil()
            as i32,
    }
}

fn missing_structure_hitpoints(health: &Health) -> f32 {
    (health.max - health.current).max(0.0)
}

fn update_manual_structure_repairs(
    mut commands: Commands,
    time: Res<Time>,
    mut structures: Query<(Entity, &mut Health, &mut ManualStructureRepair), With<Structure>>,
) {
    for (entity, mut health, mut repair) in &mut structures {
        if health.current <= 0.0 {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
            continue;
        }
        let repaired = (STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND * time.delta_secs())
            .min(repair.points_remaining)
            .min(missing_structure_hitpoints(&health));
        if repaired <= 0.0 {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
            continue;
        }
        health.current = (health.current + repaired).min(health.max);
        repair.points_remaining -= repaired;
        if repair.points_remaining <= 0.0 || health.current >= health.max {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
        }
    }
}

fn update_deploy_mode_requests(
    mut commands: Commands,
    mut units: Query<
        (
            Entity,
            &mut Unit,
            &mut HoldPosition,
            Option<&mut Weapon>,
            &mut VisionRadius,
            Option<&DeployedSiegeMode>,
            &Health,
            Option<&EmpDisabled>,
        ),
        With<DeployModeToggleRequest>,
    >,
) {
    let mut deployable_count = 0usize;
    let mut deployed_count = 0usize;
    for (_entity, unit, _hold, weapon, _vision, deployed, health, emp) in units.iter_mut() {
        if siege_drill_can_toggle_deploy_mode(&unit, weapon.is_some(), health, emp) {
            deployable_count += 1;
            if deployed.is_some() {
                deployed_count += 1;
            }
        }
    }
    let desired_deployed = deployable_count > 0 && deployed_count != deployable_count;

    for (entity, mut unit, mut hold, weapon, mut vision, deployed, health, emp) in &mut units {
        commands
            .entity(entity)
            .try_remove::<DeployModeToggleRequest>();
        if !siege_drill_can_toggle_deploy_mode(&unit, weapon.is_some(), health, emp) {
            continue;
        }
        let current_deployed = deployed.is_some();
        let deployed = deployed.copied();
        if current_deployed == desired_deployed {
            continue;
        }

        let Some(mut weapon) = weapon else {
            continue;
        };
        apply_siege_drill_deploy_mode(
            &mut commands,
            entity,
            &mut unit,
            &mut hold,
            &mut weapon,
            &mut vision,
            deployed,
            desired_deployed,
            true,
        );
    }
}

fn siege_drill_can_toggle_deploy_mode(
    unit: &Unit,
    has_weapon: bool,
    health: &Health,
    emp: Option<&EmpDisabled>,
) -> bool {
    unit.id == "SiegeDrillTank"
        && has_weapon
        && health.current > 0.0
        && !emp.is_some_and(|emp| emp.remaining > 0.0)
}

fn update_ai_siege_drill_deploy_mode(
    mut commands: Commands,
    visible_player: Option<Res<VisiblePlayer>>,
    mut drills: Query<
        (
            Entity,
            &Team,
            &mut Unit,
            &mut HoldPosition,
            &mut Weapon,
            &mut VisionRadius,
            &Transform,
            Option<&DeployedSiegeMode>,
            &Health,
            Option<&EmpDisabled>,
            Option<&AttackOrder>,
        ),
        With<Unit>,
    >,
    targets: Query<
        (
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &MovementDomain,
            &Health,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (
        entity,
        team,
        mut unit,
        mut hold,
        mut weapon,
        mut vision,
        transform,
        deployed,
        health,
        emp,
        attack_order,
    ) in &mut drills
    {
        if *team == player_team || unit.id != "SiegeDrillTank" {
            continue;
        }
        let desired_deployed = ai_siege_drill_should_deploy(
            *team,
            transform.translation,
            &weapon,
            health,
            emp,
            attack_order,
            &targets,
        );
        if desired_deployed == deployed.is_some() {
            continue;
        }
        apply_siege_drill_deploy_mode(
            &mut commands,
            entity,
            &mut unit,
            &mut hold,
            &mut weapon,
            &mut vision,
            deployed.copied(),
            desired_deployed,
            false,
        );
    }
}

fn ai_siege_drill_should_deploy(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    health: &Health,
    emp: Option<&EmpDisabled>,
    attack_order: Option<&AttackOrder>,
    targets: &Query<
        (
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &MovementDomain,
            &Health,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> bool {
    if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
        return false;
    }
    let Some(order) = attack_order else {
        return false;
    };
    let Ok((
        _structure,
        target_team,
        target_transform,
        _target_selectable,
        target_domain,
        target_health,
    )) = targets.get(order.target)
    else {
        return false;
    };
    if target_health.current <= 0.0 || *target_team == team || *target_team == Team::Neutral {
        return false;
    }
    if !can_attack_domain(weapon, *target_domain) {
        return false;
    }
    xz_distance(position, target_transform.translation) <= SIEGE_DRILL_DEPLOYED_ATTACK_RANGE
}

fn apply_siege_drill_deploy_mode(
    commands: &mut Commands,
    entity: Entity,
    unit: &mut Unit,
    hold: &mut HoldPosition,
    weapon: &mut Weapon,
    vision: &mut VisionRadius,
    deployed: Option<DeployedSiegeMode>,
    desired_deployed: bool,
    clear_attack_order: bool,
) {
    if clear_attack_order {
        clear_order_state(commands, entity);
    } else {
        clear_non_attack_order_state(commands, entity);
    }
    commands.entity(entity).try_remove::<OrderQueue>();

    match (deployed, desired_deployed) {
        (Some(deployed), false) => {
            unit.speed = deployed.base_speed;
            hold.enabled = deployed.previous_hold_position;
            weapon.range = deployed.base_attack_range;
            weapon.cooldown = deployed.base_attack_interval;
            weapon.structure_damage_multiplier = deployed.base_structure_damage_multiplier;
            vision.0 = deployed.base_sight_range;
            commands.entity(entity).try_remove::<DeployedSiegeMode>();
        }
        (None, true) => {
            commands.entity(entity).try_insert(DeployedSiegeMode {
                previous_hold_position: hold.enabled,
                base_speed: unit.speed,
                base_attack_range: weapon.range,
                base_attack_interval: weapon.cooldown,
                base_structure_damage_multiplier: weapon.structure_damage_multiplier,
                base_sight_range: vision.0,
            });
            unit.speed = 0.0;
            hold.enabled = true;
            weapon.range = SIEGE_DRILL_DEPLOYED_ATTACK_RANGE;
            weapon.cooldown = SIEGE_DRILL_DEPLOYED_ATTACK_INTERVAL;
            weapon.structure_damage_multiplier = SIEGE_DRILL_DEPLOYED_STRUCTURE_DAMAGE_MULTIPLIER;
            vision.0 = SIEGE_DRILL_DEPLOYED_SIGHT_RANGE;
        }
        _ => {}
    }
}

fn process_build_queue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    map_bounds: Res<MapBounds>,
    visible_player: Option<Res<VisiblePlayer>>,
    player_factions: Res<PlayerFactions>,
    mut build_queue: ResMut<BuildQueue>,
    mut economies: ResMut<Economies>,
    mut next_id: ResMut<NextSpawnId>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    rally_points: Query<&RallyPoint>,
    rally_targets: Query<
        (Option<&Health>, Option<&ResourceNode>),
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
    structures: Query<StructureEntityItem<'_>>,
    occupiers: Query<(Entity, &Transform, &Selectable, &Health), Or<(With<Unit>, With<Structure>)>>,
) {
    let mut occupied_spawn_points = production_occupied_spawn_points(&occupiers);
    let player_team = visible_player_team(visible_player.as_deref());
    let frame_delta = time.delta_secs();
    let mut producer_production_deltas: Vec<(Entity, f32)> = Vec::new();
    let mut blocked_producers: Vec<Entity> = Vec::new();
    let mut index = 0;
    while index < build_queue.0.len() {
        let queued_job = build_queue.0[index];
        let action_id = match queued_job.action {
            BuildAction::Train(id) | BuildAction::Build(id) => id,
            BuildAction::DeployMcv
            | BuildAction::SellStructure
            | BuildAction::RepairStructure
            | BuildAction::ToggleDeployMode
            | BuildAction::SetRallyPoint
            | BuildAction::HoldPosition
            | BuildAction::AttackMove
            | BuildAction::Patrol
            | BuildAction::GuardArea
            | BuildAction::StopSelected
            | BuildAction::ScatterSelected => {
                build_queue.0.remove(index);
                continue;
            }
            BuildAction::None => {
                index += 1;
                continue;
            }
        };
        if registry::entity(action_id).is_none() {
            let canceled_job = build_queue.0.remove(index);
            refund_build_job_cost(&canceled_job, &mut economies);
            if queued_job.team == player_team {
                record_sound_audio_feedback(
                    &mut audio_feedback,
                    SoundEffectKind::ConstructionCanceled,
                );
            }
            continue;
        }
        if !has_producer_for_job(&queued_job, &structures, &player_factions) {
            let canceled_job = build_queue.0.remove(index);
            refund_build_job_cost(&canceled_job, &mut economies);
            if queued_job.team == player_team {
                record_sound_audio_feedback(
                    &mut audio_feedback,
                    SoundEffectKind::ConstructionCanceled,
                );
            }
            continue;
        }
        if blocked_producers.contains(&queued_job.producer_entity) {
            index += 1;
            continue;
        }

        let producer_delta_index = match producer_production_deltas
            .iter()
            .position(|(producer, _)| *producer == queued_job.producer_entity)
        {
            Some(index) => index,
            None => {
                let speed_multiplier = production_speed_multiplier(economies.get(queued_job.team));
                producer_production_deltas
                    .push((queued_job.producer_entity, frame_delta * speed_multiplier));
                producer_production_deltas.len() - 1
            }
        };
        if producer_production_deltas[producer_delta_index].1 <= f32::EPSILON {
            index += 1;
            continue;
        }
        let timer_before = build_queue.0[index].timer;
        let available_production_delta = producer_production_deltas[producer_delta_index].1;
        let applied_production_delta = available_production_delta.min(build_queue.0[index].timer);
        build_queue.0[index].timer =
            (build_queue.0[index].timer - applied_production_delta).max(0.0);
        producer_production_deltas[producer_delta_index].1 =
            (available_production_delta - applied_production_delta).max(0.0);
        if build_queue.0[index].timer > 0.0 {
            index += 1;
            continue;
        }

        let ready_job = build_queue.0[index];
        let team = ready_job.team;
        let origin = ready_job.origin;
        let producer_entity = ready_job.producer_entity;
        let producer_id = ready_job.producer_id;
        let spawn_id_seed = next_id.0;
        let faction = player_factions.slot_faction(team);
        match ready_job.action {
            BuildAction::Train(id) => {
                let Some(def) = registry::entity(id) else {
                    index += 1;
                    continue;
                };
                let Some(spawn_at) = find_production_spawn_position(
                    origin,
                    producer_id,
                    def.radius,
                    spawn_id_seed,
                    &occupied_spawn_points,
                    *map_bounds,
                ) else {
                    record_production_blocked_once(
                        team,
                        player_team,
                        timer_before,
                        origin,
                        &mut audio_feedback,
                        &mut battle_log,
                    );
                    producer_production_deltas[producer_delta_index].1 = 0.0;
                    if !blocked_producers.contains(&producer_entity) {
                        blocked_producers.push(producer_entity);
                    }
                    index += 1;
                    continue;
                };
                build_queue.0.remove(index);
                occupied_spawn_points.push((spawn_at.with_y(0.0), def.radius));
                let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
                let spawned = spawn_unit_for_faction(
                    &mut commands,
                    &asset_server,
                    &mut next_id,
                    id,
                    team,
                    spawn_at,
                    initial_rank,
                    faction,
                    player_team,
                );
                let rally_point = rally_points.get(producer_entity).ok().copied();
                if let Some(target_unit) =
                    rally_point.and_then(|rally_point| rally_point.target_unit)
                    && let Ok((health, resource)) = rally_targets.get(target_unit)
                {
                    if resource.is_some_and(|resource| resource.amount > 0)
                        && def.resource_capacity > 0
                    {
                        commands.entity(spawned).try_insert(HarvestOrder {
                            resource: Some(target_unit),
                            state: HarvestState::MovingToResource,
                            collect_remaining: 0.0,
                        });
                    } else if health.is_some_and(|health| health.current > 0.0) {
                        commands.entity(spawned).try_insert(FollowOrder {
                            target: target_unit,
                            allow_enemy: false,
                            offset: Vec3::ZERO,
                        });
                    } else if let Some(rally_target) =
                        rally_point.and_then(|rally_point| rally_point.target)
                    {
                        commands.entity(spawned).try_insert(MoveOrder {
                            target: rally_target,
                        });
                    }
                } else if let Some(rally_target) =
                    rally_point.and_then(|rally_point| rally_point.target)
                {
                    commands.entity(spawned).try_insert(MoveOrder {
                        target: rally_target,
                    });
                }
                if team == player_team {
                    record_sound_audio_feedback(
                        &mut audio_feedback,
                        SoundEffectKind::ProductionReady,
                    );
                    record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::UnitReady);
                    record_production_ready_battle_log(
                        team,
                        player_team,
                        false,
                        def.label,
                        spawn_at,
                        &mut battle_log,
                    );
                }
            }
            BuildAction::Build(id) => {
                if id != "CommandCenter" {
                    let Some(def) = registry::entity(id) else {
                        index += 1;
                        continue;
                    };
                    let Some(spawn_at) = find_production_spawn_position(
                        origin,
                        producer_id,
                        def.radius,
                        spawn_id_seed + 5,
                        &occupied_spawn_points,
                        *map_bounds,
                    ) else {
                        record_production_blocked_once(
                            team,
                            player_team,
                            timer_before,
                            origin,
                            &mut audio_feedback,
                            &mut battle_log,
                        );
                        producer_production_deltas[producer_delta_index].1 = 0.0;
                        if !blocked_producers.contains(&producer_entity) {
                            blocked_producers.push(producer_entity);
                        }
                        index += 1;
                        continue;
                    };
                    build_queue.0.remove(index);
                    occupied_spawn_points.push((spawn_at.with_y(0.0), def.radius));
                    let free_harvester_origin = (id == "Refinery").then_some(origin);
                    spawn_structure_under_construction_for_faction(
                        &mut commands,
                        &asset_server,
                        &mut next_id,
                        id,
                        team,
                        spawn_at,
                        free_harvester_origin,
                        0.0,
                        player_team,
                        faction,
                    );
                    if team == player_team {
                        record_sound_audio_feedback(
                            &mut audio_feedback,
                            SoundEffectKind::ConstructionStarted,
                        );
                        push_battle_log(
                            &mut battle_log,
                            format!("开始施工: {}", def.label),
                            Some(spawn_at),
                        );
                    }
                } else {
                    build_queue.0.remove(index);
                }
            }
            BuildAction::DeployMcv
            | BuildAction::SellStructure
            | BuildAction::RepairStructure
            | BuildAction::ToggleDeployMode
            | BuildAction::SetRallyPoint
            | BuildAction::HoldPosition
            | BuildAction::AttackMove
            | BuildAction::Patrol
            | BuildAction::GuardArea
            | BuildAction::StopSelected
            | BuildAction::ScatterSelected
            | BuildAction::None => {}
        }
    }
}

fn spawn_refinery_free_harvester(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    structure_id: &'static str,
    team: Team,
    visible_team: Team,
    refinery_position: Vec3,
    build_origin: Vec3,
    spawn_seed: u32,
    bounds: MapBounds,
    visual_faction: Option<SkirmishFaction>,
) {
    if structure_id != "Refinery" {
        return;
    }
    let Some(spawn_at) =
        refinery_free_harvester_spawn_position(refinery_position, build_origin, spawn_seed, bounds)
    else {
        return;
    };
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        "OreHarvester",
        team,
        spawn_at,
        0,
        visual_faction.or_else(|| default_visual_faction(team)),
        visible_team,
    );
}

fn production_occupied_spawn_points(
    occupiers: &Query<
        (Entity, &Transform, &Selectable, &Health),
        Or<(With<Unit>, With<Structure>)>,
    >,
) -> Vec<(Vec3, f32)> {
    occupiers
        .iter()
        .filter_map(|(_, transform, selectable, health)| {
            (health.current > 0.0).then_some((transform.translation.with_y(0.0), selectable.radius))
        })
        .collect()
}

fn find_production_spawn_position(
    origin: Vec3,
    producer_id: &'static str,
    product_radius: f32,
    seed: u32,
    occupied_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) -> Option<Vec3> {
    let producer_radius = registry::entity(producer_id)
        .map(|def| def.radius)
        .unwrap_or(1.0);
    let base_radius = producer_radius + product_radius + 0.35;
    let angle_seed = seed as f32 * 1.618_034;
    let max_rings = 18;
    for ring in 0..=max_rings {
        let search_radius = base_radius + ring as f32 * 0.5;
        let samples = if ring == 0 { 8 } else { 12 + ring * 2 };
        for sample in 0..samples {
            let angle = angle_seed + sample as f32 * (std::f32::consts::TAU / samples as f32);
            let candidate = Vec3::new(
                origin.x + angle.cos() * search_radius,
                0.0,
                origin.z + angle.sin() * search_radius,
            );
            let clamped = bounds.clamp_ground_point(candidate, product_radius);
            if is_spawn_position_free(clamped, product_radius, occupied_spawn_points, bounds) {
                return Some(clamped);
            }
        }
    }
    None
}

fn record_production_blocked_once(
    team: Team,
    player_team: Team,
    timer_before: f32,
    focus: Vec3,
    audio_feedback: &mut AudioFeedback,
    battle_log: &mut BattleLog,
) {
    if team == player_team && timer_before > 0.0 {
        record_sound_audio_feedback(audio_feedback, SoundEffectKind::Error);
        push_battle_log(battle_log, "生产受阻: 出厂口被堵塞", Some(focus));
    }
}

fn record_production_ready_battle_log(
    team: Team,
    player_team: Team,
    is_structure: bool,
    label: &str,
    focus: Vec3,
    battle_log: &mut BattleLog,
) {
    if team != player_team {
        return;
    }
    let prefix = if is_structure {
        "建筑就绪"
    } else {
        "单位就绪"
    };
    push_battle_log(battle_log, format!("{prefix}: {label}"), Some(focus));
}

fn refinery_free_harvester_spawn_position(
    refinery_position: Vec3,
    build_origin: Vec3,
    spawn_seed: u32,
    bounds: MapBounds,
) -> Option<Vec3> {
    let refinery_def = registry::entity("Refinery")?;
    let harvester_def = registry::entity("OreHarvester")?;
    let mut direction = Vec3::new(
        refinery_position.x - build_origin.x,
        0.0,
        refinery_position.z - build_origin.z,
    );
    if direction.length_squared() <= f32::EPSILON {
        let angle = spawn_seed as f32 * 1.618_034;
        direction = Vec3::new(angle.cos(), 0.0, angle.sin());
    }
    direction = direction.normalize();
    let distance = refinery_def.radius + harvester_def.radius + 0.35;
    Some(bounds.clamp_ground_point(refinery_position + direction * distance, 1.0))
}

#[allow(dead_code)]
fn enqueue_build_action(
    team: Team,
    action: BuildAction,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    economies: &mut Economies,
    build_queue: &mut BuildQueue,
    batch_to_limit: bool,
) -> EnqueueBuildActionResult {
    enqueue_build_action_for_faction(
        team,
        SkirmishFaction::from_team(team),
        action,
        selected_structures,
        producer_structures,
        structures,
        economies,
        build_queue,
        batch_to_limit,
    )
}

fn enqueue_build_action_for_faction(
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    economies: &mut Economies,
    build_queue: &mut BuildQueue,
    batch_to_limit: bool,
) -> EnqueueBuildActionResult {
    let Some(faction_def) = faction_def(faction) else {
        return EnqueueBuildActionResult::Unavailable;
    };
    let def = match action {
        BuildAction::Train(id) | BuildAction::Build(id) => match registry::entity(id) {
            Some(def) => def,
            None => return EnqueueBuildActionResult::Unavailable,
        },
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => return EnqueueBuildActionResult::Unavailable,
    };
    if !requirements_met(def, team, structures) {
        return EnqueueBuildActionResult::Unavailable;
    }
    match action {
        BuildAction::Train(id) => {
            let producers = match production_origins_for_faction(
                team,
                faction,
                id,
                selected_structures,
                producer_structures,
                build_queue,
            ) {
                Ok(producer) => producer,
                Err(result) => return result,
            };
            enqueue_build_jobs_for_producers(
                team,
                action,
                def,
                &producers,
                batch_to_limit,
                economies,
                build_queue,
            )
        }
        BuildAction::Build(id) => {
            if id == "CommandCenter" {
                return EnqueueBuildActionResult::Unavailable;
            }
            if !faction_def.can_construct(id) {
                return EnqueueBuildActionResult::Unavailable;
            }
            let producers = match command_origins_for(
                team,
                selected_structures,
                producer_structures,
                build_queue,
            ) {
                Ok(producer) => producer,
                Err(result) => return result,
            };
            enqueue_build_jobs_for_producers(
                team,
                action,
                def,
                &producers,
                batch_to_limit,
                economies,
                build_queue,
            )
        }
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => EnqueueBuildActionResult::Unavailable,
    }
}

#[cfg(test)]
fn team_build_queue_len(build_queue: &BuildQueue, team: Team) -> usize {
    build_queue.0.iter().filter(|job| job.team == team).count()
}

fn producer_build_queue_len(build_queue: &BuildQueue, producer_entity: Entity) -> usize {
    build_queue
        .0
        .iter()
        .filter(|job| job.producer_entity == producer_entity)
        .count()
}

fn build_queue_has_capacity(build_queue: &BuildQueue, producer_entity: Entity) -> bool {
    producer_build_queue_len(build_queue, producer_entity) < PRODUCTION_QUEUE_LIMIT
}

fn refund_build_job_cost(job: &BuildJob, economies: &mut Economies) -> bool {
    let Some(def) = registry::entity(build_target_product(job.action)) else {
        return false;
    };
    economies.get_mut(job.team).refund(def.cost);
    true
}

fn enqueue_build_jobs_for_producers(
    team: Team,
    action: BuildAction,
    def: &registry::EntityDef,
    producers: &[(Entity, &'static str, Vec3)],
    batch_to_limit: bool,
    economies: &mut Economies,
    build_queue: &mut BuildQueue,
) -> EnqueueBuildActionResult {
    if producers.is_empty() {
        return EnqueueBuildActionResult::Unavailable;
    }
    let mut enqueued_any = false;
    let mut resource_blocked = false;
    for &(producer_entity, producer_id, origin) in producers {
        let requested_count = if batch_to_limit {
            PRODUCTION_QUEUE_LIMIT
                .saturating_sub(producer_build_queue_len(build_queue, producer_entity))
        } else {
            1
        };
        for _ in 0..requested_count {
            if !build_queue_has_capacity(build_queue, producer_entity) {
                break;
            }
            if !economies.get_mut(team).spend(def.cost) {
                resource_blocked = true;
                break;
            }
            build_queue.0.push(BuildJob {
                team,
                action,
                producer_entity,
                producer_id,
                timer: def.build_seconds,
                origin,
            });
            enqueued_any = true;
        }
    }
    if enqueued_any {
        EnqueueBuildActionResult::Enqueued
    } else if resource_blocked {
        EnqueueBuildActionResult::NotEnoughResources
    } else {
        EnqueueBuildActionResult::QueueFull
    }
}

#[allow(dead_code)]
fn production_origins_for(
    team: Team,
    product_id: &'static str,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    structures: &Query<StructureEntityItem<'_>>,
    build_queue: &BuildQueue,
) -> Result<Vec<(Entity, &'static str, Vec3)>, EnqueueBuildActionResult> {
    production_origins_for_faction(
        team,
        SkirmishFaction::from_team(team),
        product_id,
        selected_structures,
        structures,
        build_queue,
    )
}

fn production_origins_for_faction(
    team: Team,
    faction: SkirmishFaction,
    product_id: &'static str,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    structures: &Query<StructureEntityItem<'_>>,
    build_queue: &BuildQueue,
) -> Result<Vec<(Entity, &'static str, Vec3)>, EnqueueBuildActionResult> {
    let Some(faction) = faction_def(faction) else {
        return Err(EnqueueBuildActionResult::Unavailable);
    };
    let mut saw_selected_producer = false;
    let mut selected_producers = Vec::new();
    for (entity, structure, structure_team, transform, under_construction) in selected_structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && faction.can_produce(structure.id, product_id)
        {
            saw_selected_producer = true;
            if build_queue_has_capacity(build_queue, entity) {
                selected_producers.push((entity, structure.id, transform.translation));
            }
        }
    }
    if saw_selected_producer {
        return if selected_producers.is_empty() {
            Err(EnqueueBuildActionResult::QueueFull)
        } else {
            Ok(selected_producers)
        };
    }

    let mut saw_producer = false;
    for (entity, structure, structure_team, transform, under_construction) in structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && faction.can_produce(structure.id, product_id)
        {
            saw_producer = true;
            if build_queue_has_capacity(build_queue, entity) {
                return Ok(vec![(entity, structure.id, transform.translation)]);
            }
        }
    }
    if saw_producer {
        Err(EnqueueBuildActionResult::QueueFull)
    } else {
        Err(EnqueueBuildActionResult::Unavailable)
    }
}

fn command_origins_for(
    team: Team,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    structures: &Query<StructureEntityItem<'_>>,
    build_queue: &BuildQueue,
) -> Result<Vec<(Entity, &'static str, Vec3)>, EnqueueBuildActionResult> {
    let mut saw_selected_command_center = false;
    let mut selected_command_centers = Vec::new();
    for (entity, structure, structure_team, transform, under_construction) in selected_structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && structure.id == "CommandCenter"
        {
            saw_selected_command_center = true;
            if build_queue_has_capacity(build_queue, entity) {
                selected_command_centers.push((entity, "CommandCenter", transform.translation));
            }
        }
    }
    if saw_selected_command_center {
        return if selected_command_centers.is_empty() {
            Err(EnqueueBuildActionResult::QueueFull)
        } else {
            Ok(selected_command_centers)
        };
    }

    let mut saw_command_center = false;
    for (entity, structure, structure_team, transform, under_construction) in structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && structure.id == "CommandCenter"
        {
            saw_command_center = true;
            if build_queue_has_capacity(build_queue, entity) {
                return Ok(vec![(entity, "CommandCenter", transform.translation)]);
            }
        }
    }
    if saw_command_center {
        Err(EnqueueBuildActionResult::QueueFull)
    } else {
        Err(EnqueueBuildActionResult::Unavailable)
    }
}

fn has_producer_for_job(
    job: &BuildJob,
    structures: &Query<StructureEntityItem<'_>>,
    player_factions: &PlayerFactions,
) -> bool {
    let Ok((_, structure, team, _, under_construction)) = structures.get(job.producer_entity)
    else {
        return false;
    };
    if !structure_is_constructed(under_construction) {
        return false;
    }
    match job.action {
        BuildAction::Train(_) => {
            *team == job.team
                && structure.id == job.producer_id
                && faction_def(player_factions.slot_faction(job.team)).is_some_and(|faction| {
                    faction.can_produce(structure.id, build_target_product(job.action))
                })
        }
        BuildAction::Build(_) => {
            job.producer_id == "CommandCenter"
                && *team == job.team
                && structure.id == "CommandCenter"
        }
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => false,
    }
}

fn build_target_product(action: BuildAction) -> &'static str {
    match action {
        BuildAction::Train(product) | BuildAction::Build(product) => product,
        BuildAction::DeployMcv
        | BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::None => "",
    }
}

fn requirements_met(
    entity: &registry::EntityDef,
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    if entity.requirements.is_empty() {
        return true;
    }
    for requirement in entity.requirements {
        let has_requirement =
            structures
                .iter()
                .any(|(structure, structure_team, _, under_construction)| {
                    structure_is_constructed(under_construction)
                        && *structure_team == team
                        && structure.id == *requirement
                });
        if !has_requirement {
            return false;
        }
    }
    true
}

fn support_requirements_met(
    team: Team,
    required: &[&'static str],
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    if required.is_empty() {
        return true;
    }
    for requirement in required {
        if !structures
            .iter()
            .any(|(structure, structure_team, _, under_construction)| {
                structure_is_constructed(under_construction)
                    && structure_team == &team
                    && structure.id == *requirement
            })
        {
            return false;
        }
    }
    true
}

fn economy_tick(
    time: Res<Time>,
    mut economies: ResMut<Economies>,
    structures: Query<(&Structure, &Team, Option<&UnderConstruction>)>,
    mut income_sources: Query<(&Team, &mut IncomeSource, Option<&UnderConstruction>)>,
) {
    let mut power: Vec<(i32, i32)> = Vec::new();

    for (structure, team, under_construction) in &structures {
        let Some(idx) = team.economy_index() else {
            continue;
        };
        let Some(delta) = active_structure_power_delta(structure, under_construction) else {
            continue;
        };
        if power.len() <= idx {
            power.resize(idx + 1, (0, 0));
        }
        let target = &mut power[idx];
        if delta >= 0 {
            target.1 += delta;
        } else {
            target.0 += -delta;
        }
    }

    let team_count = economies.players.len().max(power.len());
    for team in player_teams(team_count) {
        let idx = team.index();
        let economy = economies.get_mut(team);
        economy.power_sabotage_remaining =
            (economy.power_sabotage_remaining - time.delta_secs()).max(0.0);
        let (power_used, power_capacity) = power.get(idx).copied().unwrap_or((0, 0));
        economy.power_used = power_used;
        economy.power_capacity = if economy.power_sabotage_remaining > 0.0 {
            0
        } else {
            power_capacity
        };
    }

    for (team, mut income, under_construction) in &mut income_sources {
        let can_pay =
            structure_is_constructed(under_construction) && team.economy_index().is_some();
        let (ore, crystal) = advance_income_source(&mut income, time.delta_secs(), can_pay);
        if ore != 0 || crystal != 0 {
            let economy = economies.get_mut(*team);
            economy.ore += ore;
            economy.crystal += crystal;
        }
    }
}

fn advance_income_source(
    income: &mut IncomeSource,
    delta_seconds: f32,
    can_pay: bool,
) -> (i32, i32) {
    income.remaining -= delta_seconds;
    let interval = income.interval.max(0.1);
    let mut ore = 0;
    let mut crystal = 0;
    while income.remaining <= 0.0 {
        if can_pay {
            ore += income.ore;
            crystal += income.crystal;
        }
        income.remaining += interval;
    }
    (ore, crystal)
}

fn auto_assign_ai_construction_workers(
    mut commands: Commands,
    time: Res<Time>,
    mut director: ResMut<AiDirector>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    workers: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Unit,
            &Health,
            Option<&OrderQueue>,
        ),
        (With<Unit>, IdleUnitOrderFilter),
    >,
    active_constructors: Query<(&Team, &Health), (With<Unit>, With<ConstructOrder>)>,
    structures: Query<
        (Entity, &Team, &Transform, &Health),
        (With<Structure>, With<UnderConstruction>),
    >,
) {
    let delta = time.delta_secs();
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, active_teams.as_deref()) {
        let Some(idx) = director.ensure_team(team) else {
            continue;
        };
        director.construction_timer[idx] -= delta;
        if director.construction_timer[idx] > 0.0 {
            continue;
        }
        director.construction_timer[idx] = AI_CONSTRUCTION_REFRESH_INTERVAL_SECONDS;
        if active_constructors
            .iter()
            .any(|(worker_team, health)| *worker_team == team && health.current > 0.0)
        {
            continue;
        }

        let mut idle_workers = Vec::new();
        for (
            worker_entity,
            worker_team,
            worker_transform,
            worker_unit,
            worker_health,
            order_queue,
        ) in &workers
        {
            if *worker_team != team
                || worker_health.current <= 0.0
                || !can_unit_construct_structures(worker_unit)
                || order_queue.is_some_and(|queue| !queue.orders.is_empty())
            {
                continue;
            }
            idle_workers.push((worker_entity, worker_transform.translation));
        }
        let unfinished_structures = structures
            .iter()
            .filter_map(
                |(structure_entity, structure_team, structure_transform, health)| {
                    (*structure_team == team && health.current > 0.0)
                        .then_some((structure_entity, structure_transform.translation))
                },
            )
            .collect::<Vec<_>>();
        let best_assignment =
            closest_construction_assignment(&idle_workers, &unfinished_structures);

        if let Some((worker_entity, structure_entity)) = best_assignment {
            issue_unit_order(
                &mut commands,
                worker_entity,
                UnitQueuedOrder::Construct(structure_entity),
            );
        }
    }
}

fn closest_construction_assignment(
    workers: &[(Entity, Vec3)],
    structures: &[(Entity, Vec3)],
) -> Option<(Entity, Entity)> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (worker_entity, worker_position) in workers {
        for (structure_entity, structure_position) in structures {
            let distance = xz_distance(*worker_position, *structure_position);
            if distance < best_distance {
                best_distance = distance;
                best = Some((*worker_entity, *structure_entity));
            }
        }
    }
    best
}

fn auto_assign_ai_resource_collectors(
    mut commands: Commands,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    units: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Health,
            &ResourceCargo,
            Option<&OrderQueue>,
        ),
        (With<Unit>, IdleUnitOrderFilter),
    >,
    resources: Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
    dropoffs: Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, team, transform, health, cargo, order_queue) in &units {
        if *team == player_team
            || !team_is_active(*team, active_teams.as_deref())
            || health.current <= 0.0
            || cargo.capacity <= 0
        {
            continue;
        }
        if order_queue.is_some_and(|queue| !queue.orders.is_empty()) {
            continue;
        }
        if nearest_resource_dropoff(*team, transform.translation, &dropoffs).is_none() {
            continue;
        }
        let nearest_resource = nearest_resource_entity(
            transform.translation,
            &resources,
            Some(RESOURCE_SEARCH_RADIUS_M),
        );
        let state = if cargo.has_any() && (cargo.is_full() || nearest_resource.is_none()) {
            HarvestState::MovingToDropoff
        } else {
            HarvestState::MovingToResource
        };
        if nearest_resource.is_none() && state == HarvestState::MovingToResource {
            continue;
        }
        commands.entity(entity).try_insert(HarvestOrder {
            resource: nearest_resource,
            state,
            collect_remaining: 0.0,
        });
    }
}

fn auto_assign_ai_supply_crate_collectors(
    mut commands: Commands,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    crates: Query<(Entity, &Transform, &SupplyCrate)>,
    team_anchors: Query<(&Team, &Transform), Or<(With<Unit>, With<Structure>)>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            &Health,
            &MovementDomain,
            &HoldPosition,
            Option<&Weapon>,
            Option<&OrderQueue>,
        ),
        With<Unit>,
    >,
    busy_units: Query<Entity, ActiveUnitOrderFilter>,
) {
    let crate_snapshots = crates
        .iter()
        .map(|(entity, transform, supply_crate)| {
            (entity, transform.translation, supply_crate.effect)
        })
        .collect::<Vec<_>>();
    if crate_snapshots.is_empty() {
        return;
    }

    for team in active_ai_teams(
        controlled_player_team(visible_player.as_deref()),
        active_teams.as_deref(),
    ) {
        let mut assignments = 0usize;
        let mut assigned_units = Vec::new();
        let mut preferred_crates = crate_snapshots.clone();
        preferred_crates.sort_by(|a, b| {
            let a_distance = ai_supply_crate_distance_to_team_units(team, a.1, &team_anchors);
            let b_distance = ai_supply_crate_distance_to_team_units(team, b.1, &team_anchors);
            a_distance.total_cmp(&b_distance)
        });

        for (_, crate_position, _effect) in preferred_crates {
            if assignments >= AI_SUPPLY_CRATE_COLLECTION_LIMIT {
                break;
            }
            let mut best = None;
            let mut best_score = f32::MAX;
            for (
                entity,
                unit,
                unit_team,
                transform,
                health,
                domain,
                hold_position,
                weapon,
                order_queue,
            ) in &units
            {
                if *unit_team != team
                    || health.current <= 0.0
                    || *domain != MovementDomain::Terrain
                    || hold_position.enabled
                    || weapon.is_none()
                    || assigned_units.contains(&entity)
                {
                    continue;
                }
                if busy_units.contains(entity)
                    || order_queue.is_some_and(|queue| !queue.orders.is_empty())
                {
                    continue;
                }
                let scout_bonus = if unit.id == "ScoutRover" { -20.0 } else { 0.0 };
                let score = xz_distance(transform.translation, crate_position) + scout_bonus;
                if score < best_score {
                    best = Some(entity);
                    best_score = score;
                }
            }

            if let Some(entity) = best {
                issue_unit_order(&mut commands, entity, UnitQueuedOrder::Move(crate_position));
                assigned_units.push(entity);
                assignments += 1;
            }
        }
    }
}

fn ai_supply_crate_distance_to_team_units(
    team: Team,
    crate_position: Vec3,
    team_anchors: &Query<(&Team, &Transform), Or<(With<Unit>, With<Structure>)>>,
) -> f32 {
    let mut closest_distance = f32::INFINITY;
    for (anchor_team, transform) in team_anchors {
        if *anchor_team == team {
            closest_distance =
                closest_distance.min(xz_distance(crate_position, transform.translation));
        }
    }
    closest_distance
}

fn update_ai_tech_bunker_garrisons(
    mut commands: Commands,
    time: Res<Time>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    mut refresh_timer: Local<Vec<f32>>,
    bunkers: Query<AiOpenBunkerItem<'_>, (With<Structure>, Without<Unit>)>,
    units: Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) {
    let delta = time.delta_secs();
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, active_teams.as_deref()) {
        let idx = team.index();
        if refresh_timer.len() <= idx {
            refresh_timer.resize(idx + 1, 0.0);
        }
        refresh_timer[idx] -= delta;
        if refresh_timer[idx] > 0.0 {
            continue;
        }
        refresh_timer[idx] = AI_TECH_BUNKER_GARRISON_REFRESH_INTERVAL_SECONDS;
        garrison_ai_tech_bunkers(&mut commands, team, &bunkers, &units);
    }
}

fn garrison_ai_tech_bunkers(
    commands: &mut Commands,
    team: Team,
    bunkers: &Query<AiOpenBunkerItem<'_>, (With<Structure>, Without<Unit>)>,
    units: &Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) {
    let mut open_bunkers = bunkers
        .iter()
        .filter_map(
            |(entity, structure, bunker_team, transform, health, garrison, under_construction)| {
                (*bunker_team == team
                    && structure.id == "TechBunker"
                    && health.current > 0.0
                    && structure_is_constructed(under_construction)
                    && garrison.count < garrison.capacity)
                    .then_some((
                        entity,
                        transform.translation,
                        garrison.count,
                        garrison.capacity,
                    ))
            },
        )
        .collect::<Vec<_>>();
    open_bunkers.sort_by_key(|(_, _, count, _)| *count);

    let mut assigned_units = Vec::new();
    for (bunker_entity, bunker_position, count, capacity) in open_bunkers {
        for _ in count..capacity {
            let Some(unit_entity) =
                best_available_ai_garrison_unit(team, bunker_position, &assigned_units, units)
            else {
                break;
            };
            issue_unit_order(
                commands,
                unit_entity,
                UnitQueuedOrder::Garrison(bunker_entity),
            );
            assigned_units.push(unit_entity);
        }
    }
}

fn best_available_ai_garrison_unit(
    team: Team,
    bunker_position: Vec3,
    assigned_units: &[Entity],
    units: &Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (entity, unit, unit_team, transform, health, order_queue) in units {
        if *unit_team != team
            || health.current <= 0.0
            || !can_unit_garrison(unit)
            || assigned_units.contains(&entity)
            || order_queue.is_some_and(|queue| !queue.orders.is_empty())
        {
            continue;
        }
        let distance = xz_distance(transform.translation, bunker_position);
        if distance <= AI_TECH_BUNKER_GARRISON_SEARCH_RADIUS && distance < best_distance {
            best = Some(entity);
            best_distance = distance;
        }
    }
    best
}

fn try_activate_ai_support_power(
    team: Team,
    player_team: Team,
    commands: &mut Commands,
    economies: &Economies,
    support_cooldowns: &mut SupportCooldowns,
    battle_log: &mut BattleLog,
    audio_feedback: &mut AudioFeedback,
    relations: &TeamRelations,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structure_targets: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> bool {
    for power in AI_SUPPORT_POWER_PRIORITY {
        if !ai_support_power_available(team, power, economies, support_cooldowns, structures) {
            continue;
        }
        let Some(target) =
            ai_support_power_target(team, power, relations, units, structure_targets)
        else {
            continue;
        };
        let support_targets = ai_support_power_targets(units, structure_targets);
        if activate_support_power(
            commands,
            target,
            power,
            team,
            player_team,
            economies,
            support_cooldowns,
            battle_log,
            relations,
            structures,
            &support_targets,
        ) {
            record_support_power_audio_feedback(audio_feedback, team, player_team, power);
            return true;
        }
    }
    false
}

fn ai_support_power_available(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    support_cooldowns: &SupportCooldowns,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    support_cooldowns.ready(team, power)
        && (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

fn ai_support_power_targets(
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Vec<SupportPowerTargetSnapshot> {
    let mut targets = units
        .iter()
        .map(
            |(entity, team, transform, _selectable, health, unit)| SupportPowerTargetSnapshot {
                entity,
                team: *team,
                position: transform.translation,
                health: *health,
                mobile: unit.speed > 0.0,
            },
        )
        .collect::<Vec<_>>();
    targets.extend(structures.iter().map(
        |(entity, _structure, team, transform, health, _under_construction)| {
            SupportPowerTargetSnapshot {
                entity,
                team: *team,
                position: transform.translation,
                health: *health,
                mobile: false,
            }
        },
    ));
    targets
}

fn support_power_available_for_audio(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

fn ai_support_power_target(
    team: Team,
    power: SupportPowerKind,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Vec3> {
    let def = power.definition();
    match power {
        SupportPowerKind::NaniteRepairSwarm => best_repair_swarm_position(
            team,
            units,
            def.radius,
            def.healing,
            AI_SUPPORT_NANITE_REPAIR_MIN_MISSING_HP,
        ),
        SupportPowerKind::EmpPulse => best_mobile_unit_cluster_position(
            team,
            units,
            def.radius,
            false,
            relations,
            AI_SUPPORT_MIN_CLUSTER_TARGETS,
        ),
        SupportPowerKind::ShieldOverdrive => best_shield_overdrive_position(
            team,
            units,
            def.radius,
            relations,
            AI_SUPPORT_SHIELD_OVERDRIVE_MIN_SCORE,
        ),
        SupportPowerKind::ChronoRelay => best_mobile_unit_cluster_position(
            team,
            units,
            def.radius,
            true,
            relations,
            AI_SUPPORT_CHRONO_RELAY_MIN_MOBILE_UNITS,
        ),
        SupportPowerKind::WeatherStorm => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_WEATHER_STORM_MIN_SCORE,
        ),
        SupportPowerKind::StrategicMissile => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_STRATEGIC_MISSILE_MIN_SCORE,
        ),
        SupportPowerKind::OrbitalStrike => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_ORBITAL_STRIKE_MIN_SCORE,
        ),
        SupportPowerKind::RadarSweep | SupportPowerKind::Paradrop => {
            any_enemy_support_target_position(team, relations, units, structures)
        }
    }
}

fn best_repair_swarm_position(
    team: Team,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    radius: f32,
    healing: f32,
    min_missing_hp: f32,
) -> Option<Vec3> {
    let mut best = None;
    let mut best_score = 0.0;
    for (_, unit_team, transform, _selectable, health, _unit) in units {
        if *unit_team != team || health.current <= 0.0 {
            continue;
        }
        let score = units
            .iter()
            .filter_map(
                |(_, other_team, other_transform, _, other_health, _other_unit)| {
                    if *other_team != team
                        || other_health.current <= 0.0
                        || other_health.max <= other_health.current
                        || xz_distance(other_transform.translation, transform.translation) > radius
                    {
                        return None;
                    }
                    Some((other_health.max - other_health.current).min(healing))
                },
            )
            .sum::<f32>();
        if score > best_score {
            best = Some(transform.translation);
            best_score = score;
        }
    }
    (best_score >= min_missing_hp).then_some(best).flatten()
}

fn best_mobile_unit_cluster_position(
    team: Team,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    radius: f32,
    friendly: bool,
    relations: &TeamRelations,
    min_count: usize,
) -> Option<Vec3> {
    let mut best = None;
    let mut best_count = 0usize;
    for (_, unit_team, transform, _selectable, health, unit) in units {
        if !ai_support_unit_side_matches(team, *unit_team, friendly, relations)
            || health.current <= 0.0
            || unit.speed <= 0.0
        {
            continue;
        }
        let mut count = 0usize;
        for (_, other_team, other_transform, _other_selectable, other_health, other_unit) in units {
            if !ai_support_unit_side_matches(team, *other_team, friendly, relations)
                || other_health.current <= 0.0
                || other_unit.speed <= 0.0
                || xz_distance(other_transform.translation, transform.translation) > radius
            {
                continue;
            }
            count += 1;
        }
        if count >= min_count && count > best_count {
            best_count = count;
            best = Some(transform.translation);
        }
    }
    best
}

fn best_shield_overdrive_position(
    team: Team,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    radius: f32,
    relations: &TeamRelations,
    min_score: f32,
) -> Option<Vec3> {
    let mut best = None;
    let mut best_score = 0.0;
    let pressure_radius = radius + AI_SUPPORT_SHIELD_PRESSURE_EXTRA_RADIUS;
    for (_, unit_team, transform, _selectable, health, unit) in units {
        if *unit_team != team || health.current <= 0.0 {
            continue;
        }
        let pressure_distance = nearest_enemy_pressure_distance(
            team,
            transform.translation,
            pressure_radius,
            relations,
            units,
        );
        if !pressure_distance.is_finite() {
            continue;
        }
        let mut score = units
            .iter()
            .filter_map(
                |(_, other_team, other_transform, _, other_health, other_unit)| {
                    if *other_team != team
                        || other_health.current <= 0.0
                        || xz_distance(other_transform.translation, transform.translation) > radius
                    {
                        return None;
                    }
                    Some(shield_target_score(other_unit))
                },
            )
            .sum::<f32>();
        if unit.speed > 0.0 {
            score += AI_SUPPORT_SHIELD_OVERDRIVE_MOBILE_PRESSURE_BONUS;
        }
        score += (pressure_radius - pressure_distance).max(0.0)
            * AI_SUPPORT_SHIELD_PRESSURE_DISTANCE_WEIGHT;
        if score > best_score {
            best_score = score;
            best = Some(transform.translation);
        }
    }
    (best_score >= min_score).then_some(best).flatten()
}

fn nearest_enemy_pressure_distance(
    team: Team,
    position: Vec3,
    radius: f32,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) -> f32 {
    let mut best_distance = f32::INFINITY;
    for (_, unit_team, transform, _, health, _) in units {
        if !relations.are_enemies(team, *unit_team) || health.current <= 0.0 {
            continue;
        }
        let distance = xz_distance(position, transform.translation);
        if distance <= radius {
            best_distance = best_distance.min(distance);
        }
    }
    best_distance
}

fn shield_target_score(unit: &Unit) -> f32 {
    let mut score = 1.0;
    if registry::entity(unit.id).is_some_and(|def| def.weapon.is_some()) {
        score += 1.0;
    }
    score
}

fn best_scored_strike_position(
    team: Team,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
    relations: &TeamRelations,
    radius: f32,
    min_score: f32,
) -> Option<Vec3> {
    let mut best = None;
    let mut best_score = 0.0;
    for (_, unit_team, transform, _, health, _unit) in units {
        if !relations.are_enemies(team, *unit_team) || health.current <= 0.0 {
            continue;
        }
        let score = strike_score_at_position(
            team,
            transform.translation,
            radius,
            relations,
            units,
            structures,
        );
        if score > best_score {
            best_score = score;
            best = Some(transform.translation);
        }
    }
    for (_, _structure, structure_team, transform, health, under_construction) in structures {
        if !relations.are_enemies(team, *structure_team)
            || health.current <= 0.0
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        let score = strike_score_at_position(
            team,
            transform.translation,
            radius,
            relations,
            units,
            structures,
        );
        if score > best_score {
            best = Some(transform.translation);
            best_score = score;
        }
    }
    (best_score >= min_score).then_some(best).flatten()
}

fn strike_score_at_position(
    team: Team,
    position: Vec3,
    radius: f32,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> f32 {
    let unit_score = units
        .iter()
        .filter_map(|(_, unit_team, transform, _, health, unit)| {
            if !relations.are_enemies(team, *unit_team)
                || health.current <= 0.0
                || xz_distance(transform.translation, position) > radius
            {
                return None;
            }
            Some(ai_strike_unit_score(unit))
        })
        .sum::<f32>();
    let structure_score = structures
        .iter()
        .filter_map(
            |(_, structure, structure_team, transform, health, under_construction)| {
                if !relations.are_enemies(team, *structure_team)
                    || health.current <= 0.0
                    || !structure_is_constructed(under_construction)
                    || xz_distance(transform.translation, position) > radius
                {
                    return None;
                }
                Some(ai_strike_structure_score(structure))
            },
        )
        .sum::<f32>();
    unit_score + structure_score
}

fn ai_strike_unit_score(unit: &Unit) -> f32 {
    let mut score = 1.0;
    if let Some(def) = registry::entity(unit.id) {
        if def.weapon.is_some() {
            score += 1.0;
        }
        score += ai_resource_score(def.cost) * 0.5;
    }
    score
}

fn ai_strike_structure_score(structure: &Structure) -> f32 {
    let mut score = 3.5;
    if let Some(def) = registry::entity(structure.id) {
        if def.weapon.is_some() {
            score += 1.0;
        }
        score += ai_resource_score(def.cost);
    }
    score
}

fn ai_resource_score(cost: registry::Cost) -> f32 {
    ((cost.ore + cost.crystal) as f32 / 8.0).min(2.0)
}

fn any_enemy_support_target_position(
    team: Team,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Vec3> {
    units
        .iter()
        .find_map(|(_, unit_team, transform, _, health, _)| {
            (relations.are_enemies(team, *unit_team) && health.current > 0.0)
                .then_some(transform.translation)
        })
        .or_else(|| {
            structures.iter().find_map(
                |(_, _, structure_team, transform, health, under_construction)| {
                    (relations.are_enemies(team, *structure_team)
                        && health.current > 0.0
                        && structure_is_constructed(under_construction))
                    .then_some(transform.translation)
                },
            )
        })
}

fn ai_support_unit_side_matches(
    team: Team,
    unit_team: Team,
    friendly: bool,
    relations: &TeamRelations,
) -> bool {
    if friendly {
        unit_team == team
    } else {
        relations.are_enemies(team, unit_team)
    }
}

fn ai_needs_more_anti_air_units(
    team: Team,
    units: impl IntoIterator<Item = (&'static str, Team)>,
) -> bool {
    let mut enemy_air_units = 0usize;
    let mut anti_air_responses = 0usize;
    for (unit_id, unit_team) in units {
        if unit_team == Team::Neutral {
            continue;
        }
        if unit_team == team {
            if ai_unit_can_attack_air(unit_id) {
                anti_air_responses += 1;
            }
        } else if ai_unit_is_air(unit_id) {
            enemy_air_units += 1;
        }
    }
    enemy_air_units > 0 && anti_air_responses < enemy_air_units
}

fn ai_unit_is_air(unit_id: &str) -> bool {
    registry::entity(unit_id).is_some_and(|def| matches!(def.domain, registry::MoveDomain::Air))
}

fn ai_unit_can_attack_air(unit_id: &str) -> bool {
    registry::entity(unit_id)
        .and_then(|def| def.weapon)
        .is_some_and(|weapon| weapon.can_attack_air)
}

fn ai_battle_unit_id(unit_id: &str) -> bool {
    if matches!(
        unit_id,
        "Worker" | "OreHarvester" | "MobileConstructionVehicle" | AI_SABOTEUR_ID
    ) {
        return false;
    }
    registry::entity(unit_id).is_some_and(|def| {
        def.speed > 0.0
            && (def.weapon.is_some()
                || def.repair_rate > 0.0
                || def.healing_rate > 0.0
                || def.support_shield_radius > 0.0
                || def.mine_deploy_radius > 0.0)
    })
}

fn ai_battlegroup_target_units(profile: &TeamAiProfile) -> usize {
    profile.expected_battlegroups * profile.expected_units_in_battlegroup
}

fn ai_battlegroup_candidate_allowed(
    candidate: &'static str,
    profile: &TeamAiProfile,
    counts: AiProductionCounts,
) -> bool {
    if ai_training_is_economy_request(candidate) || !ai_battle_unit_id(candidate) {
        return true;
    }
    if !profile.active_offense_enabled {
        return false;
    }
    let target_units = ai_battlegroup_target_units(profile);
    target_units > 0 && counts.battle_units < target_units
}

fn ai_battlegroup_repair_target(
    team: Team,
    repairer: Entity,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) -> Option<Entity> {
    let repairer_position = units
        .get(repairer)
        .ok()
        .map(|(_, _, transform, _, _, _)| transform.translation);
    let mut best = None;
    let mut best_missing_ratio = 0.0;
    let mut best_distance = f32::MAX;

    for (entity, unit_team, transform, _selectable, health, unit) in units {
        if entity == repairer
            || *unit_team != team
            || health.current <= 0.0
            || health.current >= health.max
            || !ai_battle_unit_id(unit.id)
        {
            continue;
        }

        let missing_ratio = if health.max > 0.0 {
            1.0 - (health.current / health.max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let distance = repairer_position
            .map(|position| xz_distance(position, transform.translation))
            .unwrap_or(0.0);
        if missing_ratio > best_missing_ratio
            || (missing_ratio == best_missing_ratio && distance < best_distance)
        {
            best = Some(entity);
            best_missing_ratio = missing_ratio;
            best_distance = distance;
        }
    }

    best
}

fn assign_ai_attack_wave_order(
    commands: &mut Commands,
    team: Team,
    entity: Entity,
    unit: &Unit,
    target: Entity,
    support_units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) {
    let Some(def) = registry::entity(unit.id) else {
        return;
    };
    let mut entity_commands = commands.entity(entity);
    entity_commands.try_insert(AiAttackWaveMember);
    if repair_capability(unit).is_some()
        && let Some(repair_target) = ai_battlegroup_repair_target(team, entity, support_units)
    {
        entity_commands.try_insert(RepairOrder {
            target: repair_target,
        });
    } else if def.weapon.is_some() {
        entity_commands.try_insert(AttackOrder { target });
    } else {
        entity_commands.try_insert(FollowOrder {
            target,
            allow_enemy: true,
            offset: Vec3::ZERO,
        });
    }
}

fn restore_ai_attack_wave_orders(
    mut commands: Commands,
    ai_settings: Res<AiDifficultySettings>,
    player_factions: Option<Res<PlayerFactions>>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        (With<AiAttackWaveMember>, With<Unit>, Without<Selected>),
    >,
    support_units: Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    targets: Query<(Entity, &Team, &Transform), With<Health>>,
) {
    for team in active_ai_teams(
        controlled_player_team(visible_player.as_deref()),
        active_teams.as_deref(),
    ) {
        let profile = faction_ai_profile_for_difficulty(
            slot_faction_from_option(player_factions.as_deref(), team),
            ai_settings.difficulty(team),
        );
        if !profile.active_offense_enabled {
            continue;
        }
        let Some(target) = nearest_enemy_entity(team, team_home(team), &targets) else {
            continue;
        };
        for (
            entity,
            unit,
            unit_team,
            order_queue,
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
        ) in &units
        {
            if *unit_team != team
                || !ai_battle_unit_id(unit.id)
                || !is_unit_idle(
                    order_queue,
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
                )
            {
                continue;
            }
            assign_ai_attack_wave_order(&mut commands, team, entity, unit, target, &support_units);
        }
    }
}

fn ai_director(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut resources: AiDirectorResources,
    structures: Query<StructurePrereqItem<'_>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    support_units: Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    capture_structures: Query<CaptureStructureTargetItem<'_>, With<Structure>>,
    ai_repair_structures: Query<AiRepairStructureItem<'_>, With<Structure>>,
    targets: Query<(Entity, &Team, &Transform), With<Health>>,
) {
    let delta = time.delta_secs();
    let player_team = visible_player_team(visible_player.as_deref());
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, resources.active_teams.as_deref()) {
        let Some(idx) = resources.director.ensure_team(team) else {
            continue;
        };
        let faction = resources.player_factions.slot_faction(team);
        let profile =
            faction_ai_profile_for_difficulty(faction, resources.ai_settings.difficulty(team));

        resources.director.support_timer[idx] -= delta;
        if profile.support_powers_enabled && resources.director.support_timer[idx] <= 0.0 {
            resources.director.support_timer[idx] = profile.support_interval;
            let _ = try_activate_ai_support_power(
                team,
                player_team,
                &mut commands,
                &resources.economies,
                &mut resources.support_cooldowns,
                &mut resources.battle_log,
                &mut resources.audio_feedback,
                &resources.relations,
                &structures,
                &support_units,
                &capture_structures,
            );
        } else if !profile.support_powers_enabled {
            resources.director.support_timer[idx] = profile.support_interval;
        }

        resources.director.repair_timer[idx] -= delta;
        if resources.director.repair_timer[idx] <= 0.0 {
            resources.director.repair_timer[idx] = AI_REPAIR_REFRESH_INTERVAL_SECONDS;
            let _ = repair_ai_damaged_structures(
                &mut commands,
                team,
                &ai_repair_structures,
                &mut resources.economies,
            );
        }

        resources.director.production_timer[idx] -= delta;
        let mut production_refresh_due = false;
        let mut trained_during_priority_refresh = false;
        if resources.director.production_timer[idx] <= 0.0 {
            production_refresh_due = true;
            resources.director.production_timer[idx] = profile.production_interval;
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let economy_snapshot = resources.economies.get(team).clone();
            if let Some(id) = next_ai_economy_train(
                team,
                faction,
                &profile,
                &structures,
                &economy_snapshot,
                production_counts,
            ) {
                trained_during_priority_refresh = try_spawn_ai_trained_unit(
                    &mut commands,
                    &asset_server,
                    &mut resources.economies,
                    &mut resources.next_id,
                    team,
                    faction,
                    id,
                    &structures,
                    *resources.map_bounds,
                    player_team,
                );
            }
        }

        resources.director.build_timer[idx] -= delta;
        if resources.director.build_timer[idx] <= 0.0 {
            resources.director.build_timer[idx] = profile.build_interval;
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let next_structure = next_ai_economy_structure_for_faction(
                team,
                faction,
                &profile,
                &structures,
                production_counts,
            )
            .map(|id| (id, AiStructureBuildKind::Economy))
            .or_else(|| {
                next_ai_defense_for_faction(team, faction, &profile, &structures)
                    .map(|id| (id, AiStructureBuildKind::Defense))
            })
            .or_else(|| {
                profile
                    .active_offense_enabled
                    .then(|| next_ai_offense_structure_for_faction(team, faction, &structures))
                    .flatten()
                    .map(|id| (id, AiStructureBuildKind::Offense))
            });
            if let Some((id, build_kind)) = next_structure
                && let Some(def) = registry::entity(id)
                && requirements_met(def, team, &structures)
                && let Some(origin) =
                    ai_structure_build_origin(team, build_kind, &structures, &targets)
                && resources.economies.get_mut(team).spend(def.cost)
            {
                let spawn_at = ai_structure_build_position(
                    team,
                    origin,
                    id,
                    build_kind,
                    resources.next_id.0,
                    &targets,
                    *resources.map_bounds,
                );
                spawn_structure_under_construction_for_faction(
                    &mut commands,
                    &asset_server,
                    &mut resources.next_id,
                    id,
                    team,
                    spawn_at,
                    (id == "Refinery").then_some(origin),
                    0.0,
                    player_team,
                    faction,
                );
            }
        }

        if production_refresh_due && !trained_during_priority_refresh {
            let needs_anti_air =
                ai_needs_more_anti_air_units(team, units.iter().map(|item| (item.1.id, *item.2)));
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let economy_snapshot = resources.economies.get(team).clone();
            let next_training = next_ai_train(
                team,
                faction,
                &profile,
                &structures,
                &economy_snapshot,
                &mut resources.director.production_cursor[idx],
                production_counts,
                needs_anti_air,
            );
            if let Some(id) = next_training
                && !ai_training_is_economy_request(id)
            {
                let _ = try_spawn_ai_trained_unit(
                    &mut commands,
                    &asset_server,
                    &mut resources.economies,
                    &mut resources.next_id,
                    team,
                    faction,
                    id,
                    &structures,
                    *resources.map_bounds,
                    player_team,
                );
            }
        }

        resources.director.attack_timer[idx] -= delta;
        if profile.active_offense_enabled && resources.director.attack_timer[idx] <= 0.0 {
            resources.director.attack_timer[idx] = profile.attack_interval;
            if let Some(target) = nearest_enemy_entity(team, team_home(team), &targets) {
                for (
                    entity,
                    unit,
                    unit_team,
                    _transform,
                    order_queue,
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
                ) in &units
                {
                    if *unit_team != team
                        || !ai_battle_unit_id(unit.id)
                        || order_queue.is_some_and(|queue| !queue.orders.is_empty())
                        || has_active_orders_in_query(
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
                        )
                    {
                        continue;
                    }
                    assign_ai_attack_wave_order(
                        &mut commands,
                        team,
                        entity,
                        unit,
                        target,
                        &support_units,
                    );
                }
            }
        } else if !profile.active_offense_enabled {
            resources.director.attack_timer[idx] = profile.attack_interval;
        }

        resources.director.capture_timer[idx] -= delta;
        if profile.capture_enabled && resources.director.capture_timer[idx] <= 0.0 {
            resources.director.capture_timer[idx] = profile.capture_interval;
            run_ai_capture_logic(
                team,
                faction,
                &mut commands,
                &asset_server,
                &mut resources.economies,
                &mut resources.next_id,
                player_team,
                &structures,
                &units,
                &capture_structures,
            );
        } else if !profile.capture_enabled {
            resources.director.capture_timer[idx] = profile.capture_interval;
        }

        resources.director.saboteur_timer[idx] -= delta;
        if profile.saboteur_enabled && resources.director.saboteur_timer[idx] <= 0.0 {
            resources.director.saboteur_timer[idx] = profile.saboteur_interval;
            run_ai_saboteur_logic(
                team,
                faction,
                &mut commands,
                &asset_server,
                &mut resources.economies,
                &mut resources.next_id,
                player_team,
                &structures,
                &units,
                &capture_structures,
            );
        } else if !profile.saboteur_enabled {
            resources.director.saboteur_timer[idx] = profile.saboteur_interval;
        }
    }
}

fn ai_training_is_economy_request(id: &str) -> bool {
    matches!(id, "Worker" | "OreHarvester")
}

fn next_ai_economy_train(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    if counts.workers < profile.expected_workers
        && let Some(worker_def) = registry::entity("Worker")
    {
        if !economy.can_afford(worker_def.cost) {
            return None;
        }
        if requirements_met(worker_def, team, structures)
            && ai_production_origin_for_faction(team, faction, "Worker", structures).is_some()
        {
            return Some("Worker");
        }
    }

    if counts.ore_harvesters < profile.expected_ore_harvesters
        && has_constructed_structure(team, "Refinery", structures)
        && let Some(harvester_def) = registry::entity("OreHarvester")
    {
        if !economy.can_afford(harvester_def.cost) {
            return None;
        }
        if requirements_met(harvester_def, team, structures)
            && ai_production_origin_for_faction(team, faction, "OreHarvester", structures).is_some()
        {
            return Some("OreHarvester");
        }
    }

    None
}

fn try_spawn_ai_trained_unit(
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
    map_bounds: MapBounds,
    player_team: Team,
) -> bool {
    let Some(def) = registry::entity(id) else {
        return false;
    };
    if !requirements_met(def, team, structures) {
        return false;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, id, structures)
    else {
        return false;
    };
    if !economies.get_mut(team).spend(def.cost) {
        return false;
    }

    let spawn_at = free_position_in_bounds(origin, next_id.0, 2.7, map_bounds);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        spawn_at,
        initial_rank,
        faction,
        player_team,
    );
    true
}

fn run_ai_capture_logic(
    team: Team,
    faction: SkirmishFaction,
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    visible_team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    capture_structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) {
    let mut engineer_count = 0usize;
    let mut idle_engineer = None;
    for (
        entity,
        unit,
        unit_team,
        transform,
        order_queue,
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
    ) in units
    {
        if *unit_team != team || unit.id != "EngineerDrone" {
            continue;
        }
        engineer_count += 1;
        if idle_engineer.is_none()
            && is_unit_idle(
                order_queue,
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
            )
        {
            idle_engineer = Some((entity, transform.translation));
        }
    }

    if let Some((entity, origin)) = idle_engineer {
        if let Some(target) = best_ai_capture_target(team, origin, capture_structures) {
            issue_unit_order(commands, entity, UnitQueuedOrder::Capture(target));
        }
        return;
    }

    if engineer_count >= AI_CAPTURE_ENGINEER_LIMIT {
        return;
    }
    let Some(def) = registry::entity("EngineerDrone") else {
        return;
    };
    if !requirements_met(def, team, structures) {
        return;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, "EngineerDrone", structures)
    else {
        return;
    };
    let Some(target) = best_ai_capture_target(team, origin, capture_structures) else {
        return;
    };
    if !economies.get_mut(team).spend(def.cost) {
        return;
    }

    let spawn_at = free_position(origin, next_id.0 + 13, 2.4);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    let engineer = spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        "EngineerDrone",
        team,
        spawn_at,
        initial_rank,
        faction,
        visible_team,
    );
    issue_unit_order(commands, engineer, UnitQueuedOrder::Capture(target));
}

fn run_ai_saboteur_logic(
    team: Team,
    faction: SkirmishFaction,
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    visible_team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    capture_structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) {
    let Some(saboteur_def) = registry::entity(AI_SABOTEUR_ID) else {
        return;
    };
    if saboteur_def.capture_time <= 0.0 {
        return;
    }

    let mut saboteur_count = 0usize;
    let mut idle_saboteur = None;
    for (
        entity,
        unit,
        unit_team,
        transform,
        order_queue,
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
    ) in units
    {
        if *unit_team != team || unit.id != AI_SABOTEUR_ID {
            continue;
        }
        saboteur_count += 1;
        if idle_saboteur.is_none()
            && order_queue.is_none_or(|queue| queue.orders.is_empty())
            && !has_active_orders_in_query(
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
            )
        {
            idle_saboteur = Some((entity, transform.translation));
        }
    }

    if let Some((entity, position)) = idle_saboteur {
        if let Some(target) =
            best_ai_saboteur_target(team, position, saboteur_def, economies, capture_structures)
        {
            issue_unit_order(commands, entity, UnitQueuedOrder::Capture(target));
        }
        return;
    }

    if saboteur_count >= AI_SABOTEUR_LIMIT
        || !requirements_met(saboteur_def, team, structures)
        || !economies.get(team).can_afford(saboteur_def.cost)
    {
        return;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, AI_SABOTEUR_ID, structures)
    else {
        return;
    };
    let Some(target) =
        best_ai_saboteur_target(team, origin, saboteur_def, economies, capture_structures)
    else {
        return;
    };
    if !economies.get_mut(team).spend(saboteur_def.cost) {
        return;
    }

    let spawn_at = free_position(origin, next_id.0 + 17, 2.2);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    let saboteur = spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        AI_SABOTEUR_ID,
        team,
        spawn_at,
        initial_rank,
        faction,
        visible_team,
    );
    issue_unit_order(commands, saboteur, UnitQueuedOrder::Capture(target));
}

fn best_ai_saboteur_target(
    team: Team,
    origin: Vec3,
    saboteur_def: &registry::EntityDef,
    economies: &Economies,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_score = f32::MIN;
    for (entity, structure, structure_team, transform, health, under_construction) in structures {
        if health.current <= 0.0
            || *structure_team == team
            || *structure_team == Team::Neutral
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        let Some(target_def) = registry::entity(structure.id) else {
            continue;
        };
        if !ai_saboteur_target_has_value(team, *structure_team, saboteur_def, target_def, economies)
        {
            continue;
        }
        let score = ai_saboteur_target_score(
            *structure_team,
            target_def,
            transform.translation,
            origin,
            economies,
        );
        if score > best_score {
            best_score = score;
            best = Some(entity);
        }
    }
    best
}

fn ai_saboteur_target_has_value(
    team: Team,
    victim_team: Team,
    saboteur_def: &registry::EntityDef,
    target_def: &registry::EntityDef,
    economies: &Economies,
) -> bool {
    if let Some(producer_id) = target_def.infiltration_production_veterancy_producer
        && saboteur_def.infiltration_production_veterancy_rank > 0
        && economies.get(team).production_veterancy_rank(producer_id)
            < saboteur_def.infiltration_production_veterancy_rank
    {
        return true;
    }
    if target_def.is_infiltration_resource_target {
        let victim = economies.get(victim_team);
        if victim.ore > 0 || victim.crystal > 0 {
            return true;
        }
    }
    if target_def.is_infiltration_power_sabotage_target
        && economies.get(victim_team).power_sabotage_remaining <= 0.0
    {
        return true;
    }
    false
}

fn ai_saboteur_target_score(
    victim_team: Team,
    target_def: &registry::EntityDef,
    target_position: Vec3,
    origin: Vec3,
    economies: &Economies,
) -> f32 {
    let mut score = match target_def.id {
        "Barracks" => 120.0,
        "VehicleFactory" => 116.0,
        "AircraftFactory" => 112.0,
        "AdvancedReactorPlant" => 106.0,
        "PowerReactor" => 104.0,
        "OrePurifier" => 96.0,
        "Refinery" => 88.0,
        "CommandCenter" => 82.0,
        _ => 30.0,
    };
    score += (target_def.cost.ore + target_def.cost.crystal) as f32;
    if target_def.is_infiltration_resource_target {
        let victim = economies.get(victim_team);
        score += (victim.ore + victim.crystal) as f32 * 0.5;
    }
    if target_def.is_infiltration_power_sabotage_target {
        score += target_def.power_delta.max(0) as f32 * 0.5;
    }
    score - xz_distance(origin, target_position) * 0.06
}

fn best_ai_capture_target(
    team: Team,
    origin: Vec3,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_score = f32::MIN;
    for (entity, structure, structure_team, transform, health, under_construction) in structures {
        if health.current <= 0.0
            || *structure_team == team
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        let Some(target_def) = registry::entity(structure.id) else {
            continue;
        };
        let Some(priority) = ai_capture_priority(target_def.id) else {
            continue;
        };
        let owner_bonus = if *structure_team == Team::Neutral {
            AI_CAPTURE_NEUTRAL_TECH_TARGET_BONUS
        } else {
            0.0
        };
        let structure_value = (target_def.cost.ore + target_def.cost.crystal) as f32
            + target_def.power_delta.abs() as f32;
        let distance_penalty = xz_distance(origin, transform.translation) * 0.08;
        let score = priority + structure_value + owner_bonus - distance_penalty;
        if score > best_score {
            best_score = score;
            best = Some(entity);
        }
    }
    best
}

fn ai_capture_priority(structure_id: &str) -> Option<f32> {
    match structure_id {
        "CommandCenter" => Some(120.0),
        "TechLab" => Some(105.0),
        "RoboticsBay" => Some(95.0),
        "VehicleFactory" => Some(90.0),
        "AircraftFactory" => Some(88.0),
        "TechAirport" => Some(87.0),
        "TechOilDerrick" => Some(86.0),
        "TechHospital" => Some(85.0),
        "TechBunker" => Some(84.75),
        "TechRepairDepot" => Some(84.5),
        "Barracks" => Some(84.0),
        "Refinery" => Some(78.0),
        "PowerReactor" => Some(72.0),
        "RadarUplink" => Some(70.0),
        "LanceBeamDefenseTower" => Some(62.0),
        "ArcCoilDefenseTower" => Some(58.0),
        "AntiGroundTurret" => Some(48.0),
        "AntiAirTurret" => Some(44.0),
        _ => None,
    }
}

fn active_ai_teams(
    controlled_team: Option<Team>,
    active_teams: Option<&ActiveTeams>,
) -> impl Iterator<Item = Team> + '_ {
    let team_count = active_teams.map(|active| active.0.len()).unwrap_or(0);
    player_teams(team_count)
        .filter(move |team| Some(*team) != controlled_team)
        .filter(move |team| team_is_active(*team, active_teams))
}

fn team_is_active(team: Team, active_teams: Option<&ActiveTeams>) -> bool {
    let Some(index) = team.economy_index() else {
        return false;
    };
    active_teams.map_or(true, |active| active.0.get(index).copied().unwrap_or(false))
}

fn active_match_perspectives(active_teams: &ActiveTeams) -> Vec<Team> {
    player_teams(active_teams.0.len())
        .filter(|team| team_is_active(*team, Some(active_teams)))
        .collect()
}

fn spectator_perspective_switch_enabled(
    visible_player: &VisiblePlayer,
    active_teams: &ActiveTeams,
) -> bool {
    visible_player.is_spectator() && active_match_perspectives(active_teams).len() > 1
}

fn cycle_spectator_visible_player(
    visible_player: &mut VisiblePlayer,
    active_teams: &ActiveTeams,
    direction: i32,
) -> bool {
    if !spectator_perspective_switch_enabled(visible_player, active_teams) {
        return false;
    }
    let perspectives = active_match_perspectives(active_teams);
    let len = perspectives.len();
    let next_index = match perspectives
        .iter()
        .position(|team| *team == visible_player.team)
    {
        Some(index) if direction < 0 => (index + len - 1) % len,
        Some(index) => (index + 1) % len,
        None if direction < 0 => len - 1,
        None => 0,
    };
    let next_team = perspectives[next_index];
    if next_team == visible_player.team {
        return false;
    }
    visible_player.team = next_team;
    true
}

fn visible_player_team(visible_player: Option<&VisiblePlayer>) -> Team {
    visible_player.map_or(Team::Player(0), |visible| visible.team)
}

fn controlled_player_team(visible_player: Option<&VisiblePlayer>) -> Option<Team> {
    match visible_player {
        Some(visible) if visible.is_spectator() => None,
        Some(visible) => Some(visible.team),
        None => Some(Team::Player(0)),
    }
}

fn next_ai_train(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    cursor: &mut usize,
    counts: AiProductionCounts,
    needs_anti_air: bool,
) -> Option<&'static str> {
    if profile.production_priority.is_empty() {
        return None;
    }
    if counts.workers < profile.expected_workers
        && let Some(worker_def) = registry::entity("Worker")
    {
        if !economy.can_afford(worker_def.cost) {
            return None;
        }
        if requirements_met(worker_def, team, structures)
            && ai_production_origin_for_faction(team, faction, "Worker", structures).is_some()
        {
            return Some("Worker");
        }
    }
    if needs_anti_air
        && let Some(candidate) = next_ai_train_matching(
            team,
            faction,
            profile,
            structures,
            economy,
            cursor,
            counts,
            |def| def.weapon.is_some_and(|weapon| weapon.can_attack_air),
        )
    {
        return Some(candidate);
    }
    next_ai_train_matching(
        team,
        faction,
        profile,
        structures,
        economy,
        cursor,
        counts,
        |_| true,
    )
}

fn next_ai_train_matching(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    cursor: &mut usize,
    counts: AiProductionCounts,
    mut predicate: impl FnMut(&registry::EntityDef) -> bool,
) -> Option<&'static str> {
    let len = profile.production_priority.len();
    let start = *cursor % len;
    for offset in 0..len {
        let index = (start + offset) % len;
        let candidate = profile.production_priority[index];
        let Some(def) = registry::entity(candidate) else {
            continue;
        };
        if !predicate(def) {
            continue;
        }
        if !ai_economy_candidate_allowed(team, candidate, profile, counts, structures) {
            continue;
        }
        if !ai_battlegroup_candidate_allowed(candidate, profile, counts) {
            continue;
        }
        if !economy.can_afford(def.cost) {
            continue;
        }
        if !requirements_met(def, team, structures) {
            continue;
        }
        if ai_production_origin_for_faction(team, faction, candidate, structures).is_none() {
            continue;
        }
        *cursor = (index + 1) % len;
        return Some(candidate);
    }
    None
}

fn ai_production_counts<'a>(
    team: Team,
    units: impl IntoIterator<Item = (&'a Unit, &'a Team)>,
) -> AiProductionCounts {
    let mut counts = AiProductionCounts::default();
    for (unit, unit_team) in units {
        if *unit_team != team {
            continue;
        }
        if can_unit_construct_structures(unit) {
            counts.workers += 1;
        }
        if unit.id == "OreHarvester" {
            counts.ore_harvesters += 1;
        }
        if ai_battle_unit_id(unit.id) {
            counts.battle_units += 1;
        }
    }
    counts
}

fn ai_economy_candidate_allowed(
    team: Team,
    candidate: &'static str,
    profile: &TeamAiProfile,
    counts: AiProductionCounts,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    match candidate {
        "Worker" => counts.workers < profile.expected_workers,
        "OreHarvester" => {
            counts.ore_harvesters < profile.expected_ore_harvesters
                && has_constructed_structure(team, "Refinery", structures)
        }
        _ => true,
    }
}

fn has_constructed_structure(
    team: Team,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    structures
        .iter()
        .any(|(structure, structure_team, _, under_construction)| {
            *structure_team == team
                && structure.id == structure_id
                && structure_is_constructed(under_construction)
        })
}

fn ai_structure_count(
    team: Team,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
    constructed_only: bool,
) -> usize {
    structures
        .iter()
        .filter(|(structure, structure_team, _, under_construction)| {
            **structure_team == team
                && structure.id == structure_id
                && (!constructed_only || structure_is_constructed(*under_construction))
        })
        .count()
}

#[allow(dead_code)]
fn next_ai_economy_structure(
    team: Team,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    next_ai_economy_structure_for_faction(
        team,
        SkirmishFaction::from_team(team),
        profile,
        structures,
        counts,
    )
}

fn next_ai_economy_structure_for_faction(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    if counts.workers == 0 {
        return None;
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "CommandCenter", structures)
        && ai_structure_count(team, "CommandCenter", structures, false)
            < profile.expected_command_centers
    {
        return Some("CommandCenter");
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "Refinery", structures)
        && ai_structure_count(team, "Refinery", structures, false) < profile.expected_refineries
    {
        return Some("Refinery");
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "OrePurifier", structures)
        && ai_structure_count(team, "OrePurifier", structures, false) == 0
        && has_constructed_structure(team, "Refinery", structures)
    {
        return Some("OrePurifier");
    }
    None
}

#[allow(dead_code)]
fn ai_economy_structure_allowed(
    team: Team,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    ai_economy_structure_allowed_for_faction(
        team,
        SkirmishFaction::from_team(team),
        structure_id,
        structures,
    )
}

fn ai_economy_structure_allowed_for_faction(
    team: Team,
    faction: SkirmishFaction,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let Some(faction) = faction_def(faction) else {
        return false;
    };
    let Some(def) = registry::entity(structure_id) else {
        return false;
    };
    faction.can_construct(structure_id) && requirements_met(def, team, structures)
}

#[allow(dead_code)]
fn next_ai_offense_structure(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_offense_structure_for_faction(team, SkirmishFaction::from_team(team), structures)
}

fn next_ai_offense_structure_for_faction(
    team: Team,
    faction: SkirmishFaction,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    let faction = faction_def(faction)?;
    for &candidate in AI_OFFENSE_STRUCTURE_PRIORITY {
        let Some(def) = registry::entity(candidate) else {
            continue;
        };
        if faction.can_construct(candidate)
            && ai_structure_count(team, candidate, structures, false) == 0
            && requirements_met(def, team, structures)
        {
            return Some(candidate);
        }
    }
    None
}

fn ai_structure_build_origin(
    team: Team,
    build_kind: AiStructureBuildKind,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    match build_kind {
        AiStructureBuildKind::Economy => ai_economy_structure_origin(team, structures),
        AiStructureBuildKind::Defense => ai_frontline_command_origin(team, structures, targets),
        AiStructureBuildKind::Offense => ai_frontline_command_origin(team, structures, targets),
    }
}

fn ai_economy_structure_origin(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<Vec3> {
    structures.iter().find_map(
        |(structure, structure_team, transform, under_construction)| {
            (*structure_team == team
                && structure.id == "CommandCenter"
                && structure_is_constructed(under_construction))
            .then_some(transform.translation)
        },
    )
}

fn ai_structure_build_position(
    team: Team,
    origin: Vec3,
    structure_id: &'static str,
    build_kind: AiStructureBuildKind,
    seed: u32,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
    bounds: MapBounds,
) -> Vec3 {
    match build_kind {
        AiStructureBuildKind::Economy => free_position_in_bounds(origin, seed + 19, 5.0, bounds),
        AiStructureBuildKind::Defense => {
            ai_defense_position(team, origin, structure_id, seed + 7, targets, bounds)
        }
        AiStructureBuildKind::Offense => {
            ai_defense_position(team, origin, structure_id, seed + 13, targets, bounds)
        }
    }
}

#[allow(dead_code)]
fn next_ai_defense(
    team: Team,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_defense_for_faction(team, SkirmishFaction::from_team(team), profile, structures)
}

fn next_ai_defense_for_faction(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    let faction = faction_def(faction)?;
    for candidate in profile.defense_priority {
        if let Some(def) = registry::entity(candidate) {
            if faction.can_construct(candidate)
                && ai_structure_under_profile_limit(team, candidate, structures, profile)
                && requirements_met(def, team, structures)
            {
                return Some(candidate);
            }
        }
    }
    None
}

fn ai_defense_position(
    team: Team,
    origin: Vec3,
    structure_id: &'static str,
    seed: u32,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
    bounds: MapBounds,
) -> Vec3 {
    let Some(enemy_position) = nearest_enemy_position(team, origin, targets) else {
        return free_position_in_bounds(origin, seed, 5.0, bounds);
    };
    let direction = Vec3::new(
        enemy_position.x - origin.x,
        0.0,
        enemy_position.z - origin.z,
    )
    .try_normalize()
    .unwrap_or(Vec3::Z);
    ai_defense_position_in_direction_in_bounds(origin, direction, structure_id, seed, bounds)
}

fn ai_defense_position_in_direction_in_bounds(
    origin: Vec3,
    direction: Vec3,
    structure_id: &'static str,
    seed: u32,
    bounds: MapBounds,
) -> Vec3 {
    let direction = Vec3::new(direction.x, 0.0, direction.z)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let lateral = Vec3::new(-direction.z, 0.0, direction.x);
    let structure_radius = registry::entity(structure_id).map_or(0.75, |def| def.radius);
    let command_radius = registry::entity("CommandCenter").map_or(1.8, |def| def.radius);
    let forward = command_radius + structure_radius * 3.0 + 1.5;
    let side_step = (structure_radius * 2.6).max(1.6);
    let side_slot = match seed % 5 {
        0 => 0.0,
        1 => side_step,
        2 => -side_step,
        3 => side_step * 2.0,
        _ => -side_step * 2.0,
    };
    let candidate = origin + direction * forward + lateral * side_slot;
    bounds.clamp_ground_point(candidate, 1.0)
}

fn ai_structure_under_profile_limit(
    team: Team,
    structure_id: &str,
    structures: &Query<StructurePrereqItem<'_>>,
    profile: &TeamAiProfile,
) -> bool {
    ai_structure_under_limit(
        team,
        structure_id,
        structures,
        profile.defense_limits,
        profile.defense_limit_bonus,
    )
}

fn ai_structure_under_limit(
    team: Team,
    structure_id: &str,
    structures: &Query<StructurePrereqItem<'_>>,
    limits: &[(&'static str, usize)],
    limit_bonus: usize,
) -> bool {
    let max = limits
        .iter()
        .find_map(|(id, max)| (*id == structure_id).then_some(*max))
        .unwrap_or(0);
    if max == 0 {
        return false;
    }
    let max = max.saturating_add(limit_bonus);
    let count = structures
        .iter()
        .filter(|(structure, structure_team, _, under_construction)| {
            structure_is_constructed(*under_construction)
                && **structure_team == team
                && structure.id == structure_id
        })
        .count();
    count < max
}

fn team_home(team: Team) -> Vec3 {
    match team {
        Team::Player(index) => {
            const GOLDEN_ANGLE: f32 = 2.399_963_1;
            let angle = index as f32 * GOLDEN_ANGLE;
            let radius = 14.0 + (index / MAX_SKIRMISH_LOBBY_SLOTS) as f32 * 5.0;
            Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius)
        }
        Team::Neutral => Vec3::ZERO,
    }
}

fn ai_frontline_command_origin(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    let mut fallback = None;
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (structure, structure_team, transform, under_construction) in structures {
        if *structure_team != team
            || !structure_is_constructed(under_construction)
            || structure.id != "CommandCenter"
        {
            continue;
        }
        fallback.get_or_insert(transform.translation);
        let Some(enemy_position) = nearest_enemy_position(team, transform.translation, targets)
        else {
            continue;
        };
        let distance = xz_distance(transform.translation, enemy_position);
        if distance < best_distance {
            best_distance = distance;
            best = Some(transform.translation);
        }
    }
    best.or(fallback)
}

fn nearest_enemy_position(
    team: Team,
    origin: Vec3,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    let mut nearest = None;
    let mut distance = f32::MAX;
    for (_, target_team, transform) in targets {
        if *target_team == team || *target_team == Team::Neutral {
            continue;
        }
        let d = xz_distance(origin, transform.translation);
        if d < distance {
            nearest = Some(transform.translation);
            distance = d;
        }
    }
    nearest
}

fn nearest_enemy_entity(
    team: Team,
    origin: Vec3,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut distance = f32::MAX;
    for (entity, target_team, transform) in targets {
        if *target_team == team || *target_team == Team::Neutral {
            continue;
        }
        let d = xz_distance(origin, transform.translation);
        if d < distance {
            nearest = Some(entity);
            distance = d;
        }
    }
    nearest
}

#[allow(dead_code)]
fn ai_production_origin(
    team: Team,
    product_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<(&'static str, Vec3)> {
    ai_production_origin_for_faction(
        team,
        SkirmishFaction::from_team(team),
        product_id,
        structures,
    )
}

fn ai_production_origin_for_faction(
    team: Team,
    faction: SkirmishFaction,
    product_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<(&'static str, Vec3)> {
    let faction = faction_def(faction)?;
    for (structure, structure_team, transform, under_construction) in structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && faction.can_produce(structure.id, product_id)
        {
            return Some((structure.id, transform.translation));
        }
    }
    None
}

fn update_capture_orders(
    mut commands: Commands,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut economies: ResMut<Economies>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut capturers: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &mut CaptureOrder,
        ),
        (With<Unit>, Without<Structure>),
    >,
    mut structures: Query<
        (
            Entity,
            &Structure,
            &mut Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&mut Garrison>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (
        capturer_entity,
        capturer_team,
        capturer_transform,
        capturer_selectable,
        unit,
        mut order,
    ) in &mut capturers
    {
        if !can_unit_capture(unit) {
            commands
                .entity(capturer_entity)
                .try_remove::<CaptureOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        let capture_time = capture_time_for_unit(unit);

        let Ok((
            _target_entity,
            structure,
            mut target_team,
            target_transform,
            target_selectable,
            target_health,
            target_garrison,
        )) = structures.get_mut(order.target)
        else {
            commands
                .entity(capturer_entity)
                .try_remove::<CaptureOrder>()
                .try_remove::<MoveOrder>();
            continue;
        };

        if target_health.current <= 0.0
            || (target_team.economy_index().is_some()
                && !relations.are_enemies(*capturer_team, *target_team))
        {
            commands
                .entity(capturer_entity)
                .try_remove::<CaptureOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        let entry_range = contact_action_entry_range(
            capturer_selectable.radius,
            target_selectable.radius,
            CAPTURE_ENTRY_MARGIN_M,
        );
        if xz_distance(capturer_transform.translation, target_transform.translation) > entry_range {
            if unit.speed <= 0.0 {
                commands
                    .entity(capturer_entity)
                    .try_remove::<CaptureOrder>()
                    .try_remove::<MoveOrder>();
                continue;
            }
            order.elapsed = 0.0;
            commands.entity(capturer_entity).try_insert(MoveOrder {
                target: unit_contact_move_target_position(
                    capturer_transform.translation,
                    capturer_selectable.radius,
                    target_transform.translation,
                    target_selectable.radius,
                ),
            });
            continue;
        }

        commands.entity(capturer_entity).try_remove::<MoveOrder>();
        order.elapsed += time.delta_secs();
        if order.elapsed < capture_time {
            continue;
        }

        let victim_team = *target_team;
        if let (Some(capturer_def), Some(target_def)) =
            (registry::entity(unit.id), registry::entity(structure.id))
        {
            apply_infiltration_on_capture(
                capturer_def,
                target_def,
                *capturer_team,
                victim_team,
                &mut economies,
            );
        }

        *target_team = *capturer_team;
        let structure_label = registry::entity(structure.id)
            .map(|def| def.label)
            .unwrap_or(structure.id);
        if *capturer_team == player_team {
            push_battle_log(
                &mut battle_log,
                format!("已占领 {structure_label}"),
                Some(target_transform.translation),
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::StructureCaptured);
        } else if relations.are_allied(victim_team, player_team) {
            push_battle_log(
                &mut battle_log,
                format!("失去 {structure_label}"),
                Some(target_transform.translation),
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::StructureLost);
        }
        if let Some(mut garrison) = target_garrison {
            garrison.count = 0;
        }
        if let Some(def) = registry::entity(structure.id) {
            let economy = economies.get_mut(*capturer_team);
            economy.ore += def.capture_bonus_ore;
            economy.crystal += def.capture_bonus_crystal;
        }
        latest_battle_event.focus = Some(target_transform.translation);
        commands.entity(capturer_entity).try_despawn();
    }
}

fn update_garrison_orders(
    mut commands: Commands,
    relations: Res<TeamRelations>,
    units: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &GarrisonOrder,
        ),
        (With<Unit>, Without<Structure>),
    >,
    mut bunkers: Query<
        (
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            &mut Garrison,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    for (unit_entity, unit_team, unit_transform, unit_selectable, unit, order) in &units {
        if !can_unit_garrison(unit) {
            commands
                .entity(unit_entity)
                .try_remove::<GarrisonOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        let Ok((
            structure,
            bunker_team,
            bunker_transform,
            bunker_selectable,
            bunker_health,
            mut garrison,
            under_construction,
        )) = bunkers.get_mut(order.target)
        else {
            commands
                .entity(unit_entity)
                .try_remove::<GarrisonOrder>()
                .try_remove::<MoveOrder>();
            continue;
        };

        if !can_garrison_structure_target(
            *unit_team,
            structure,
            *bunker_team,
            bunker_health,
            &garrison,
            under_construction,
            &relations,
        ) {
            commands
                .entity(unit_entity)
                .try_remove::<GarrisonOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        let entry_range = contact_action_entry_range(
            unit_selectable.radius,
            bunker_selectable.radius,
            CAPTURE_ENTRY_MARGIN_M,
        );
        if xz_distance(unit_transform.translation, bunker_transform.translation) > entry_range {
            if unit.speed <= 0.0 {
                commands
                    .entity(unit_entity)
                    .try_remove::<GarrisonOrder>()
                    .try_remove::<MoveOrder>();
                continue;
            }
            commands.entity(unit_entity).try_insert(MoveOrder {
                target: unit_contact_move_target_position(
                    unit_transform.translation,
                    unit_selectable.radius,
                    bunker_transform.translation,
                    bunker_selectable.radius,
                ),
            });
            continue;
        }

        garrison.count += 1;
        commands.entity(unit_entity).try_despawn();
    }
}

fn update_harvest_orders(
    mut commands: Commands,
    time: Res<Time>,
    mut economies: ResMut<Economies>,
    mut harvesters: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &Health,
            &mut ResourceCargo,
            &mut HarvestOrder,
            Option<&MoveOrder>,
            Option<&EmpDisabled>,
        ),
        (With<Unit>, Without<Structure>, Without<ResourceNode>),
    >,
    mut resource_queries: ParamSet<(
        Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
        Query<(Entity, &Transform, &Selectable, &mut ResourceNode)>,
    )>,
    dropoffs: Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    for (
        unit_entity,
        team,
        transform,
        selectable,
        unit,
        health,
        mut cargo,
        mut order,
        move_order,
        emp,
    ) in &mut harvesters
    {
        if cargo.capacity <= 0
            || health.current <= 0.0
            || emp.is_some_and(|emp| emp.remaining > 0.0)
        {
            commands
                .entity(unit_entity)
                .try_remove::<HarvestOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        if cargo.is_full() {
            order.state = HarvestState::MovingToDropoff;
        }

        match order.state {
            HarvestState::MovingToResource => {
                let target = {
                    let resources = resource_queries.p0();
                    resolve_harvest_resource_target(
                        order.resource,
                        transform.translation,
                        &resources,
                    )
                };
                let Some(target) = target else {
                    if cargo.has_any() {
                        order.state = HarvestState::MovingToDropoff;
                    } else {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                    }
                    continue;
                };
                order.resource = Some(target);

                let in_range = {
                    let resources = resource_queries.p0();
                    let Ok((_, resource_transform, resource_selectable, resource)) =
                        resources.get(target)
                    else {
                        order.resource = None;
                        continue;
                    };
                    if resource.amount <= 0 {
                        order.resource = None;
                        continue;
                    }
                    let entry_range = contact_action_entry_range(
                        selectable.radius,
                        resource_selectable.radius,
                        RESOURCE_ENTRY_MARGIN_M,
                    );
                    xz_distance(transform.translation, resource_transform.translation)
                        <= entry_range
                };

                if !in_range {
                    if unit.speed <= 0.0 {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                        continue;
                    }
                    let resources = resource_queries.p0();
                    if let Ok((_, resource_transform, resource_selectable, _)) =
                        resources.get(target)
                    {
                        if !move_order_targets_contact(
                            move_order,
                            resource_transform.translation,
                            selectable.radius,
                            resource_selectable.radius,
                        ) {
                            commands.entity(unit_entity).try_insert(MoveOrder {
                                target: unit_contact_move_target_position(
                                    transform.translation,
                                    selectable.radius,
                                    resource_transform.translation,
                                    resource_selectable.radius,
                                ),
                            });
                        }
                    }
                    continue;
                }

                commands.entity(unit_entity).try_remove::<MoveOrder>();
                let resources = resource_queries.p0();
                if let Ok((_, _, _, resource)) = resources.get(target) {
                    order.state = HarvestState::Collecting;
                    order.collect_remaining = resource.kind.collect_seconds();
                }
            }
            HarvestState::Collecting => {
                let Some(target) = order.resource else {
                    order.state = HarvestState::MovingToResource;
                    continue;
                };
                let mut depleted_position = None;
                {
                    let mut resources = resource_queries.p1();
                    let Ok((
                        resource_entity,
                        resource_transform,
                        resource_selectable,
                        mut resource,
                    )) = resources.get_mut(target)
                    else {
                        order.resource = None;
                        order.state = HarvestState::MovingToResource;
                        continue;
                    };
                    let entry_range = contact_action_entry_range(
                        selectable.radius,
                        resource_selectable.radius,
                        RESOURCE_ENTRY_MARGIN_M,
                    );
                    if resource.amount <= 0
                        || xz_distance(transform.translation, resource_transform.translation)
                            > entry_range
                    {
                        order.state = HarvestState::MovingToResource;
                        if unit.speed <= 0.0 {
                            commands
                                .entity(unit_entity)
                                .try_remove::<HarvestOrder>()
                                .try_remove::<MoveOrder>();
                        }
                        continue;
                    }

                    order.collect_remaining -= time.delta_secs();
                    while order.collect_remaining <= 0.0 && resource.amount > 0 && !cargo.is_full()
                    {
                        resource.amount -= 1;
                        let _ = cargo.add_one(resource.kind);
                        order.collect_remaining += resource.kind.collect_seconds();
                    }

                    if resource.amount <= 0 {
                        depleted_position = Some(resource_transform.translation);
                        commands.entity(resource_entity).try_despawn();
                    }
                }

                if let Some(position) = depleted_position {
                    commands.spawn((
                        ShotPulse {
                            from: transform.translation + Vec3::Y * 0.25,
                            to: position + Vec3::Y * 0.25,
                            ttl: 0.18,
                            team: *team,
                        },
                        MatchScopedEntity,
                    ));
                    order.resource = None;
                    order.state = if cargo.has_any() {
                        HarvestState::MovingToDropoff
                    } else {
                        HarvestState::MovingToResource
                    };
                } else if cargo.is_full() {
                    order.resource = None;
                    order.state = HarvestState::MovingToDropoff;
                }
            }
            HarvestState::MovingToDropoff => {
                let Some((_, dropoff_position, dropoff_radius)) =
                    nearest_resource_dropoff(*team, transform.translation, &dropoffs)
                else {
                    commands
                        .entity(unit_entity)
                        .try_remove::<HarvestOrder>()
                        .try_remove::<MoveOrder>();
                    continue;
                };

                let entry_range = contact_action_entry_range(
                    selectable.radius,
                    dropoff_radius,
                    RESOURCE_DROPOFF_ENTRY_MARGIN_M,
                );
                if xz_distance(transform.translation, dropoff_position) > entry_range {
                    if unit.speed <= 0.0 {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                        continue;
                    }
                    if !move_order_targets_contact(
                        move_order,
                        dropoff_position,
                        selectable.radius,
                        dropoff_radius,
                    ) {
                        commands.entity(unit_entity).try_insert(MoveOrder {
                            target: unit_contact_move_target_position(
                                transform.translation,
                                selectable.radius,
                                dropoff_position,
                                dropoff_radius,
                            ),
                        });
                    }
                    continue;
                }

                commands.entity(unit_entity).try_remove::<MoveOrder>();
                let (ore, crystal) = cargo.clear();
                let apply_bonus = resource_dropoff_bonus_applies(*team, &economies, &dropoffs);
                let economy = economies.get_mut(*team);
                economy.ore += resource_amount_after_dropoff_bonus(ore, apply_bonus);
                economy.crystal += resource_amount_after_dropoff_bonus(crystal, apply_bonus);
                commands.spawn((
                    ShotPulse {
                        from: transform.translation + Vec3::Y * 0.35,
                        to: dropoff_position + Vec3::Y * 0.35,
                        ttl: 0.2,
                        team: *team,
                    },
                    MatchScopedEntity,
                ));
                order.state = HarvestState::MovingToResource;
                order.resource = None;
                order.collect_remaining = 0.0;
            }
        }
    }
}

fn resolve_harvest_resource_target(
    current: Option<Entity>,
    position: Vec3,
    resources: &Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
) -> Option<Entity> {
    if let Some(current) = current
        && let Ok((_, _, _, resource)) = resources.get(current)
        && resource.amount > 0
    {
        return Some(current);
    }
    nearest_resource_entity(position, resources, Some(RESOURCE_SEARCH_RADIUS_M))
}

fn nearest_resource_entity(
    position: Vec3,
    resources: &Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
    max_distance: Option<f32>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, resource) in resources {
        if resource.amount <= 0 {
            continue;
        }
        let distance = xz_distance(position, transform.translation) - selectable.radius;
        if max_distance.is_some_and(|max| distance > max) {
            continue;
        }
        if distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

fn nearest_resource_dropoff(
    team: Team,
    position: Vec3,
    dropoffs: &Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> Option<(Entity, Vec3, f32)> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, structure, structure_team, transform, selectable, health, under_construction) in
        dropoffs
    {
        if *structure_team != team
            || health.current <= 0.0
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        if !is_resource_dropoff_structure(structure) {
            continue;
        }
        let distance = xz_distance(position, transform.translation);
        if distance < nearest_distance {
            nearest = Some((entity, transform.translation, selectable.radius));
            nearest_distance = distance;
        }
    }
    nearest
}

fn resource_dropoff_bonus_applies(
    team: Team,
    economies: &Economies,
    structures: &Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> bool {
    if economies.get(team).low_power() {
        return false;
    }
    structures.iter().any(
        |(_, structure, structure_team, _, _, health, under_construction)| {
            *structure_team == team
                && structure.id == "OrePurifier"
                && health.current > 0.0
                && structure_is_constructed(under_construction)
        },
    )
}

fn resource_amount_after_dropoff_bonus(amount: i32, apply_bonus: bool) -> i32 {
    if amount <= 0 || !apply_bonus {
        return amount;
    }
    amount + ((amount as f32) * ORE_PURIFIER_BONUS_RATIO).ceil() as i32
}

fn is_resource_dropoff_structure(structure: &Structure) -> bool {
    matches!(structure.id, "CommandCenter" | "Refinery")
}

fn update_mine_layers(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    map_bounds: Res<MapBounds>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut next_id: ResMut<NextSpawnId>,
    mut layers: Query<(
        Entity,
        &Team,
        &Transform,
        &Health,
        &mut MineLayer,
        Option<&EmpDisabled>,
        Option<&VisualFaction>,
    )>,
    mines: Query<(&Team, &Transform, &Mine)>,
) {
    let Some(mine_def) = registry::entity("LandMine") else {
        return;
    };
    let player_team = visible_player_team(visible_player.as_deref());
    for (layer_entity, team, transform, health, mut layer, emp, visual_faction) in &mut layers {
        if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
            continue;
        }
        layer.cooldown -= time.delta_secs();
        if layer.cooldown > 0.0 {
            continue;
        }
        let active_mines = mines
            .iter()
            .filter(|(mine_team, _, mine)| {
                **mine_team == *team && mine.source == Some(layer_entity)
            })
            .count();
        if active_mines >= layer.limit {
            continue;
        }
        layer.cooldown = layer.deploy_interval;
        let deploy_position =
            next_mine_deploy_position_in_bounds(transform.translation, &mut layer, *map_bounds);
        let nearby_friendly_mine = mines.iter().any(|(mine_team, mine_transform, _)| {
            *mine_team == *team
                && xz_distance(mine_transform.translation, deploy_position) <= layer.spacing
        });
        if nearby_friendly_mine {
            continue;
        }
        let mine_entity = spawn_unit_with_visual_faction(
            &mut commands,
            &asset_server,
            &mut next_id,
            "LandMine",
            *team,
            deploy_position,
            0,
            visual_faction
                .copied()
                .map(|faction| faction.0)
                .or_else(|| default_visual_faction(*team)),
            player_team,
        );
        commands.entity(mine_entity).try_insert(Mine {
            damage: layer.damage,
            trigger_radius: mine_def.mine_trigger_radius,
            blast_radius: mine_def.mine_blast_radius,
            arming_remaining: mine_def.mine_arming_delay,
            source: Some(layer_entity),
        });
    }
}

fn next_mine_deploy_position_in_bounds(
    origin: Vec3,
    layer: &mut MineLayer,
    bounds: MapBounds,
) -> Vec3 {
    let (x, z) = MINE_DEPLOY_OFFSETS[layer.deploy_index % MINE_DEPLOY_OFFSETS.len()];
    layer.deploy_index += 1;
    let direction = Vec2::new(x, z).normalize_or_zero();
    bounds.clamp_ground_point(
        Vec3::new(
            origin.x + direction.x * layer.deploy_radius,
            0.0,
            origin.z + direction.y * layer.deploy_radius,
        ),
        0.4,
    )
}

#[derive(Clone, Copy)]
struct MineTargetSnapshot {
    entity: Entity,
    team: Team,
    position: Vec3,
    radius: f32,
}

fn update_mines(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut kill_credits: ResMut<KillCredits>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut mine_queries: ParamSet<(
        Query<(Entity, &Team, &Transform, &mut Mine, &Health)>,
        Query<
            (
                Entity,
                &Team,
                &Transform,
                &Selectable,
                &Unit,
                &MovementDomain,
                &Health,
            ),
            (With<Unit>, Without<Mine>),
        >,
        Query<(
            &mut Health,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
        )>,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let target_snapshots = {
        let targets = mine_queries.p1();
        targets
            .iter()
            .filter_map(
                |(entity, team, transform, selectable, unit, domain, health)| {
                    if health.current <= 0.0
                        || unit.speed <= 0.0
                        || *domain != MovementDomain::Terrain
                    {
                        return None;
                    }
                    Some(MineTargetSnapshot {
                        entity,
                        team: *team,
                        position: transform.translation,
                        radius: selectable.radius,
                    })
                },
            )
            .collect::<Vec<_>>()
    };

    let mut damage_events = Vec::new();
    let mut mines_to_despawn = Vec::new();
    {
        let mut mines = mine_queries.p0();
        for (mine_entity, mine_team, mine_transform, mut mine, mine_health) in &mut mines {
            if mine_health.current <= 0.0 {
                continue;
            }
            mine.arming_remaining -= time.delta_secs();
            if mine.arming_remaining > 0.0 {
                continue;
            }
            let triggered = target_snapshots.iter().any(|target| {
                mine_can_damage_target(
                    *mine_team,
                    mine_transform.translation,
                    mine.trigger_radius,
                    target,
                    &relations,
                )
            });
            if !triggered {
                continue;
            }
            let source = mine.source.unwrap_or(mine_entity);
            let mut impacted = false;
            for target in &target_snapshots {
                if !mine_can_damage_target(
                    *mine_team,
                    mine_transform.translation,
                    mine.blast_radius,
                    target,
                    &relations,
                ) {
                    continue;
                }
                damage_events.push((
                    target.entity,
                    mine.damage,
                    mine_transform.translation,
                    target.position,
                    target.radius,
                    *mine_team,
                    target.team,
                    source,
                ));
                impacted = true;
            }
            if impacted {
                latest_battle_event.focus = Some(mine_transform.translation);
                commands.spawn((
                    ShotPulse {
                        from: mine_transform.translation + Vec3::Y * 0.2,
                        to: mine_transform.translation + Vec3::Y * 1.0,
                        ttl: 0.24,
                        team: *mine_team,
                    },
                    MatchScopedEntity,
                ));
            }
            mines_to_despawn.push(mine_entity);
        }
    }

    for mine_entity in mines_to_despawn {
        commands.entity(mine_entity).try_despawn();
    }

    {
        let mut health_q = mine_queries.p2();
        for (target, damage, from, to, target_radius, team, target_team, source) in damage_events {
            let Ok((mut health, shield, passive_shield)) = health_q.get_mut(target) else {
                continue;
            };
            if health.current <= 0.0 {
                continue;
            }
            let applied_damage = damage * support_damage_scale(shield, passive_shield);
            health.current -= applied_damage;
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.45,
                    to: to + Vec3::Y * 0.45,
                    ttl: 0.16,
                    team,
                },
                MatchScopedEntity,
            ));
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    match_state.units_lost += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                spawn_combat_wreckage(&mut commands, &asset_server, to, target_radius);
                commands.entity(target).try_despawn();
            }
        }
    }
}

fn mine_can_damage_target(
    mine_team: Team,
    mine_position: Vec3,
    radius: f32,
    target: &MineTargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    relations.are_enemies(mine_team, target.team)
        && xz_distance(mine_position, target.position) <= radius + target.radius
}

fn update_follow_orders(
    mut commands: Commands,
    relations: Res<TeamRelations>,
    followers: Query<(
        Entity,
        &Team,
        &Transform,
        &Selectable,
        &FollowOrder,
        &Health,
        Option<&Unit>,
        Option<&EmpDisabled>,
    )>,
    targets: Query<(&Team, &Transform, &Selectable, Option<&Health>), With<Selectable>>,
) {
    for (entity, team, transform, selectable, follow, health, unit, emp) in &followers {
        if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
            continue;
        }
        if unit.is_some_and(|unit| unit.speed <= 0.0) {
            commands
                .entity(entity)
                .try_remove::<FollowOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        let Ok((target_team, target_transform, target_selectable, target_health)) =
            targets.get(follow.target)
        else {
            commands
                .entity(entity)
                .try_remove::<FollowOrder>()
                .try_remove::<MoveOrder>();
            continue;
        };
        if (!follow.allow_enemy && !relations.are_allied(*team, *target_team))
            || target_health.is_some_and(|health| health.current <= 0.0)
        {
            commands
                .entity(entity)
                .try_remove::<FollowOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        let target_position = follow_order_reference_position(target_transform.translation, follow);
        let move_target = follow_order_move_target_position(
            transform.translation,
            selectable.radius,
            target_transform.translation,
            target_selectable.radius,
            follow,
        );
        let desired_distance =
            follow_order_desired_distance(selectable.radius, target_selectable.radius, follow);
        if xz_distance(transform.translation, target_position) > desired_distance {
            commands.entity(entity).try_insert(MoveOrder {
                target: move_target,
            });
        } else {
            commands.entity(entity).try_remove::<MoveOrder>();
        }
    }
}

fn follow_order_reference_position(target_position: Vec3, follow: &FollowOrder) -> Vec3 {
    target_position + follow.offset
}

fn follow_order_move_target_position(
    source_position: Vec3,
    source_radius: f32,
    target_position: Vec3,
    target_radius: f32,
    follow: &FollowOrder,
) -> Vec3 {
    if follow.offset.length_squared() > f32::EPSILON {
        return target_position + follow.offset;
    }
    unit_contact_move_target_position(
        source_position,
        source_radius,
        target_position,
        target_radius,
    )
}

fn unit_contact_move_target_position(
    source_position: Vec3,
    source_radius: f32,
    target_position: Vec3,
    target_radius: f32,
) -> Vec3 {
    let mut direction_from_target = Vec3::new(
        source_position.x - target_position.x,
        0.0,
        source_position.z - target_position.z,
    );
    if direction_from_target.length_squared() <= f32::EPSILON {
        direction_from_target = Vec3::X;
    } else {
        direction_from_target = direction_from_target.normalize();
    }
    target_position
        + direction_from_target * (source_radius + target_radius + UNIT_ADHERENCE_MARGIN_M)
}

fn contact_action_entry_range(source_radius: f32, target_radius: f32, margin: f32) -> f32 {
    source_radius + target_radius + margin + CONTACT_ACTION_REACHED_TOLERANCE_M
}

fn move_order_targets_contact(
    move_order: Option<&MoveOrder>,
    target_position: Vec3,
    source_radius: f32,
    target_radius: f32,
) -> bool {
    let Some(move_order) = move_order else {
        return false;
    };
    let expected_contact_distance = source_radius + target_radius + UNIT_ADHERENCE_MARGIN_M;
    (xz_distance(move_order.target, target_position) - expected_contact_distance).abs()
        <= CONTACT_ACTION_REACHED_TOLERANCE_M * 2.0
}

fn follow_order_desired_distance(
    source_radius: f32,
    target_radius: f32,
    follow: &FollowOrder,
) -> f32 {
    if follow.offset.length_squared() > f32::EPSILON {
        source_radius + FOLLOW_TARGET_DISTANCE_MARGIN_M
    } else {
        source_radius + target_radius + FOLLOW_TARGET_DISTANCE_MARGIN_M
    }
}

#[derive(Clone, Copy)]
struct RepairerSnapshot {
    entity: Entity,
    team: Team,
    position: Vec3,
    radius: f32,
    target: Entity,
    capability: RepairCapability,
    can_move: bool,
    disabled: bool,
    alive: bool,
}

fn update_repair_orders(
    mut commands: Commands,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    mut repair_params: ParamSet<(
        Query<(
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &RepairOrder,
            Option<&EmpDisabled>,
            &Health,
        )>,
        Query<(
            &Team,
            &Transform,
            &Selectable,
            Option<&Unit>,
            Option<&Structure>,
            Option<&UnderConstruction>,
            &mut Health,
        )>,
    )>,
) {
    let repairers = {
        let repairers_q = repair_params.p0();
        repairers_q
            .iter()
            .filter_map(
                |(entity, team, transform, selectable, unit, order, emp, health)| {
                    repair_capability(unit).map(|capability| RepairerSnapshot {
                        entity,
                        team: *team,
                        position: transform.translation,
                        radius: selectable.radius,
                        target: order.target,
                        capability,
                        can_move: unit.speed > 0.0,
                        disabled: emp.is_some_and(|emp| emp.remaining > 0.0),
                        alive: health.current > 0.0,
                    })
                },
            )
            .collect::<Vec<_>>()
    };

    let mut targets = repair_params.p1();
    for repairer in repairers {
        if !repairer.alive {
            commands
                .entity(repairer.entity)
                .try_remove::<RepairOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        if repairer.disabled {
            continue;
        }
        if repairer.target == repairer.entity {
            commands
                .entity(repairer.entity)
                .try_remove::<RepairOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        let Ok((
            target_team,
            target_transform,
            target_selectable,
            target_unit,
            target_structure,
            target_under_construction,
            mut target_health,
        )) = targets.get_mut(repairer.target)
        else {
            commands
                .entity(repairer.entity)
                .try_remove::<RepairOrder>()
                .try_remove::<MoveOrder>();
            continue;
        };
        if !relations.are_allied(repairer.team, *target_team)
            || !can_repair_order_target(
                target_unit,
                target_structure,
                target_under_construction,
                &target_health,
            )
        {
            commands
                .entity(repairer.entity)
                .try_remove::<RepairOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        let range = repair_order_range(
            repairer.capability,
            repairer.radius,
            target_selectable.radius,
        ) + CONTACT_ACTION_REACHED_TOLERANCE_M;
        if xz_distance(repairer.position, target_transform.translation) > range {
            if !repairer.can_move {
                commands
                    .entity(repairer.entity)
                    .try_remove::<RepairOrder>()
                    .try_remove::<MoveOrder>();
                continue;
            }
            commands.entity(repairer.entity).try_insert(MoveOrder {
                target: unit_contact_move_target_position(
                    repairer.position,
                    repairer.radius,
                    target_transform.translation,
                    target_selectable.radius,
                ),
            });
            continue;
        }
        commands.entity(repairer.entity).try_remove::<MoveOrder>();
        target_health.current = (target_health.current
            + repairer.capability.rate * time.delta_secs())
        .min(target_health.max);
        if target_health.current >= target_health.max {
            commands.entity(repairer.entity).try_remove::<RepairOrder>();
        }
    }
}

fn update_construct_orders(
    mut commands: Commands,
    time: Res<Time>,
    constructors: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &ConstructOrder,
            Option<&MoveOrder>,
            Option<&EmpDisabled>,
            &Health,
        ),
        (With<Unit>, Without<Structure>),
    >,
    mut structures: Query<
        (
            &Team,
            &Transform,
            &Selectable,
            &mut Health,
            &mut UnderConstruction,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    for (
        constructor_entity,
        constructor_team,
        constructor_transform,
        constructor_selectable,
        constructor_unit,
        order,
        move_order,
        emp,
        constructor_health,
    ) in &constructors
    {
        if constructor_health.current <= 0.0
            || emp.is_some_and(|emp| emp.remaining > 0.0)
            || !can_unit_construct_structures(constructor_unit)
        {
            commands
                .entity(constructor_entity)
                .try_remove::<ConstructOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        let Ok((
            target_team,
            target_transform,
            target_selectable,
            mut target_health,
            mut construction,
        )) = structures.get_mut(order.target)
        else {
            commands
                .entity(constructor_entity)
                .try_remove::<ConstructOrder>()
                .try_remove::<MoveOrder>();
            continue;
        };

        if *target_team != *constructor_team || target_health.current <= 0.0 {
            commands
                .entity(constructor_entity)
                .try_remove::<ConstructOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        let range = contact_action_entry_range(
            constructor_selectable.radius,
            target_selectable.radius,
            CONSTRUCTION_ENTRY_MARGIN_M,
        );
        if xz_distance(
            constructor_transform.translation,
            target_transform.translation,
        ) > range
        {
            if constructor_unit.speed <= 0.0 {
                commands
                    .entity(constructor_entity)
                    .try_remove::<ConstructOrder>()
                    .try_remove::<MoveOrder>();
                continue;
            }
            if !move_order_targets_contact(
                move_order,
                target_transform.translation,
                constructor_selectable.radius,
                target_selectable.radius,
            ) {
                commands.entity(constructor_entity).try_insert(MoveOrder {
                    target: unit_contact_move_target_position(
                        constructor_transform.translation,
                        constructor_selectable.radius,
                        target_transform.translation,
                        target_selectable.radius,
                    ),
                });
            }
            continue;
        }

        commands
            .entity(constructor_entity)
            .try_remove::<MoveOrder>();
        apply_structure_construction_progress(
            &mut construction,
            &mut target_health,
            time.delta_secs(),
        );
        if construction.remaining <= 0.0 {
            commands
                .entity(constructor_entity)
                .try_remove::<ConstructOrder>();
        }
    }
}

fn collect_supply_crates(
    mut commands: Commands,
    mut economies: ResMut<Economies>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    crates: Query<(Entity, &Transform, &SupplyCrate)>,
    mut units: ParamSet<(
        Query<(Entity, &Team, &Transform, &Selectable, &Unit, &Health)>,
        Query<(&Team, &Transform, &Selectable, &mut Health), With<Unit>>,
        Query<
            (
                Entity,
                &Team,
                &Transform,
                &Selectable,
                &mut Health,
                Option<&mut Weapon>,
                &mut VisionRadius,
                &mut Veterancy,
                Option<&VisibilityState>,
            ),
            With<Unit>,
        >,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let mut collections = Vec::new();
    {
        let unit_scan = units.p0();
        for (crate_entity, crate_transform, supply_crate) in &crates {
            let mut collector = None;
            let mut nearest_distance = f32::MAX;
            for (unit_entity, team, unit_transform, unit_selectable, unit, health) in &unit_scan {
                if team.economy_index().is_none() || health.current <= 0.0 || unit.speed <= 0.0 {
                    continue;
                }
                let distance = xz_distance(crate_transform.translation, unit_transform.translation);
                if distance <= supply_crate.pickup_radius + unit_selectable.radius
                    && distance < nearest_distance
                {
                    collector = Some((unit_entity, *team));
                    nearest_distance = distance;
                }
            }
            if let Some((collector_entity, collector_team)) = collector {
                collections.push((
                    crate_entity,
                    crate_transform.translation,
                    *supply_crate,
                    collector_entity,
                    collector_team,
                ));
            }
        }
    }

    for (crate_entity, crate_position, supply_crate, collector_entity, collector_team) in
        collections
    {
        match supply_crate.effect {
            SupplyCrateEffect::Resources => {
                let economy = economies.get_mut(collector_team);
                economy.ore += supply_crate.resource_ore;
                economy.crystal += supply_crate.resource_crystal;
            }
            SupplyCrateEffect::Repair => {
                let mut repair_targets = units.p1();
                for (team, transform, selectable, mut health) in &mut repair_targets {
                    if !relations.are_allied(collector_team, *team)
                        || health.current <= 0.0
                        || health.current >= health.max
                    {
                        continue;
                    }
                    if xz_distance(crate_position, transform.translation)
                        <= supply_crate.repair_radius + selectable.radius
                    {
                        health.current =
                            (health.current + supply_crate.repair_amount).min(health.max);
                    }
                }
            }
            SupplyCrateEffect::Veterancy => {
                let mut promote_targets = units.p2();
                if !try_grant_veterancy_rank(
                    &mut commands,
                    collector_entity,
                    1,
                    &mut promote_targets,
                ) {
                    let mut best = None;
                    let mut best_distance = f32::MAX;
                    for (entity, team, transform, _, _, _, _, veteran, _) in &promote_targets {
                        if !relations.are_allied(collector_team, *team)
                            || veteran.rank >= VETERANCY_MAX_RANK
                        {
                            continue;
                        }
                        let distance = xz_distance(crate_position, transform.translation);
                        if distance <= supply_crate.repair_radius && distance < best_distance {
                            best = Some(entity);
                            best_distance = distance;
                        }
                    }
                    if let Some(entity) = best {
                        let _ = try_grant_veterancy_rank(
                            &mut commands,
                            entity,
                            1,
                            &mut promote_targets,
                        );
                    }
                }
            }
        }
        commands.spawn((
            ShotPulse {
                from: crate_position + Vec3::Y * 0.35,
                to: crate_position + Vec3::Y * 1.15,
                ttl: 0.24,
                team: collector_team,
            },
            MatchScopedEntity,
        ));
        if collector_team == player_team {
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::SupplyCrate);
            push_battle_log(
                &mut battle_log,
                format!("补给箱: {}", supply_crate.effect.label()),
                Some(crate_position),
            );
        }
        commands.entity(crate_entity).try_despawn();
    }
}

fn spawn_veterancy_promotion_effect(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    team: Team,
    rank: u8,
    visibility: Option<&VisibilityState>,
) {
    if visibility.is_some_and(|visibility| !visibility.visible) {
        return;
    }
    commands.spawn((
        Transform::from_translation(position + Vec3::Y * 0.08),
        VeterancyPromotionEffect {
            rank: rank.min(VETERANCY_MAX_RANK),
            remaining: VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS,
            total: VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS,
            radius: radius.max(0.75) * 1.45,
            team,
        },
        MatchScopedEntity,
    ));
}

fn try_grant_veterancy_rank(
    commands: &mut Commands,
    entity: Entity,
    rank_delta: u8,
    units: &mut Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &mut Health,
            Option<&mut Weapon>,
            &mut VisionRadius,
            &mut Veterancy,
            Option<&VisibilityState>,
        ),
        With<Unit>,
    >,
) -> bool {
    let Ok((
        _entity,
        team,
        transform,
        selectable,
        mut health,
        weapon,
        mut vision,
        mut veteran,
        visibility,
    )) = units.get_mut(entity)
    else {
        return false;
    };
    let target_rank = veteran
        .rank
        .saturating_add(rank_delta)
        .min(VETERANCY_MAX_RANK);
    if target_rank <= veteran.rank {
        return false;
    }

    let old_health_ratio = if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let idx = target_rank as usize;
    veteran.experience_points = veteran.experience_points.max(VETERANCY_KILLS_BY_RANK[idx]);
    veteran.rank = target_rank;
    health.max = (veteran.base_health * VETERANCY_HP_MULTIPLIER_BY_RANK[idx]).ceil();
    health.current = (old_health_ratio * health.max)
        .ceil()
        .clamp(1.0, health.max);
    if let Some(mut weapon) = weapon {
        weapon.damage =
            (veteran.base_damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[idx] * 10.0).round() / 10.0;
        weapon.range = veteran.base_range + VETERANCY_RANGE_BONUS_BY_RANK[idx];
    }
    vision.0 = veteran.base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[idx];
    spawn_veterancy_promotion_effect(
        commands,
        transform.translation,
        selectable.radius,
        *team,
        target_rank,
        visibility,
    );
    true
}

fn apply_kill_credits(
    mut commands: Commands,
    mut kill_credits: ResMut<KillCredits>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut units: Query<
        (
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &mut Health,
            Option<&mut Weapon>,
            &mut VisionRadius,
            &mut Veterancy,
            Option<&VisibilityState>,
        ),
        With<Unit>,
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let credits = std::mem::take(&mut kill_credits.0);
    for source in credits {
        let Ok((
            team,
            transform,
            selectable,
            unit,
            mut health,
            weapon,
            mut vision,
            mut veteran,
            visibility,
        )) = units.get_mut(source)
        else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        veteran.experience_points = veteran.experience_points.saturating_add(1);
        let target_rank = rank_for_experience_points(veteran.experience_points);
        if target_rank <= veteran.rank {
            continue;
        }

        let old_health_ratio = if health.max > 0.0 {
            (health.current / health.max).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let idx = target_rank as usize;
        veteran.rank = target_rank;
        health.max = (veteran.base_health * VETERANCY_HP_MULTIPLIER_BY_RANK[idx]).ceil();
        health.current = (old_health_ratio * health.max)
            .ceil()
            .clamp(1.0, health.max);
        if let Some(mut weapon) = weapon {
            weapon.damage = (veteran.base_damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[idx] * 10.0)
                .round()
                / 10.0;
            weapon.range = veteran.base_range + VETERANCY_RANGE_BONUS_BY_RANK[idx];
        }
        vision.0 = veteran.base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[idx];
        spawn_veterancy_promotion_effect(
            &mut commands,
            transform.translation,
            selectable.radius,
            *team,
            target_rank,
            visibility,
        );
        if *team == player_team {
            let unit_label = registry::entity(unit.id)
                .map(|def| def.label)
                .unwrap_or(unit.id);
            push_battle_log(
                &mut battle_log,
                format!("单位晋升: {unit_label} Lv{target_rank}"),
                Some(transform.translation),
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::UnitPromoted);
        }
    }
}

fn rank_for_experience_points(points: u32) -> u8 {
    let mut rank = 0;
    for (idx, kills) in VETERANCY_KILLS_BY_RANK.iter().enumerate() {
        if points >= *kills {
            rank = idx as u8;
        }
    }
    rank.min(VETERANCY_MAX_RANK)
}

fn update_veterancy_regeneration(
    time: Res<Time>,
    mut units: Query<(&Veterancy, &mut Health, Option<&EmpDisabled>), With<Unit>>,
) {
    for (veteran, mut health, emp) in &mut units {
        if veteran.rank < VETERANCY_MAX_RANK
            || health.current <= 0.0
            || health.current >= health.max
            || emp.is_some_and(|emp| emp.remaining > 0.0)
        {
            continue;
        }
        health.current =
            (health.current + VETERANCY_ELITE_REGEN_PER_SECOND * time.delta_secs()).min(health.max);
    }
}

fn chase_attack_targets(
    mut commands: Commands,
    relations: Res<TeamRelations>,
    attackers: Query<(Entity, &Transform, &Unit, &Weapon, &AttackOrder)>,
    targets: Query<(&Transform, &Team, &MovementDomain, &Health)>,
    teams: Query<&Team>,
) {
    for (entity, transform, unit, weapon, attack_order) in &attackers {
        let Ok(attacker_team) = teams.get(entity) else {
            continue;
        };
        let Ok((target_transform, target_team, target_domain, target_health)) =
            targets.get(attack_order.target)
        else {
            clear_attack_chase_order(&mut commands, entity);
            continue;
        };
        if !attack_order_target_valid(
            attacker_team,
            target_team,
            *target_domain,
            target_health,
            weapon,
            &relations,
        ) {
            clear_attack_chase_order(&mut commands, entity);
            continue;
        }
        let distance = xz_distance(transform.translation, target_transform.translation);
        if distance > weapon.range * 0.9 {
            if unit.speed <= 0.0 {
                if distance > weapon.range {
                    clear_attack_chase_order(&mut commands, entity);
                } else {
                    commands.entity(entity).try_remove::<MoveOrder>();
                }
                continue;
            }
            commands.entity(entity).try_insert(MoveOrder {
                target: target_transform.translation,
            });
        } else {
            commands.entity(entity).try_remove::<MoveOrder>();
        }
    }
}

fn clear_attack_chase_order(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .try_remove::<AttackOrder>()
        .try_remove::<MoveOrder>();
}

fn attack_order_target_valid(
    attacker_team: &Team,
    target_team: &Team,
    target_domain: MovementDomain,
    target_health: &Health,
    weapon: &Weapon,
    relations: &TeamRelations,
) -> bool {
    target_health.current > 0.0
        && relations.are_enemies(*attacker_team, *target_team)
        && can_attack_domain(weapon, target_domain)
}

fn update_attack_move_and_patrol_orders(
    mut commands: Commands,
    relations: Res<TeamRelations>,
    units: Query<(
        Entity,
        &Transform,
        &Team,
        &Weapon,
        &VisionRadius,
        Option<&Unit>,
        Option<&AttackMoveOrder>,
        Option<&PatrolOrder>,
        Option<&MoveOrder>,
    )>,
    targets: Query<
        (
            Entity,
            &Transform,
            &Team,
            &Selectable,
            Option<&Structure>,
            &MovementDomain,
            Option<&VisibilityState>,
        ),
        With<Health>,
    >,
) {
    let target_snapshots: Vec<TargetSnapshot> = targets
        .iter()
        .map(
            |(entity, transform, team, selectable, structure, movement_domain, visibility)| {
                TargetSnapshot {
                    entity,
                    team: *team,
                    position: transform.translation,
                    radius: selectable.radius,
                    visible: visibility.is_none_or(|visibility| visibility.visible),
                    is_structure: structure.is_some(),
                    movement_domain: *movement_domain,
                }
            },
        )
        .collect();

    for (entity, transform, team, weapon, vision, unit, attack_move, patrol, move_order) in &units {
        if unit.is_some_and(|unit| unit.speed <= 0.0) {
            commands
                .entity(entity)
                .try_remove::<AttackMoveOrder>()
                .try_remove::<PatrolOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }
        if let Some(patrol_order) = patrol {
            let current_target = if patrol_order.moving_to_destination {
                patrol_order.destination
            } else {
                patrol_order.origin
            };
            if let Some(enemy) = nearest_enemy_in_range(
                *team,
                transform.translation,
                vision.0,
                weapon.can_attack_air,
                weapon.can_attack_ground,
                &target_snapshots,
                &relations,
            ) {
                commands
                    .entity(entity)
                    .try_insert(AttackOrder {
                        target: enemy.entity,
                    })
                    .try_remove::<MoveOrder>();
                continue;
            }

            if xz_distance(transform.translation, current_target) <= PATROL_TURN_DISTANCE {
                let moving_to_destination = !patrol_order.moving_to_destination;
                let next_target = if moving_to_destination {
                    patrol_order.destination
                } else {
                    patrol_order.origin
                };
                commands
                    .entity(entity)
                    .try_insert(PatrolOrder {
                        moving_to_destination,
                        ..*patrol_order
                    })
                    .try_insert(MoveOrder {
                        target: next_target,
                    });
                continue;
            }
            if move_order.is_none() {
                commands.entity(entity).try_insert(MoveOrder {
                    target: current_target,
                });
            }
            continue;
        }

        if let Some(attack_move_order) = attack_move {
            if let Some(enemy) = nearest_enemy_in_range(
                *team,
                transform.translation,
                vision.0,
                weapon.can_attack_air,
                weapon.can_attack_ground,
                &target_snapshots,
                &relations,
            ) {
                commands
                    .entity(entity)
                    .try_insert(AttackOrder {
                        target: enemy.entity,
                    })
                    .try_remove::<MoveOrder>();
                continue;
            }
            if xz_distance(transform.translation, attack_move_order.destination)
                <= ATTACK_MOVE_REACHED_DISTANCE
            {
                commands
                    .entity(entity)
                    .try_remove::<AttackMoveOrder>()
                    .try_remove::<MoveOrder>();
                continue;
            }
            if move_order.is_none() {
                commands.entity(entity).try_insert(MoveOrder {
                    target: attack_move_order.destination,
                });
            }
        }
    }
}

fn move_units(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut kill_credits: ResMut<KillCredits>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut unit_queries: ParamSet<(
        Query<(
            Entity,
            &Team,
            &Unit,
            &MovementDomain,
            &Selectable,
            &mut Transform,
            &MoveOrder,
            Option<&ChronoRelay>,
            Option<&EmpDisabled>,
            &Health,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &MovementDomain,
            &Health,
        )>,
        Query<(
            &mut Health,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
        )>,
        Query<
            (&Transform, &Selectable, Option<&Health>),
            (Or<(With<Structure>, With<ResourceNode>)>, Without<Unit>),
        >,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let static_obstacles = {
        let obstacles = unit_queries.p3();
        obstacles
            .iter()
            .filter_map(|(transform, selectable, health)| {
                if health.is_some_and(|health| health.current <= 0.0) {
                    return None;
                }
                Some(MovementObstacleSnapshot {
                    position: transform.translation,
                    radius: selectable.radius,
                })
            })
            .collect::<Vec<_>>()
    };
    let crush_targets = {
        let units = unit_queries.p1();
        units
            .iter()
            .filter_map(
                |(entity, team, transform, selectable, unit, domain, health)| {
                    if health.current <= 0.0
                        || !unit.can_be_crushed
                        || unit.speed <= 0.0
                        || *domain != MovementDomain::Terrain
                    {
                        return None;
                    }
                    Some(CrushTargetSnapshot {
                        entity,
                        team: *team,
                        position: transform.translation,
                        radius: selectable.radius,
                    })
                },
            )
            .collect::<Vec<_>>()
    };

    let mut crush_events = Vec::new();
    {
        let mut movers = unit_queries.p0();
        for (entity, team, unit, domain, selectable, mut transform, order, chrono, emp, health) in
            &mut movers
        {
            if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
                continue;
            }
            if unit.speed <= 0.0 {
                commands.entity(entity).try_remove::<MoveOrder>();
                continue;
            }
            let mut target = order.target;
            target.y = transform.translation.y;
            let delta = target - transform.translation;
            let distance = delta.length();
            if distance < MOVE_ORDER_REACHED_DISTANCE_M {
                commands.entity(entity).try_remove::<MoveOrder>();
                continue;
            }
            let speed = unit.speed * chrono.map_or(1.0, |chrono| chrono.speed_multiplier);
            let step = speed * time.delta_secs();
            let previous_position = transform.translation;
            let direction = delta.normalize();
            let move_direction = if *domain == MovementDomain::Terrain {
                movement_direction_around_static_obstacles(
                    previous_position,
                    target,
                    direction,
                    selectable.radius,
                    &static_obstacles,
                )
            } else {
                direction
            };
            let intended_position = previous_position + move_direction * step;
            transform.translation += move_direction * step.min(distance);
            let actual_position = transform.translation;
            let look_at = transform.translation + move_direction;
            if xz_distance(transform.translation, look_at) > 0.05 {
                transform.look_at(look_at, Vec3::Y);
            }
            if !unit.can_crush || *domain != MovementDomain::Terrain {
                continue;
            }
            let actual_displacement = xz_distance(previous_position, actual_position);
            let intended_displacement = xz_distance(previous_position, intended_position);
            if actual_displacement.max(intended_displacement) < CRUSH_MIN_FRAME_DISPLACEMENT_M {
                continue;
            }
            for target in &crush_targets {
                if target.entity == entity
                    || !relations.are_enemies(*team, target.team)
                    || !can_crush_target(
                        previous_position,
                        actual_position,
                        intended_position,
                        selectable.radius,
                        target,
                    )
                {
                    continue;
                }
                crush_events.push((
                    target.entity,
                    entity,
                    *team,
                    target.team,
                    previous_position,
                    target.position,
                    target.radius,
                ));
            }
        }
    }

    {
        let mut health_q = unit_queries.p2();
        for (target, source, team, target_team, from, to, target_radius) in crush_events {
            let Ok((mut health, shield, passive_shield)) = health_q.get_mut(target) else {
                continue;
            };
            if health.current <= 0.0 {
                continue;
            }
            let damage = CRUSH_DAMAGE * support_damage_scale(shield, passive_shield);
            health.current -= damage;
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.45,
                    to: to + Vec3::Y * 0.45,
                    ttl: 0.18,
                    team,
                },
                MatchScopedEntity,
            ));
            latest_battle_event.focus = Some(to);
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    match_state.units_lost += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                spawn_combat_wreckage(&mut commands, &asset_server, to, target_radius);
                commands.entity(target).try_despawn();
            }
        }
    }
}

#[derive(Clone, Copy)]
struct CrushTargetSnapshot {
    entity: Entity,
    team: Team,
    position: Vec3,
    radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct MovementObstacleSnapshot {
    position: Vec3,
    radius: f32,
}

fn movement_direction_around_static_obstacles(
    position: Vec3,
    target: Vec3,
    desired_direction: Vec3,
    mover_radius: f32,
    obstacles: &[MovementObstacleSnapshot],
) -> Vec3 {
    let desired_xz = Vec2::new(desired_direction.x, desired_direction.z).normalize_or_zero();
    if desired_xz == Vec2::ZERO {
        return desired_direction;
    }
    let target_distance = xz_distance(position, target);
    if target_distance <= f32::EPSILON {
        return desired_direction;
    }
    let lookahead = target_distance.min(MOVEMENT_OBSTACLE_LOOKAHEAD_M);
    let start = Vec2::new(position.x, position.z);
    let end = start + desired_xz * lookahead;

    let mut best_steer = None;
    for obstacle in obstacles {
        let center = Vec2::new(obstacle.position.x, obstacle.position.z);
        let clearance = mover_radius + obstacle.radius + MOVEMENT_OBSTACLE_CLEARANCE_M;
        let to_center = center - start;
        let projection = to_center.dot(desired_xz);
        if projection < -clearance || projection > lookahead {
            continue;
        }
        let segment_distance = distance_point_to_xz_segment(
            obstacle.position,
            position,
            Vec3::new(end.x, position.y, end.y),
        );
        let current_distance = start.distance(center);
        if segment_distance > clearance && current_distance > clearance {
            continue;
        }

        let side = desired_xz.perp_dot(to_center);
        let tangent = if side >= 0.0 {
            Vec2::new(desired_xz.y, -desired_xz.x)
        } else {
            Vec2::new(-desired_xz.y, desired_xz.x)
        };
        let away = (start - center).normalize_or_zero();
        let steer = (desired_xz + tangent * MOVEMENT_OBSTACLE_STEER_WEIGHT + away * 0.35)
            .normalize_or_zero();
        if steer == Vec2::ZERO {
            continue;
        }
        let urgency = clearance - segment_distance.min(current_distance);
        if best_steer.is_none_or(|(best_urgency, _)| urgency > best_urgency) {
            best_steer = Some((urgency, steer));
        }
    }

    if let Some((_, steer)) = best_steer {
        Vec3::new(steer.x, desired_direction.y, steer.y).normalize_or_zero()
    } else {
        desired_direction
    }
}

fn can_crush_target(
    from_position: Vec3,
    actual_position: Vec3,
    intended_position: Vec3,
    crusher_radius: f32,
    target: &CrushTargetSnapshot,
) -> bool {
    let crush_distance = crusher_radius + target.radius + CRUSH_RADIUS_MARGIN_M;
    distance_point_to_xz_segment(target.position, from_position, actual_position) <= crush_distance
        || (xz_distance(actual_position, intended_position) > CRUSH_MIN_FRAME_DISPLACEMENT_M
            && distance_point_to_xz_segment(target.position, from_position, intended_position)
                <= crush_distance)
}

#[derive(Clone, Copy)]
struct TargetSnapshot {
    entity: Entity,
    team: Team,
    position: Vec3,
    radius: f32,
    visible: bool,
    is_structure: bool,
    movement_domain: MovementDomain,
}

fn combat(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    economies: Res<Economies>,
    relations: Res<TeamRelations>,
    player_factions: Res<PlayerFactions>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut attackers: Query<(
        Entity,
        &Transform,
        &Team,
        &mut Weapon,
        &VisionRadius,
        Option<&Unit>,
        Option<&HoldPosition>,
        Option<&AttackOrder>,
        Option<&FollowOrder>,
        Option<&Garrison>,
        Option<&EmpDisabled>,
        Option<&MoveOrder>,
        Option<&Structure>,
    )>,
    mut health_q: Query<(
        Entity,
        &Transform,
        &Team,
        &Selectable,
        &MovementDomain,
        Option<&Structure>,
        &mut Health,
        Option<&SupportShield>,
        Option<&PassiveSupportShield>,
        Option<&FogMemoryVisible>,
    )>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut kill_credits: ResMut<KillCredits>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let targets: Vec<_> = health_q
        .iter()
        .filter(|(_, _, _, _, _, _, health, _, _, _)| health.current > 0.0)
        .map(
            |(entity, transform, team, selectable, movement_domain, structure, _, _, _, _)| {
                TargetSnapshot {
                    entity,
                    team: *team,
                    position: transform.translation,
                    radius: selectable.radius,
                    visible: true,
                    movement_domain: *movement_domain,
                    is_structure: structure.is_some(),
                }
            },
        )
        .collect();
    let mut damage_events = Vec::new();

    for (
        entity,
        transform,
        team,
        mut weapon,
        vision,
        unit,
        hold_position,
        attack_order,
        follow_order,
        garrison,
        emp,
        move_order,
        structure,
    ) in &mut attackers
    {
        if emp.is_some_and(|emp| emp.remaining > 0.0) {
            continue;
        }
        if follow_order.is_some_and(|follow| follow.allow_enemy) {
            continue;
        }
        if powered_combat_offline(team, structure, &economies) {
            continue;
        }
        weapon.cooldown_left = (weapon.cooldown_left - time.delta_secs()).max(0.0);
        let attack_damage = garrison.map_or(weapon.damage, |garrison| {
            garrison.count as f32 * garrison.damage_per_unit
        });
        if attack_damage <= 0.0 {
            continue;
        }
        let attacker_faction = player_factions.faction(*team);
        if is_tesla_fence_structure(structure) {
            if weapon.cooldown_left > 0.0 {
                continue;
            }
            let zap_targets = targets
                .iter()
                .copied()
                .filter(|target| {
                    can_tesla_fence_zap_target(
                        *team,
                        transform.translation,
                        weapon.range,
                        target,
                        &relations,
                    )
                })
                .collect::<Vec<_>>();
            if zap_targets.is_empty() {
                continue;
            }
            weapon.cooldown_left = weapon_cooldown_for_faction(attacker_faction, weapon.cooldown);
            for target in zap_targets {
                damage_events.push((
                    target.entity,
                    attack_damage,
                    transform.translation,
                    target.position,
                    target.radius,
                    *team,
                    attacker_faction,
                    target.is_structure,
                    target.team,
                    player_factions.faction(target.team),
                    entity,
                ));
            }
            continue;
        }
        let ordered_target = attack_order.and_then(|order| {
            targets
                .iter()
                .find(|target| {
                    target.entity == order.target
                        && relations.are_enemies(*team, target.team)
                        && can_attack_domain(&weapon, target.movement_domain)
                })
                .copied()
        });
        let moving_with_active_order = moving_weapon_fire_blocked(unit, move_order);
        let auto_target = if ordered_target.is_none()
            && !moving_with_active_order
            && !hold_position.is_some_and(|hold| hold.enabled)
        {
            nearest_enemy_for_auto_acquire(
                *team,
                transform.translation,
                &weapon,
                vision,
                unit,
                &targets,
                &relations,
            )
        } else {
            None
        };
        let target = ordered_target.or(auto_target);

        let Some(target) = target else {
            continue;
        };
        if ordered_target.is_none() {
            commands.entity(entity).try_insert(AttackOrder {
                target: target.entity,
            });
        }
        if moving_with_active_order {
            continue;
        }
        if xz_distance(transform.translation, target.position) > weapon.range
            || weapon.cooldown_left > 0.0
        {
            continue;
        }
        weapon.cooldown_left = weapon_cooldown_for_faction(attacker_faction, weapon.cooldown);
        let damage = weapon_damage_against_target(&weapon, attack_damage, target.is_structure);
        damage_events.push((
            target.entity,
            damage,
            transform.translation,
            target.position,
            target.radius,
            *team,
            attacker_faction,
            target.is_structure,
            target.team,
            player_factions.faction(target.team),
            entity,
        ));
        if weapon.splash_radius > 0.0 && weapon.splash_damage_multiplier > 0.0 {
            for splash_target in &targets {
                if splash_target.entity == target.entity
                    || !relations.are_enemies(*team, splash_target.team)
                    || !can_attack_domain(&weapon, splash_target.movement_domain)
                    || xz_distance(splash_target.position, target.position) > weapon.splash_radius
                {
                    continue;
                }
                let splash_damage = weapon_damage_against_target(
                    &weapon,
                    attack_damage,
                    splash_target.is_structure,
                ) * weapon.splash_damage_multiplier;
                damage_events.push((
                    splash_target.entity,
                    splash_damage,
                    target.position,
                    splash_target.position,
                    splash_target.radius,
                    *team,
                    attacker_faction,
                    splash_target.is_structure,
                    splash_target.team,
                    player_factions.faction(splash_target.team),
                    entity,
                ));
            }
        }
    }

    for (
        target,
        damage,
        from,
        to,
        target_radius,
        team,
        attacker_faction,
        target_is_structure,
        target_team,
        target_faction,
        source,
    ) in damage_events
    {
        if let Ok((entity, _, _, _, _, _, mut health, shield, passive_shield, fog_memory)) =
            health_q.get_mut(target)
        {
            if health.current <= 0.0 {
                continue;
            }
            let applied_damage = applied_weapon_damage(
                damage,
                attacker_faction,
                target_team,
                target_faction,
                target_is_structure,
                shield,
                passive_shield,
            );
            health.current -= applied_damage;
            if relations.are_allied(target_team, player_team) && applied_damage > 0.0 {
                record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::WeaponHit);
                if push_under_attack_log(&mut battle_log, to, target_is_structure) {
                    record_voice_audio_feedback(
                        &mut audio_feedback,
                        if target_is_structure {
                            UnitVoiceEvent::BaseUnderAttack
                        } else {
                            UnitVoiceEvent::UnitUnderAttack
                        },
                    );
                }
            }
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.6,
                    to: to + Vec3::Y * 0.6,
                    ttl: 0.16,
                    team,
                },
                MatchScopedEntity,
            ));
            latest_battle_event.focus = Some(to);
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    if target_is_structure {
                        match_state.structures_lost += 1;
                    } else {
                        match_state.units_lost += 1;
                    }
                } else if target_is_structure {
                    match_state.enemy_structures_destroyed += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                if relations.are_allied(target_team, player_team) {
                    record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::Explosion);
                    if !target_is_structure {
                        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::UnitLost);
                    }
                }
                spawn_destruction_effects(
                    &mut commands,
                    &asset_server,
                    to,
                    target_radius,
                    target_is_structure,
                    target_team,
                    target_is_structure && fog_memory.is_some(),
                );
                commands.entity(entity).try_despawn();
            }
        }
    }
}

fn moving_weapon_fire_blocked(unit: Option<&Unit>, move_order: Option<&MoveOrder>) -> bool {
    move_order.is_some() && unit.is_some_and(|unit| unit.speed > 0.0)
}

fn powered_combat_offline(
    team: &Team,
    structure: Option<&Structure>,
    economies: &Economies,
) -> bool {
    let Some(structure) = structure else {
        return false;
    };
    if matches!(team, Team::Neutral) || !economies.get(*team).low_power() {
        return false;
    }
    registry::entity(structure.id).is_some_and(|def| def.power_delta < 0 && def.weapon.is_some())
}

fn is_tesla_fence_structure(structure: Option<&Structure>) -> bool {
    structure.is_some_and(|structure| structure.id == "TeslaFenceSegment")
}

fn can_tesla_fence_zap_target(
    team: Team,
    position: Vec3,
    range: f32,
    target: &TargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    relations.are_enemies(team, target.team)
        && !target.is_structure
        && target.movement_domain == MovementDomain::Terrain
        && xz_distance(position, target.position) <= range + target.radius
}

fn weapon_cooldown_for_faction(faction: Option<SkirmishFaction>, cooldown: f32) -> f32 {
    if faction == Some(SkirmishFaction::Alliance) {
        cooldown
    } else {
        cooldown * 1.08
    }
}

fn weapon_damage_against_target(
    weapon: &Weapon,
    base_damage: f32,
    target_is_structure: bool,
) -> f32 {
    if target_is_structure {
        base_damage * weapon.structure_damage_multiplier
    } else {
        base_damage
    }
}

fn nearest_enemy_in_range(
    team: Team,
    position: Vec3,
    range: f32,
    can_attack_air: bool,
    can_attack_ground: bool,
    targets: &[TargetSnapshot],
    relations: &TeamRelations,
) -> Option<TargetSnapshot> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for target in targets {
        if !relations.are_enemies(team, target.team) {
            continue;
        }
        if !target.visible {
            continue;
        }
        if !can_attack_domain_for_movement(
            can_attack_air,
            can_attack_ground,
            target.movement_domain,
        ) {
            continue;
        }
        let distance = xz_distance(position, target.position);
        if distance <= range && distance < nearest_distance {
            nearest = Some(*target);
            nearest_distance = distance;
        }
    }
    nearest
}

fn nearest_enemy_for_auto_acquire(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    vision: &VisionRadius,
    unit: Option<&Unit>,
    targets: &[TargetSnapshot],
    relations: &TeamRelations,
) -> Option<TargetSnapshot> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for target in targets {
        if !can_auto_acquire_target(team, position, weapon, vision, unit, target, relations) {
            continue;
        }
        let distance = xz_distance(position, target.position);
        if distance < nearest_distance {
            nearest = Some(*target);
            nearest_distance = distance;
        }
    }
    nearest
}

fn can_auto_acquire_target(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    vision: &VisionRadius,
    unit: Option<&Unit>,
    target: &TargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    if !relations.are_enemies(team, target.team) {
        return false;
    }
    if !can_attack_domain(weapon, target.movement_domain) {
        return false;
    }
    let distance = xz_distance(position, target.position);
    if unit.is_some_and(|unit| unit.speed > 0.0) {
        distance <= vision.0
    } else {
        distance <= weapon.range
    }
}

fn can_attack_domain(weapon: &Weapon, domain: MovementDomain) -> bool {
    can_attack_domain_for_movement(weapon.can_attack_air, weapon.can_attack_ground, domain)
}

fn can_attack_domain_for_movement(
    can_attack_air: bool,
    can_attack_ground: bool,
    domain: MovementDomain,
) -> bool {
    match domain {
        MovementDomain::Air => can_attack_air,
        MovementDomain::Terrain => can_attack_ground,
    }
}

fn update_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut pulses: Query<(Entity, &mut ShotPulse)>,
) {
    for (entity, mut pulse) in &mut pulses {
        pulse.ttl -= time.delta_secs();
        if pulse.ttl <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

fn update_objective_tracker_hud(
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    mut objective_tracker: ResMut<ObjectiveTrackerState>,
    structures: Query<(&Structure, &Team, &Health)>,
    units: Query<(&Unit, &Team, &Health)>,
    mut objective_text: Query<&mut Text, With<ObjectiveTrackerText>>,
) {
    let Ok(mut text) = objective_text.single_mut() else {
        return;
    };
    let snapshot = objective_tracker_snapshot(
        visible_player.team,
        &relations,
        &structures,
        &units,
        &mut objective_tracker,
    );
    **text = objective_tracker_text(snapshot);
}

fn update_hud(
    economies: Res<Economies>,
    build_queue: Res<BuildQueue>,
    visible_player: Res<VisiblePlayer>,
    selected: Query<
        (
            Entity,
            Option<&Unit>,
            Option<&Structure>,
            Option<&Garrison>,
            Option<&ResourceCargo>,
            Option<&Veterancy>,
            Option<&Weapon>,
            &Health,
            &Team,
        ),
        With<Selected>,
    >,
    units: Query<&Team, With<Unit>>,
    support_cooldowns: Res<SupportCooldowns>,
    mut stats_text: Query<
        &mut Text,
        (
            With<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueText>,
        ),
    >,
    mut selection_text: Query<
        &mut Text,
        (
            With<SelectionText>,
            Without<StatsText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueText>,
        ),
    >,
    mut production_queue_text: Query<
        &mut Text,
        (
            With<ProductionQueueText>,
            Without<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
        ),
    >,
    mut production_queue_slots: Query<(
        &ProductionQueueSlot,
        &mut ProductionQueueSlotTarget,
        &mut BackgroundColor,
        &mut Visibility,
    )>,
    mut production_queue_slot_labels: Query<
        (&ProductionQueueSlotLabel, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ProductionQueueText>,
            Without<ObjectiveTrackerText>,
        ),
    >,
    command_mode: Res<CommandMode>,
    placement_feedback: Res<StructurePlacementFeedback>,
    unit_groups: Res<UnitGroups>,
    ai_settings: Res<AiDifficultySettings>,
    active_teams: Option<Res<ActiveTeams>>,
) {
    let visible_team = visible_player.team;
    if let Ok(mut text) = stats_text.single_mut() {
        let visible_economy = economies.get(visible_team);
        let mut unit_count = Vec::new();
        for team in &units {
            if let Some(idx) = team.economy_index() {
                if unit_count.len() <= idx {
                    unit_count.resize(idx + 1, 0);
                }
                unit_count[idx] += 1;
            }
        }
        let unit_status = dynamic_unit_status_text(&unit_count, active_teams.as_deref());
        let mode_text = if let Some(pending) = command_mode.pending_structure_placement {
            let label = registry::entity(pending.id).map_or(pending.id, |def| def.label);
            let feedback = placement_feedback
                .validity
                .and_then(structure_placement_feedback_text)
                .map(|message| format!(" {message}"))
                .unwrap_or_default();
            format!(" 摆放:{label}{feedback} R旋转 右键取消")
        } else if command_mode.attack_move {
            " 模式:攻击移动".to_string()
        } else if command_mode.patrol {
            " 模式:巡逻".to_string()
        } else if command_mode.rally_point {
            " 模式:设置集结".to_string()
        } else if let Some(power) = command_mode.support_power {
            let remaining = support_cooldowns.remaining_for(visible_team, power);
            if remaining > 0.0 {
                format!(" 支援:{} (冷却{remaining:.1}s)", power.label())
            } else {
                " 支援:就绪".to_string()
            }
        } else {
            String::new()
        };
        let mut support_status = " 支援CD:".to_string();
        for power in SupportPowerKind::ALL {
            let remaining = support_cooldowns.remaining_for(visible_team, power);
            if remaining > 0.0 {
                support_status.push_str(&format!(" {} {remaining:.0}", power.label()));
            } else {
                support_status.push_str(&format!(" {} 就绪", power.label()));
            }
        }
        **text = format!(
            "{}  Ore {}  Crystal {}  {}{}{}    Units {}    {}",
            visible_team.label(),
            visible_economy.ore,
            visible_economy.crystal,
            power_status_text(visible_economy),
            mode_text,
            support_status,
            unit_status,
            ai_hud_status_text(
                controlled_player_team(Some(&*visible_player)),
                &ai_settings,
                active_teams.as_deref(),
            ),
        );
        text.push_str(&ai_low_power_status_text(
            controlled_player_team(Some(&*visible_player)),
            &economies,
            active_teams.as_deref(),
        ));
    }

    if let Ok(mut text) = selection_text.single_mut() {
        let mut items = Vec::new();
        let mut selected_visible_entities = Vec::new();
        let mut selected_visible_count = 0usize;
        let mut selected_queue_producers = Vec::new();
        for (entity, unit, structure, garrison, cargo, veteran, weapon, health, team) in &selected {
            if *team == visible_team {
                selected_visible_entities.push(entity);
                selected_visible_count += 1;
                if structure.is_some_and(|structure| structure_has_production_queue(structure.id)) {
                    selected_queue_producers.push(entity);
                }
            }
            let label = unit
                .and_then(|unit| registry::entity(unit.id).map(|def| def.label))
                .or_else(|| {
                    structure
                        .and_then(|structure| registry::entity(structure.id).map(|def| def.label))
                })
                .unwrap_or("Entity");
            items.push(SelectionHudItem {
                label: label.to_string(),
                team: *team,
                health_current: health.current.max(0.0),
                health_max: health.max,
                attack: weapon.map(|weapon| (weapon.damage, weapon.range)),
                rank: veteran.map_or(0, |veteran| veteran.rank),
                garrison: garrison.map(|garrison| (garrison.count, garrison.capacity)),
                cargo: cargo
                    .filter(|cargo| cargo.capacity > 0)
                    .map(|cargo| (cargo.total(), cargo.capacity, cargo.ore, cargo.crystal)),
            });
        }
        **text = selection_hud_text(
            &items,
            exact_control_group_slot(&unit_groups, &selected_visible_entities),
        );
        if let Ok(mut text) = production_queue_text.single_mut() {
            let observed_queue_producers =
                if selected_visible_count == selected_queue_producers.len() {
                    selected_queue_producers.as_slice()
                } else {
                    &[]
                };
            **text = production_queue_hud_text(
                visible_team,
                &build_queue,
                &economies,
                observed_queue_producers,
            );
            render_production_queue_slots(
                visible_team,
                &build_queue,
                &economies,
                observed_queue_producers,
                &mut production_queue_slots,
                &mut production_queue_slot_labels,
            );
        }
    }
}

#[derive(Clone)]
struct SelectionHudItem {
    label: String,
    team: Team,
    health_current: f32,
    health_max: f32,
    attack: Option<(f32, f32)>,
    rank: u8,
    garrison: Option<(usize, usize)>,
    cargo: Option<(i32, i32, i32, i32)>,
}

fn selection_hud_text(items: &[SelectionHudItem], control_group: Option<usize>) -> String {
    if items.is_empty() {
        return String::new();
    }
    let group_text = control_group
        .map(|slot| format!("  编组 {slot}"))
        .unwrap_or_default();
    if items.len() == 1 {
        let item = &items[0];
        let attack_text = item
            .attack
            .map(|(damage, range)| format!("ATK {damage:.1} RNG {range:.1}"))
            .unwrap_or_else(|| "ATK -".to_string());
        let mut parts = vec![
            format!("{}  {}", item.team.label(), item.label),
            format!("HP {:.0}/{:.0}", item.health_current, item.health_max),
            attack_text,
            format!("军阶: {}", veterancy_rank_label(item.rank)),
        ];
        if let Some(badge) = veterancy_rank_badge(item.rank) {
            parts.push(format!("徽章 {badge}"));
        }
        if let Some((count, capacity)) = item.garrison {
            parts.push(format!("驻军 {count}/{capacity}"));
        }
        if let Some((total, capacity, ore, crystal)) = item.cargo {
            parts.push(format!("载货 {total}/{capacity} ({ore}/{crystal})"));
        }
        return format!("{}{}", parts.join("  "), group_text);
    }

    let mut type_counts = BTreeMap::new();
    let mut rank_counts = BTreeMap::new();
    for item in items {
        *type_counts.entry(item.label.clone()).or_insert(0usize) += 1;
        if item.rank > 0 {
            *rank_counts
                .entry(veterancy_rank_label(item.rank).to_string())
                .or_insert(0usize) += 1;
        }
    }
    let type_text = type_counts
        .iter()
        .map(|(label, count)| format!("{label} x{count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let rank_text = if rank_counts.is_empty() {
        "军阶: 新兵".to_string()
    } else {
        format!(
            "军阶: {}",
            rank_counts
                .iter()
                .map(|(rank, count)| format!("{rank} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "已选择 {}{}  类型: {}  {}",
        items.len(),
        group_text,
        type_text,
        rank_text
    )
}

fn veterancy_rank_label(rank: u8) -> &'static str {
    match rank {
        0 => "新兵",
        1 => "老兵",
        _ => "精英",
    }
}

fn veterancy_rank_badge(rank: u8) -> Option<&'static str> {
    match rank {
        1 => Some("V"),
        2.. => Some("E"),
        _ => None,
    }
}

fn exact_control_group_slot(
    unit_groups: &UnitGroups,
    selected_entities: &[Entity],
) -> Option<usize> {
    if selected_entities.is_empty() {
        return None;
    }
    unit_groups
        .slots
        .iter()
        .position(|slot| is_exact_current_selection(selected_entities, slot))
        .map(|index| index + 1)
}

fn power_status_text(economy: &TeamEconomy) -> String {
    let base = format!("电力 {}/{}", economy.power_capacity, economy.power_used);
    if economy.low_power() {
        format!("{base} 低电力: 生产减速/防御停火/雷达离线")
    } else {
        base
    }
}

fn dynamic_unit_status_text(unit_count: &[usize], active_teams: Option<&ActiveTeams>) -> String {
    let slot_count = active_teams.map_or(unit_count.len(), |active| {
        active.0.len().max(unit_count.len())
    });
    let mut parts = Vec::new();
    for index in 0..slot_count {
        let count = unit_count.get(index).copied().unwrap_or(0);
        let active = active_teams
            .and_then(|active| active.0.get(index).copied())
            .unwrap_or(false);
        if active || count > 0 {
            parts.push(format!("P{}:{}", index + 1, count));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

fn team_hud_short_label(team: Team) -> String {
    match team {
        Team::Player(index) => format!("P{}", index + 1),
        Team::Neutral => "中".to_string(),
    }
}

fn ai_hud_status_text(
    controlled_team: Option<Team>,
    ai_settings: &AiDifficultySettings,
    active_teams: Option<&ActiveTeams>,
) -> String {
    let mut text = String::from("AI");
    for team in active_ai_teams(controlled_team, active_teams) {
        text.push_str(&format!(
            " {}:{}",
            team_hud_short_label(team),
            ai_settings.difficulty(team).label()
        ));
    }
    text
}

fn ai_low_power_status_text(
    controlled_team: Option<Team>,
    economies: &Economies,
    active_teams: Option<&ActiveTeams>,
) -> String {
    let mut text = String::new();
    for team in active_ai_teams(controlled_team, active_teams) {
        if economies.get(team).low_power() {
            text.push_str(&format!("    {} LOW", team_hud_short_label(team)));
        }
    }
    text
}

#[derive(Clone, Copy)]
struct ProductionQueueHudEntry {
    producer_entity: Entity,
    local_index: usize,
    action: BuildAction,
    progress: f32,
    active: bool,
}

fn production_queue_hud_text(
    team: Team,
    build_queue: &BuildQueue,
    economies: &Economies,
    producer_entities: &[Entity],
) -> String {
    if producer_entities.is_empty() {
        return String::new();
    }

    let mut rows = Vec::new();
    for producer_entity in producer_entities {
        let mut active = true;
        for job in build_queue
            .0
            .iter()
            .filter(|job| job.team == team && job.producer_entity == *producer_entity)
        {
            if let Some(row) = production_queue_job_text(active, job, economies) {
                rows.push(row);
            }
            active = false;
        }
    }

    if rows.is_empty() {
        String::new()
    } else {
        format!("生产队列: {}", rows.join("  |  "))
    }
}

fn render_production_queue_slots(
    team: Team,
    build_queue: &BuildQueue,
    economies: &Economies,
    producer_entities: &[Entity],
    slots: &mut Query<(
        &ProductionQueueSlot,
        &mut ProductionQueueSlotTarget,
        &mut BackgroundColor,
        &mut Visibility,
    )>,
    labels: &mut Query<
        (&ProductionQueueSlotLabel, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ProductionQueueText>,
            Without<ObjectiveTrackerText>,
        ),
    >,
) {
    let entries = production_queue_hud_entries(team, build_queue, producer_entities);
    for (slot, mut target, mut color, mut visibility) in slots {
        if let Some(entry) = entries.get(slot.0).copied() {
            target.producer_entity = Some(entry.producer_entity);
            target.local_index = entry.local_index;
            *visibility = Visibility::Visible;
            *color = BackgroundColor(production_queue_slot_color(team, entry, economies));
        } else {
            *target = ProductionQueueSlotTarget::default();
            *visibility = Visibility::Hidden;
            *color = BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.9));
        }
    }
    for (label, mut text) in labels {
        **text = entries
            .get(label.0)
            .map(|entry| production_queue_slot_text(team, label.0, *entry, economies))
            .unwrap_or_default();
    }
}

fn production_queue_hud_entries(
    team: Team,
    build_queue: &BuildQueue,
    producer_entities: &[Entity],
) -> Vec<ProductionQueueHudEntry> {
    let mut entries = Vec::new();
    for producer_entity in producer_entities {
        let mut local_index = 0usize;
        for job in build_queue
            .0
            .iter()
            .filter(|job| job.team == team && job.producer_entity == *producer_entity)
        {
            let progress = registry::entity(build_target_product(job.action))
                .map(|def| production_job_progress(job, def))
                .unwrap_or(100.0);
            entries.push(ProductionQueueHudEntry {
                producer_entity: *producer_entity,
                local_index,
                action: job.action,
                progress,
                active: local_index == 0,
            });
            local_index += 1;
        }
    }
    entries
}

fn production_queue_slot_text(
    team: Team,
    display_index: usize,
    entry: ProductionQueueHudEntry,
    economies: &Economies,
) -> String {
    let label = build_action_target_label(entry.action).unwrap_or_else(|| "无效".to_string());
    let status = if entry.active && entry.progress >= 100.0 {
        "就绪"
    } else if !entry.active {
        "等待"
    } else if economies.get(team).low_power() {
        "低电"
    } else {
        "生产"
    };
    format!(
        "{} {} {:.0}%\n{}",
        display_index + 1,
        compact_label(&label),
        entry.progress,
        status
    )
}

fn production_queue_slot_color(
    team: Team,
    entry: ProductionQueueHudEntry,
    economies: &Economies,
) -> Color {
    if entry.active && entry.progress >= 100.0 {
        Color::srgba(0.18, 0.11, 0.02, 0.96)
    } else if !entry.active {
        Color::srgba(0.025, 0.035, 0.045, 0.9)
    } else if economies.get(team).low_power() {
        Color::srgba(0.18, 0.13, 0.04, 0.96)
    } else {
        Color::srgba(0.05, 0.11, 0.14, 0.94)
    }
}

fn structure_has_production_queue(structure_id: &str) -> bool {
    matches!(
        structure_id,
        "CommandCenter" | "Barracks" | "VehicleFactory" | "AircraftFactory"
    )
}

fn production_queue_job_text(
    active: bool,
    job: &BuildJob,
    economies: &Economies,
) -> Option<String> {
    let label = build_action_target_label(job.action)?;
    let Some(def) = registry::entity(build_target_product(job.action)) else {
        return Some(format!("{label} 无效"));
    };
    let progress = production_job_progress(job, def);
    let status = if active && progress >= 100.0 {
        "就绪/阻塞".to_string()
    } else if !active {
        "排队".to_string()
    } else if economies.get(job.team).low_power() {
        "低电力生产中".to_string()
    } else {
        "生产中".to_string()
    };
    Some(format!("{label} {progress:.0}% {status}"))
}

fn production_job_progress(job: &BuildJob, def: &registry::EntityDef) -> f32 {
    if def.build_seconds <= 0.0 {
        return 100.0;
    }
    ((def.build_seconds - job.timer).max(0.0) / def.build_seconds * 100.0).clamp(0.0, 100.0)
}

fn build_action_target_label(action: BuildAction) -> Option<String> {
    let id = match action {
        BuildAction::Train(id) | BuildAction::Build(id) => id,
        _ => return None,
    };
    registry::entity(id).map(|def| compact_label(def.label))
}

fn draw_world_overlays(
    mut gizmos: Gizmos,
    selected: Query<
        (
            &Transform,
            &Selectable,
            &Team,
            &Health,
            &MovementDomain,
            Option<&MoveOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
            Option<&OrderQueue>,
            Option<&Unit>,
            Option<&Structure>,
        ),
        With<Selected>,
    >,
    all: Query<
        (
            &Transform,
            &Selectable,
            &Team,
            &Health,
            Option<&Unit>,
            Option<&Structure>,
        ),
        Without<Selected>,
    >,
    pulses: Query<&ShotPulse>,
    warnings: Query<(&Transform, &SupportWarning)>,
    reveals: Query<(&Transform, &TemporarySupportReveal)>,
    orbital_strikes: Query<(&Transform, &PendingOrbitalStrike)>,
    click_markers: Query<(&Transform, &ClickMarker)>,
    destruction_vfx: Query<(&Transform, &StructureDestructionVfx)>,
    promotion_vfx: Query<(&Transform, &VeterancyPromotionEffect)>,
    resources: Query<(&Transform, &Selectable, &ResourceNode, &VisibilityState)>,
    supply_crates: Query<(&Transform, &Selectable, &SupplyCrate, &VisibilityState)>,
    visible_player: Res<VisiblePlayer>,
    player_colors: Res<PlayerColorSlots>,
    placement_preview: StructurePlacementPreviewParams,
) {
    let visible_team = visible_player.team;
    for (
        transform,
        selectable,
        team,
        health,
        movement_domain,
        move_order,
        attack_move_order,
        patrol_order,
        order_queue,
        unit,
        structure,
    ) in &selected
    {
        draw_ring(
            &mut gizmos,
            transform.translation,
            selectable.radius + 0.18,
            Color::srgb(0.62, 0.95, 0.64),
        );
        if should_draw_air_to_terrain_marker(*movement_domain) {
            draw_air_to_terrain_marker(
                &mut gizmos,
                transform.translation,
                selectable.radius,
                *team,
                visible_team,
            );
        }
        draw_health_bar(
            &mut gizmos,
            transform.translation,
            selectable.radius,
            *health,
        );
        if should_draw_action_queue_path(*team, visible_team) {
            let path_points = selected_terrain_order_path_points(
                move_order,
                attack_move_order,
                patrol_order,
                order_queue,
            );
            draw_terrain_order_path(&mut gizmos, transform.translation, &path_points);
        }
        if should_draw_team_marker_for_entity(unit, structure) && *team != visible_team {
            draw_team_marker(
                &mut gizmos,
                transform.translation,
                selectable.radius,
                *team,
                &player_colors,
            );
        }
    }
    for (transform, selectable, team, health, unit, structure) in &all {
        if should_draw_team_marker_for_entity(unit, structure) {
            draw_team_marker(
                &mut gizmos,
                transform.translation,
                selectable.radius,
                *team,
                &player_colors,
            );
        }
        if health.current < health.max {
            draw_health_bar(
                &mut gizmos,
                transform.translation,
                selectable.radius,
                *health,
            );
        }
    }
    for pulse in &pulses {
        gizmos.line(pulse.from, pulse.to, player_colors.color(pulse.team));
    }
    for (transform, marker) in &click_markers {
        draw_ring(
            &mut gizmos,
            transform.translation,
            marker.radius,
            Color::srgba(1.0, 1.0, 1.0, 0.55),
        );
    }
    for (transform, selectable, resource, visibility) in &resources {
        if !visibility.visible || resource.amount <= 0 {
            continue;
        }
        draw_ring(
            &mut gizmos,
            transform.translation,
            selectable.radius + 0.08,
            resource.kind.color(),
        );
    }
    for (transform, selectable, supply_crate, visibility) in &supply_crates {
        if !visibility.visible {
            continue;
        }
        draw_ring(
            &mut gizmos,
            transform.translation,
            selectable.radius + 0.1,
            supply_crate.effect.color(),
        );
    }
    for (transform, warning) in &warnings {
        draw_ring(
            &mut gizmos,
            transform.translation,
            warning.radius,
            warning.color,
        );
    }
    for (transform, reveal) in &reveals {
        draw_ring(
            &mut gizmos,
            transform.translation,
            reveal.radius,
            Color::srgba(0.2, 0.86, 0.94, 0.35),
        );
    }
    for (transform, strike) in &orbital_strikes {
        draw_ring(
            &mut gizmos,
            transform.translation,
            strike.radius,
            Color::srgba(1.0, 0.4, 0.15, 0.42),
        );
    }
    for (transform, effect) in &destruction_vfx {
        draw_structure_destruction_vfx(&mut gizmos, transform.translation, effect, &player_colors);
    }
    for (transform, effect) in &promotion_vfx {
        draw_veterancy_promotion_effect(&mut gizmos, transform.translation, effect, &player_colors);
    }
    for i in -24..=24 {
        let c = if i % 4 == 0 {
            Color::srgba(0.5, 0.58, 0.6, 0.16)
        } else {
            Color::srgba(0.5, 0.58, 0.6, 0.06)
        };
        gizmos.line(
            Vec3::new(i as f32, 0.012, -MAP_HALF_EXTENT),
            Vec3::new(i as f32, 0.012, MAP_HALF_EXTENT),
            c,
        );
        gizmos.line(
            Vec3::new(-MAP_HALF_EXTENT, 0.012, i as f32),
            Vec3::new(MAP_HALF_EXTENT, 0.012, i as f32),
            c,
        );
    }
    if let Some(pending) = placement_preview.command_mode.pending_structure_placement
        && let Ok(window) = placement_preview.window_q.single()
        && let Some(point) = pending.position.or_else(|| {
            (!cursor_is_over_hud(window))
                .then(|| pointer_ground(window, &placement_preview.camera_q))
                .flatten()
        })
    {
        draw_structure_placement_preview(
            &mut gizmos,
            pending,
            placement_preview.visible_player.team,
            placement_preview
                .player_factions
                .slot_faction(placement_preview.visible_player.team),
            point,
            *placement_preview.map_bounds,
            &placement_preview.economies,
            &placement_preview.structures,
            &placement_preview.occupiers,
        );
    }
}

fn should_draw_team_marker_for_entity(unit: Option<&Unit>, structure: Option<&Structure>) -> bool {
    unit.is_some() && structure.is_none()
}

fn draw_structure_placement_preview(
    gizmos: &mut Gizmos,
    pending: PendingStructurePlacement,
    team: Team,
    faction: SkirmishFaction,
    point: Vec3,
    bounds: MapBounds,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
) {
    let Some(def) = registry::entity(pending.id) else {
        return;
    };
    let validity = structure_placement_validity_for_faction(
        team, faction, pending.id, point, bounds, economies, structures, occupiers,
    );
    let color = structure_placement_preview_color(validity);
    draw_structure_placement_footprint(
        gizmos,
        point,
        def.radius,
        pending.rotation_y_radians(),
        color,
    );
    if validity != StructurePlacementValidity::Valid {
        draw_ring(gizmos, point, def.radius + 0.28, color);
    }
}

fn draw_structure_placement_footprint(
    gizmos: &mut Gizmos,
    point: Vec3,
    radius: f32,
    rotation_y_radians: f32,
    color: Color,
) {
    draw_ring(gizmos, point, radius, color);
    let center = terrain_overlay_point(point);
    let rotation = Quat::from_rotation_y(rotation_y_radians);
    let forward = rotation * Vec3::Z;
    let right = rotation * Vec3::X;
    let half_width = radius.max(0.45);
    gizmos.line(
        center - right * half_width,
        center + right * half_width,
        color,
    );
    gizmos.line(
        center - forward * (half_width * 0.55),
        center + forward * (half_width + 0.65),
        color,
    );
}

fn structure_placement_preview_color(validity: StructurePlacementValidity) -> Color {
    match validity {
        StructurePlacementValidity::Valid => Color::srgba(0.32, 1.0, 0.58, 0.88),
        StructurePlacementValidity::NotEnoughResources => Color::srgba(1.0, 0.78, 0.24, 0.9),
        StructurePlacementValidity::OutOfBaseRadius => Color::srgba(1.0, 0.5, 0.18, 0.9),
        StructurePlacementValidity::MissingTech => Color::srgba(0.72, 0.54, 1.0, 0.9),
        StructurePlacementValidity::OutOfMap | StructurePlacementValidity::CollidesWithObject => {
            Color::srgba(1.0, 0.2, 0.16, 0.9)
        }
    }
}

fn selected_terrain_order_path_points(
    move_order: Option<&MoveOrder>,
    attack_move_order: Option<&AttackMoveOrder>,
    patrol_order: Option<&PatrolOrder>,
    order_queue: Option<&OrderQueue>,
) -> Vec<Vec3> {
    let mut path = Vec::new();
    if let Some(target) = active_terrain_order_target(move_order, attack_move_order, patrol_order) {
        path.push(target);
    }
    if let Some(order_queue) = order_queue {
        path.extend(
            order_queue
                .orders
                .iter()
                .filter_map(queued_terrain_order_target),
        );
    }
    path
}

fn active_terrain_order_target(
    move_order: Option<&MoveOrder>,
    attack_move_order: Option<&AttackMoveOrder>,
    patrol_order: Option<&PatrolOrder>,
) -> Option<Vec3> {
    if let Some(patrol_order) = patrol_order {
        return Some(if patrol_order.moving_to_destination {
            patrol_order.destination
        } else {
            patrol_order.origin
        });
    }
    if let Some(attack_move_order) = attack_move_order {
        return Some(attack_move_order.destination);
    }
    move_order.map(|order| order.target)
}

fn queued_terrain_order_target(order: &UnitQueuedOrder) -> Option<Vec3> {
    match order {
        UnitQueuedOrder::Move(target) | UnitQueuedOrder::AttackMove(target) => Some(*target),
        UnitQueuedOrder::Patrol { destination, .. } => Some(*destination),
        UnitQueuedOrder::Attack(_)
        | UnitQueuedOrder::Capture(_)
        | UnitQueuedOrder::Follow { .. }
        | UnitQueuedOrder::Garrison(_)
        | UnitQueuedOrder::Harvest { .. }
        | UnitQueuedOrder::Construct(_)
        | UnitQueuedOrder::Repair(_)
        | UnitQueuedOrder::ForceFollow { .. } => None,
    }
}

fn draw_terrain_order_path(gizmos: &mut Gizmos, start: Vec3, targets: &[Vec3]) {
    let mut from = start;
    for (index, target) in targets.iter().enumerate() {
        let color = if index == 0 {
            Color::srgba(0.65, 0.9, 1.0, 0.82)
        } else {
            Color::srgba(0.32, 0.78, 1.0, 0.55)
        };
        gizmos.line(
            terrain_overlay_point(from),
            terrain_overlay_point(*target),
            color,
        );
        draw_ring(gizmos, *target, 0.24, color);
        from = *target;
    }
}

fn terrain_overlay_point(position: Vec3) -> Vec3 {
    Vec3::new(position.x, 0.08, position.z)
}

fn should_draw_action_queue_path(team: Team, visible_team: Team) -> bool {
    team == visible_team
}

fn should_draw_air_to_terrain_marker(domain: MovementDomain) -> bool {
    domain == MovementDomain::Air
}

fn draw_air_to_terrain_marker(
    gizmos: &mut Gizmos,
    position: Vec3,
    radius: f32,
    team: Team,
    visible_team: Team,
) {
    let Some(color) = air_to_terrain_marker_color(team, visible_team) else {
        return;
    };
    let ground_position = Vec3::new(position.x, 0.04, position.z);
    draw_ring(gizmos, ground_position, radius + 0.24, color);
    gizmos.line(
        ground_position,
        position + Vec3::Y * 0.08,
        Color::srgba(0.82, 0.94, 1.0, 0.45),
    );
}

fn air_to_terrain_marker_color(team: Team, visible_team: Team) -> Option<Color> {
    if team == Team::Neutral {
        return None;
    }
    if team == visible_team {
        Some(Color::srgba(0.3, 0.95, 0.65, 0.8))
    } else {
        Some(Color::srgba(1.0, 0.28, 0.2, 0.8))
    }
}

fn draw_ring(gizmos: &mut Gizmos, position: Vec3, radius: f32, color: Color) {
    gizmos.circle(
        Isometry3d::new(
            Vec3::new(position.x, 0.05, position.z),
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        radius,
        color,
    );
}

fn draw_structure_destruction_vfx(
    gizmos: &mut Gizmos,
    position: Vec3,
    effect: &StructureDestructionVfx,
    player_colors: &PlayerColorSlots,
) {
    let life_ratio = if effect.total > 0.0 {
        (effect.remaining / effect.total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    match effect.kind {
        StructureDestructionVfxKind::ExplosionFireball => {
            let rise = (1.0 - life_ratio) * effect.radius * 1.3;
            let center = position + Vec3::Y * rise;
            let radius = effect.radius * (0.35 + (1.0 - life_ratio) * 0.45);
            let color = Color::srgba(1.0, 0.48, 0.14, 0.28 + life_ratio * 0.52);
            gizmos.circle(
                Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
                radius,
                color,
            );
            gizmos.line(
                Vec3::new(center.x, 0.08, center.z),
                center + Vec3::Y * effect.radius * 0.55,
                color,
            );
        }
        StructureDestructionVfxKind::SmokeColumn => {
            let color = structure_smoke_color(effect.team, life_ratio, player_colors);
            let base = Vec3::new(position.x, 0.1, position.z);
            let top = position + Vec3::Y * (effect.radius * (1.8 - life_ratio * 0.4));
            gizmos.line(base, top, color);
            gizmos.circle(
                Isometry3d::new(top, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
                effect.radius * (0.38 + (1.0 - life_ratio) * 0.28),
                color,
            );
        }
    }
}

fn structure_smoke_color(team: Team, life_ratio: f32, player_colors: &PlayerColorSlots) -> Color {
    let alpha = (0.18 + life_ratio * 0.38).clamp(0.0, 0.62);
    let [r, g, b] = player_colors.color_rgb(team);
    Color::srgba(0.12 + r * 0.08, 0.12 + g * 0.08, 0.12 + b * 0.08, alpha)
}

fn draw_veterancy_promotion_effect(
    gizmos: &mut Gizmos,
    position: Vec3,
    effect: &VeterancyPromotionEffect,
    player_colors: &PlayerColorSlots,
) {
    let life_ratio = if effect.total > 0.0 {
        (effect.remaining / effect.total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let rank_color = veterancy_promotion_color(effect.rank, life_ratio);
    let team_color = player_colors.color(effect.team);
    let center = Vec3::new(position.x, position.y + 0.08, position.z);
    let expanding_radius = effect.radius * (1.05 + (1.0 - life_ratio) * 0.5);

    draw_ring(gizmos, center, expanding_radius, rank_color);
    draw_ring(
        gizmos,
        center,
        effect.radius * 0.7,
        Color::srgba(1.0, 1.0, 1.0, 0.2 + life_ratio * 0.35),
    );
    gizmos.line(
        center,
        center + Vec3::Y * (effect.radius * (1.5 + (1.0 - life_ratio) * 0.35)),
        team_color,
    );
    let crown_y = center.y + effect.radius * 1.65;
    let badge_width = effect.radius
        * if effect.rank >= VETERANCY_MAX_RANK {
            0.55
        } else {
            0.42
        };
    gizmos.line(
        Vec3::new(center.x - badge_width, crown_y, center.z),
        Vec3::new(center.x + badge_width, crown_y, center.z),
        rank_color,
    );
}

fn veterancy_promotion_color(rank: u8, life_ratio: f32) -> Color {
    let alpha = (0.22 + life_ratio * 0.6).clamp(0.0, 0.86);
    if rank >= VETERANCY_MAX_RANK {
        Color::srgba(0.18, 0.9, 1.0, alpha)
    } else {
        Color::srgba(1.0, 0.78, 0.16, alpha)
    }
}

fn draw_team_marker(
    gizmos: &mut Gizmos,
    position: Vec3,
    radius: f32,
    team: Team,
    player_colors: &PlayerColorSlots,
) {
    gizmos.circle(
        Isometry3d::new(
            Vec3::new(position.x, position.y + 0.08, position.z),
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        radius * 0.55,
        player_colors.color(team),
    );
}

fn draw_health_bar(gizmos: &mut Gizmos, position: Vec3, radius: f32, health: Health) {
    let width = radius * 1.7;
    let y = position.y + 1.25;
    let left = Vec3::new(position.x - width * 0.5, y, position.z);
    let right = Vec3::new(position.x + width * 0.5, y, position.z);
    let fill = Vec3::new(
        position.x - width * 0.5 + width * health.ratio(),
        y,
        position.z,
    );
    gizmos.line(left, right, Color::srgba(0.04, 0.06, 0.06, 0.92));
    gizmos.line(left, fill, Color::srgb(0.25, 0.92, 0.36));
}

fn pointer_ground(
    window: &Window,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<Vec3> {
    let cursor = window.cursor_position()?;
    let (camera, camera_transform) = camera_q.single().ok()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
}

fn map_contains_ground_point_in_bounds(point: Vec3, bounds: MapBounds) -> bool {
    bounds.contains_ground_point(point)
}

fn validated_terrain_target_in_bounds(point: Vec3, bounds: MapBounds) -> Option<Vec3> {
    if !point.is_finite() {
        return None;
    }
    if point.x < -bounds.half_width - TERRAIN_TARGET_MAP_MARGIN_M
        || point.z < -bounds.half_depth - TERRAIN_TARGET_MAP_MARGIN_M
        || point.x > bounds.half_width + TERRAIN_TARGET_MAP_MARGIN_M
        || point.z > bounds.half_depth + TERRAIN_TARGET_MAP_MARGIN_M
    {
        return None;
    }
    Some(bounds.clamp_ground_point(Vec3::new(point.x, 0.0, point.z), 0.0))
}

fn cursor_is_over_hud(window: &Window) -> bool {
    let Some(cursor) = window.cursor_position() else {
        return false;
    };
    cursor_is_over_top_status_hud(cursor)
        || cursor.y > window.height() - 148.0
        || battle_log_contains_cursor(window, cursor)
        || minimap_contains_cursor(window, cursor)
}

fn cursor_is_over_top_status_hud(cursor: Vec2) -> bool {
    cursor.y < 76.0
}

fn cursor_blocks_world_order_controls(window: &Window, cursor: Vec2) -> bool {
    cursor.y > window.height() - 148.0
        || battle_log_contains_cursor(window, cursor)
        || minimap_contains_cursor(window, cursor)
}

fn battle_log_contains_cursor(window: &Window, cursor: Vec2) -> bool {
    let min_x = window.width() - BATTLE_LOG_RIGHT_PX - BATTLE_LOG_WIDTH_PX;
    cursor.x >= min_x
        && cursor.x <= min_x + BATTLE_LOG_WIDTH_PX
        && cursor.y >= BATTLE_LOG_TOP_PX
        && cursor.y <= BATTLE_LOG_TOP_PX + BATTLE_LOG_HIT_HEIGHT_PX
}

fn minimap_contains_cursor(window: &Window, cursor: Vec2) -> bool {
    let min = minimap_screen_min(window);
    cursor.x >= min.x
        && cursor.x <= min.x + MINIMAP_SIZE_PX
        && cursor.y >= min.y
        && cursor.y <= min.y + MINIMAP_SIZE_PX
}

fn cursor_minimap_local(window: &Window) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    if !minimap_contains_cursor(window, cursor) {
        return None;
    }
    Some(cursor - minimap_screen_min(window))
}

fn minimap_screen_min(window: &Window) -> Vec2 {
    Vec2::new(
        window.width() - MINIMAP_RIGHT_PX - MINIMAP_SIZE_PX,
        window.height() - MINIMAP_BOTTOM_PX - MINIMAP_SIZE_PX,
    )
}

#[cfg(test)]
fn minimap_local_position(world: Vec3) -> Vec2 {
    minimap_local_position_in_bounds(world, MapBounds::default())
}

fn minimap_local_position_in_bounds(world: Vec3, bounds: MapBounds) -> Vec2 {
    bounds.minimap_local_position(world)
}

#[cfg(test)]
fn minimap_world_position(local: Vec2) -> Vec3 {
    minimap_world_position_in_bounds(local, MapBounds::default())
}

fn minimap_world_position_from_local_in_bounds(local: Vec2, bounds: MapBounds) -> Option<Vec3> {
    bounds.minimap_world_position_checked(local)
}

#[cfg(test)]
fn minimap_world_position_in_bounds(local: Vec2, bounds: MapBounds) -> Vec3 {
    bounds.minimap_world_position(local)
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
    xz_distance_squared(a, b).sqrt()
}

fn xz_distance_squared(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length_squared()
}

fn distance_point_to_xz_segment(point: Vec3, start: Vec3, end: Vec3) -> f32 {
    let point = Vec2::new(point.x, point.z);
    let start = Vec2::new(start.x, start.z);
    let end = Vec2::new(end.x, end.z);
    let segment = end - start;
    let segment_length_squared = segment.length_squared();
    if segment_length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let projection = ((point - start).dot(segment) / segment_length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * projection)
}

fn screen_polygon_for_drag(start: Vec2, end: Vec2) -> Option<Vec<Vec2>> {
    let min = start.min(end);
    let max = start.max(end);
    if (max.x - min.x).abs() < 0.001 || (max.y - min.y).abs() < 0.001 {
        return None;
    }

    Some(vec![
        Vec2::new(min.x, min.y),
        Vec2::new(max.x, min.y),
        Vec2::new(max.x, max.y),
        Vec2::new(min.x, max.y),
    ])
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];

        let intersects = (a.y > point.y) != (b.y > point.y);
        if intersects {
            let x_at_y = (b.x - a.x) * ((point.y - a.y) / (b.y - a.y)) + a.x;
            if point.x <= x_at_y {
                inside = !inside;
            }
        }
    }
    inside
}

fn point_is_on_screen(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    world_position: Vec3,
) -> bool {
    let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_position) else {
        return false;
    };
    viewport_pos.x >= 0.0
        && viewport_pos.x <= window.width()
        && viewport_pos.y >= 0.0
        && viewport_pos.y <= window.height()
}

fn formation_offset(index: usize, count: usize) -> Vec3 {
    if count <= 1 {
        return Vec3::ZERO;
    }
    let side = (count as f32).sqrt().ceil() as usize;
    let x = (index % side) as f32 - (side as f32 - 1.0) * 0.5;
    let z = (index / side) as f32 - (side as f32 - 1.0) * 0.5;
    Vec3::new(x * 0.9, 0.0, z * 0.9)
}

fn free_position(origin: Vec3, seed: u32, radius: f32) -> Vec3 {
    free_position_in_bounds(origin, seed, radius, MapBounds::default())
}

fn free_position_in_bounds(origin: Vec3, seed: u32, radius: f32, bounds: MapBounds) -> Vec3 {
    let angle = seed as f32 * 1.618_034;
    bounds.clamp_ground_point(
        Vec3::new(
            origin.x + angle.cos() * radius,
            0.0,
            origin.z + angle.sin() * radius,
        ),
        1.0,
    )
}

#[cfg(test)]
mod current_tests {
    use super::*;

    #[test]
    fn active_ai_iteration_uses_all_runtime_players_not_three_slots() {
        let active = ActiveTeams(vec![true, true, true, true, true, true]);

        let teams = active_ai_teams(Some(Team::Player(0)), Some(&active)).collect::<Vec<_>>();

        assert_eq!(
            teams,
            vec![
                Team::Player(1),
                Team::Player(2),
                Team::Player(3),
                Team::Player(4),
                Team::Player(5),
            ]
        );
    }

    #[test]
    fn dynamic_team_relations_and_start_status_work_beyond_three_players() {
        let active = vec![true, true, true, true, true, true];
        let team_ids = vec![0, 1, 2, 0, 1, 2];
        let relations = skirmish_team_relations_from_team_ids(&active, &team_ids);

        assert!(relations.are_allied(Team::Player(0), Team::Player(3)));
        assert!(relations.are_allied(Team::Player(1), Team::Player(4)));
        assert!(relations.are_enemies(Team::Player(0), Team::Player(4)));
        assert_eq!(
            skirmish_start_status_for_setup(6, 8, &active, &relations),
            SkirmishStartStatus::Ready
        );
    }

    #[test]
    fn lobby_team_ids_and_capture_teams_are_not_limited_to_three() {
        assert_eq!(
            DEFAULT_LOBBY_TEAM_IDS,
            [0, 1, 2, 3, 4, 5, 6, 7],
            "default lobby rows should not fold 8 players into three teams"
        );
        assert_eq!(
            SKIRMISH_TEAM_OPTION_COUNT as usize,
            MAX_SKIRMISH_LOBBY_SLOTS
        );
        assert_eq!(
            capture_team_from_team(Team::Player(7)),
            CaptureTeam::Player(7)
        );
    }

    #[test]
    fn runtime_resources_expand_for_late_player_slots() {
        let late_team = Team::Player(11);
        let mut economies = Economies::default();
        assert!(economies.players.is_empty());
        economies.get_mut(late_team).ore = 1234;
        assert_eq!(economies.get(late_team).ore, 1234);

        let mut director = AiDirector::default();
        assert!(director.attack_timer.is_empty());
        let late_index = director
            .ensure_team(late_team)
            .expect("late player slots should be valid runtime teams");
        director.attack_timer[late_index] = 0.25;
        assert_eq!(director.attack_timer[late_index], 0.25);

        let mut support_cooldowns = SupportCooldowns::default();
        assert!(support_cooldowns.remaining.is_empty());
        support_cooldowns.set(late_team, SupportPowerKind::Paradrop, 9.0);
        assert_eq!(
            support_cooldowns.remaining_for(late_team, SupportPowerKind::Paradrop),
            9.0
        );
    }

    #[test]
    fn runtime_team_helpers_do_not_wrap_after_lobby_slot_count() {
        let active = ActiveTeams(vec![true; 16]);
        let teams = active_ai_teams(Some(Team::Player(0)), Some(&active)).collect::<Vec<_>>();
        assert_eq!(teams.len(), 15);
        assert!(teams.contains(&Team::Player(15)));

        assert_eq!(
            default_skirmish_opponent(Team::Player(12), 16),
            Some(Team::Player(0))
        );
        assert_eq!(
            allied_skirmish_ally(Team::Player(12), 16),
            Some(Team::Player(0))
        );
        assert_eq!(
            allied_skirmish_enemy(Team::Player(12), 16),
            Some(Team::Player(1))
        );
        assert_eq!(Team::Neutral.index(), usize::MAX);
        assert!(xz_distance(team_home(Team::Player(0)), team_home(Team::Player(8))) > 1.0);
        assert!(xz_distance(team_home(Team::Player(8)), team_home(Team::Player(16))) > 1.0);
    }
}
