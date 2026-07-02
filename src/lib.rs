#[cfg(feature = "audio")]
use bevy::audio::Volume;
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin, UntypedHandle},
    camera::primitives::Aabb,
    ecs::query::Or,
    ecs::system::SystemParam,
    gizmos::config::GizmoConfigStore,
    input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
    math::primitives::{ConicalFrustum, Cuboid, Cylinder, Torus},
    prelude::*,
    render::error_handler::{ErrorType, RenderError, RenderErrorHandler, RenderErrorPolicy},
    window::{CursorIcon, PrimaryWindow, WindowMode, WindowResolution},
};
use bevy_common_assets::{json::JsonAssetPlugin, ron::RonAssetPlugin};
use bevy_cursor_kit::prelude::{CursorAssetPlugin, CustomCursorImageBuilder, StaticCursor};
use bevy_fluent::FluentPlugin;
use bevy_rts_camera::{RtsCameraControls, RtsCameraPlugin, RtsCameraSystemSet};
use iyes_progress::prelude::*;
use moonshine_kind::prelude::Instance;
use serde::Deserialize;
use std::collections::{BTreeMap, VecDeque};

mod capture_api;
pub use capture_api::*;
mod audio;
pub(crate) use audio::*;
mod ai;
mod save;
pub(crate) use ai::*;
pub(crate) use save::*;
mod combat;
pub(crate) use combat::*;
mod orders;
pub(crate) use orders::*;
mod production;
pub(crate) use production::*;
mod economy;
pub(crate) use economy::*;
mod hud;
pub(crate) use hud::*;
mod menu;
pub(crate) use menu::*;
mod camera;
mod generated_registry;
pub(crate) use camera::*;
mod nav;
pub(crate) use nav::*;

use generated_registry as registry;

#[derive(Asset, TypePath, Deserialize)]
#[allow(dead_code)]
pub(crate) struct RtsDataManifest {
    pub(crate) name: String,
}

#[derive(Asset, TypePath, Deserialize)]
#[allow(dead_code)]
pub(crate) struct GodotModelMapAsset {
    pub(crate) source: String,
    pub(crate) generated_by: String,
    pub(crate) entities: Vec<GodotModelMapEntity>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub(crate) struct GodotModelMapEntity {
    pub(crate) id: String,
    pub(crate) scene_path: String,
    pub(crate) parts: Vec<GodotModelMapPart>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub(crate) struct GodotModelMapPart {
    pub(crate) model: String,
    pub(crate) translation: Vec<f32>,
    pub(crate) rotation: Vec<f32>,
    pub(crate) scale: Vec<f32>,
}

#[derive(Resource, Clone)]
#[allow(dead_code)]
pub(crate) struct GodotModelMapHandle(pub(crate) Handle<GodotModelMapAsset>);

pub(crate) fn handle_render_error(
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

pub(crate) const MAP_HALF_EXTENT: f32 = 24.0;
pub(crate) const CAMERA_START_PRIMARY_UNITS: &[&str] = &["Worker"];
pub(crate) const CAMERA_START_PRIMARY_STRUCTURES: &[&str] = &["CommandCenter"];
pub(crate) const ENEMY_ORDER_SCREEN_PICK_MIN_RADIUS_PX: f32 = 32.0;
pub(crate) const ENEMY_ORDER_SCREEN_PICK_MAX_RADIUS_PX: f32 = 96.0;
pub(crate) const DEFAULT_MODEL_FALLBACK: &str = "models/kenney-spacekit/rover.glb";
pub(crate) const GODOT_MODEL_MAP_ASSET_PATH: &str = "data/godot_model_map.model_map.ron";
pub(crate) const COMMAND_SLOT_COUNT: usize = 24;
pub(crate) const COMMAND_TOOLTIP_WIDTH_PX: f32 = 330.0;
pub(crate) const COMMAND_TOOLTIP_OFFSET_X_PX: f32 = 18.0;
pub(crate) const COMMAND_TOOLTIP_OFFSET_Y_PX: f32 = 92.0;
pub(crate) const SUPPORT_POWER_BUTTON_SIZE_PX: f32 = 64.0;
pub(crate) const SUPPORT_POWER_BUTTON_GAP_PX: f32 = 5.0;
pub(crate) const SUPPORT_POWER_PANEL_PADDING_PX: f32 = 5.0;
pub(crate) const SUPPORT_POWER_PANEL_TOP_PX: f32 = 8.0;
pub(crate) const SUPPORT_POWER_PANEL_RIGHT_PX: f32 = 12.0;
pub(crate) const SUPPORT_POWER_PANEL_WIDTH_PX: f32 = SUPPORT_POWER_PANEL_PADDING_PX * 2.0
    + SUPPORT_POWER_BUTTON_SIZE_PX * 9.0
    + SUPPORT_POWER_BUTTON_GAP_PX * 8.0;
pub(crate) const SUPPORT_POWER_PANEL_HEIGHT_PX: f32 =
    SUPPORT_POWER_PANEL_PADDING_PX * 2.0 + SUPPORT_POWER_BUTTON_SIZE_PX;
pub(crate) const OBJECTIVE_TRACKER_TOP_PX: f32 =
    SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX + 8.0;
pub(crate) const COMMAND_KEY_CANCEL: &str = "cancel";
pub(crate) const COMMAND_KEY_GUARD_AREA: &str = "guard_area";
pub(crate) const COMMAND_KEY_SCATTER: &str = "scatter";
pub(crate) const COMMAND_KEY_HOLD_POSITION: &str = "hold_position";
pub(crate) const COMMAND_KEY_MINIMAP_MOVE: &str = "minimap_move";
pub(crate) const COMMAND_KEY_TOGGLE_DEPLOY: &str = "toggle_deploy";
pub(crate) const MOVE_ORDER_REACHED_DISTANCE_M: f32 = 0.22;
pub(crate) const CONTACT_ACTION_REACHED_TOLERANCE_M: f32 = MOVE_ORDER_REACHED_DISTANCE_M;
pub(crate) const ATTACK_MOVE_REACHED_DISTANCE: f32 = 2.0;
pub(crate) const PATROL_TURN_DISTANCE: f32 = 2.0;
pub(crate) const SCATTER_DISTANCE: f32 = 4.0;
pub(crate) const DRAG_SELECT_THRESHOLD: f32 = 6.0;
pub(crate) const SELECTION_DRAG_INTERRUPT_MARGIN_PX: f32 = 1.0;
pub(crate) const DOUBLE_CLICK_MIN_SECONDS: f32 = 0.05;
pub(crate) const DOUBLE_CLICK_MAX_SECONDS: f32 = 0.6;
pub(crate) const SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PX: f32 = 38.0;
pub(crate) const SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PER_METER_PX: f32 = 18.0;
pub(crate) const FOG_REVEAL_RADIUS: f32 = 11.5;
pub(crate) const FOG_COMPENSATION: f32 = 2.0;
pub(crate) const MATCH_END_TITLE_COLOR: Color = Color::srgb(0.98, 0.96, 0.42);
pub(crate) const MATCH_END_BG_COLOR: Color = Color::srgba(0.04, 0.05, 0.08, 0.86);
pub(crate) const MATCH_END_TITLE_FONT_SIZE: f32 = 34.0;
pub(crate) const MATCH_END_TEXT_FONT_SIZE: f32 = 19.0;
pub(crate) const CLICK_MARKER_TTL_SECONDS: f32 = 0.5;
pub(crate) const CLICK_MARKER_RADIUS_START: f32 = 0.7;
pub(crate) const CLICK_MARKER_RADIUS_END: f32 = 0.05;
pub(crate) const UNIT_ADHERENCE_MARGIN_M: f32 = 0.3;
pub(crate) const CAPTURE_ENTRY_MARGIN_M: f32 = 1.3;
pub(crate) const FOLLOW_TARGET_DISTANCE_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
pub(crate) const REPAIR_ADHERENCE_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;
pub(crate) const REPAIR_ENTRY_MARGIN_M: f32 = 1.0;
pub(crate) const STRUCTURE_SELL_REFUND_RATIO: f32 = 0.5;
pub(crate) const STRUCTURE_MANUAL_REPAIR_COST_RATIO: f32 = 0.5;
pub(crate) const STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND: f32 = 3.0;
pub(crate) const STRUCTURE_CONSTRUCTION_PROGRESS_PER_SECOND: f32 = 0.3;
pub(crate) const BASE_CONSTRUCTION_RADIUS_M: f32 = 9.0;
pub(crate) const SHIELD_TROOPER_PASSIVE_DAMAGE_SCALE: f32 = 0.65;
pub(crate) const SIEGE_DRILL_DEPLOYED_ATTACK_RANGE: f32 = 6.5;
pub(crate) const SIEGE_DRILL_DEPLOYED_ATTACK_INTERVAL: f32 = 1.0;
pub(crate) const SIEGE_DRILL_DEPLOYED_STRUCTURE_DAMAGE_MULTIPLIER: f32 = 3.6;
pub(crate) const SIEGE_DRILL_DEPLOYED_SIGHT_RANGE: f32 = 9.5;
pub(crate) const VETERANCY_MAX_RANK: u8 = 2;
pub(crate) const VETERANCY_DAMAGE_MULTIPLIER_BY_RANK: [f32; 3] = [1.0, 1.25, 1.5];
pub(crate) const VETERANCY_HP_MULTIPLIER_BY_RANK: [f32; 3] = [1.0, 1.2, 1.5];
pub(crate) const VETERANCY_RANGE_BONUS_BY_RANK: [f32; 3] = [0.0, 0.5, 1.0];
pub(crate) const VETERANCY_SIGHT_BONUS_BY_RANK: [f32; 3] = [0.0, 1.0, 2.0];
pub(crate) const VETERANCY_ELITE_REGEN_PER_SECOND: f32 = 1.0;
pub(crate) const VETERANCY_KILLS_BY_RANK: [u32; 3] = [0, 2, 5];
pub(crate) const VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS: f32 = 1.1;
pub(crate) const CRUSH_DAMAGE: f32 = 999.0;
pub(crate) const CRUSH_RADIUS_MARGIN_M: f32 = 0.15;
pub(crate) const CRUSH_MIN_FRAME_DISPLACEMENT_M: f32 = 0.005;
pub(crate) const MOVEMENT_OBSTACLE_CLEARANCE_M: f32 = 0.18;
pub(crate) const MOVEMENT_OBSTACLE_LOOKAHEAD_M: f32 = 2.4;
pub(crate) const MOVEMENT_OBSTACLE_STEER_WEIGHT: f32 = 1.15;
pub(crate) const COMBAT_WRECKAGE_LIFETIME_SECONDS: f32 = 10.0;
pub(crate) const STRUCTURE_FIREBALL_LIFETIME_SECONDS: f32 = 1.4;
pub(crate) const STRUCTURE_SMOKE_COLUMN_LIFETIME_SECONDS: f32 = 4.5;
pub(crate) const BATTLE_LOG_ENTRY_TTL_SECONDS: f32 = 6.5;
pub(crate) const BATTLE_EVENT_PING_LIFETIME_SECONDS: f32 = 4.0;
pub(crate) const BATTLE_LOG_MAX_ENTRIES: usize = 5;
pub(crate) const BATTLE_LOG_UNDER_ATTACK_COOLDOWN_SECONDS: f32 = 7.0;
pub(crate) const BATTLE_LOG_TOP_PX: f32 = 104.0;
pub(crate) const BATTLE_LOG_WIDTH_PX: f32 = 390.0;
// Per-row height of the battle-log click/hover hit area; the hit rect scales with
// the number of visible entries so an EMPTY log never swallows world clicks (it
// used to be a fixed 168px band across the top-center of the screen).
pub(crate) const BATTLE_LOG_ROW_HIT_PX: f32 = 34.0;
// Bottom-right command card hit geometry (146x46 buttons + 8px gap, 4 per row) and
// the production-queue rows that stack above it (92px slots, 6 per row).
pub(crate) const COMMAND_CARD_WIDTH_PX: f32 = 612.0;
pub(crate) const COMMAND_CARD_ROW_HIT_PX: f32 = 54.0;
pub(crate) const MINIMAP_SIZE_PX: f32 = 158.0;
// godot anchors the minimap/radar in the bottom-LEFT corner.
pub(crate) const MINIMAP_LEFT_PX: f32 = 12.0;
pub(crate) const MINIMAP_BOTTOM_PX: f32 = 12.0;
pub(crate) const MINIMAP_ENTITY_MARKER_PX: f32 = 4.6;
pub(crate) const MINIMAP_STRUCTURE_MARKER_PX: f32 = 6.2;
pub(crate) const MINIMAP_RESOURCE_MARKER_PX: f32 = 3.8;
pub(crate) const TERRAIN_TARGET_MAP_MARGIN_M: f32 = 2.5;
pub(crate) const MAX_SKIRMISH_LOBBY_SLOTS: usize = 8;
pub(crate) const DEFAULT_LOBBY_CONTROLLERS: [SkirmishPlayerController; MAX_SKIRMISH_LOBBY_SLOTS] = [
    SkirmishPlayerController::Human,
    SkirmishPlayerController::Ai(AiDifficulty::Easy),
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
    SkirmishPlayerController::None,
];
pub(crate) const DEFAULT_LOBBY_FACTIONS: [SkirmishFaction; MAX_SKIRMISH_LOBBY_SLOTS] = [
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
    SkirmishFaction::Chaos,
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
    SkirmishFaction::Chaos,
    SkirmishFaction::Alliance,
    SkirmishFaction::Demon,
];
pub(crate) const DEFAULT_LOBBY_TEAM_IDS: [u8; MAX_SKIRMISH_LOBBY_SLOTS] = [0, 1, 2, 3, 4, 5, 6, 7];
pub(crate) const DEFAULT_LOBBY_COLOR_SLOTS: [usize; MAX_SKIRMISH_LOBBY_SLOTS] =
    [0, 1, 2, 3, 4, 5, 6, 7];
pub(crate) const PLAYER_COLOR_PALETTE: [[f32; 3]; 20] = [
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
pub(crate) const SKIRMISH_MAP_PREVIEW_SIZE: Vec2 = Vec2::new(232.0, 168.0);
pub(crate) const SKIRMISH_MAP_PREVIEW_PADDING: f32 = 12.0;
pub(crate) const SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS: usize = 4;
pub(crate) const MINE_DEPLOY_OFFSETS: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (1.0, -1.0),
    (-1.0, 1.0),
    (1.0, 1.0),
    (0.0, -1.0),
    (1.0, 0.0),
    (0.0, 1.0),
    (-1.0, 0.0),
];
pub(crate) const STRUCTURE_PLACEMENT_ROTATION_STEP_RADIANS: f32 = std::f32::consts::FRAC_PI_4;
pub(crate) const STRUCTURE_PLACEMENT_ROTATION_DEAD_ZONE_M: f32 = 0.1;
pub(crate) const COMMAND_SLOT_HOTKEYS: [CommandHotkey; COMMAND_SLOT_COUNT] = [
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
pub(crate) const GROUP_SLOT_KEYS: [KeyCode; 9] = [
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SupportPowerKind {
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
pub(crate) enum SimulationPhase {
    UiAndManagement,
    BuildProcessing,
    Combat,
    PostCombat,
}

#[derive(States, Default, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AppScreen {
    #[default]
    AssetLoading,
    MainMenu,
    SkirmishSetup,
    OptionsMenu,
    CreditsMenu,
    InMatch,
    RestartingMatch,
}

#[derive(Component)]
pub(crate) struct MatchScopedEntity;

#[derive(Resource, Default)]
pub(crate) struct StartupLoadingAssets {
    pub(crate) handles: Vec<UntypedHandle>,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct StartupLoadingPolicy {
    pub(crate) preload_assets: bool,
}

impl Default for StartupLoadingPolicy {
    fn default() -> Self {
        Self {
            preload_assets: true,
        }
    }
}

#[derive(Component)]
pub(crate) struct StartupLoadingFill;

#[derive(Component)]
pub(crate) struct StartupLoadingText;

#[derive(Component, Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) struct ModelHarnessRoot {
    pub(crate) index: usize,
    pub(crate) id: &'static str,
}

pub(crate) const MODEL_HARNESS_ENTITY_IDS: [&str; registry::ENTITY_DEFS.len()] = [
    "AdvancedReactorPlant",
    "AircraftFactory",
    "AntiAirTurret",
    "AntiAirWalker",
    "AntiGroundTurret",
    "ArcCoilDefenseTower",
    "Barracks",
    "BomberVTOL",
    "CommandCenter",
    "CryoSprayer",
    "Drone",
    "DroneMineLayer",
    "EngineerDrone",
    "FieldMedic",
    "FlakHoverTank",
    "FlakRocketTeam",
    "FlakRocketTeamMk2",
    "FlameAssaultBuggy",
    "GrenadierTrooper",
    "HammerSiegeTank",
    "HeavyBombardmentAirship",
    "HeavyMachinegunTrooper",
    "HeavySiegeWalker",
    "Helicopter",
    "InterceptorVTOL",
    "JammerVehicle",
    "LanceBeamDefenseTower",
    "LanceBeamTank",
    "LandMine",
    "LightRifleInfantry",
    "LongbowMissileCrawler",
    "MirageScoutTank",
    "MobileRepairCrawler",
    "MobileShieldProjector",
    "ModularMissileCarrier",
    "MortarTeam",
    "OrePurifier",
    "PhaseSaboteur",
    "PowerReactor",
    "PrismDefenseObelisk",
    "PulseRifleCommando",
    "RadarUplink",
    "RailArtilleryWalker",
    "RailCannonBunker",
    "RailSniperTeam",
    "RailgunTank",
    "Refinery",
    "RepairPad",
    "RoboticsBay",
    "RocketGunship",
    "RocketInfantry",
    "RocketTrooperRobot",
    "SaboteurInfiltrator",
    "ScoutRover",
    "ShieldTrooper",
    "ShockTrooper",
    "SiegeAirship",
    "SiegeArtilleryVehicle",
    "SiegeDrillTank",
    "SniperScout",
    "TacticalOfficer",
    "Tank",
    "TechAirport",
    "TechBunker",
    "TechHospital",
    "TechLab",
    "TechOilDerrick",
    "TechRepairDepot",
    "TeslaCrawlerMk2",
    "TeslaFenceSegment",
    "VehicleFactory",
    "WeatherControlSpire",
    "Worker",
];

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

    pub(crate) fn idx(self) -> usize {
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

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::RadarSweep => t("雷达扫描", "Radar Sweep"),
            Self::OrbitalStrike => t("轨道打击", "Orbital Strike"),
            Self::EmpPulse => t("EMP脉冲", "EMP Pulse"),
            Self::ChronoRelay => t("时光回响", "Chrono Relay"),
            Self::ShieldOverdrive => t("护盾超载", "Shield Overdrive"),
            Self::NaniteRepairSwarm => t("纳米修复", "Nanite Repair Swarm"),
            Self::WeatherStorm => t("气象风暴", "Weather Storm"),
            Self::StrategicMissile => t("战略导弹", "Strategic Missile"),
            Self::Paradrop => t("空投", "Paradrop"),
        }
    }

    pub(crate) fn is_superweapon(self) -> bool {
        matches!(self, Self::WeatherStorm | Self::StrategicMissile)
    }

    pub(crate) fn hotkey(self) -> KeyCode {
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

    pub(crate) fn hotkey_label(self) -> &'static str {
        match self {
            Self::RadarSweep => "F1",
            Self::OrbitalStrike => "F2",
            Self::EmpPulse => "F3",
            Self::ChronoRelay => "F4",
            Self::ShieldOverdrive => "F5",
            Self::NaniteRepairSwarm => "F6",
            Self::WeatherStorm => "F7",
            Self::StrategicMissile => "F8",
            Self::Paradrop => "F9",
        }
    }

    pub(crate) fn icon_path(self) -> &'static str {
        match self {
            Self::RadarSweep => "ui/icons/RadarSweep.png",
            Self::OrbitalStrike => "ui/icons/OrbitalStrike.png",
            Self::EmpPulse => "ui/icons/EmpPulse.png",
            Self::ChronoRelay => "ui/icons/ChronoRelay.png",
            Self::ShieldOverdrive => "ui/icons/ShieldOverdrive.png",
            Self::NaniteRepairSwarm => "ui/icons/NaniteRepairSwarm.png",
            Self::WeatherStorm => "ui/icons/WeatherStorm.png",
            Self::StrategicMissile => "ui/icons/StrategicMissile.png",
            Self::Paradrop => "ui/icons/Paradrop.png",
        }
    }

    pub(crate) fn definition(self) -> SupportPowerDef {
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

pub(crate) const RTS_CURSOR_ASSET_PATH: &str = "ui/cursors/rts_cursor.cur.ron";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PendingStructurePlacement {
    pub(crate) id: &'static str,
    pub(crate) rotation_y_radians: f32,
    pub(crate) position: Option<Vec3>,
    pub(crate) drag_rotation_origin: Option<Vec3>,
}

impl PendingStructurePlacement {
    pub(crate) fn new(id: &'static str) -> Self {
        Self {
            id,
            rotation_y_radians: 0.0,
            position: None,
            drag_rotation_origin: None,
        }
    }

    pub(crate) fn rotation_y_radians(self) -> f32 {
        self.rotation_y_radians
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MatchSpeedPreset {
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

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Slow => "0.75x",
            Self::Normal => "1x",
            Self::Fast => "1.25x",
            Self::Faster => "1.5x",
            Self::Max => "2x",
        }
    }

    pub(crate) fn scale(self) -> f32 {
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
pub(crate) struct MatchSpeed {
    pub(crate) preset: MatchSpeedPreset,
}

pub(crate) const MATCH_BRIEFING_AUTO_HIDE_SECONDS: f32 = 14.0;

pub(crate) type StructurePrereqItem<'a> = (
    &'a Structure,
    &'a Team,
    &'a Transform,
    Option<&'a UnderConstruction>,
);
pub(crate) type StructureEntityItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    Option<&'a UnderConstruction>,
);
pub(crate) type ProductionHotkeyStructureItem<'a> = (
    Entity,
    &'a Team,
    &'a Structure,
    &'a Health,
    &'a VisibilityState,
    Option<&'a UnderConstruction>,
);
pub(crate) type SelectedSellStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a UnderConstruction>,
);
pub(crate) type SelectedRepairStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a ManualStructureRepair>,
    Option<&'a UnderConstruction>,
);
pub(crate) type CommandOrderStateItem<'a> = (
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
pub(crate) type SelectedCommandUnitItem<'a> = (
    Entity,
    &'a Unit,
    &'a Team,
    &'a Transform,
    &'a HoldPosition,
    CommandOrderStateItem<'a>,
);
pub(crate) type CommandPanelUnitItem<'a> = (
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
pub(crate) type IdleWorkerSelectionItem<'a> = (
    Entity,
    &'a Team,
    &'a Unit,
    Option<&'a OrderQueue>,
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
    &'a VisibilityState,
);
pub(crate) type SelectedOrderUnitItem<'a> = (
    Entity,
    &'a Transform,
    &'a Unit,
    &'a Team,
    CommandOrderStateItem<'a>,
    Option<&'a ResourceCargo>,
);
pub(crate) type SelectableOrderTargetItem<'a> = (
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
pub(crate) type SelectedCommandUnitFilter = (With<Selected>, With<Unit>, Without<Structure>);
pub(crate) type SelectedOrderUnitFilter = (With<Selected>, With<Unit>, Without<Structure>);
pub(crate) type SelectedRallyPointFilter = (With<Selected>, With<Structure>, Without<Unit>);
pub(crate) type PlacementOccupierItem<'a> = (
    Entity,
    &'a Transform,
    &'a Selectable,
    Option<&'a Health>,
    Option<&'a ResourceNode>,
);
pub(crate) type AiRepairStructureItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Health,
    Option<&'a ManualStructureRepair>,
    Option<&'a UnderConstruction>,
);
pub(crate) type CaptureStructureTargetItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    &'a Health,
    Option<&'a UnderConstruction>,
);
pub(crate) type AiOpenBunkerItem<'a> = (
    Entity,
    &'a Structure,
    &'a Team,
    &'a Transform,
    &'a Health,
    &'a Garrison,
    Option<&'a UnderConstruction>,
);
pub(crate) type AiGarrisonUnitItem<'a> = (
    Entity,
    &'a Unit,
    &'a Team,
    &'a Transform,
    &'a Health,
    Option<&'a OrderQueue>,
);

#[derive(Component)]
pub(crate) struct SupportWarning {
    pub(crate) remaining: f32,
    pub(crate) radius: f32,
    pub(crate) color: Color,
}

#[derive(Component)]
pub(crate) struct TemporarySupportReveal {
    pub(crate) remaining: f32,
    pub(crate) radius: f32,
}

#[derive(Component)]
pub(crate) struct PendingOrbitalStrike {
    pub(crate) remaining: f32,
    pub(crate) radius: f32,
    pub(crate) damage: f32,
    pub(crate) impact_scale: f32,
    pub(crate) team: Team,
}

#[derive(Component)]
pub(crate) struct PendingParadrop {
    pub(crate) remaining: f32,
    pub(crate) team: Team,
    pub(crate) target: Vec3,
    pub(crate) unit_paths: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub(crate) struct SupportPowerTargetSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) health: Health,
    pub(crate) mobile: bool,
}

#[derive(Resource)]
pub(crate) struct SupportCooldowns {
    pub(crate) remaining: Vec<[f32; SupportPowerKind::ALL.len()]>,
    pub(crate) initial_charge_started: Vec<[bool; SupportPowerKind::ALL.len()]>,
}

impl SupportCooldowns {
    pub(crate) fn ensure_team(&mut self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        if self.remaining.len() <= index {
            self.remaining
                .resize(index + 1, [0.0; SupportPowerKind::ALL.len()]);
            self.initial_charge_started
                .resize(index + 1, [false; SupportPowerKind::ALL.len()]);
        }
        Some(index)
    }

    pub(crate) fn ready(&self, team: Team, power: SupportPowerKind) -> bool {
        self.remaining_for(team, power) <= 0.0
    }

    pub(crate) fn remaining_for(&self, team: Team, power: SupportPowerKind) -> f32 {
        team.economy_index()
            .and_then(|index| self.remaining.get(index))
            .map_or(0.0, |remaining| remaining[power.idx()])
    }

    pub(crate) fn set(&mut self, team: Team, power: SupportPowerKind, base: f32) {
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
pub(crate) struct MatchState {
    pub(crate) phase: MatchPhase,
    pub(crate) result_reason: &'static str,
    pub(crate) start_time_sec: f32,
    pub(crate) remaining_teams: u32,
    pub(crate) remaining_anchors: u32,
    pub(crate) enemy_units_destroyed: u32,
    pub(crate) enemy_structures_destroyed: u32,
    pub(crate) units_lost: u32,
    pub(crate) structures_lost: u32,
}

impl MatchState {
    pub(crate) fn is_running(&self) -> bool {
        matches!(self.phase, MatchPhase::Running)
    }

    pub(crate) fn finish_if_not_set(&mut self, reason: MatchPhase, reason_text: &'static str) {
        if !self.is_running() {
            return;
        }
        self.phase = reason;
        self.result_reason = reason_text;
    }
}

pub(crate) fn match_in_progress(
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

pub(crate) fn finalize_match(
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
pub(crate) struct MatchFlow {
    pub(crate) active: bool,
}

impl MatchFlow {
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for MatchFlow {
    fn default() -> Self {
        Self { active: true }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum MatchPhase {
    #[default]
    Running,
    HumanDefeat,
    HumanVictory,
    MatchFinished,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisibilityState {
    pub(crate) visible: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FogMemoryVisible;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct FogMemoryStructureRemnant {
    pub(crate) radius: f32,
}

#[derive(Component)]
pub(crate) struct VisionRadius(pub(crate) f32);

/// Fog-of-war shroud texture resolution (pixels per side) drawn over the map.
pub(crate) const FOG_OVERLAY_RES: usize = 192;
/// Alpha of the dim shroud over explored-but-not-currently-visible terrain.
pub(crate) const FOG_OVERLAY_EXPLORED_ALPHA: u8 = 150;
/// Height above the terrain at which the shroud plane sits.
pub(crate) const FOG_OVERLAY_Y: f32 = 0.06;

/// Marker for the textured shroud plane covering the whole map.
#[derive(Component)]
pub(crate) struct FogOverlayPlane;

/// Live fog-of-war shroud: a CPU-updated texture sampled over the map. Each cell
/// is clear where the viewing player (or an ally) currently sees, dimmed where it
/// was explored before, and black where never seen (godot's shroud+fog layers).
#[derive(Resource)]
pub(crate) struct FogOverlay {
    pub(crate) handle: Handle<Image>,
    pub(crate) explored: Vec<bool>,
}

#[derive(Component)]
pub(crate) struct MatchEndOverlay;

#[derive(Component)]
pub(crate) struct MatchEndTitle;

#[derive(Component)]
pub(crate) struct MatchEndReason;

#[derive(Component)]
pub(crate) struct MatchEndStats;

/// Match-end sparkline container: one bar per replay keyframe per team.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchEndChart {
    Army,
    Economy,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchEndButton {
    pub(crate) action: MatchEndAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatchEndAction {
    Restart,
    ReturnToSetup,
    ExitToMenu,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpawnSpec {
    pub(crate) id: &'static str,
    pub(crate) offset: (f32, f32),
}

#[derive(Clone, Copy)]
pub(crate) struct NeutralTechSpec {
    pub(crate) name: &'static str,
    pub(crate) id: &'static str,
    pub(crate) position: (f32, f32),
}

#[derive(Clone, Copy)]
pub(crate) struct SkirmishMapDef {
    pub(crate) id: &'static str,
    pub(crate) godot_path: &'static str,
    pub(crate) name: &'static str,
    pub(crate) name_key: &'static str,
    pub(crate) players: usize,
    pub(crate) size: (f32, f32),
    pub(crate) spawn_points: &'static [(f32, f32)],
    pub(crate) resources: &'static [ResourceSpec],
    pub(crate) neutral_tech: &'static [NeutralTechSpec],
    pub(crate) supply_crates: &'static [NamedSupplyCrateSpec],
}

impl SkirmishMapDef {
    pub(crate) fn contains_point(self, point: (f32, f32)) -> bool {
        point.0 >= 0.0 && point.1 >= 0.0 && point.0 <= self.size.0 && point.1 <= self.size.1
    }

    pub(crate) fn is_catalog_consistent(self) -> bool {
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
pub(crate) struct SelectedSkirmishMap {
    pub(crate) godot_path: &'static str,
}

impl Default for SelectedSkirmishMap {
    fn default() -> Self {
        Self {
            godot_path: SKIRMISH_MAPS[0].godot_path,
        }
    }
}

impl SelectedSkirmishMap {
    pub(crate) fn definition(self) -> &'static SkirmishMapDef {
        skirmish_map_by_path(self.godot_path).unwrap_or(&SKIRMISH_MAPS[0])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StartingResources {
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
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
pub(crate) struct StartingResourceOption {
    pub(crate) key: &'static str,
    pub(crate) resources: StartingResources,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkirmishFaction {
    Alliance,
    Demon,
    Chaos,
}

impl SkirmishFaction {
    const ALL: [Self; 3] = [Self::Alliance, Self::Demon, Self::Chaos];

    pub(crate) fn registry_id(self) -> &'static str {
        match self {
            Self::Alliance => "alliance",
            Self::Demon => "demon",
            Self::Chaos => "chaos",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Alliance => 0,
            Self::Demon => 1,
            Self::Chaos => 2,
        }
    }

    pub(crate) fn emblem_path(self) -> &'static str {
        match self {
            Self::Alliance => "ui/factions/alliance_emblem.png",
            Self::Demon => "ui/factions/demon_emblem.png",
            Self::Chaos => "ui/factions/chaos_emblem.png",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Alliance => t("苍穹联盟", "Alliance"),
            Self::Demon => t("炽炎魔军", "Demon"),
            Self::Chaos => t("混沌裂隙", "Chaos"),
        }
    }

    pub(crate) fn from_team(team: Team) -> Self {
        match team {
            Team::Player(index) => DEFAULT_LOBBY_FACTIONS
                .get(index)
                .copied()
                .unwrap_or(Self::Alliance),
            Team::Neutral => Self::Alliance,
        }
    }

    pub(crate) fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|faction| *faction == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

pub(crate) const GODOT_STANDARD_STARTING_RESOURCE_INDEX: usize = 1;
pub(crate) const DEFAULT_STARTING_RESOURCE_INDEX: usize = 3;
pub(crate) const SKIRMISH_TEAM_OPTION_COUNT: u8 = MAX_SKIRMISH_LOBBY_SLOTS as u8;
pub(crate) const BEVY_PLAYTEST_STARTING_RESOURCES: StartingResources =
    StartingResources::new(260, 80);
pub(crate) fn default_active_teams() -> Vec<bool> {
    DEFAULT_LOBBY_CONTROLLERS
        .into_iter()
        .map(SkirmishPlayerController::is_active)
        .collect()
}

pub(crate) fn default_player_factions() -> Vec<SkirmishFaction> {
    DEFAULT_LOBBY_FACTIONS.to_vec()
}

pub(crate) fn default_player_color_slots() -> Vec<usize> {
    DEFAULT_LOBBY_COLOR_SLOTS.to_vec()
}

pub(crate) fn default_player_controllers() -> Vec<SkirmishPlayerController> {
    DEFAULT_LOBBY_CONTROLLERS.to_vec()
}

pub(crate) fn default_player_spawn_slots() -> Vec<usize> {
    (0..MAX_SKIRMISH_LOBBY_SLOTS).collect()
}

pub(crate) const GODOT_STARTING_RESOURCE_OPTIONS: &[StartingResourceOption] = &[
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
pub(crate) struct MatchSetupSettings {
    pub(crate) map_path: &'static str,
    pub(crate) starting_resources: StartingResources,
    pub(crate) visible_player: VisiblePlayer,
    pub(crate) ai_difficulties: AiDifficultySettings,
    pub(crate) team_relations: TeamRelations,
    pub(crate) startup_loadout: StartupLoadoutMode,
    pub(crate) active_teams: Vec<bool>,
    pub(crate) player_factions: Vec<SkirmishFaction>,
    pub(crate) player_color_slots: Vec<usize>,
    pub(crate) player_controllers: Vec<SkirmishPlayerController>,
    pub(crate) player_spawn_slots: Vec<usize>,
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
            // Minimal RA2/SC1-style opening: one base + workers, build the rest.
            // (PlaytestExpanded dumped ~10-20 buildings that cluttered/blocked the start.)
            startup_loadout: StartupLoadoutMode::GodotSkirmish,
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
    pub(crate) fn with_map(mut self, map_path: &'static str) -> Self {
        self.map_path = map_path;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_starting_resources(mut self, starting_resources: StartingResources) -> Self {
        self.starting_resources = starting_resources;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_visible_player(mut self, visible_player: VisiblePlayer) -> Self {
        self.visible_player = visible_player;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_ai_difficulties(mut self, ai_difficulties: AiDifficultySettings) -> Self {
        self.ai_difficulties = ai_difficulties;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_startup_loadout(mut self, startup_loadout: StartupLoadoutMode) -> Self {
        self.startup_loadout = startup_loadout;
        self
    }

    pub(crate) fn team_active(&self, team: Team) -> bool {
        team.economy_index()
            .and_then(|index| self.active_teams.get(index).copied())
            .unwrap_or(false)
    }

    pub(crate) fn player_faction(&self, team: Team) -> SkirmishFaction {
        team.economy_index()
            .and_then(|index| self.player_factions.get(index).copied())
            .unwrap_or_else(|| SkirmishFaction::from_team(team))
    }

    pub(crate) fn player_spawn_slot(&self, team: Team) -> usize {
        team.economy_index()
            .and_then(|index| self.player_spawn_slots.get(index).copied())
            .unwrap_or_else(|| team.economy_index().unwrap_or(0))
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveTeams(pub(crate) Vec<bool>);

impl Default for ActiveTeams {
    fn default() -> Self {
        Self(default_active_teams())
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlayerFactions(pub(crate) Vec<SkirmishFaction>);

impl Default for PlayerFactions {
    fn default() -> Self {
        Self(default_player_factions())
    }
}

impl PlayerFactions {
    pub(crate) fn faction(&self, team: Team) -> Option<SkirmishFaction> {
        team.economy_index()
            .and_then(|index| self.0.get(index).copied())
    }

    pub(crate) fn slot_faction(&self, team: Team) -> SkirmishFaction {
        self.faction(team)
            .unwrap_or_else(|| SkirmishFaction::from_team(team))
    }
}

pub(crate) fn faction_def(faction: SkirmishFaction) -> Option<&'static registry::FactionDef> {
    registry::faction(faction.registry_id())
}

pub(crate) fn slot_faction_from_option(
    player_factions: Option<&PlayerFactions>,
    team: Team,
) -> SkirmishFaction {
    player_factions.map_or_else(
        || SkirmishFaction::from_team(team),
        |factions| factions.slot_faction(team),
    )
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlayerColorSlots(pub(crate) Vec<usize>);

impl Default for PlayerColorSlots {
    fn default() -> Self {
        Self(default_player_color_slots())
    }
}

impl PlayerColorSlots {
    pub(crate) fn slot(&self, team: Team) -> Option<usize> {
        team.economy_index()
            .and_then(|index| self.0.get(index).copied())
            .map(|slot| slot % PLAYER_COLOR_PALETTE.len())
    }

    pub(crate) fn color(&self, team: Team) -> Color {
        self.slot(team)
            .map(player_color)
            .unwrap_or_else(|| Color::srgb(0.74, 0.77, 0.72))
    }

    pub(crate) fn color_rgb(&self, team: Team) -> [f32; 3] {
        self.slot(team)
            .map(player_color_rgb)
            .unwrap_or([0.74, 0.77, 0.72])
    }

    pub(crate) fn minimap_color(&self, team: Team) -> Color {
        self.slot(team)
            .map(|slot| player_color_with_alpha(slot, 0.95))
            .unwrap_or_else(|| Color::srgba(0.78, 0.78, 0.68, 0.86))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkirmishPlayerController {
    None,
    Human,
    Ai(AiDifficulty),
}

impl SkirmishPlayerController {
    pub(crate) fn is_active(self) -> bool {
        self != Self::None
    }

    pub(crate) fn is_human(self) -> bool {
        self == Self::Human
    }

    pub(crate) fn ai_difficulty(self) -> Option<AiDifficulty> {
        match self {
            Self::Ai(difficulty) => Some(difficulty),
            Self::None | Self::Human => None,
        }
    }

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Self::None => t("无", "None"),
            Self::Human => t("人族玩家", "Human"),
            Self::Ai(AiDifficulty::Beginner) => t("电脑新手", "AI Beginner"),
            Self::Ai(AiDifficulty::Easy) => t("电脑简单", "AI Easy"),
            Self::Ai(AiDifficulty::Normal) => t("电脑普通", "AI Normal"),
            Self::Ai(AiDifficulty::Hard) => t("电脑困难", "AI Hard"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SkirmishMatchMode {
    #[default]
    OneVsOne,
    FreeForAll,
    AiVsAi,
    AlliedTwoVsOne,
}

impl SkirmishMatchMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OneVsOne => "1v1",
            Self::FreeForAll => t("自由混战", "Free-for-All"),
            Self::AiVsAi => t("AI对战", "AI vs AI"),
            Self::AlliedTwoVsOne => t("盟军2v1", "Allied 2v1"),
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkirmishMenuSelection {
    pub(crate) map_index: usize,
    pub(crate) starting_resource_index: usize,
    pub(crate) match_mode: SkirmishMatchMode,
    pub(crate) ai_difficulty: AiDifficulty,
    pub(crate) lobby_controllers: [SkirmishPlayerController; MAX_SKIRMISH_LOBBY_SLOTS],
    pub(crate) lobby_factions: [SkirmishFaction; MAX_SKIRMISH_LOBBY_SLOTS],
    pub(crate) lobby_team_ids: [u8; MAX_SKIRMISH_LOBBY_SLOTS],
    pub(crate) lobby_color_slots: [usize; MAX_SKIRMISH_LOBBY_SLOTS],
    pub(crate) controller_dropdown_open: Option<usize>,
    pub(crate) faction_dropdown_open: Option<usize>,
    pub(crate) team_dropdown_open: Option<usize>,
    pub(crate) color_dropdown_open: Option<usize>,
    pub(crate) map_dropdown_open: bool,
    pub(crate) resources_dropdown_open: bool,
}

impl Default for SkirmishMenuSelection {
    fn default() -> Self {
        Self {
            map_index: 0,
            starting_resource_index: DEFAULT_STARTING_RESOURCE_INDEX,
            match_mode: SkirmishMatchMode::OneVsOne,
            ai_difficulty: AiDifficulty::Easy,
            lobby_controllers: DEFAULT_LOBBY_CONTROLLERS,
            controller_dropdown_open: None,
            faction_dropdown_open: None,
            team_dropdown_open: None,
            color_dropdown_open: None,
            map_dropdown_open: false,
            resources_dropdown_open: false,
            lobby_factions: DEFAULT_LOBBY_FACTIONS,
            lobby_team_ids: DEFAULT_LOBBY_TEAM_IDS,
            lobby_color_slots: DEFAULT_LOBBY_COLOR_SLOTS,
        }
    }
}

impl SkirmishMenuSelection {
    pub(crate) fn map(self) -> &'static SkirmishMapDef {
        if self.map_choice_is_random() {
            return largest_skirmish_map();
        }
        &SKIRMISH_MAPS[self.map_index.min(SKIRMISH_MAPS.len().saturating_sub(1))]
    }

    pub(crate) fn map_choice_is_random(self) -> bool {
        is_random_map_index(self.map_index)
    }

    pub(crate) fn map_label(self) -> &'static str {
        if self.map_choice_is_random() {
            random_map_label()
        } else {
            localized_skirmish_map_name(self.map())
        }
    }

    pub(crate) fn from_match_setup(settings: MatchSetupSettings) -> Self {
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
            controller_dropdown_open: None,
            faction_dropdown_open: None,
            team_dropdown_open: None,
            color_dropdown_open: None,
            map_dropdown_open: false,
            resources_dropdown_open: false,
            lobby_factions: lobby_factions_from_match_setup(&settings),
            lobby_team_ids: lobby_team_ids_from_match_setup(&settings),
            lobby_color_slots: lobby_color_slots_from_match_setup(&settings),
        }
    }

    pub(crate) fn starting_resources(self) -> StartingResources {
        GODOT_STARTING_RESOURCE_OPTIONS
            .get(self.starting_resource_index)
            .unwrap_or(&GODOT_STARTING_RESOURCE_OPTIONS[GODOT_STANDARD_STARTING_RESOURCE_INDEX])
            .resources
    }

    pub(crate) fn active_teams(self) -> Vec<bool> {
        skirmish_active_teams_from_controllers(&self.runtime_player_controllers())
    }

    pub(crate) fn active_team_count(self) -> usize {
        self.active_lobby_slot_count()
    }

    pub(crate) fn lobby_slot_limit(self) -> usize {
        if self.map_choice_is_random() {
            largest_skirmish_map().players.min(MAX_SKIRMISH_LOBBY_SLOTS)
        } else {
            self.map().players.min(MAX_SKIRMISH_LOBBY_SLOTS)
        }
    }

    pub(crate) fn active_lobby_slot_count(self) -> usize {
        (0..self.lobby_slot_limit())
            .filter(|slot| self.lobby_controllers[*slot].is_active())
            .count()
    }

    pub(crate) fn active_lobby_slots(self) -> Vec<usize> {
        (0..self.lobby_slot_limit())
            .filter(|slot| self.lobby_controllers[*slot].is_active())
            .collect()
    }

    pub(crate) fn runtime_slot_for_team(self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        self.active_lobby_slots().get(index).copied()
    }

    pub(crate) fn runtime_player_controllers(self) -> Vec<SkirmishPlayerController> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_controllers[slot])
            .collect()
    }

    pub(crate) fn runtime_player_factions(self) -> Vec<SkirmishFaction> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_factions[slot])
            .collect()
    }

    pub(crate) fn runtime_team_ids(self) -> Vec<usize> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| (self.lobby_team_ids[slot] % SKIRMISH_TEAM_OPTION_COUNT) as usize)
            .collect()
    }

    pub(crate) fn runtime_color_slots(self) -> Vec<usize> {
        self.active_lobby_slots()
            .into_iter()
            .map(|slot| self.lobby_color_slots[slot] % PLAYER_COLOR_PALETTE.len())
            .collect()
    }

    pub(crate) fn runtime_spawn_slots(self) -> Vec<usize> {
        self.active_lobby_slots()
    }

    pub(crate) fn resolved_map(self, seed: u32) -> &'static SkirmishMapDef {
        if self.map_choice_is_random() {
            random_map_for_required_slots(self.required_player_slots(), seed)
        } else {
            self.map()
        }
    }

    pub(crate) fn required_player_slots(self) -> usize {
        self.active_lobby_slot_count().max(2)
    }

    pub(crate) fn selected_map_player_slots(self) -> usize {
        if self.map_choice_is_random() {
            random_map_for_required_slots(self.required_player_slots(), 0).players
        } else {
            self.map().players
        }
    }

    pub(crate) fn start_status(self) -> SkirmishStartStatus {
        skirmish_start_status_for_setup(
            self.required_player_slots(),
            self.selected_map_player_slots(),
            &self.active_teams(),
            &self.team_relations(),
        )
    }

    pub(crate) fn can_start(self) -> bool {
        self.start_status().can_start()
    }

    pub(crate) fn match_setup_with_map_seed(self, seed: u32) -> MatchSetupSettings {
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
    pub(crate) fn match_setup(self) -> MatchSetupSettings {
        self.match_setup_with_map_seed(0)
    }

    pub(crate) fn set_match_mode(&mut self, mode: SkirmishMatchMode) {
        self.match_mode = mode;
    }

    pub(crate) fn lobby_slot_in_selected_map(self, slot: usize) -> bool {
        slot < self.lobby_slot_limit()
    }

    pub(crate) fn select_lobby_slot(&mut self, slot: usize) {
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

    pub(crate) fn set_lobby_slot_controller(
        &mut self,
        slot: usize,
        controller: SkirmishPlayerController,
    ) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        if controller.is_human() {
            self.select_lobby_slot(slot);
        } else {
            self.lobby_controllers[slot] = controller;
        }
    }

    pub(crate) fn cycle_lobby_slot_controller(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        // Three-state cycle so closing a slot is always 1-2 clicks (RA2/Warcraft
        // style): 关闭 -> 我方 -> 电脑 -> 关闭. AI difficulty comes from the global
        // F1-F4 selector, so it isn't buried in this per-slot cycle.
        let next = match self.lobby_controllers[slot] {
            SkirmishPlayerController::None => SkirmishPlayerController::Human,
            SkirmishPlayerController::Human => SkirmishPlayerController::Ai(self.ai_difficulty),
            SkirmishPlayerController::Ai(_) => SkirmishPlayerController::None,
        };
        self.set_lobby_slot_controller(slot, next);
    }

    pub(crate) fn close_all_lobby_dropdowns(&mut self) {
        self.controller_dropdown_open = None;
        self.faction_dropdown_open = None;
        self.team_dropdown_open = None;
        self.color_dropdown_open = None;
        self.map_dropdown_open = false;
        self.resources_dropdown_open = false;
    }

    pub(crate) fn toggle_map_dropdown(&mut self) {
        let was_open = self.map_dropdown_open;
        self.close_all_lobby_dropdowns();
        self.map_dropdown_open = !was_open;
    }

    pub(crate) fn toggle_resources_dropdown(&mut self) {
        let was_open = self.resources_dropdown_open;
        self.close_all_lobby_dropdowns();
        self.resources_dropdown_open = !was_open;
    }

    pub(crate) fn set_map_choice(&mut self, index: usize) {
        if index < SKIRMISH_MAPS.len() || is_random_map_index(index) {
            self.map_index = index;
        }
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn set_starting_resource_choice(&mut self, index: usize) {
        if index < GODOT_STARTING_RESOURCE_OPTIONS.len() {
            self.starting_resource_index = index;
        }
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn toggle_controller_dropdown(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        let was_open = self.controller_dropdown_open == Some(slot);
        self.close_all_lobby_dropdowns();
        self.controller_dropdown_open = (!was_open).then_some(slot);
    }

    pub(crate) fn toggle_faction_dropdown(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        let was_open = self.faction_dropdown_open == Some(slot);
        self.close_all_lobby_dropdowns();
        self.faction_dropdown_open = (!was_open).then_some(slot);
    }

    pub(crate) fn toggle_team_dropdown(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        let was_open = self.team_dropdown_open == Some(slot);
        self.close_all_lobby_dropdowns();
        self.team_dropdown_open = (!was_open).then_some(slot);
    }

    pub(crate) fn toggle_color_dropdown(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        let was_open = self.color_dropdown_open == Some(slot);
        self.close_all_lobby_dropdowns();
        self.color_dropdown_open = (!was_open).then_some(slot);
    }

    pub(crate) fn set_lobby_slot_faction_choice(&mut self, slot: usize, faction: SkirmishFaction) {
        if self.lobby_slot_in_selected_map(slot) {
            self.lobby_factions[slot] = faction;
        }
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn set_lobby_slot_controller_choice(
        &mut self,
        slot: usize,
        controller: SkirmishPlayerController,
    ) {
        self.set_lobby_slot_controller(slot, controller);
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn set_lobby_slot_team_choice(&mut self, slot: usize, team_index: usize) {
        if self.lobby_slot_in_selected_map(slot) {
            self.lobby_team_ids[slot] = (team_index as u8) % SKIRMISH_TEAM_OPTION_COUNT;
        }
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn set_lobby_slot_color_choice(&mut self, slot: usize, color_index: usize) {
        if self.lobby_slot_in_selected_map(slot) {
            self.lobby_color_slots[slot] = color_index % PLAYER_COLOR_PALETTE.len();
        }
        self.close_all_lobby_dropdowns();
    }

    pub(crate) fn cycle_lobby_slot_faction(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_factions[slot] = self.lobby_factions[slot].next();
    }

    pub(crate) fn cycle_lobby_slot_team_id(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_team_ids[slot] = (self.lobby_team_ids[slot] + 1) % SKIRMISH_TEAM_OPTION_COUNT;
    }

    pub(crate) fn cycle_lobby_slot_color(&mut self, slot: usize) {
        if !self.lobby_slot_in_selected_map(slot) {
            return;
        }
        self.lobby_color_slots[slot] =
            (self.lobby_color_slots[slot] + 1) % PLAYER_COLOR_PALETTE.len();
    }

    pub(crate) fn team_id(self, team: Team) -> Option<usize> {
        self.runtime_slot_for_team(team)
            .map(|slot| (self.lobby_team_ids[slot] % SKIRMISH_TEAM_OPTION_COUNT) as usize)
    }

    pub(crate) fn player_faction(self, team: Team) -> Option<SkirmishFaction> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_factions[slot])
    }

    pub(crate) fn focus_faction(self) -> SkirmishFaction {
        self.player_faction(self.focus_team())
            .unwrap_or_else(|| SkirmishFaction::from_team(self.focus_team()))
    }

    pub(crate) fn player_color_slot(self, team: Team) -> Option<usize> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_color_slots[slot] % PLAYER_COLOR_PALETTE.len())
    }

    pub(crate) fn set_ai_difficulty(&mut self, difficulty: AiDifficulty) {
        self.ai_difficulty = difficulty;
        for controller in &mut self.lobby_controllers {
            if matches!(controller, SkirmishPlayerController::Ai(_)) {
                *controller = SkirmishPlayerController::Ai(difficulty);
            }
        }
    }

    pub(crate) fn player_controller(self, team: Team) -> Option<SkirmishPlayerController> {
        self.runtime_slot_for_team(team)
            .map(|slot| self.lobby_controllers[slot])
    }

    pub(crate) fn human_lobby_slot(self) -> Option<usize> {
        (0..self.lobby_slot_limit()).find(|slot| self.lobby_controllers[*slot].is_human())
    }

    pub(crate) fn human_team(self) -> Option<Team> {
        player_teams(self.active_lobby_slot_count()).find(|team| {
            self.player_controller(*team)
                .is_some_and(|controller| controller.is_human())
        })
    }

    pub(crate) fn focus_team(self) -> Team {
        self.human_team()
            .or_else(|| {
                player_teams(self.active_lobby_slot_count()).find(|team| {
                    self.player_controller(*team)
                        .is_some_and(|controller| controller.is_active())
                })
            })
            .unwrap_or(Team::Player(0))
    }

    pub(crate) fn focus_lobby_slot(self) -> Option<usize> {
        self.human_lobby_slot()
            .or_else(|| self.active_lobby_slots().into_iter().next())
    }

    pub(crate) fn team_relations(self) -> TeamRelations {
        let active_teams = self.active_teams();
        let team_ids = self.runtime_team_ids();
        skirmish_team_relations_from_team_ids(&active_teams, &team_ids)
    }
}

pub(crate) fn match_setup_from_menu_selection(
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

pub(crate) fn request_shared_match_scene_start(
    setup_settings: &mut MatchSetupSettings,
    next_state: &mut NextState<AppScreen>,
    settings: MatchSetupSettings,
) {
    *setup_settings = settings;
    next_state.set(AppScreen::InMatch);
}

pub(crate) fn start_shared_match_from_menu_selection(
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
pub(crate) enum SkirmishStartStatus {
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

pub(crate) fn skirmish_start_status(
    required_slots: usize,
    available_slots: usize,
) -> SkirmishStartStatus {
    if available_slots < required_slots {
        SkirmishStartStatus::MapTooSmall {
            required_slots,
            available_slots,
        }
    } else {
        SkirmishStartStatus::Ready
    }
}

pub(crate) fn skirmish_start_status_for_setup(
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
    pub(crate) fn can_start(self) -> bool {
        matches!(self, Self::Ready)
    }

    pub(crate) fn summary_label(self) -> String {
        match self {
            Self::Ready => t("状态: 可开始", "Status: Ready").to_string(),
            Self::MapTooSmall {
                required_slots,
                available_slots,
            } => {
                format!(
                    "{}({available_slots}/{required_slots})",
                    t("状态: 地图出生点不足", "Status: Not enough map spawns ")
                )
            }
            Self::NotEnoughPlayers {
                active_players,
                required_players,
            } => {
                format!(
                    "{}{required_players}{}({active_players}/{required_players})",
                    t("状态: 需要至少", "Status: Need at least "),
                    t("名玩家", " players ")
                )
            }
            Self::NoOpposingTeams => {
                t("状态: 缺少敌对队伍", "Status: No opposing teams").to_string()
            }
        }
    }
}

pub(crate) fn random_map_label() -> &'static str {
    t("随机地图", "Random Map")
}

pub(crate) fn localized_skirmish_map_name(map: &SkirmishMapDef) -> &'static str {
    match map.name_key {
        "MAP_NAME_PLAIN_AND_SIMPLE" => t("简明战场", "Plain & Simple"),
        "MAP_NAME_FOUR_CORNERS" => t("四角战场", "Four Corners"),
        "MAP_NAME_TECH_DIVIDE" => t("科技分界线", "Tech Divide"),
        "MAP_NAME_BIG_ARENA" => t("大型竞技场", "Big Arena"),
        _ => map.name,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RouletteWheel<T> {
    pub(crate) values_w_accumulated_shares: Vec<(T, f32)>,
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

pub(crate) fn is_random_map_index(index: usize) -> bool {
    index == random_map_index()
}

pub(crate) fn random_map_candidates_for_required_slots(
    required_player_slots: usize,
) -> impl Iterator<Item = &'static SkirmishMapDef> {
    let required_player_slots = required_player_slots.max(2);
    SKIRMISH_MAPS
        .iter()
        .filter(move |map| map.players >= required_player_slots)
}

pub(crate) fn random_map_for_required_slots(
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

pub(crate) fn roulette_bucket_probability(seed: u32, bucket_count: usize) -> f32 {
    if bucket_count == 0 {
        return 0.0;
    }
    let bucket = seed as usize % bucket_count;
    (bucket as f32 + 0.5) / bucket_count as f32
}

pub(crate) fn largest_skirmish_map() -> &'static SkirmishMapDef {
    SKIRMISH_MAPS
        .iter()
        .max_by(|left, right| {
            left.players
                .cmp(&right.players)
                .then_with(|| skirmish_map_area(left).total_cmp(&skirmish_map_area(right)))
        })
        .unwrap_or(&SKIRMISH_MAPS[0])
}

pub(crate) fn skirmish_map_area(map: &SkirmishMapDef) -> f32 {
    map.size.0 * map.size.1
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RandomMapCursor(pub(crate) u32);

impl Default for RandomMapCursor {
    fn default() -> Self {
        Self(0x4f1b_2c3d)
    }
}

impl RandomMapCursor {
    pub(crate) fn next_seed(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

pub(crate) fn skirmish_active_teams_from_controllers(
    controllers: &[SkirmishPlayerController],
) -> Vec<bool> {
    controllers
        .iter()
        .copied()
        .map(SkirmishPlayerController::is_active)
        .collect()
}

pub(crate) fn skirmish_ai_difficulties_from_controllers(
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

pub(crate) fn skirmish_player_controllers_from_match_setup(
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

pub(crate) fn lobby_controllers_from_match_setup(
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

pub(crate) fn lobby_factions_from_match_setup(
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

pub(crate) fn lobby_team_ids_from_match_setup(
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
            team_ids[slot] = (team_id % SKIRMISH_TEAM_OPTION_COUNT as usize) as u8;
        }
    }
    team_ids
}

pub(crate) fn lobby_color_slots_from_match_setup(
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

pub(crate) fn skirmish_mode_from_active_teams(active_teams: &[bool]) -> SkirmishMatchMode {
    if active_teams.iter().filter(|active| **active).count() >= 3 {
        SkirmishMatchMode::FreeForAll
    } else {
        SkirmishMatchMode::OneVsOne
    }
}

pub(crate) fn skirmish_mode_from_match_setup(settings: &MatchSetupSettings) -> SkirmishMatchMode {
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

pub(crate) fn skirmish_team_relations_from_team_ids(
    active_teams: &[bool],
    team_ids: &[usize],
) -> TeamRelations {
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

pub(crate) fn skirmish_team_ids_from_relations(
    active_teams: &[bool],
    relations: &TeamRelations,
) -> Vec<usize> {
    let mut team_ids = (0..active_teams.len()).collect::<Vec<_>>();
    let mut assigned = vec![false; active_teams.len()];
    let mut next_team_id = 0usize;
    for leader_index in 0..active_teams.len() {
        let leader = Team::Player(leader_index);
        if !active_teams[leader_index] || assigned[leader_index] {
            continue;
        }
        let team_id = next_team_id;
        next_team_id += 1;
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

pub(crate) fn skirmish_has_opposing_active_teams(
    active_teams: &[bool],
    relations: &TeamRelations,
) -> bool {
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

pub(crate) fn allied_skirmish_ally(player_team: Team, active_team_count: usize) -> Option<Team> {
    default_skirmish_opponent(player_team, active_team_count)
}

pub(crate) fn allied_skirmish_enemy(player_team: Team, active_team_count: usize) -> Option<Team> {
    let ally = allied_skirmish_ally(player_team, active_team_count)?;
    player_teams(active_team_count).find(|team| *team != player_team && *team != ally)
}

pub(crate) fn skirmish_has_cross_team_alliance(
    relations: &TeamRelations,
    active_teams: &[bool],
) -> bool {
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

pub(crate) fn default_skirmish_opponent(
    player_team: Team,
    active_team_count: usize,
) -> Option<Team> {
    player_teams(active_team_count).find(|team| *team != player_team)
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MapBounds {
    pub(crate) half_width: f32,
    pub(crate) half_depth: f32,
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
    pub(crate) fn from_size(size: (f32, f32)) -> Self {
        Self {
            half_width: size.0 * 0.5,
            half_depth: size.1 * 0.5,
        }
    }

    pub(crate) fn from_map(map: &SkirmishMapDef) -> Self {
        Self::from_size(map.size)
    }

    pub(crate) fn contains_ground_point(self, point: Vec3) -> bool {
        point.x >= -self.half_width
            && point.x <= self.half_width
            && point.z >= -self.half_depth
            && point.z <= self.half_depth
    }

    pub(crate) fn clamp_ground_point(self, point: Vec3, margin: f32) -> Vec3 {
        let half_width = (self.half_width - margin).max(0.0);
        let half_depth = (self.half_depth - margin).max(0.0);
        Vec3::new(
            point.x.clamp(-half_width, half_width),
            point.y,
            point.z.clamp(-half_depth, half_depth),
        )
    }

    pub(crate) fn minimap_local_position(self, world: Vec3) -> Vec2 {
        let rect = self.minimap_content_rect();
        let x = ((world.x + self.half_width) / (self.half_width * 2.0)).clamp(0.0, 1.0);
        let z = ((world.z + self.half_depth) / (self.half_depth * 2.0)).clamp(0.0, 1.0);
        // Minimap top = world -Z, matching the view convention (screen-up = -Z,
        // same as the edge-pan / WASD fixes). Previously top mapped to +Z, so the
        // minimap was inverted vs the world and clicks moved the camera the wrong way.
        Vec2::new(rect.left + x * rect.width, rect.top + z * rect.height)
    }

    pub(crate) fn minimap_world_position(self, local: Vec2) -> Vec3 {
        let rect = self.minimap_content_rect();
        let x = ((local.x - rect.left) / rect.width).clamp(0.0, 1.0);
        let z = ((local.y - rect.top) / rect.height).clamp(0.0, 1.0);
        Vec3::new(
            x * self.half_width * 2.0 - self.half_width,
            0.0,
            z * self.half_depth * 2.0 - self.half_depth,
        )
    }

    pub(crate) fn minimap_world_position_checked(self, local: Vec2) -> Option<Vec3> {
        let rect = self.minimap_content_rect();
        (local.x >= rect.left
            && local.x <= rect.left + rect.width
            && local.y >= rect.top
            && local.y <= rect.top + rect.height)
            .then(|| self.minimap_world_position(local))
    }

    pub(crate) fn minimap_content_rect(self) -> MinimapContentRect {
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
pub(crate) struct TeamStartup {
    pub(crate) structures: &'static [SpawnSpec],
    pub(crate) units: &'static [SpawnSpec],
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupLoadoutMode {
    PlaytestExpanded,
    GodotSkirmish,
}

#[derive(Clone, Copy)]
pub(crate) struct TeamAiProfile {
    pub(crate) production_priority: &'static [&'static str],
    pub(crate) defense_priority: &'static [&'static str],
    pub(crate) defense_limits: &'static [(&'static str, usize)],
    pub(crate) expected_command_centers: usize,
    pub(crate) expected_workers: usize,
    pub(crate) expected_refineries: usize,
    pub(crate) expected_battlegroups: usize,
    pub(crate) expected_units_in_battlegroup: usize,
    pub(crate) active_offense_enabled: bool,
    pub(crate) opening_attack_grace: f32,
    pub(crate) capture_enabled: bool,
    pub(crate) saboteur_enabled: bool,
    pub(crate) support_powers_enabled: bool,
    pub(crate) production_interval: f32,
    pub(crate) attack_interval: f32,
    pub(crate) build_interval: f32,
    pub(crate) capture_interval: f32,
    pub(crate) saboteur_interval: f32,
    pub(crate) support_interval: f32,
    pub(crate) defense_limit_bonus: usize,
    pub(crate) tesla_fence_limit_bonus: usize,
}

pub(crate) const HUMAN_STARTUP: TeamStartup = TeamStartup {
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
            id: "Drone",
            offset: (0.7, -3.0),
        },
        SpawnSpec {
            id: "Worker",
            offset: (2.3, -3.8),
        },
    ],
};

pub(crate) const DEMON_STARTUP: TeamStartup = TeamStartup {
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

pub(crate) const CHAOS_STARTUP: TeamStartup = TeamStartup {
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

pub(crate) const HUMAN_GODOT_SKIRMISH_STARTUP: TeamStartup = TeamStartup {
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

pub(crate) const DEMON_GODOT_SKIRMISH_STARTUP: TeamStartup = TeamStartup {
    structures: &[SpawnSpec {
        id: "CommandCenter",
        offset: (0.0, 0.0),
    }],
    units: &[
        SpawnSpec {
            id: "RocketInfantry",
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

pub(crate) const CHAOS_GODOT_SKIRMISH_STARTUP: TeamStartup = TeamStartup {
    structures: &[SpawnSpec {
        id: "CommandCenter",
        offset: (0.0, 0.0),
    }],
    units: &[
        SpawnSpec {
            id: "ShieldTrooper",
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

pub(crate) const SKIRMISH_MAP_ORE_AMOUNT: i32 = 240;
pub(crate) const SKIRMISH_MAP_CRYSTAL_AMOUNT: i32 = 140;

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

pub(crate) const PLAIN_AND_SIMPLE_SPAWNS: &[(f32, f32)] =
    &[(10.0, 7.0), (40.0, 7.0), (40.0, 43.0), (10.0, 43.0)];

pub(crate) const PLAIN_AND_SIMPLE_RESOURCES: &[ResourceSpec] = &[
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

pub(crate) const FOUR_CORNERS_SPAWNS: &[(f32, f32)] =
    &[(10.0, 10.0), (62.0, 10.0), (62.0, 62.0), (10.0, 62.0)];

pub(crate) const FOUR_CORNERS_RESOURCES: &[ResourceSpec] = &[
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

pub(crate) const FOUR_CORNERS_NEUTRAL_TECH: &[NeutralTechSpec] = &[
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

pub(crate) const FOUR_CORNERS_CRATES: &[NamedSupplyCrateSpec] = &[
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

pub(crate) const TECH_DIVIDE_SPAWNS: &[(f32, f32)] = &[
    (10.0, 16.0),
    (10.0, 42.0),
    (10.0, 68.0),
    (74.0, 16.0),
    (74.0, 42.0),
    (74.0, 68.0),
];

pub(crate) const TECH_DIVIDE_RESOURCES: &[ResourceSpec] = &[
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

pub(crate) const TECH_DIVIDE_NEUTRAL_TECH: &[NeutralTechSpec] = &[
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

pub(crate) const TECH_DIVIDE_CRATES: &[NamedSupplyCrateSpec] = &[
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

pub(crate) const BIG_ARENA_SPAWNS: &[(f32, f32)] = &[
    (10.0, 30.0),
    (35.0, 10.0),
    (65.0, 10.0),
    (90.0, 30.0),
    (90.0, 70.0),
    (65.0, 90.0),
    (35.0, 90.0),
    (10.0, 70.0),
];

pub(crate) const BIG_ARENA_RESOURCES: &[ResourceSpec] = &[
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

pub(crate) const BIG_ARENA_NEUTRAL_TECH: &[NeutralTechSpec] = &[
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

pub(crate) const BIG_ARENA_CRATES: &[NamedSupplyCrateSpec] = &[
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

pub(crate) const EMPTY_NEUTRAL_TECH: &[NeutralTechSpec] = &[];
pub(crate) const EMPTY_NAMED_CRATES: &[NamedSupplyCrateSpec] = &[];

pub(crate) const SKIRMISH_MAPS: &[SkirmishMapDef] = &[
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
pub(crate) fn skirmish_maps() -> &'static [SkirmishMapDef] {
    SKIRMISH_MAPS
}

pub(crate) fn skirmish_map_by_path(path: &str) -> Option<&'static SkirmishMapDef> {
    SKIRMISH_MAPS.iter().find(|map| map.godot_path == path)
}

pub(crate) fn map_local_to_world(map: &SkirmishMapDef, point: (f32, f32)) -> Vec3 {
    Vec3::new(point.0 - map.size.0 * 0.5, 0.0, point.1 - map.size.1 * 0.5)
}

pub(crate) fn team_start_position(map: &SkirmishMapDef, team: Team) -> Vec3 {
    let spawn_index = team.economy_index().unwrap_or(0);
    team_start_position_for_spawn_slot(map, spawn_index)
}

pub(crate) fn team_start_position_for_spawn_slot(map: &SkirmishMapDef, spawn_index: usize) -> Vec3 {
    map.spawn_points
        .get(spawn_index)
        .copied()
        .map(|spawn_point| map_local_to_world(map, spawn_point))
        .unwrap_or_else(|| fallback_team_start_position_for_spawn_slot(map, spawn_index))
}

pub(crate) fn fallback_team_start_position_for_spawn_slot(
    map: &SkirmishMapDef,
    spawn_index: usize,
) -> Vec3 {
    const GOLDEN_ANGLE: f32 = 2.399_963_1;
    let bounds = MapBounds::from_map(map);
    let spawn_ring = spawn_index / map.spawn_points.len().max(1);
    let radius =
        (bounds.half_width.min(bounds.half_depth) * 0.58 - spawn_ring as f32 * 6.0).max(10.0);
    let angle = spawn_index as f32 * GOLDEN_ANGLE;
    bounds.clamp_ground_point(
        Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
        10.0,
    )
}

#[allow(dead_code)]
pub(crate) fn team_start_camera_focus(
    map: &SkirmishMapDef,
    team: Team,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    team_start_camera_focus_for_faction(map, team, SkirmishFaction::from_team(team), loadout)
}

pub(crate) fn team_start_camera_focus_for_faction(
    map: &SkirmishMapDef,
    team: Team,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    let base = team_start_position(map, team);
    team_start_camera_focus_from_base(base, faction, loadout)
}

pub(crate) fn team_start_camera_focus_for_spawn_slot(
    map: &SkirmishMapDef,
    spawn_index: usize,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    let base = team_start_position_for_spawn_slot(map, spawn_index);
    team_start_camera_focus_from_base(base, faction, loadout)
}

pub(crate) fn team_start_camera_focus_from_base(
    base: Vec3,
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> Vec3 {
    base + startup_camera_focus_offset(faction_startup_for_loadout(faction, loadout))
}

pub(crate) fn startup_camera_focus_offset(startup: &TeamStartup) -> Vec3 {
    startup_spawn_offset(startup.structures, CAMERA_START_PRIMARY_STRUCTURES)
        .or_else(|| startup_spawn_offset(startup.units, CAMERA_START_PRIMARY_UNITS))
        .or_else(|| startup_aabb_pivot_offset(startup.units))
        .unwrap_or(Vec3::ZERO)
}

pub(crate) fn startup_spawn_offset(spawns: &[SpawnSpec], priority_ids: &[&str]) -> Option<Vec3> {
    priority_ids.iter().find_map(|priority_id| {
        spawns
            .iter()
            .find(|spawn| spawn.id == *priority_id)
            .map(spawn_offset_to_ground_vec)
    })
}

pub(crate) fn startup_aabb_pivot_offset(spawns: &[SpawnSpec]) -> Option<Vec3> {
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

pub(crate) fn spawn_offset_to_ground_vec(spawn: &SpawnSpec) -> Vec3 {
    Vec3::new(spawn.offset.0, 0.0, spawn.offset.1)
}

pub(crate) const HUMAN_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "ScoutRover",
    "RocketInfantry",
    "InterceptorVTOL",
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

pub(crate) const DEMON_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "FlameAssaultBuggy",
    "RocketInfantry",
    "BomberVTOL",
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

pub(crate) const CHAOS_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "MirageScoutTank",
    "TeslaCrawlerMk2",
    "ShieldTrooper",
    "InterceptorVTOL",
    "ScoutRover",
    "FieldMedic",
    "Drone",
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

pub(crate) fn add_shared_match_resources(app: &mut App) -> &mut App {
    app.init_state::<AppScreen>()
        .init_resource::<Economies>()
        .init_resource::<TeamRelations>()
        .init_resource::<NavGrid>()
        .init_resource::<HudHitZones>()
        .init_resource::<TabSubgroupState>()
        .init_resource::<PendingLoadedSave>()
        .init_resource::<ReplayTimeline>()
        .init_resource::<BuildQueue>()
        .init_resource::<BuildStructureTab>()
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
        .init_resource::<MenuOptionsState>()
        .init_resource::<RandomMapCursor>()
        .init_resource::<MapBounds>()
        .init_resource::<CommandMode>()
        .init_resource::<SupportPowerPanelState>()
        .init_resource::<HoveredResource>()
        .init_resource::<HunyuanModelMaterialCache>()
        .init_resource::<StructurePlacementFeedback>()
        .init_resource::<MatchMenuState>()
        .init_resource::<MatchSpeed>()
        .init_resource::<MatchBriefingState>()
        .init_resource::<SelectionDragState>()
        .init_resource::<UnitGroups>()
        .init_resource::<IdleWorkerCycleState>()
        .init_resource::<CameraBookmarks>()
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

pub(crate) fn add_main_menu_scene(app: &mut App) -> &mut App {
    app.add_systems(
        OnEnter(AppScreen::AssetLoading),
        (queue_startup_loading_assets, setup_asset_loading_screen).chain(),
    )
    .add_systems(
        Update,
        update_asset_loading_screen.run_if(in_state(AppScreen::AssetLoading)),
    )
    .add_systems(OnEnter(AppScreen::MainMenu), setup_front_menu)
    .add_systems(
        Update,
        (front_menu_buttons, resize_front_menu_roster_preview)
            .run_if(in_state(AppScreen::MainMenu)),
    )
    .add_systems(OnEnter(AppScreen::OptionsMenu), setup_options_menu)
    .add_systems(
        Update,
        options_menu_buttons.run_if(in_state(AppScreen::OptionsMenu)),
    )
    .add_systems(OnEnter(AppScreen::CreditsMenu), setup_credits_menu)
    .add_systems(
        Update,
        credits_menu_buttons.run_if(in_state(AppScreen::CreditsMenu)),
    )
    .add_systems(
        OnEnter(AppScreen::SkirmishSetup),
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
            update_main_menu_map_resource_controls,
            update_skirmish_map_preview,
            update_main_menu_lobby_slots,
        )
            .chain()
            .run_if(in_state(AppScreen::SkirmishSetup)),
    )
}

/// Registers the live match scene shared by `cargo run`, capture, and gameplay tests.
pub fn add_shared_match_scene(app: &mut App) -> &mut App {
    if !app.is_plugin_added::<RtsCameraPlugin>() {
        app.add_plugins(RtsCameraPlugin);
    }
    add_shared_match_resources(app)
        .add_systems(
            OnEnter(AppScreen::InMatch),
            (
                apply_match_setup_settings,
                begin_match_from_setup,
                setup_support_cooldowns,
                setup,
                start_battle_music,
                reset_replay_timeline_for_new_match,
                apply_loaded_save,
                resume_replay_recording_after_load,
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
    if !app.is_plugin_added::<CursorAssetPlugin>() {
        app.add_plugins(CursorAssetPlugin);
    }
    if !app.is_plugin_added::<FluentPlugin>() {
        app.add_plugins(FluentPlugin);
    }
    app.add_plugins(
        ProgressPlugin::<AppScreen>::new()
            .with_asset_tracking()
            .with_state_transition(AppScreen::AssetLoading, AppScreen::MainMenu),
    );
    app.add_plugins(SharedMatchScenePlugin);
    add_main_menu_scene(app);
    app.init_resource::<StartupLoadingAssets>();
    app.init_resource::<Locale>();
    app.add_systems(Startup, (load_godot_model_map, load_rts_cursor));
    app.add_systems(
        Update,
        (
            sync_locale,
            toggle_language_hotkey,
            update_localized_text,
            update_rts_cursor,
        ),
    );
    // A thick gizmo group for HUD-in-world elements (health bars, shot tracers) so
    // they're chunky/visible, while the default group (grid, rings, order paths)
    // stays thin. A single wide line beats stacking thin lines, which the angled
    // camera spreads into separate slivers.
    app.init_gizmo_group::<HudGizmos>();
    {
        let mut store = app.world_mut().resource_mut::<GizmoConfigStore>();
        store.config_mut::<HudGizmos>().0.line.width = HUD_GIZMO_LINE_WIDTH;
    }
    app
}

pub(crate) fn queue_startup_loading_assets(
    mut loading: ResMut<AssetsLoading<AppScreen>>,
    mut retained: ResMut<StartupLoadingAssets>,
    policy: Res<StartupLoadingPolicy>,
    asset_server: Res<AssetServer>,
) {
    loading.allow_failures = false;
    loading.track_dependencies = true;
    retained.handles.clear();
    if !policy.preload_assets {
        return;
    }

    retain_loading_asset(
        &mut loading,
        &mut retained,
        asset_server.load::<Font>(UI_FONT_PATH),
    );
    retain_loading_asset(
        &mut loading,
        &mut retained,
        asset_server.load::<Image>("ui/background.png"),
    );
    retain_loading_asset(
        &mut loading,
        &mut retained,
        asset_server.load::<Image>("ui/icons/RosterPreview.png"),
    );
    for faction in [
        SkirmishFaction::Alliance,
        SkirmishFaction::Demon,
        SkirmishFaction::Chaos,
    ] {
        retain_loading_asset(
            &mut loading,
            &mut retained,
            asset_server.load::<Image>(faction.emblem_path()),
        );
    }
    retain_loading_asset(
        &mut loading,
        &mut retained,
        asset_server.load::<StaticCursor>(RTS_CURSOR_ASSET_PATH),
    );
    retain_loading_asset(
        &mut loading,
        &mut retained,
        asset_server.load::<GodotModelMapAsset>(GODOT_MODEL_MAP_ASSET_PATH),
    );

    for entity in registry::ENTITY_DEFS {
        if let Some(icon) = entity.icon {
            retain_loading_asset(
                &mut loading,
                &mut retained,
                asset_server.load::<Image>(icon),
            );
        }
        if entity.render_parts.is_empty()
            && ProceduralEntityModel::for_entity_id(entity.id).is_none()
        {
            retain_loading_asset(
                &mut loading,
                &mut retained,
                asset_server.load::<WorldAsset>(
                    GltfAssetLabel::Scene(0).from_asset(DEFAULT_MODEL_FALLBACK),
                ),
            );
        }
        for part in entity.render_parts {
            retain_loading_asset(
                &mut loading,
                &mut retained,
                asset_server.load::<WorldAsset>(GltfAssetLabel::Scene(0).from_asset(part.model)),
            );
        }
    }
}

pub(crate) fn retain_loading_asset<A: Asset>(
    loading: &mut AssetsLoading<AppScreen>,
    retained: &mut StartupLoadingAssets,
    handle: Handle<A>,
) {
    loading.add(&handle);
    retained.handles.push(handle.untyped());
}

pub(crate) fn setup_asset_loading_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        Name::new("Loading Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::AssetLoading),
    ));
    commands
        .spawn((
            Name::new("Startup Loading Screen"),
            DespawnOnExit(AppScreen::AssetLoading),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.006, 0.01, 0.01)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(t("部署指挥系统", "Deploying command systems")),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(34.0),
                    ..default()
                },
                TextColor(Color::srgb(0.74, 1.0, 0.92)),
            ));
            root.spawn((
                Text::new(t("正在加载资产 0/0", "Loading assets 0/0")),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.62, 0.78, 0.74)),
                StartupLoadingText,
            ));
            root.spawn((
                Node {
                    width: Val::Px(480.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.16, 0.56, 0.5)),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.17, 0.82, 0.58)),
                    StartupLoadingFill,
                ));
            });
        });
}

pub(crate) fn update_asset_loading_screen(
    tracker: Res<ProgressTracker<AppScreen>>,
    mut fill_q: Query<&mut Node, With<StartupLoadingFill>>,
    mut text_q: Query<&mut Text, With<StartupLoadingText>>,
) {
    let progress = tracker.get_global_progress();
    let ratio = if progress.total == 0 {
        1.0
    } else {
        (progress.done as f32 / progress.total as f32).clamp(0.0, 1.0)
    };
    for mut node in &mut fill_q {
        node.width = Val::Percent(ratio * 100.0);
    }
    let label = if current_language() == Language::Zh {
        format!("正在加载资产 {}/{}", progress.done, progress.total)
    } else {
        format!("Loading assets {}/{}", progress.done, progress.total)
    };
    for mut text in &mut text_q {
        **text = label.clone();
    }
}

pub(crate) fn load_godot_model_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GodotModelMapHandle(
        asset_server.load(GODOT_MODEL_MAP_ASSET_PATH),
    ));
}

pub(crate) const HUD_GIZMO_LINE_WIDTH: f32 = 6.0;

/// F12 toggles the UI language (Chinese / English). Input may be absent in pure
/// headless apps, so the keyboard resource is optional.
pub(crate) fn toggle_language_hotkey(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut locale: ResMut<Locale>,
    menu_selection: Option<ResMut<SkirmishMenuSelection>>,
) {
    if let Some(keyboard) = keyboard
        && keyboard.just_pressed(KeyCode::F12)
    {
        locale.0 = locale.0.toggled();
        // The menu rebuilds its dynamic rows/buttons on selection change; nudge it
        // so lobby labels re-evaluate `t()` in the new language immediately.
        if let Some(mut menu_selection) = menu_selection {
            menu_selection.set_changed();
        }
    }
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

pub(crate) fn add_headless_game_plugins(app: &mut App) -> &mut App {
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
    .init_asset::<Image>()
    .init_asset::<bevy::image::TextureAtlasLayout>()
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
        .insert_resource(StartupLoadingPolicy {
            preload_assets: matches!(mode, GameAppMode::Interactive),
        })
        .add_plugins((
            JsonAssetPlugin::<RtsDataManifest>::new(&["rts.json"]),
            RonAssetPlugin::<RtsDataManifest>::new(&["rts.ron"]),
            RonAssetPlugin::<GodotModelMapAsset>::new(&["model_map.ron"]),
        ))
        .insert_resource(RenderErrorHandler(handle_render_error));
    add_game_scenes(&mut app);
    app
}

pub(crate) const MODEL_HARNESS_ASPECT_RATIO: f32 = 1600.0 / 1000.0;

pub(crate) fn cleanup_match_scoped_entities(
    mut commands: Commands,
    entities: Query<Entity, With<MatchScopedEntity>>,
) {
    for entity in &entities {
        commands.entity(entity).try_despawn();
    }
}

pub(crate) fn stop_match_flow_on_exit(
    mut match_flow: ResMut<MatchFlow>,
    mut menu: ResMut<MatchMenuState>,
) {
    match_flow.active = false;
    menu.visible = false;
}

pub(crate) fn reset_match_speed_on_exit(
    mut match_speed: ResMut<MatchSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    *match_speed = MatchSpeed::default();
    virtual_time.set_relative_speed(MatchSpeedPreset::Normal.scale());
}

pub(crate) fn advance_match_restart(mut next_state: ResMut<NextState<AppScreen>>) {
    next_state.set(AppScreen::InMatch);
}

pub(crate) fn add_runtime_systems(app: &mut App) -> &mut App {
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
            camera_control
                .before(RtsCameraSystemSet)
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            apply_camera_settings
                .before(RtsCameraSystemSet)
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            minimap_input
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            update_command_mode
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            (
                refresh_hud_hit_zones.before(select_entities),
                select_entities,
            )
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            (selection_hotkeys, cycle_selection_subgroup)
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            (
                quicksave_hotkey,
                quickload_hotkey,
                record_replay_keyframes,
                replay_jump_hotkeys,
            )
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
            process_idle_worker_selection_requests
                .after(selection_hotkeys)
                .after(command_shortcuts)
                .after(command_buttons)
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
        ),
    );
    app.add_systems(
        Update,
        (
            auto_assign_idle_resource_collectors
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
            auto_assign_ai_supply_crate_collectors
                .in_set(SimulationPhase::UiAndManagement)
                .run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        support_power_buttons
            .after(issue_orders)
            .after(select_entities)
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        refresh_support_power_panel
            .after(update_support_cooldowns)
            .after(support_power_buttons)
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_command_tooltip
            .after(refresh_command_panel)
            .after(refresh_support_power_panel)
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_ai_drone_scouting
            .in_set(SimulationPhase::UiAndManagement)
            .run_if(match_in_progress),
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
            (
                rebuild_nav_grid.before(plan_unit_paths),
                plan_unit_paths.before(move_units),
            )
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            move_units
                .in_set(SimulationPhase::Combat)
                .run_if(match_in_progress),
            separate_units
                .after(move_units)
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
            update_idle_tower_scan
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
            update_fog_overlay
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
            (update_battle_log, update_battle_music_volume)
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_minimap
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_hud
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_resource_bar
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_selection_text_visibility
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_selection_portrait
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            recenter_entity_models
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            tint_resource_models
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            update_resource_hover
                .in_set(SimulationPhase::PostCombat)
                .before(draw_world_overlays)
                .run_if(match_in_progress),
            draw_world_overlays
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
            draw_selected_rally_flags
                .in_set(SimulationPhase::PostCombat)
                .run_if(match_in_progress),
        ),
    )
    .add_systems(
        Update,
        apply_hunyuan_model_materials
            .in_set(SimulationPhase::PostCombat)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_impact_bursts
            .in_set(SimulationPhase::PostCombat)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        update_objective_tracker_hud
            .in_set(SimulationPhase::PostCombat)
            .run_if(match_in_progress),
    )
    .add_systems(
        Update,
        (
            match_end_buttons,
            update_match_end_overlay,
            update_match_end_charts,
        )
            .chain()
            .run_if(in_state(AppScreen::InMatch)),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScreenRect {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Resource, Default)]
pub(crate) struct LatestBattleEvent {
    pub(crate) focus: Option<Vec3>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattleEventPingKind {
    Generic,
    SupportPower,
    EnemySupportPower,
    EnemySuperweapon,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Team {
    Player(usize),
    Neutral,
}

impl Team {
    #[allow(dead_code)]
    pub(crate) fn faction_id(self) -> &'static str {
        match self {
            Team::Player(_) => "player",
            Team::Neutral => "neutral",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Team::Player(index) => index,
            Team::Neutral => usize::MAX,
        }
    }

    pub(crate) fn economy_index(self) -> Option<usize> {
        match self {
            Team::Player(index) => Some(index),
            Team::Neutral => None,
        }
    }

    pub(crate) fn label(self) -> String {
        match self {
            Team::Player(index) => format!("{}{}", t("玩家", "Player "), index + 1),
            Team::Neutral => t("中立", "Neutral").to_string(),
        }
    }

    pub(crate) fn from_playable_index(index: usize) -> Option<Self> {
        Some(Team::Player(index))
    }
}

pub(crate) fn player_teams(count: usize) -> impl Iterator<Item = Team> {
    (0..count).map(Team::Player)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlayerVisibilityMode {
    PerPlayer,
    AllPlayers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlayerControlMode {
    Player,
    Spectator,
}

// --- Localization (i18n) ---------------------------------------------------
// UI strings are bilingual (Chinese / English). Rather than thread a `Res<Locale>`
// through every text system (several are already at Bevy's 16-param system
// limit), the active language is mirrored into a process-global flag that the
// `t(zh, en)` helper reads. `sync_locale` keeps the flag in step with the
// `Locale` resource each frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Zh,
    En,
}

impl Language {
    pub(crate) fn toggled(self) -> Self {
        match self {
            Language::Zh => Language::En,
            Language::En => Language::Zh,
        }
    }

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Language::Zh => "中文",
            Language::En => "EN",
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Locale(pub(crate) Language);

impl Default for Locale {
    fn default() -> Self {
        Locale(Language::Zh)
    }
}

pub(crate) static CURRENT_LANGUAGE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);
// The SUBSET font (~128 KB, tracked + deployed). NOT the full 3.2 MB face, which is
// gitignored and 404s on GitHub Pages. Re-run scripts/subset_font.sh after adding new
// Chinese UI text so the subset covers it (else tofu).
pub(crate) const UI_FONT_PATH: &str = "fonts/wqy-microhei-ui.ttf";

pub(crate) fn set_current_language(language: Language) {
    let value = match language {
        Language::Zh => 0,
        Language::En => 1,
    };
    CURRENT_LANGUAGE.store(value, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn current_language() -> Language {
    match CURRENT_LANGUAGE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Language::En,
        _ => Language::Zh,
    }
}

/// Picks the Chinese or English variant of a UI string per the active language.
pub(crate) fn t(zh: &'static str, en: &'static str) -> &'static str {
    match current_language() {
        Language::Zh => zh,
        Language::En => en,
    }
}

pub(crate) fn entity_label_zh(id: &str) -> Option<&'static str> {
    Some(match id {
        "AdvancedReactorPlant" => "高级反应堆",
        "AircraftFactory" => "飞机工厂",
        "AntiAirTurret" => "防空炮塔",
        "AntiAirWalker" => "防空机甲",
        "AntiGroundTurret" => "对地炮塔",
        "ArcCoilDefenseTower" => "电弧防御塔",
        "Barracks" => "兵营",
        "BomberVTOL" => "轰炸垂直机",
        "CommandCenter" => "指挥中心",
        "CryoSprayer" => "冷冻喷射兵",
        "Drone" => "无人机",
        "DroneMineLayer" => "布雷无人机",
        "EngineerDrone" => "工程无人机",
        "FieldMedic" => "战地医疗兵",
        "FlakHoverTank" => "防空悬浮坦克",
        "FlakRocketTeam" => "防空火箭小队",
        "FlakRocketTeamMk2" => "二型防空火箭小队",
        "FlameAssaultBuggy" => "火焰突击车",
        "GrenadierTrooper" => "榴弹兵",
        "HammerSiegeTank" => "重锤攻城坦克",
        "HeavyBombardmentAirship" => "重型轰炸飞艇",
        "HeavyMachinegunTrooper" => "重机枪兵",
        "HeavySiegeWalker" => "重型攻城机甲",
        "Helicopter" => "直升机",
        "InterceptorVTOL" => "截击垂直机",
        "JammerVehicle" => "干扰车",
        "LanceBeamDefenseTower" => "光矛防御塔",
        "LanceBeamTank" => "光矛坦克",
        "LandMine" => "地雷",
        "LightRifleInfantry" => "轻步枪兵",
        "LongbowMissileCrawler" => "长弓导弹车",
        "MirageScoutTank" => "幻影侦察坦克",
        "MobileRepairCrawler" => "机动维修车",
        "MobileShieldProjector" => "机动护盾车",
        "ModularMissileCarrier" => "模块化导弹车",
        "MortarTeam" => "迫击炮小队",
        "OrePurifier" => "矿石净化器",
        "PhaseSaboteur" => "相位破坏者",
        "PowerReactor" => "发电站",
        "PrismDefenseObelisk" => "棱镜方尖塔",
        "PulseRifleCommando" => "脉冲步枪突击队",
        "RadarUplink" => "雷达站",
        "RailArtilleryWalker" => "轨道炮机甲",
        "RailCannonBunker" => "轨道炮地堡",
        "RailSniperTeam" => "轨道狙击队",
        "RailgunTank" => "轨道炮坦克",
        "Refinery" => "精炼厂",
        "RepairPad" => "维修平台",
        "RoboticsBay" => "机器人车间",
        "RocketGunship" => "火箭武装艇",
        "RocketInfantry" => "火箭兵",
        "RocketTrooperRobot" => "火箭机器人",
        "SaboteurInfiltrator" => "渗透破坏者",
        "ScoutRover" => "侦察车",
        "ShieldTrooper" => "护盾兵",
        "ShockTrooper" => "震击兵",
        "SiegeAirship" => "攻城飞艇",
        "SiegeArtilleryVehicle" => "攻城火炮车",
        "SiegeDrillTank" => "攻城钻地坦克",
        "SniperScout" => "狙击侦察兵",
        "TacticalOfficer" => "战术军官",
        "Tank" => "坦克",
        "TechAirport" => "科技机场",
        "TechBunker" => "科技地堡",
        "TechHospital" => "科技医院",
        "TechLab" => "科技实验室",
        "TechOilDerrick" => "科技油井",
        "TechRepairDepot" => "科技维修站",
        "TeslaCrawlerMk2" => "二型特斯拉履带车",
        "TeslaFenceSegment" => "特斯拉围栏段",
        "VehicleFactory" => "载具工厂",
        "WeatherControlSpire" => "气象控制塔",
        "Worker" => "工人",
        _ => return None,
    })
}

pub(crate) fn localized_entity_label(id: &str) -> String {
    let en = registry::entity(id).map_or(id, |def| def.label);
    match current_language() {
        Language::Zh => entity_label_zh(id).unwrap_or(en).to_string(),
        Language::En => en.to_string(),
    }
}

pub(crate) fn localized_compact_entity_label(id: &str) -> String {
    compact_label(&localized_entity_label(id))
}

pub(crate) fn sync_locale(locale: Res<Locale>) {
    set_current_language(locale.0);
}

/// Tags a `Text` whose content is a fixed bilingual pair, so it re-translates
/// live when the language toggles (static `Text::new` content is otherwise frozen
/// at spawn). Dynamic text rebuilt every frame doesn't need this.
#[derive(Component, Clone, Copy)]
pub(crate) struct LocalizedText {
    pub(crate) zh: &'static str,
    pub(crate) en: &'static str,
}

pub(crate) fn update_localized_text(mut query: Query<(&LocalizedText, &mut Text)>) {
    for (localized, mut text) in &mut query {
        let wanted = t(localized.zh, localized.en);
        if text.0 != wanted {
            text.0 = wanted.to_string();
        }
    }
}

/// Bundle for a static UI label that re-translates on language toggle.
pub(crate) fn localized_text(zh: &'static str, en: &'static str) -> impl Bundle {
    (Text::new(t(zh, en)), LocalizedText { zh, en })
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisiblePlayer {
    pub(crate) team: Team,
    pub(crate) visibility: PlayerVisibilityMode,
    pub(crate) control: PlayerControlMode,
}

impl VisiblePlayer {
    pub(crate) fn per_player(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::PerPlayer,
            control: PlayerControlMode::Player,
        }
    }

    pub(crate) fn all_players(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::AllPlayers,
            control: PlayerControlMode::Spectator,
        }
    }

    #[cfg(test)]
    pub(crate) fn spectator_per_player(team: Team) -> Self {
        Self {
            team,
            visibility: PlayerVisibilityMode::PerPlayer,
            control: PlayerControlMode::Spectator,
        }
    }

    pub(crate) fn all_players_visible(self) -> bool {
        self.visibility == PlayerVisibilityMode::AllPlayers
    }

    pub(crate) fn is_spectator(self) -> bool {
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
pub(crate) struct TeamRelations {
    pub(crate) allied: Vec<Vec<bool>>,
}

impl Default for TeamRelations {
    fn default() -> Self {
        Self { allied: Vec::new() }
    }
}

impl TeamRelations {
    pub(crate) fn ensure_player_count(&mut self, count: usize) {
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

    pub(crate) fn set_allied(&mut self, a: Team, b: Team, allied: bool) {
        let (Some(a), Some(b)) = (a.economy_index(), b.economy_index()) else {
            return;
        };
        self.ensure_player_count(a.max(b) + 1);
        self.allied[a][b] = allied;
        self.allied[b][a] = allied;
    }

    pub(crate) fn are_allied(&self, a: Team, b: Team) -> bool {
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

    pub(crate) fn are_enemies(&self, a: Team, b: Team) -> bool {
        a.economy_index().is_some() && b.economy_index().is_some() && !self.are_allied(a, b)
    }
}

pub(crate) fn player_color(slot: usize) -> Color {
    let [r, g, b] = player_color_rgb(slot);
    Color::srgb(r, g, b)
}

pub(crate) fn player_color_with_alpha(slot: usize, alpha: f32) -> Color {
    let [r, g, b] = player_color_rgb(slot);
    Color::srgba(r, g, b, alpha)
}

pub(crate) fn player_color_rgb(slot: usize) -> [f32; 3] {
    PLAYER_COLOR_PALETTE[slot % PLAYER_COLOR_PALETTE.len()]
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Selectable {
    pub(crate) radius: f32,
}

#[derive(Component)]
pub(crate) struct Selected;

#[derive(Component, Clone, Copy)]
pub(crate) struct Unit {
    pub(crate) id: &'static str,
    pub(crate) speed: f32,
    pub(crate) can_crush: bool,
    pub(crate) can_be_crushed: bool,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Structure {
    pub(crate) id: &'static str,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Garrison {
    pub(crate) capacity: usize,
    pub(crate) damage_per_unit: f32,
    pub(crate) count: usize,
}

pub(crate) type ActiveUnitOrderFilter = Or<(
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

pub(crate) type IdleUnitOrderFilter = (
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
pub(crate) enum MovementDomain {
    Terrain,
    Air,
}

impl MovementDomain {
    pub(crate) fn from_registry(domain: registry::MoveDomain) -> Self {
        match domain {
            registry::MoveDomain::Terrain => Self::Terrain,
            registry::MoveDomain::Air => Self::Air,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct NextSpawnId(pub(crate) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QueueButtonState {
    pub(crate) count: usize,
    pub(crate) full: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisualFaction(pub(crate) SkirmishFaction);

#[derive(SystemParam)]
pub(crate) struct OrderResources<'w> {
    pub(crate) map_bounds: Res<'w, MapBounds>,
    pub(crate) relations: Res<'w, TeamRelations>,
    pub(crate) command_mode: ResMut<'w, CommandMode>,
    pub(crate) support_cooldowns: ResMut<'w, SupportCooldowns>,
    pub(crate) battle_log: ResMut<'w, BattleLog>,
    pub(crate) audio_feedback: ResMut<'w, AudioFeedback>,
}

#[derive(SystemParam)]
pub(crate) struct CommandActionResources<'w> {
    pub(crate) build_queue: ResMut<'w, BuildQueue>,
    pub(crate) build_structure_tab: ResMut<'w, BuildStructureTab>,
    pub(crate) command_mode: ResMut<'w, CommandMode>,
    pub(crate) economies: ResMut<'w, Economies>,
    pub(crate) player_factions: Res<'w, PlayerFactions>,
    pub(crate) audio_feedback: ResMut<'w, AudioFeedback>,
    pub(crate) battle_log: ResMut<'w, BattleLog>,
    pub(crate) idle_worker_cycle: ResMut<'w, IdleWorkerCycleState>,
}

#[derive(SystemParam)]
pub(crate) struct StructurePlacementInputResources<'w, 's> {
    pub(crate) visible_player: Res<'w, VisiblePlayer>,
    pub(crate) player_factions: Res<'w, PlayerFactions>,
    pub(crate) asset_server: Res<'w, AssetServer>,
    pub(crate) map_bounds: Res<'w, MapBounds>,
    pub(crate) next_id: ResMut<'w, NextSpawnId>,
    pub(crate) economies: ResMut<'w, Economies>,
    pub(crate) command_mode: ResMut<'w, CommandMode>,
    pub(crate) hud_zones: Res<'w, HudHitZones>,
    pub(crate) placement_feedback: ResMut<'w, StructurePlacementFeedback>,
    pub(crate) audio_feedback: ResMut<'w, AudioFeedback>,
    pub(crate) battle_log: ResMut<'w, BattleLog>,
    pub(crate) selected_constructors: Query<
        'w,
        's,
        (Entity, &'static Unit, &'static Team, &'static Health),
        (With<Selected>, With<Unit>, Without<Structure>),
    >,
    pub(crate) constructors: Query<
        'w,
        's,
        (
            Entity,
            &'static Unit,
            &'static Team,
            &'static Transform,
            &'static Health,
        ),
        (With<Unit>, Without<Structure>),
    >,
}

#[derive(SystemParam)]
pub(crate) struct StructurePlacementPreviewParams<'w, 's> {
    pub(crate) command_mode: Res<'w, CommandMode>,
    pub(crate) hud_zones: Res<'w, HudHitZones>,
    pub(crate) visible_player: Res<'w, VisiblePlayer>,
    pub(crate) player_factions: Res<'w, PlayerFactions>,
    pub(crate) economies: Res<'w, Economies>,
    pub(crate) map_bounds: Res<'w, MapBounds>,
    pub(crate) window_q: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub(crate) camera_q:
        Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
    pub(crate) structures: Query<'w, 's, StructurePrereqItem<'static>>,
    pub(crate) occupiers: Query<
        'w,
        's,
        PlacementOccupierItem<'static>,
        Or<(With<Unit>, With<Structure>, With<ResourceNode>)>,
    >,
}

#[derive(Component)]
pub(crate) struct ButtonLabel;

#[derive(Component)]
pub(crate) struct SupportPowersPanel;

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerButton {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerCooldownLabel {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerHotkeyLabel {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SupportPowerButtonSpec {
    pub(crate) kind: SupportPowerKind,
    pub(crate) icon_path: &'static str,
    pub(crate) hotkey_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SupportPowerButtonState {
    pub(crate) enabled: bool,
    pub(crate) unlocked: bool,
    pub(crate) active: bool,
    pub(crate) low_power: bool,
    pub(crate) cooldown_seconds: Option<u32>,
    pub(crate) badge_text: String,
}

pub(crate) fn support_power_button_specs() -> Vec<SupportPowerButtonSpec> {
    SupportPowerKind::ALL
        .into_iter()
        .map(|kind| SupportPowerButtonSpec {
            kind,
            icon_path: kind.icon_path(),
            hotkey_label: kind.hotkey_label(),
        })
        .collect()
}

pub(crate) fn apply_match_setup_settings(
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

pub(crate) fn begin_match_from_setup(
    mut match_flow: ResMut<MatchFlow>,
    mut match_state: ResMut<MatchState>,
    mut camera_state: ResMut<RtsCamera>,
    selected_map: Res<SelectedSkirmishMap>,
    setup_settings: Res<MatchSetupSettings>,
    mut command_mode: ResMut<CommandMode>,
    mut support_power_panel: ResMut<SupportPowerPanelState>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut unit_groups: ResMut<UnitGroups>,
    mut idle_worker_cycle: ResMut<IdleWorkerCycleState>,
    mut camera_bookmarks: ResMut<CameraBookmarks>,
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
    *support_power_panel = SupportPowerPanelState::default();
    *selection_drag = SelectionDragState::default();
    *unit_groups = UnitGroups::default();
    *idle_worker_cycle = IdleWorkerCycleState::default();
    *camera_bookmarks = CameraBookmarks::default();
    match_menu.visible = false;
    *briefing = MatchBriefingState::default();
    briefing.show();
    battle_log.entries.clear();
    *audio_feedback = AudioFeedback::default();
    *objective_tracker = ObjectiveTrackerState::default();
}

pub(crate) fn window_is_fullscreen(window: &Window) -> bool {
    !matches!(window.mode, WindowMode::Windowed)
}

pub(crate) fn set_window_fullscreen(window: &mut Window, fullscreen: bool) {
    window.mode = if fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

pub(crate) fn toggle_window_fullscreen(window: &mut Window) -> bool {
    let fullscreen = !window_is_fullscreen(window);
    set_window_fullscreen(window, fullscreen);
    fullscreen
}

pub(crate) fn spawn_options_group(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    title_zh: &'static str,
    title_en: &'static str,
    rows: &[(OptionsMenuAction, &'static str, &'static str)],
) {
    parent.spawn(options_group_node()).with_children(|group| {
        group.spawn(options_group_header(title_zh, title_en, font.clone()));
        for (action, zh, en) in rows {
            group
                .spawn(options_button(*action, 32.0))
                .with_children(|button| {
                    button.spawn((
                        localized_text(zh, en),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.88, 0.86)),
                    ));
                });
        }
    });
}

pub(crate) fn credits_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(&Interaction, &OptionsMenuButton, &mut BackgroundColor)>,
) {
    for (interaction, button, mut background) in &mut buttons {
        if *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left)
            && matches!(button.action, OptionsMenuAction::Back)
        {
            next_state.set(AppScreen::MainMenu);
        }
        *background = BackgroundColor(match interaction {
            Interaction::Pressed => Color::srgba(0.12, 0.13, 0.125, 0.96),
            Interaction::Hovered => Color::srgba(0.08, 0.085, 0.082, 0.94),
            Interaction::None => Color::srgba(0.05, 0.05, 0.048, 0.92),
        });
    }
}

pub(crate) fn restore_main_menu_selection_from_match_setup(
    setup_settings: Res<MatchSetupSettings>,
    mut selection: ResMut<SkirmishMenuSelection>,
) {
    *selection = SkirmishMenuSelection::from_match_setup(setup_settings.clone());
}

/// "队N" / "Team N" label for a 0-based team index.
pub(crate) fn skirmish_team_label(team_index: usize) -> String {
    format!("{}{}", t("队", "Team "), team_index + 1)
}

/// "色N" / "Color N" label for a 0-based color-palette index.
pub(crate) fn skirmish_color_label(color_index: usize) -> String {
    format!("{}{}", t("色", "Color "), color_index + 1)
}

pub(crate) fn starting_resource_option_label(option: &StartingResourceOption) -> &'static str {
    match option.key {
        "STARTING_RESOURCES_LOW" => t("低 4/2", "Low 4/2"),
        "STARTING_RESOURCES_STANDARD" => t("标准 8/4", "Standard 8/4"),
        "STARTING_RESOURCES_HIGH" => t("高 16/8", "High 16/8"),
        "STARTING_RESOURCES_RICH" => t("富矿 32/16", "Rich 32/16"),
        _ => t("资源", "Resources"),
    }
}

pub(crate) fn skirmish_map_preview_root() -> impl Bundle {
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

pub(crate) fn skirmish_map_preview_frame_node(rect: SkirmishMapPreviewRect) -> impl Bundle {
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

pub(crate) fn skirmish_map_preview_grid_line_node(
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

pub(crate) fn skirmish_map_preview_marker_node(
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

pub(crate) fn skirmish_map_preview_spawn_marker_node(
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

pub(crate) fn skirmish_spawn_slot_color(selection: SkirmishMenuSelection, slot: usize) -> Color {
    Team::from_playable_index(slot)
        .and_then(|team| selection.player_color_slot(team))
        .map(player_color)
        .unwrap_or_else(|| player_color(slot))
}

pub(crate) fn skirmish_map_preview_marker_color(
    kind: SkirmishMapPreviewMarkerKind,
    alpha: f32,
) -> Color {
    match kind {
        SkirmishMapPreviewMarkerKind::Spawn => Color::srgba(1.0, 1.0, 1.0, alpha),
        SkirmishMapPreviewMarkerKind::Ore => Color::srgba(0.25, 0.66, 1.0, alpha),
        SkirmishMapPreviewMarkerKind::Crystal => Color::srgba(1.0, 0.45, 0.24, alpha),
        SkirmishMapPreviewMarkerKind::NeutralTech => Color::srgba(1.0, 0.86, 0.28, alpha),
        SkirmishMapPreviewMarkerKind::SupplyCrate => Color::srgba(0.42, 1.0, 0.52, alpha),
    }
}

pub(crate) fn skirmish_map_preview_rect(
    map: &SkirmishMapDef,
    preview_size: Vec2,
) -> SkirmishMapPreviewRect {
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

pub(crate) fn skirmish_map_preview_point(
    map: &SkirmishMapDef,
    point: (f32, f32),
    preview_size: Vec2,
) -> Vec2 {
    let rect = skirmish_map_preview_rect(map, preview_size);
    Vec2::new(
        rect.left + rect.width * point.0 / map.size.0.max(1.0),
        rect.top + rect.height * point.1 / map.size.1.max(1.0),
    )
}

pub(crate) fn update_skirmish_map_preview(
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

pub(crate) fn update_main_menu_map_resource_controls(
    mut commands: Commands,
    selection: Res<SkirmishMenuSelection>,
    root_q: Query<(Entity, &MainMenuMapResourceControlsRoot)>,
    elements: Query<Entity, With<MainMenuMapResourceControlElement>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok((root, controls_root)) = root_q.single() else {
        return;
    };
    for entity in &elements {
        commands.entity(entity).try_despawn();
    }
    let Ok(mut root_commands) = commands.get_entity(root) else {
        return;
    };
    root_commands.with_children(|parent| {
        spawn_menu_map_resource_controls(parent, controls_root.font.clone(), *selection);
    });
}

pub(crate) fn update_main_menu_lobby_slots(
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
            spawn_menu_lobby_slot_row(
                parent,
                slot,
                list_root.font.clone(),
                &list_root.faction_emblems,
                *selection,
            );
        }
    });
}

pub(crate) fn update_main_menu_summary(
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

pub(crate) fn skirmish_player_controller_text(selection: SkirmishMenuSelection) -> String {
    player_teams(selection.active_teams().len())
        .filter_map(|team| {
            selection
                .player_controller(team)
                .map(|controller| format!("{}={}", team.label(), controller.short_label()))
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn skirmish_team_setup_text(selection: SkirmishMenuSelection) -> String {
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

pub(crate) fn skirmish_player_faction_text(selection: SkirmishMenuSelection) -> String {
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

pub(crate) fn skirmish_player_color_text(selection: SkirmishMenuSelection) -> String {
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

pub(crate) fn skirmish_opponents_text(selection: SkirmishMenuSelection) -> String {
    let focus_team = selection.focus_team();
    let active_teams = selection.active_teams();
    let relations = selection.team_relations();
    match selection.match_mode {
        SkirmishMatchMode::OneVsOne | SkirmishMatchMode::AiVsAi => player_teams(active_teams.len())
            .find(|team| *team != focus_team && relations.are_enemies(focus_team, *team))
            .map(|team| team.label().to_string())
            .unwrap_or_else(|| t("无", "None").to_string()),
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
                format!(
                    "{}（{}: {}）",
                    enemy.label(),
                    t("盟友", "Ally"),
                    ally.label()
                )
            }
            _ => t("无", "None").to_string(),
        },
    }
}

pub(crate) fn skirmish_faction_roster_summary(faction: SkirmishFaction) -> String {
    faction_roster_summary_for_id(faction.registry_id())
}

pub(crate) fn faction_roster_summary_for_id(faction_id: &str) -> String {
    let Some(faction) = registry::faction(faction_id) else {
        return t("资料缺失", "No data").to_string();
    };
    format!(
        "{} {}  {} {}  {} {}  {} {}",
        t("建筑", "Buildings"),
        faction.structures.len(),
        t("步兵", "Infantry"),
        faction_product_count(faction, "Barracks"),
        t("载具", "Vehicles"),
        faction_product_count(faction, "VehicleFactory"),
        t("空军", "Air"),
        faction_product_count(faction, "AircraftFactory")
    )
}

pub(crate) fn faction_product_count(faction: &registry::FactionDef, producer: &str) -> usize {
    faction.production_for(producer).map_or(0, <[&str]>::len)
}

pub(crate) fn faction_playstyle_summary(faction: SkirmishFaction) -> &'static str {
    match faction {
        SkirmishFaction::Alliance => t(
            "苍穹联盟: 全科技混合军，防御和兵种最完整，适合稳步推进",
            "Alliance: full-tech combined army; best defense and unit roster, for steady pushes",
        ),
        SkirmishFaction::Demon => t(
            "炽炎魔军: 火力突击和攻城压制，单位线更集中，适合快速正面进攻",
            "Demon: firepower rushes and siege pressure; tighter unit line, for fast frontal assaults",
        ),
        SkirmishFaction::Chaos => t(
            "混沌裂隙: 护盾、无人机、干扰和高阶防御，适合控场消耗",
            "Chaos: shields, drones, jamming and high-tier defense, for zone control and attrition",
        ),
    }
}

pub(crate) fn skirmish_faction_playstyle_summary(faction: SkirmishFaction) -> &'static str {
    faction_playstyle_summary(faction)
}

pub(crate) fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Option<ResMut<Assets<Image>>>,
    mut next_id: ResMut<NextSpawnId>,
    selected_map: Res<SelectedSkirmishMap>,
    setup_settings: Res<MatchSetupSettings>,
    camera_state: Res<RtsCamera>,
    options: Res<MenuOptionsState>,
) {
    let skirmish_map = selected_map.definition();
    let catalog_consistent = skirmish_map.is_catalog_consistent();
    debug_assert!(catalog_consistent);
    let map_bounds = MapBounds::from_map(skirmish_map);
    commands.insert_resource(map_bounds);

    commands.spawn((
        rts_camera_component(&camera_state, map_bounds, options.camera_tilt),
        RtsCameraControls {
            // Pan with the arrow keys + screen-edge; rotate with middle-drag or [ ].
            // WASD / Q / E are deliberately left to the godot-style command hotkeys.
            key_up: KeyCode::ArrowUp,
            key_down: KeyCode::ArrowDown,
            key_left: KeyCode::ArrowLeft,
            key_right: KeyCode::ArrowRight,
            key_rotate_left: KeyCode::BracketLeft,
            key_rotate_right: KeyCode::BracketRight,
            pan_speed: options.camera_pan_speed,
            edge_pan_width: if options.camera_edge_pan {
                CAMERA_EDGE_PAN_WIDTH
            } else {
                0.0
            },
            ..default()
        },
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

    // Fog shroud needs the image asset system (real render / capture). Pure
    // headless logic tests have no `Assets<Image>`, so skip it there.
    if let Some(mut images) = images {
        spawn_fog_overlay(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            map_bounds,
        );
    }

    commands.insert_resource(ResourceTintMaterials {
        ore: materials.add(resource_tint_material(ResourceKind::Ore)),
        crystal: materials.add(resource_tint_material(ResourceKind::Crystal)),
    });

    // Decorative scenery rocks (godot's decorations/RockLargeA.tscn uses the PLAIN
    // rock_largeA, NOT the crystal model — using rock_crystalsLargeA here made them
    // look like harvestable ore that couldn't be selected/harvested).
    for x in [-17.0, -8.0, 6.0, 15.0] {
        spawn_prop(
            &mut commands,
            &asset_server,
            "models/kenney-spacekit/rock_largeA.glb",
            Vec3::new(x, 0.0, -2.0 + x.sin() * 8.0),
            0.9,
        );
    }

    for team in player_teams(setup_settings.active_teams.len())
        .filter(|team| setup_settings.team_active(*team))
    {
        let spawn_slot = setup_settings.player_spawn_slot(team);
        let visible_team = setup_settings.visible_player.team;
        setup_team(
            &mut commands,
            &asset_server,
            &mut next_id,
            team,
            setup_settings.player_faction(team),
            visible_team,
            team_start_position_for_spawn_slot(skirmish_map, spawn_slot),
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
pub(crate) fn team_startup(team: Team) -> &'static TeamStartup {
    team_startup_for_loadout(team, StartupLoadoutMode::PlaytestExpanded)
}

#[allow(dead_code)]
pub(crate) fn team_startup_for_loadout(
    team: Team,
    loadout: StartupLoadoutMode,
) -> &'static TeamStartup {
    faction_startup_for_loadout(SkirmishFaction::from_team(team), loadout)
}

pub(crate) fn faction_startup_for_loadout(
    faction: SkirmishFaction,
    loadout: StartupLoadoutMode,
) -> &'static TeamStartup {
    if loadout == StartupLoadoutMode::GodotSkirmish {
        return match faction {
            SkirmishFaction::Alliance => &HUMAN_GODOT_SKIRMISH_STARTUP,
            SkirmishFaction::Demon => &DEMON_GODOT_SKIRMISH_STARTUP,
            SkirmishFaction::Chaos => &CHAOS_GODOT_SKIRMISH_STARTUP,
        };
    }
    match faction {
        SkirmishFaction::Alliance => &HUMAN_STARTUP,
        SkirmishFaction::Demon => &DEMON_STARTUP,
        SkirmishFaction::Chaos => &CHAOS_STARTUP,
    }
}

#[allow(dead_code)]
pub(crate) fn team_ai_profile(team: Team) -> &'static TeamAiProfile {
    faction_ai_profile(SkirmishFaction::from_team(team))
}

pub(crate) fn faction_ai_profile(faction: SkirmishFaction) -> &'static TeamAiProfile {
    match faction {
        SkirmishFaction::Alliance => &HUMAN_AI_PROFILE,
        SkirmishFaction::Demon => &DEMON_AI_PROFILE,
        SkirmishFaction::Chaos => &CHAOS_AI_PROFILE,
    }
}

#[allow(dead_code)]
pub(crate) fn team_ai_profile_for_difficulty(
    team: Team,
    difficulty: AiDifficulty,
) -> TeamAiProfile {
    faction_ai_profile_for_difficulty(SkirmishFaction::from_team(team), difficulty)
}

pub(crate) fn faction_ai_profile_for_difficulty(
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
            profile.expected_workers = 3;
            profile.expected_refineries = 1;
            profile.expected_battlegroups = 0;
            profile.expected_units_in_battlegroup = 0;
            profile.active_offense_enabled = false;
            profile.opening_attack_grace = 120.0;
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
            profile.tesla_fence_limit_bonus = 0;
        }
        AiDifficulty::Easy => {
            profile.defense_priority = BEGINNER_AI_DEFENSE_PRIORITY;
            profile.defense_limits = BEGINNER_AI_DEFENSE_LIMITS;
            profile.expected_command_centers = 1;
            profile.expected_workers = 2;
            profile.expected_refineries = 1;
            profile.expected_battlegroups = 1;
            profile.expected_units_in_battlegroup = 2;
            profile.active_offense_enabled = false;
            profile.opening_attack_grace = 90.0;
            profile.capture_enabled = false;
            profile.saboteur_enabled = false;
            profile.support_powers_enabled = false;
            profile.production_interval = 6.5;
            profile.attack_interval = 14.0;
            profile.build_interval = 13.0;
            profile.capture_interval = AI_CAPTURE_INTERVAL_SECONDS + 2.0;
            profile.saboteur_interval = AI_SABOTEUR_INTERVAL_SECONDS + 3.0;
            profile.support_interval = 5.5;
            profile.defense_limit_bonus = 0;
            profile.tesla_fence_limit_bonus = 0;
        }
        AiDifficulty::Normal => {}
        AiDifficulty::Hard => {
            profile.expected_command_centers = 2;
            profile.expected_workers = 3;
            profile.expected_refineries = 2;
            profile.expected_battlegroups = 3;
            profile.expected_units_in_battlegroup = 5;
            profile.opening_attack_grace = 35.0;
            profile.production_interval = 3.0;
            profile.attack_interval = 4.5;
            profile.build_interval = 8.0;
            profile.capture_interval = (AI_CAPTURE_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.saboteur_interval = (AI_SABOTEUR_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.support_interval = 2.5;
            profile.defense_limit_bonus = 1;
            profile.tesla_fence_limit_bonus = 2;
        }
    }
    if matches!(difficulty, AiDifficulty::Beginner) {
        debug_assert!(!ai_profile_requests_offensive_combat_units(&profile));
    }
    profile
}

pub(crate) fn setup_team(
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

pub(crate) fn setup_neutral_tech(
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

pub(crate) fn spawn_prop(
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
pub(crate) fn spawn_unit(
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

pub(crate) fn spawn_unit_for_faction(
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

pub(crate) fn spawn_unit_with_visual_faction(
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

pub(crate) fn spawn_structure(
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

pub(crate) fn spawn_structure_for_faction(
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
pub(crate) fn spawn_structure_with_rotation(
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

pub(crate) fn spawn_structure_with_rotation_for_faction(
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

pub(crate) fn spawn_structure_for_visual_faction(
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
            mode: RallyMode::Move,
        });
    }
    attach_support_effects(commands, entity_id, def);
    entity_id
}

pub(crate) fn structure_construction_progress(construction: UnderConstruction) -> f32 {
    if construction.total <= 0.0 {
        return 1.0;
    }
    ((construction.total - construction.remaining) / construction.total).clamp(0.0, 1.0)
}

pub(crate) fn structure_construction_health(max_health: f32, progress: f32) -> f32 {
    if max_health <= 1.0 {
        return max_health;
    }
    (1.0 + (max_health - 1.0) * progress.clamp(0.0, 1.0)).clamp(1.0, max_health)
}

pub(crate) fn apply_structure_construction_progress(
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

pub(crate) fn structure_is_constructed(under_construction: Option<&UnderConstruction>) -> bool {
    under_construction.is_none()
}

pub(crate) const HUMAN_SUPPORT_RATE_MULTIPLIER: f32 = 1.15;
pub(crate) const DEMON_STRUCTURE_WEAPON_DAMAGE_MULTIPLIER: f32 = 1.12;
pub(crate) const CHAOS_INCOMING_WEAPON_DAMAGE_SCALE: f32 = 0.9;

pub(crate) fn faction_support_rate_multiplier(faction: Option<SkirmishFaction>) -> f32 {
    match faction {
        Some(SkirmishFaction::Alliance) => HUMAN_SUPPORT_RATE_MULTIPLIER,
        Some(SkirmishFaction::Demon | SkirmishFaction::Chaos) | None => 1.0,
    }
}

pub(crate) fn is_infantry_unit(unit: &Unit) -> bool {
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
pub(crate) enum ProceduralEntityModel {
    LandMine,
    TeslaFenceSegment,
}

impl ProceduralEntityModel {
    pub(crate) fn for_entity_id(id: &str) -> Option<Self> {
        match id {
            "LandMine" => Some(Self::LandMine),
            "TeslaFenceSegment" => Some(Self::TeslaFenceSegment),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn part_count(self) -> usize {
        match self {
            Self::LandMine => 2,
            Self::TeslaFenceSegment => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FactionIdentityMarker {
    Human,
    Demon,
    Chaos,
}

impl FactionIdentityMarker {
    pub(crate) fn for_faction(faction: SkirmishFaction) -> Self {
        match faction {
            SkirmishFaction::Alliance => Self::Human,
            SkirmishFaction::Demon => Self::Demon,
            SkirmishFaction::Chaos => Self::Chaos,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn for_team(team: Team) -> Option<Self> {
        team.economy_index()
            .map(|_| Self::for_faction(SkirmishFaction::from_team(team)))
    }

    #[cfg(test)]
    pub(crate) fn part_count(self) -> usize {
        match self {
            Self::Human => 2,
            Self::Demon => 3,
            Self::Chaos => 2,
        }
    }
}

pub(crate) fn default_visual_faction(team: Team) -> Option<SkirmishFaction> {
    team.economy_index()
        .map(|_| SkirmishFaction::from_team(team))
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HunyuanModelPart {
    pub(crate) entity_id: &'static str,
}

impl HunyuanModelPart {
    pub(crate) fn for_render_part(
        entity_id: &'static str,
        part: &registry::RenderPart,
    ) -> Option<Self> {
        is_hunyuan_model_path(part.model).then_some(Self { entity_id })
    }
}

#[derive(Component)]
pub(crate) struct HunyuanModelMaterialized;

#[derive(Resource, Default)]
pub(crate) struct HunyuanModelMaterialCache {
    pub(crate) by_entity: BTreeMap<&'static str, Handle<StandardMaterial>>,
}

impl HunyuanModelMaterialCache {
    pub(crate) fn handle_for(
        &mut self,
        entity_id: &'static str,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.by_entity.get(entity_id) {
            return handle.clone();
        }
        let handle = materials.add(hunyuan_model_material(entity_id));
        self.by_entity.insert(entity_id, handle.clone());
        handle
    }
}

pub(crate) fn is_hunyuan_model_path(model: &str) -> bool {
    model.starts_with("models/hunyuan3d/")
}

pub(crate) fn hunyuan_model_material(entity_id: &str) -> StandardMaterial {
    let (base, metallic, roughness, glow) = match entity_id {
        "CryoSprayer" => (Color::srgb(0.70, 0.92, 1.0), 0.72, 0.28, 0.30),
        "LongbowMissileCrawler" => (Color::srgb(0.26, 0.29, 0.32), 0.90, 0.34, 0.10),
        "FlameAssaultBuggy" => (Color::srgb(0.92, 0.28, 0.10), 0.62, 0.36, 0.45),
        "HammerSiegeTank" => (Color::srgb(0.54, 0.55, 0.50), 0.88, 0.32, 0.08),
        "HeavySiegeWalker" => (Color::srgb(0.46, 0.49, 0.52), 0.88, 0.30, 0.08),
        "RailArtilleryWalker" => (Color::srgb(0.58, 0.53, 0.44), 0.84, 0.34, 0.10),
        "FlakHoverTank" => (Color::srgb(0.40, 0.48, 0.38), 0.82, 0.38, 0.08),
        "LanceBeamTank" => (Color::srgb(0.22, 0.50, 0.92), 0.78, 0.30, 0.32),
        "RailgunTank" => (Color::srgb(0.50, 0.55, 0.58), 0.90, 0.28, 0.16),
        "FlakRocketTeam" => (Color::srgb(0.45, 0.42, 0.38), 0.72, 0.44, 0.10),
        "FlakRocketTeamMk2" => (Color::srgb(0.58, 0.44, 0.34), 0.76, 0.40, 0.12),
        "MobileShieldProjector" => (Color::srgb(0.48, 0.32, 0.86), 0.74, 0.30, 0.35),
        "ModularMissileCarrier" => (Color::srgb(0.34, 0.35, 0.36), 0.90, 0.35, 0.14),
        "TeslaCrawlerMk2" => (Color::srgb(0.16, 0.42, 0.90), 0.80, 0.26, 0.55),
        _ => (Color::srgb(0.58, 0.56, 0.50), 0.78, 0.36, 0.10),
    };
    let lin = base.to_linear();
    StandardMaterial {
        base_color: base,
        metallic,
        perceptual_roughness: roughness,
        emissive: LinearRgba::new(lin.red * glow, lin.green * glow, lin.blue * glow, 1.0),
        ..default()
    }
}

pub(crate) fn spawn_entity_models(
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
            let mut spawned = commands.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(part.model))),
                render_part_transform(part),
            ));
            if let Some(marker) = HunyuanModelPart::for_render_part(def.id, part) {
                spawned.insert(marker);
            }
        }
    }
    spawn_faction_identity_marker(commands, root, visual_faction, def);
}

pub(crate) fn spawn_entity_models_for_harness(
    world: &mut World,
    root: Instance<ModelHarnessRoot>,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    let root = root.entity();
    let asset_server = world.resource::<AssetServer>().clone();
    if def.render_parts.is_empty() {
        if let Some(model) = ProceduralEntityModel::for_entity_id(def.id) {
            match model {
                ProceduralEntityModel::LandMine => spawn_land_mine_procedural_model(world, root),
                ProceduralEntityModel::TeslaFenceSegment => {
                    spawn_tesla_fence_segment_procedural_model(world, root)
                }
            }
        } else {
            world.spawn((
                ChildOf(root),
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(DEFAULT_MODEL_FALLBACK)),
                ),
                Transform::IDENTITY,
            ));
        }
    } else {
        for part in def.render_parts {
            let mut spawned = world.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(part.model))),
                render_part_transform(part),
            ));
            if let Some(marker) = HunyuanModelPart::for_render_part(def.id, part) {
                spawned.insert(marker);
            }
        }
    }
    if let Some(marker) = visual_faction.map(FactionIdentityMarker::for_faction) {
        let inv_scale = 1.0 / def.scale.max(0.1);
        let world_size = match def.role {
            registry::EntityRole::Structure => (def.radius * 0.26).clamp(0.28, 0.68),
            registry::EntityRole::Unit => (def.radius * 0.22).clamp(0.12, 0.26),
        };
        let local_size = world_size * inv_scale;
        let local_offset = Vec3::new(0.0, 0.12 * inv_scale, -def.radius * 0.72 * inv_scale);
        spawn_faction_identity_marker_model(world, root, marker, local_offset, local_size);
    }
}

pub(crate) fn render_part_transform(part: &registry::RenderPart) -> Transform {
    Transform::from_translation(Vec3::new(
        part.translation[0],
        part.translation[1],
        part.translation[2],
    ))
    .with_rotation(Quat::from_xyzw(
        part.rotation[0],
        part.rotation[1],
        part.rotation[2],
        part.rotation[3],
    ))
    .with_scale(Vec3::new(part.scale[0], part.scale[1], part.scale[2]))
}

pub(crate) fn spawn_faction_identity_marker(
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

pub(crate) fn spawn_faction_identity_marker_model(
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

pub(crate) fn spawn_procedural_entity_model(
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

pub(crate) fn add_procedural_mesh(
    world: &mut World,
    mesh: impl Into<Mesh>,
) -> Option<Handle<Mesh>> {
    let mut meshes = world.get_resource_mut::<Assets<Mesh>>()?;
    Some(meshes.add(mesh))
}

pub(crate) fn add_procedural_material(
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

pub(crate) fn spawn_procedural_mesh_child(
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

pub(crate) fn unit_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        def.radius * 5.0 + 3.5
    } else {
        FOG_REVEAL_RADIUS
    }
}

pub(crate) fn structure_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        (def.radius * 4.5 + 2.5).clamp(1.5, FOG_REVEAL_RADIUS)
    } else {
        0.0
    }
}

pub(crate) fn initial_visibility_state(team: Team, visible_team: Team) -> VisibilityState {
    VisibilityState {
        visible: team == visible_team,
    }
}

pub(crate) fn initial_visibility(team: Team, visible_team: Team) -> Visibility {
    if team == visible_team {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

pub(crate) fn match_end_button(action: MatchEndAction) -> impl Bundle {
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

pub(crate) fn match_end_button_label(
    zh: &'static str,
    en: &'static str,
    font: Handle<Font>,
) -> impl Bundle {
    (
        localized_text(zh, en),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

pub(crate) fn match_briefing_buttons(
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

pub(crate) fn match_briefing_button_visual(interaction: Interaction) -> (Color, Color) {
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

pub(crate) fn update_match_briefing_overlay(
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

pub(crate) fn match_briefing_text(
    settings: &MatchSetupSettings,
    visible_team: Team,
    relations: &TeamRelations,
    active_teams: &ActiveTeams,
) -> String {
    let (enemies, allies) = match_briefing_player_counts(visible_team, relations, active_teams);
    format!(
        "{}\n{}: {enemies}\n{}: {allies}\n{}: {} / {}: {}\n{}",
        t(
            "目标：摧毁所有敌方指挥中心，并保住至少一个我方指挥中心",
            "Objective: destroy all enemy Command Centers while keeping at least one of yours",
        ),
        t("敌人", "Enemies"),
        t("盟友", "Allies"),
        ResourceKind::Ore.label(),
        settings.starting_resources.ore,
        ResourceKind::Crystal.label(),
        settings.starting_resources.crystal,
        t(
            "推荐开局\n\
             - 派工人采集附近水晶，并尽快补充工人\n\
             - 在雷达、防御和高级生产耗电前先补电力\n\
             - 用兵营做廉价克制，或用战车工厂施加装甲压力\n\
             - 侦察敌方科技、占领中立建筑，并在后期武器到来前打击扩张",
            "Opening tips\n\
             - Send workers to gather nearby crystal and add workers quickly\n\
             - Build power before radar, defense, and advanced production draw it down\n\
             - Use the Barracks for cheap counters, or the Vehicle Factory for armor pressure\n\
             - Scout enemy tech, capture neutral buildings, and strike expansions before late-game weapons arrive",
        ),
    )
}

pub(crate) fn match_briefing_player_counts(
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

pub(crate) fn setup_support_cooldowns(mut support_cooldowns: ResMut<SupportCooldowns>) {
    *support_cooldowns = SupportCooldowns::default();
}

pub(crate) fn update_support_cooldowns(
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

pub(crate) fn spawn_paradrop_units(
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

pub(crate) fn find_paradrop_spawn_position(
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

pub(crate) fn is_spawn_position_free(
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

pub(crate) fn update_match_clock(mut match_state: ResMut<MatchState>, time: Res<Time>) {
    if match_state.is_running() {
        match_state.start_time_sec += time.delta_secs();
    }
}

pub(crate) fn match_end_buttons(
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

pub(crate) fn match_end_button_visual(interaction: Interaction, enabled: bool) -> (Color, Color) {
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

pub(crate) fn update_match_end_overlay(
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
        **title_text = t("对局结算", "Match Results").to_string();
    }
    if let Ok(mut reason_text) = reason_text_q.single_mut() {
        **reason_text = match_state.result_reason.to_string();
    }
    if let Ok(mut stats_text) = stats_text_q.single_mut() {
        let minutes = (match_state.start_time_sec.max(0.0) / 60.0).floor() as u32;
        let seconds = (match_state.start_time_sec.max(0.0) as u32) % 60;
        let visible_economy = economies.get(visible_player.team);
        let human_losses = format!(
            "{}: {} {}  {} {}",
            t("己方损失", "Your losses"),
            t("单位", "units"),
            match_state.units_lost,
            t("建筑", "buildings"),
            match_state.structures_lost
        );
        let enemy_losses = format!(
            "{}: {} {}  {} {}",
            t("敌方击杀", "Enemy kills"),
            t("单位", "units"),
            match_state.enemy_units_destroyed,
            t("建筑", "buildings"),
            match_state.enemy_structures_destroyed
        );
        let resources = format!(
            "{}{}: {} {}  {} {}",
            visible_player.team.label(),
            t("资源", " resources"),
            ResourceKind::Ore.label(),
            visible_economy.ore,
            ResourceKind::Crystal.label(),
            visible_economy.crystal
        );
        **stats_text = format!(
            "{}: {}  {}: {}  {}: {:02}:{:02}\n{enemy_losses}\n{human_losses}\n{resources}",
            t("剩余阵营", "Teams left"),
            match_state.remaining_teams,
            t("剩余锚点", "Anchors left"),
            match_state.remaining_anchors,
            t("用时", "Time"),
            minutes,
            seconds
        );
    }
}

pub(crate) fn evaluate_match_end(
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
                t("战斗结束", "Battle Over"),
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
            t(
                "失利：己方锚点被全部摧毁",
                "Defeat: all your anchors were destroyed",
            ),
        );
        return;
    }

    if !has_enemy {
        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Victory);
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::HumanVictory,
            t("胜利：敌方锚点全部丢失", "Victory: all enemy anchors lost"),
        );
        return;
    }

    if active_teams <= 1 {
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::MatchFinished,
            t("战斗结束", "Battle Over"),
        );
    }
}

pub(crate) fn is_worker_elimination_anchor(unit: &Unit) -> bool {
    unit.id == "Worker"
}

pub(crate) fn is_structure_elimination_anchor(structure: &Structure) -> bool {
    structure.id == "CommandCenter"
}

pub(crate) fn record_active_elimination_anchor(
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

pub(crate) fn objective_tracker_snapshot(
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

pub(crate) fn objective_completion_percent(
    remaining_anchors: u32,
    total_anchors: u32,
    complete: bool,
) -> u32 {
    if complete {
        return 100;
    }
    if total_anchors == 0 {
        return 0;
    }
    let destroyed = total_anchors.saturating_sub(remaining_anchors);
    ((destroyed as f32 / total_anchors as f32) * 100.0).round() as u32
}

pub(crate) fn objective_tracker_text(snapshot: ObjectiveTrackerSnapshot) -> String {
    let title = if snapshot.total_anchors > 0 && snapshot.remaining_anchors == 0 {
        t("目标完成", "Objective complete")
    } else {
        t("目标: 消灭敌方锚点", "Objective: eliminate enemy anchors")
    };
    format!(
        "{title}\n{}: {}  {}: {}/{}\n{}: {}  {}: {}  {}: {}%",
        t("敌方队伍", "Enemy teams"),
        snapshot.enemy_teams,
        t("锚点", "Anchors"),
        snapshot.remaining_anchors,
        snapshot.total_anchors,
        t("结构", "Structures"),
        snapshot.structures,
        t("工人", "Workers"),
        snapshot.workers,
        t("进度", "Progress"),
        snapshot.completion_percent,
    )
}

pub(crate) fn spawn_fog_overlay(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bounds: MapBounds,
) {
    use bevy::image::ImageSampler;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    // Start fully shrouded (opaque black, alpha 255). `update_fog_overlay` carves
    // out the seen/explored areas each frame.
    let data = [0u8, 0, 0, 255].repeat(FOG_OVERLAY_RES * FOG_OVERLAY_RES);
    let mut image = Image::new(
        Extent3d {
            width: FOG_OVERLAY_RES as u32,
            height: FOG_OVERLAY_RES as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );
    // Linear filtering smooths the low-res shroud into soft fog edges.
    image.sampler = ImageSampler::linear();
    let handle = images.add(image);

    commands.spawn((
        Name::new("Fog of war shroud"),
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(bounds.half_width * 2.0, bounds.half_depth * 2.0),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(handle.clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, FOG_OVERLAY_Y, 0.0),
        FogOverlayPlane,
        MatchScopedEntity,
    ));

    commands.insert_resource(FogOverlay {
        handle,
        explored: vec![false; FOG_OVERLAY_RES * FOG_OVERLAY_RES],
    });
}

/// Repaints the fog shroud texture from the viewing player's (and allies')
/// current vision. Clear where seen now, dim where explored before, black where
/// never seen. Hidden entirely in spectator/all-visible mode.
pub(crate) fn update_fog_overlay(
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    bounds: Option<Res<MapBounds>>,
    fog: Option<ResMut<FogOverlay>>,
    images: Option<ResMut<Assets<Image>>>,
    revealers: Query<(&Team, &Transform, &VisionRadius, &VisibilityState)>,
    mut overlay_vis: Query<&mut Visibility, With<FogOverlayPlane>>,
) {
    let (Some(bounds), Some(mut fog), Some(mut images)) = (bounds, fog, images) else {
        return;
    };
    let bounds = *bounds;
    let hide_fog = visible_player.all_players_visible();
    for mut visibility in &mut overlay_vis {
        *visibility = if hide_fog {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if hide_fog {
        return;
    }
    let visible_team = visible_player.team;
    let res = FOG_OVERLAY_RES;
    let mut visible_now = vec![false; res * res];
    let (hw, hd) = (bounds.half_width.max(0.01), bounds.half_depth.max(0.01));
    for (team, transform, vision, state) in &revealers {
        if !state.visible || !relations.are_allied(visible_team, *team) {
            continue;
        }
        let r = vision.0 + FOG_COMPENSATION;
        let pos = transform.translation;
        // Center + radius in texture pixels (map is centered on the origin).
        let cu = (pos.x + hw) / (2.0 * hw) * res as f32;
        let cv = (pos.z + hd) / (2.0 * hd) * res as f32;
        let pr_x = (r / (2.0 * hw) * res as f32).max(0.5);
        let pr_z = (r / (2.0 * hd) * res as f32).max(0.5);
        let min_x = ((cu - pr_x).floor().max(0.0)) as usize;
        let max_x = ((cu + pr_x).ceil().min(res as f32 - 1.0)) as usize;
        let min_y = ((cv - pr_z).floor().max(0.0)) as usize;
        let max_y = ((cv + pr_z).ceil().min(res as f32 - 1.0)) as usize;
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = (px as f32 + 0.5 - cu) / pr_x;
                let dy = (py as f32 + 0.5 - cv) / pr_z;
                if dx * dx + dy * dy <= 1.0 {
                    visible_now[py * res + px] = true;
                }
            }
        }
    }
    let Some(mut image) = images.get_mut(&fog.handle) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    for i in 0..res * res {
        if visible_now[i] {
            fog.explored[i] = true;
        }
        data[i * 4 + 3] = if visible_now[i] {
            0
        } else if fog.explored[i] {
            FOG_OVERLAY_EXPLORED_ALPHA
        } else {
            255
        };
    }
}

pub(crate) fn update_visibility(
    mut commands: Commands,
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
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
                // Allies share vision: any allied unit/structure reveals fog for the
                // viewing player (godot `is_allied_with(visible_player)`).
                if relations.are_allied(visible_team, *team) && visibility_state.visible {
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
            let should_be_visible = if all_players_visible
                || relations.are_allied(visible_team, *team)
            {
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

pub(crate) fn push_under_attack_log(
    battle_log: &mut BattleLog,
    focus: Vec3,
    target_is_structure: bool,
) -> bool {
    if battle_log.under_attack_cooldown > 0.0 {
        return false;
    }
    let message = if target_is_structure {
        t("基地遭到攻击", "Base under attack")
    } else {
        t("单位遭到攻击", "Unit under attack")
    };
    push_battle_log(battle_log, message, Some(focus));
    battle_log.under_attack_cooldown = BATTLE_LOG_UNDER_ATTACK_COOLDOWN_SECONDS;
    true
}

pub(crate) fn update_minimap(
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

pub(crate) fn radar_state_for_team(
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

pub(crate) fn record_low_power_battle_log(battle_log: &mut BattleLog) {
    push_battle_log(
        battle_log,
        t(
            "低电力: 生产减速/防御停火/雷达离线",
            "Low power: slowed production / defenses offline / radar offline",
        ),
        None,
    );
}

pub(crate) fn record_insufficient_funds_battle_log(
    team: Team,
    player_team: Team,
    battle_log: &mut BattleLog,
) {
    if team == player_team {
        push_battle_log(battle_log, t("资源不足", "Not enough resources"), None);
    }
}

pub(crate) fn record_structure_placement_failure_battle_log(
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

pub(crate) fn record_support_power_audio_feedback(
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

pub(crate) fn record_build_action_audio_feedback(
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
        | BuildAction::SelectBuildTab(_)
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::SelectIdleWorker
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected => {}
    }
}

pub(crate) fn record_support_power_charging_feedback(
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
            format!(
                "{}: {} {charge_seconds}s",
                t("超级武器充能", "Superweapon charging"),
                power.label()
            ),
            None,
        );
    } else {
        push_battle_log(
            battle_log,
            format!(
                "{}: {} {charge_seconds}s",
                t("敌方超级武器充能", "Enemy superweapon charging"),
                power.label()
            ),
            None,
        );
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
    }
}

pub(crate) fn record_support_power_ready_battle_log(
    battle_log: &mut BattleLog,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        push_battle_log(
            battle_log,
            format!("{}: {}", t("支援就绪", "Support ready"), power.label()),
            None,
        );
    } else if power.is_superweapon() {
        push_battle_log(
            battle_log,
            format!(
                "{}: {}",
                t("敌方超级武器就绪", "Enemy superweapon ready"),
                power.label()
            ),
            None,
        );
    }
}

pub(crate) fn selected_query_has_owned_voice_unit(
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    team: Team,
) -> bool {
    selected_units
        .iter()
        .any(|(_, unit, unit_team, ..)| *unit_team == team && is_voice_unit(unit))
}

pub(crate) fn spawn_fog_memory_structure_remnant(
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

pub(crate) fn support_power_button(kind: SupportPowerKind) -> impl Bundle {
    (
        Button,
        SupportPowerButton { kind },
        Node {
            display: Display::None,
            position_type: PositionType::Relative,
            width: px(SUPPORT_POWER_BUTTON_SIZE_PX),
            height: px(SUPPORT_POWER_BUTTON_SIZE_PX),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(px(0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.32, 0.42, 0.46)),
        BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.9)),
        Visibility::Hidden,
    )
}

#[allow(dead_code)]
pub(crate) fn place_structure_at(
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

pub(crate) fn place_structure_at_for_faction(
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
    let free_worker_origin = if id == "Refinery" {
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
        free_worker_origin,
        rotation_y_radians,
        visible_team,
        faction,
    );
    Ok((entity, id))
}

pub(crate) fn update_pending_structure_placement_pointer(
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

pub(crate) fn finish_pending_structure_drag(pending: &mut PendingStructurePlacement) {
    pending.drag_rotation_origin = None;
}

pub(crate) fn rotate_pending_structure_drag_towards(
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

pub(crate) fn structure_drag_rotation_y(origin: Vec3, target: Vec3) -> Option<f32> {
    let delta = Vec2::new(target.x - origin.x, target.z - origin.z);
    if delta.length() < STRUCTURE_PLACEMENT_ROTATION_DEAD_ZONE_M {
        return None;
    }
    Some(normalize_structure_rotation_y(delta.x.atan2(delta.y)))
}

pub(crate) fn normalize_structure_rotation_y(rotation_y_radians: f32) -> f32 {
    let normalized = rotation_y_radians.rem_euclid(std::f32::consts::TAU);
    if normalized < 0.0001 || (std::f32::consts::TAU - normalized) < 0.0001 {
        0.0
    } else {
        normalized
    }
}

pub(crate) fn nearest_base_construction_anchor(
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

pub(crate) fn structure_placement_collides(
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

pub(crate) fn focus_latest_battle_event(
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

pub(crate) fn disarm_support_power_on_left_click(
    command_mode: &mut CommandMode,
    mouse: &ButtonInput<MouseButton>,
    cursor_over_hud: bool,
) -> bool {
    if mouse.just_pressed(MouseButton::Left)
        && command_mode.support_power.is_some()
        && !cursor_over_hud
    {
        command_mode.support_power = None;
        return true;
    }
    false
}

pub(crate) fn window_size(window: &Window) -> Vec2 {
    Vec2::new(window.width(), window.height())
}

pub(crate) fn support_power_target_snapshots(
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

pub(crate) fn nearest_resource_order_target(
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
    let screen_pick_max_radius = if favor_resource_collectors {
        RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX
    } else {
        RESOURCE_ORDER_SCREEN_PICK_MAX_RADIUS_PX
    };
    // No ground-snap fallback: you must put the cursor on the ore to harvest;
    // clicking anywhere else is a plain move.
    let _ = point;
    resource_target_at_cursor(cursor, camera_q, resources, screen_pick_max_radius)
}

/// Shortest screen-space distance from `p` to the segment `a`..`b`.
pub(crate) fn point_to_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

pub(crate) fn selectable_cursor_pick_distance(
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

pub(crate) fn apply_infiltration_on_capture(
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

pub(crate) fn infiltration_steal_amount(available: i32, ratio: f32, cap: i32) -> i32 {
    if available <= 0 || ratio <= 0.0 || cap <= 0 {
        return 0;
    }
    cap.min(available)
        .min(INFILTRATION_RESOURCE_STEAL_MIN.max(((available as f32) * ratio).ceil() as i32))
}

pub(crate) fn can_unit_construct_structures(unit: &Unit) -> bool {
    unit.id == "Worker"
}

pub(crate) fn activate_support_power(
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
            format!("{}: {}", t("支援已使用", "Support used"), power.label()),
            BattleEventPingKind::SupportPower,
        )
    } else if matches!(
        power,
        SupportPowerKind::StrategicMissile | SupportPowerKind::WeatherStorm
    ) {
        (
            format!(
                "{}: {}",
                t("敌方超级武器", "Enemy superweapon"),
                power.label()
            ),
            BattleEventPingKind::EnemySuperweapon,
        )
    } else {
        (
            format!("{}: {}", t("敌方支援", "Enemy support"), power.label()),
            BattleEventPingKind::EnemySupportPower,
        )
    };
    push_battle_log_with_kind(battle_log, message, Some(target), ping_kind);
    support_cooldowns.set(team, power, def.cooldown);
    true
}

pub(crate) fn update_match_menu_overlay(
    match_menu: Res<MatchMenuState>,
    match_speed: Res<MatchSpeed>,
    selected_map: Res<SelectedSkirmishMap>,
    match_state: Res<MatchState>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut overlay_q: Query<&mut Visibility, With<MatchMenuOverlay>>,
    mut status_q: Query<&mut Text, (With<MatchMenuStatusText>, Without<MatchMenuFullscreenText>)>,
    mut fullscreen_text_q: Query<
        &mut Text,
        (With<MatchMenuFullscreenText>, Without<MatchMenuStatusText>),
    >,
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

    let fullscreen = window_q.single().ok().is_some_and(window_is_fullscreen);
    for mut text in &mut fullscreen_text_q {
        **text = match_menu_fullscreen_button_text(fullscreen).to_string();
    }

    let minutes = (match_state.start_time_sec.max(0.0) / 60.0).floor() as u32;
    let seconds = (match_state.start_time_sec.max(0.0) as u32) % 60;
    let economy = economies.get(visible_player.team);
    let perspective_label = if visible_player.is_spectator() {
        t("观战视角", "Spectating")
    } else {
        t("阵营", "Team")
    };
    for mut text in &mut status_q {
        **text = format!(
            "{}: {}\n{}: {}  {}: {:02}:{:02}\n{}: {}\n{} {}  {} {}  {} {}/{}",
            t("地图", "Map"),
            localized_skirmish_map_name(selected_map.definition()),
            perspective_label,
            visible_player.team.label(),
            t("用时", "Time"),
            minutes,
            seconds,
            t("速度", "Speed"),
            match_speed.preset.label(),
            ResourceKind::Ore.label(),
            economy.ore,
            ResourceKind::Crystal.label(),
            economy.crystal,
            t("电力", "Power"),
            economy.power_capacity,
            economy.power_used
        );
    }
}

pub(crate) fn support_hotkey_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::AltLeft)
        || keyboard.pressed(KeyCode::AltRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
}

pub(crate) fn player_support_power_available(
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

pub(crate) fn support_power_unlocked(
    team: Team,
    power: SupportPowerKind,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    support_requirements_met(team, power.definition().requirements, structures)
}

pub(crate) fn visible_support_power_count(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> usize {
    SupportPowerKind::ALL
        .into_iter()
        .filter(|power| support_power_unlocked(team, *power, structures))
        .count()
}

pub(crate) fn support_power_button_state(
    power: SupportPowerKind,
    unlocked: bool,
    low_power: bool,
    cooldown_remaining: f32,
    active: bool,
) -> SupportPowerButtonState {
    let cooldown_seconds = (cooldown_remaining > 0.0).then_some(cooldown_remaining.ceil() as u32);
    let enabled = unlocked
        && (!power.definition().requires_power || !low_power)
        && cooldown_seconds.is_none();
    let badge_text = if unlocked {
        cooldown_seconds.map_or_else(String::new, |seconds| seconds.to_string())
    } else {
        String::new()
    };
    SupportPowerButtonState {
        enabled,
        unlocked,
        active: active && enabled,
        low_power,
        cooldown_seconds,
        badge_text,
    }
}

pub(crate) fn support_power_missing_requirement_labels(
    team: Team,
    requirements: &[&'static str],
    structures: &Query<StructurePrereqItem<'_>>,
) -> Vec<String> {
    requirements
        .iter()
        .filter(|requirement| !team_has_constructed_structure(team, requirement, structures))
        .map(|requirement| localized_compact_entity_label(requirement))
        .collect()
}

pub(crate) fn support_power_requirement_text(requirements: &[&'static str]) -> String {
    if requirements.is_empty() {
        return t("无", "None").to_string();
    }
    requirements
        .iter()
        .map(|requirement| localized_compact_entity_label(requirement))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn support_power_tooltip(
    power: SupportPowerKind,
    state: &SupportPowerButtonState,
    missing_requirements: &[String],
) -> String {
    let def = power.definition();
    let mut lines = vec![
        format!("{}  {}", power.hotkey_label(), power.label()),
        format!("{}: {:.0}s", t("冷却", "Cooldown"), def.cooldown),
        format!(
            "{}: {}",
            t("需求", "Requires"),
            support_power_requirement_text(def.requirements)
        ),
        format!("{}: {:.1}", t("半径", "Radius"), def.radius),
    ];
    if def.damage > 0.0 {
        lines.push(format!("{}: {:.0}", t("伤害", "Damage"), def.damage));
    }
    if def.healing > 0.0 {
        lines.push(format!("{}: {:.0}", t("治疗", "Healing"), def.healing));
    }
    if def.duration > 0.0 {
        lines.push(format!("{}: {:.0}s", t("持续", "Duration"), def.duration));
    }
    if def.impact_delay > 0.0 {
        lines.push(format!(
            "{}: {:.1}s",
            t("落点延迟", "Impact Delay"),
            def.impact_delay
        ));
    }
    if !missing_requirements.is_empty() {
        lines.push(format!(
            "{}: {}",
            t("缺少科技", "Missing tech"),
            missing_requirements.join(", ")
        ));
    } else if state.low_power && def.requires_power {
        lines.push(t("低电力: 支援离线", "Low power: support offline").to_string());
    } else if let Some(seconds) = state.cooldown_seconds {
        lines.push(format!("{}: {seconds}s", t("冷却中", "Cooling down")));
    } else if state.active {
        lines.push(t("选择目标位置", "Choose a target position").to_string());
    } else if state.enabled {
        lines.push(t("就绪: 点击后选择目标", "Ready: click then choose a target").to_string());
    } else {
        lines.push(t("不可用", "Unavailable").to_string());
    }
    lines.join("\n")
}

pub(crate) fn support_power_button_colors(
    state: &SupportPowerButtonState,
    interaction: Interaction,
) -> (Color, Color) {
    if state.active {
        return (
            Color::srgba(0.18, 0.14, 0.04, 0.97),
            Color::srgb(0.96, 0.72, 0.24),
        );
    }
    if !state.unlocked {
        return (
            Color::srgba(0.045, 0.052, 0.056, 0.72),
            Color::srgb(0.58, 0.34, 0.18),
        );
    }
    if !state.enabled {
        return (
            Color::srgba(0.045, 0.052, 0.06, 0.74),
            Color::srgb(0.22, 0.28, 0.31),
        );
    }
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.14, 0.24, 0.28, 0.97),
            Color::srgb(0.42, 0.72, 0.76),
        ),
        Interaction::Hovered => (
            Color::srgba(0.08, 0.12, 0.14, 0.94),
            Color::srgb(0.34, 0.58, 0.60),
        ),
        Interaction::None => (
            Color::srgba(0.035, 0.045, 0.055, 0.9),
            Color::srgb(0.32, 0.42, 0.46),
        ),
    }
}

pub(crate) fn support_power_badge_color(state: &SupportPowerButtonState) -> TextColor {
    if !state.unlocked {
        TextColor(Color::srgb(1.0, 0.56, 0.24))
    } else if state.cooldown_seconds.is_some() {
        TextColor(Color::srgb(0.98, 0.84, 0.42))
    } else if state.low_power {
        TextColor(Color::srgb(1.0, 0.42, 0.32))
    } else {
        TextColor(Color::srgb(0.98, 0.84, 0.42))
    }
}

pub(crate) fn support_power_hotkey_color(state: &SupportPowerButtonState) -> TextColor {
    if state.enabled || state.active {
        TextColor(Color::srgb(0.78, 0.96, 0.92))
    } else {
        TextColor(Color::srgba(0.56, 0.68, 0.68, 0.78))
    }
}

pub(crate) fn refresh_support_power_panel(
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    mut command_mode: ResMut<CommandMode>,
    mut panel_state: ResMut<SupportPowerPanelState>,
    structures: Query<StructurePrereqItem<'_>>,
    mut panel_q: Query<
        &mut Visibility,
        (
            With<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerCooldownLabel>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut buttons: Query<
        (
            &SupportPowerButton,
            &Interaction,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerCooldownLabel>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut cooldown_labels: Query<
        (&SupportPowerCooldownLabel, &mut Text, &mut TextColor),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut hotkey_labels: Query<
        (&SupportPowerHotkeyLabel, &mut TextColor),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerCooldownLabel>,
        ),
    >,
) {
    let Some(team) = controlled_player_team(Some(&*visible_player)) else {
        panel_state.visible_count = 0;
        command_mode.support_power = None;
        for mut visibility in &mut panel_q {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    if command_mode
        .support_power
        .is_some_and(|power| !support_power_unlocked(team, power, &structures))
    {
        command_mode.support_power = None;
    }

    let low_power = economies.get(team).low_power();
    let mut visible_count = 0usize;
    for (button, interaction, mut node, mut button_visibility, mut background, mut border) in
        &mut buttons
    {
        let unlocked = support_power_unlocked(team, button.kind, &structures);
        if unlocked {
            visible_count += 1;
            node.display = Display::Flex;
            *button_visibility = Visibility::Inherited;
        } else {
            node.display = Display::None;
            *button_visibility = Visibility::Hidden;
        }
        let state = support_power_button_state(
            button.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, button.kind),
            command_mode.support_power == Some(button.kind),
        );
        let (bg, border_color) = support_power_button_colors(&state, *interaction);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
    panel_state.visible_count = visible_count;
    debug_assert_eq!(
        visible_count,
        visible_support_power_count(team, &structures)
    );
    for mut visibility in &mut panel_q {
        *visibility = if visible_count > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (label, mut text, mut text_color) in &mut cooldown_labels {
        let unlocked = support_power_unlocked(team, label.kind, &structures);
        let state = support_power_button_state(
            label.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, label.kind),
            command_mode.support_power == Some(label.kind),
        );
        **text = state.badge_text.clone();
        *text_color = support_power_badge_color(&state);
    }
    for (label, mut text_color) in &mut hotkey_labels {
        let unlocked = support_power_unlocked(team, label.kind, &structures);
        let state = support_power_button_state(
            label.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, label.kind),
            command_mode.support_power == Some(label.kind),
        );
        *text_color = support_power_hotkey_color(&state);
    }
}

pub(crate) fn support_power_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    structures: Query<StructurePrereqItem<'_>>,
    mut command_mode: ResMut<CommandMode>,
    buttons: Query<(&Interaction, &SupportPowerButton)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(team) = controlled_player_team(Some(&*visible_player)) else {
        command_mode.support_power = None;
        return;
    };
    let Some((_, button)) = buttons
        .iter()
        .filter(|(interaction, _)| **interaction == Interaction::Pressed)
        .min_by_key(|(_, button)| button.kind.idx())
    else {
        return;
    };
    if player_support_power_available(
        team,
        button.kind,
        &economies,
        &support_cooldowns,
        &structures,
    ) {
        toggle_support_power_mode(&mut command_mode, button.kind);
    } else if command_mode.support_power == Some(button.kind) {
        command_mode.support_power = None;
    }
}

pub(crate) fn toggle_support_power_mode(
    command_mode: &mut CommandMode,
    power: SupportPowerKind,
) -> bool {
    let enabled = command_mode.support_power != Some(power);
    clear_targeting_modes(command_mode);
    if enabled {
        command_mode.support_power = Some(power);
    }
    enabled
}

pub(crate) fn push_action_unique(actions: &mut Vec<BuildAction>, action: BuildAction) {
    if !actions.contains(&action) {
        actions.push(action);
    }
}

pub(crate) fn sorted_worker_build_structures(
    faction: &'static registry::FactionDef,
) -> Vec<&'static str> {
    let mut structures = faction
        .structures
        .iter()
        .copied()
        .filter(|id| registry::entity(id).is_some())
        .collect::<Vec<_>>();
    structures.sort_by(build_structure_order_compare);
    structures
}

pub(crate) fn signed_number(value: i32) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

pub(crate) fn missing_requirement_labels(
    entity: &registry::EntityDef,
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Vec<String> {
    entity
        .requirements
        .iter()
        .filter(|requirement| !team_has_constructed_structure(team, requirement, structures))
        .map(|requirement| localized_compact_entity_label(requirement))
        .collect()
}

pub(crate) fn team_has_constructed_structure(
    team: Team,
    requirement: &str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    structures
        .iter()
        .any(|(structure, structure_team, _, under_construction)| {
            structure_is_constructed(under_construction)
                && *structure_team == team
                && structure.id == requirement
        })
}

pub(crate) fn compact_label(label: &str) -> String {
    let mut value = label.to_string();
    if current_language() == Language::En {
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
    }
    if value.chars().count() > 18 {
        let suffix = if current_language() == Language::Zh {
            "..."
        } else {
            "."
        };
        value.chars().take(17).collect::<String>() + suffix
    } else {
        value
    }
}

pub(crate) fn is_production_structure_hotkey_candidate(
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

pub(crate) fn is_army_selection_unit(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some() && !is_economy_worker_selection_unit(unit)
}

pub(crate) fn is_visible_army_selection_candidate(
    team: Team,
    visible_team: Team,
    unit: &Unit,
    visibility: &VisibilityState,
) -> bool {
    team == visible_team && visibility.visible && is_army_selection_unit(unit)
}

pub(crate) fn is_builder_worker_selection_unit(unit: &Unit) -> bool {
    unit.id == "Worker"
}

pub(crate) fn is_exact_current_selection(current: &[Entity], target: &[Entity]) -> bool {
    if target.is_empty() || current.len() != target.len() {
        return false;
    }

    current.iter().all(|entity| target.contains(entity))
}

pub(crate) fn record_control_group_assigned_battle_log(
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
        format!(
            "{} {} {}: {} {}",
            t("编组", "Group"),
            index + 1,
            t("已设置", "set"),
            count,
            t("个单位", "units")
        ),
        focus,
    );
}

pub(crate) fn record_control_group_cleared_battle_log(battle_log: &mut BattleLog, index: usize) {
    push_battle_log(
        battle_log,
        format!(
            "{} {} {}",
            t("编组", "Group"),
            index + 1,
            t("已清空", "cleared")
        ),
        None,
    );
}

pub(crate) fn select_production_structures_for_hotkey(
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

pub(crate) fn structure_sell_refund(def: &registry::EntityDef, health: &Health) -> (i32, i32) {
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

pub(crate) fn cancel_jobs_for_producer(
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

pub(crate) fn cancellation_producers_for_action(
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
        BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::SelectIdleWorker
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::SelectBuildTab(_)
        | BuildAction::None => Vec::new(),
    }
}

pub(crate) fn queue_button_state_for_product(
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

pub(crate) fn queue_button_badge_text(state: QueueButtonState) -> String {
    if state.full {
        t(" [满]", " [Full]").to_string()
    } else {
        format!(" [x{}]", state.count)
    }
}

pub(crate) fn repair_ai_damaged_structures(
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

pub(crate) fn update_ai_siege_drill_deploy_mode(
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

pub(crate) fn find_production_spawn_position(
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

pub(crate) fn record_production_blocked_once(
    team: Team,
    player_team: Team,
    timer_before: f32,
    focus: Vec3,
    audio_feedback: &mut AudioFeedback,
    battle_log: &mut BattleLog,
) {
    if team == player_team && timer_before > 0.0 {
        record_sound_audio_feedback(audio_feedback, SoundEffectKind::Error);
        push_battle_log(
            battle_log,
            t(
                "生产受阻: 出厂口被堵塞",
                "Production blocked: exit obstructed",
            ),
            Some(focus),
        );
    }
}

pub(crate) fn record_production_ready_battle_log(
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
        t("建筑就绪", "Building ready")
    } else {
        t("单位就绪", "Unit ready")
    };
    push_battle_log(battle_log, format!("{prefix}: {label}"), Some(focus));
}

#[cfg(test)]
pub(crate) fn team_build_queue_len(build_queue: &BuildQueue, team: Team) -> usize {
    build_queue.0.iter().filter(|job| job.team == team).count()
}

pub(crate) fn producer_build_queue_len(build_queue: &BuildQueue, producer_entity: Entity) -> usize {
    build_queue
        .0
        .iter()
        .filter(|job| job.producer_entity == producer_entity)
        .count()
}

pub(crate) fn refund_build_job_cost(job: &BuildJob, economies: &mut Economies) -> bool {
    let Some(def) = registry::entity(build_target_product(job.action)) else {
        return false;
    };
    economies.get_mut(job.team).refund(def.cost);
    true
}

pub(crate) fn has_producer_for_job(
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
        BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::SelectIdleWorker
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::StopSelected
        | BuildAction::ScatterSelected
        | BuildAction::SelectBuildTab(_)
        | BuildAction::None => false,
    }
}

pub(crate) fn requirements_met(
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

pub(crate) fn support_requirements_met(
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

pub(crate) fn auto_assign_ai_construction_workers(
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

pub(crate) fn closest_construction_assignment(
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

pub(crate) fn auto_assign_ai_supply_crate_collectors(
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

pub(crate) fn update_ai_drone_scouting(
    mut commands: Commands,
    time: Res<Time>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    relations: Res<TeamRelations>,
    mut drones: Query<
        (
            Entity,
            &Team,
            &Unit,
            &Health,
            Option<&OrderQueue>,
            Option<&mut AiDroneScout>,
        ),
        (With<Unit>, IdleUnitOrderFilter, Without<Selected>),
    >,
    targets: Query<(Entity, &Team, &Transform, &Health, &Unit), With<Unit>>,
) {
    let controlled_team = controlled_player_team(visible_player.as_deref());
    let delta = time.delta_secs();
    for (drone_entity, drone_team, drone_unit, drone_health, order_queue, scout_state) in
        &mut drones
    {
        if drone_unit.id != "Drone"
            || drone_health.current <= 0.0
            || controlled_team == Some(*drone_team)
            || !team_is_active(*drone_team, active_teams.as_deref())
            || order_queue.is_some_and(|queue| !queue.orders.is_empty())
        {
            continue;
        }

        let last_target = scout_state.as_ref().and_then(|state| state.last_target);
        let Some((target, target_position)) = choose_ai_drone_scout_target(
            *drone_team,
            drone_entity,
            last_target,
            &relations,
            &targets,
        ) else {
            continue;
        };

        if let Some(mut state) = scout_state {
            state.cooldown_remaining -= delta;
            if state.cooldown_remaining > 0.0 {
                continue;
            }
            state.last_target = Some(target);
            state.cooldown_remaining = ai_drone_scout_delay(drone_entity, target);
        } else {
            commands.entity(drone_entity).try_insert(AiDroneScout {
                last_target: Some(target),
                cooldown_remaining: ai_drone_scout_delay(drone_entity, target),
            });
        }

        issue_unit_order(
            &mut commands,
            drone_entity,
            UnitQueuedOrder::Move(target_position),
        );
    }
}

pub(crate) fn choose_ai_drone_scout_target(
    drone_team: Team,
    drone_entity: Entity,
    last_target: Option<Entity>,
    relations: &TeamRelations,
    targets: &Query<(Entity, &Team, &Transform, &Health, &Unit), With<Unit>>,
) -> Option<(Entity, Vec3)> {
    let mut best_new_target = None;
    let mut best_new_score = u64::MAX;
    let mut best_any_target = None;
    let mut best_any_score = u64::MAX;
    for (target_entity, target_team, target_transform, target_health, target_unit) in targets {
        if target_health.current <= 0.0
            || target_unit.speed <= 0.0
            || !relations.are_enemies(drone_team, *target_team)
        {
            continue;
        }
        let score = entity_pair_hash(drone_entity, target_entity);
        if score < best_any_score {
            best_any_score = score;
            best_any_target = Some((target_entity, target_transform.translation));
        }
        if Some(target_entity) != last_target && score < best_new_score {
            best_new_score = score;
            best_new_target = Some((target_entity, target_transform.translation));
        }
    }
    best_new_target.or(best_any_target)
}

pub(crate) fn entity_pair_hash(a: Entity, b: Entity) -> u64 {
    let mut x = a.to_bits().wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ b.to_bits().wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

pub(crate) fn update_ai_tech_bunker_garrisons(
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

pub(crate) fn garrison_ai_tech_bunkers(
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

pub(crate) fn best_available_ai_garrison_unit(
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

pub(crate) fn try_activate_ai_support_power(
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

pub(crate) fn support_power_available_for_audio(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

pub(crate) fn best_repair_swarm_position(
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

pub(crate) fn best_mobile_unit_cluster_position(
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

pub(crate) fn best_shield_overdrive_position(
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

pub(crate) fn shield_target_score(unit: &Unit) -> f32 {
    let mut score = 1.0;
    if registry::entity(unit.id).is_some_and(|def| def.weapon.is_some()) {
        score += 1.0;
    }
    score
}

pub(crate) fn best_scored_strike_position(
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

pub(crate) fn strike_score_at_position(
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

pub(crate) fn any_enemy_support_target_position(
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

pub(crate) fn assign_ai_attack_wave_order(
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

pub(crate) fn restore_ai_attack_wave_orders(
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

pub(crate) fn next_ai_economy_train(
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

    None
}

pub(crate) fn try_spawn_ai_trained_unit(
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

pub(crate) fn run_ai_capture_logic(
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

pub(crate) fn run_ai_saboteur_logic(
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

pub(crate) fn best_ai_saboteur_target(
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

pub(crate) fn best_ai_capture_target(
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

pub(crate) fn active_ai_teams(
    controlled_team: Option<Team>,
    active_teams: Option<&ActiveTeams>,
) -> impl Iterator<Item = Team> + '_ {
    let team_count = active_teams.map(|active| active.0.len()).unwrap_or(0);
    player_teams(team_count)
        .filter(move |team| Some(*team) != controlled_team)
        .filter(move |team| team_is_active(*team, active_teams))
}

pub(crate) fn team_is_active(team: Team, active_teams: Option<&ActiveTeams>) -> bool {
    let Some(index) = team.economy_index() else {
        return false;
    };
    active_teams.map_or(true, |active| active.0.get(index).copied().unwrap_or(false))
}

pub(crate) fn active_match_perspectives(active_teams: &ActiveTeams) -> Vec<Team> {
    player_teams(active_teams.0.len())
        .filter(|team| team_is_active(*team, Some(active_teams)))
        .collect()
}

pub(crate) fn spectator_perspective_switch_enabled(
    visible_player: &VisiblePlayer,
    active_teams: &ActiveTeams,
) -> bool {
    visible_player.is_spectator() && active_match_perspectives(active_teams).len() > 1
}

pub(crate) fn cycle_spectator_visible_player(
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

pub(crate) fn visible_player_team(visible_player: Option<&VisiblePlayer>) -> Team {
    visible_player.map_or(Team::Player(0), |visible| visible.team)
}

pub(crate) fn controlled_player_team(visible_player: Option<&VisiblePlayer>) -> Option<Team> {
    match visible_player {
        Some(visible) if visible.is_spectator() => None,
        Some(visible) => Some(visible.team),
        None => Some(Team::Player(0)),
    }
}

pub(crate) fn next_ai_train(
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

pub(crate) fn next_ai_train_matching(
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
        if !ai_economy_candidate_allowed(candidate, profile, counts) {
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

pub(crate) fn has_constructed_structure(
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

#[allow(dead_code)]
pub(crate) fn next_ai_economy_structure(
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

pub(crate) fn next_ai_economy_structure_for_faction(
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
pub(crate) fn next_ai_offense_structure(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_offense_structure_for_faction(team, SkirmishFaction::from_team(team), structures)
}

pub(crate) fn next_ai_offense_structure_for_faction(
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

#[allow(dead_code)]
pub(crate) fn next_ai_defense(
    team: Team,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_defense_for_faction(team, SkirmishFaction::from_team(team), profile, structures)
}

pub(crate) fn next_ai_defense_for_faction(
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

pub(crate) fn team_home(team: Team) -> Vec3 {
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

/// Degrees/second a defense tower slowly sweeps while scanning for targets.
pub(crate) const IDLE_TOWER_SCAN_DEG_PER_SEC: f32 = 45.0;

#[derive(Clone)]
pub(crate) struct SelectionHudItem {
    pub(crate) label: String,
    pub(crate) team: Team,
    pub(crate) health_current: f32,
    pub(crate) health_max: f32,
    pub(crate) attack: Option<(f32, f32)>,
    pub(crate) rank: u8,
    pub(crate) garrison: Option<(usize, usize)>,
    pub(crate) cargo: Option<(i32, i32, i32, i32)>,
}

/// Marks an entity whose model children have been recentered onto its origin.
#[derive(Component)]
pub(crate) struct ModelRecentered;

/// Counts frames a model has had meshes present, so the recenter waits a short
/// settle window (all parts loaded) before correcting once. Frame-based rather
/// than mesh-count-based because animated models' mesh counts jitter and never
/// "stabilize".
#[derive(Component)]
pub(crate) struct ModelRecenterTracking {
    pub(crate) frames: u8,
}

/// Frames a model must have meshes present before we recenter it (≈0.2s @30fps) —
/// long enough for all GLB parts to spawn, short enough that freshly-trained units
/// snap into alignment quickly.
pub(crate) const MODEL_RECENTER_SETTLE_FRAMES: u8 = 6;

/// Recenters each selectable entity's loaded model so its visible geometry's
/// horizontal center coincides with the entity `Transform.translation` — the
/// point gizmos (selection/hover rings) and every cursor hit-test project.
///
/// Root cause this fixes: the GLB scenes (and the migrated `render_parts`
/// offsets, e.g. a turret part at [-2,0,-1.5]) place geometry off the entity
/// origin, so the *visible* model rendered far from where clicks were judged —
/// left/right-clicking the model selected/targeted nothing. Runs once per entity,
/// after its scene meshes have spawned (their `Aabb`s exist).
pub(crate) fn recenter_entity_models(
    mut commands: Commands,
    roots: Query<
        (Entity, &GlobalTransform, Option<&ModelRecenterTracking>),
        (With<Selectable>, Without<ModelRecentered>),
    >,
    children_q: Query<&Children>,
    aabb_q: Query<(&GlobalTransform, &Aabb)>,
    mut model_tf: Query<&mut Transform, With<WorldAssetRoot>>,
) {
    for (root, root_gt, tracking) in &roots {
        // Combined world-space AABB of the GLB model meshes + how many meshes.
        // Measure ONLY the WorldAssetRoot (GLB) subtrees — the same children the
        // shift below moves. The faction identity banner is a procedural child of
        // the root sitting forward at -radius*0.72 in Z; including it pulled the
        // measured center toward the flag, so the building (and its selection
        // brackets) ended up offset from the entity origin.
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut count: u32 = 0;
        let model_roots: Vec<Entity> = children_q
            .get(root)
            .map(|c| c.iter().filter(|e| model_tf.contains(*e)).collect())
            .unwrap_or_default();
        if model_roots.is_empty() {
            // Procedural-only model (authored at the origin) — nothing to shift.
            commands.entity(root).insert(ModelRecentered);
            commands.entity(root).remove::<ModelRecenterTracking>();
            continue;
        }
        let mut stack: Vec<Entity> = model_roots;
        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter());
            }
            if let Ok((gt, aabb)) = aabb_q.get(entity) {
                count += 1;
                let center = Vec3::from(aabb.center);
                let half = Vec3::from(aabb.half_extents);
                for sx in [-1.0_f32, 1.0] {
                    for sy in [-1.0_f32, 1.0] {
                        for sz in [-1.0_f32, 1.0] {
                            let corner = center + Vec3::new(sx * half.x, sy * half.y, sz * half.z);
                            let world = gt.transform_point(corner);
                            min = min.min(world);
                            max = max.max(world);
                        }
                    }
                }
            }
        }
        if count == 0 {
            // Scene meshes not spawned yet; try again next frame.
            continue;
        }
        // Wait a short settle window after meshes first appear, then correct ONCE.
        // (Applying on first sight left late-loading multi-part models misaligned;
        // re-applying every frame diverged because GlobalTransform lags a frame;
        // gating on mesh-count stability failed for animated models whose count
        // jitters and never settles.)
        let frames = tracking.map(|t| t.frames).unwrap_or(0).saturating_add(1);
        if frames < MODEL_RECENTER_SETTLE_FRAMES {
            commands
                .entity(root)
                .insert(ModelRecenterTracking { frames });
            continue;
        }
        let visual_center = (min + max) * 0.5;
        let (scale, rotation, translation) = root_gt.to_scale_rotation_translation();
        let scale = scale.x.abs().max(1e-3);
        // World shift to move the visible center onto the entity origin (XZ only —
        // keep models sitting on the ground), converted into the root's LOCAL frame
        // (children's Transforms are parent-local). The root may be rotated (units
        // face their movement direction), so undo its rotation AND scale — using
        // only `/scale` left rotated units (workers) misaligned.
        let world_delta = Vec3::new(
            translation.x - visual_center.x,
            0.0,
            translation.z - visual_center.z,
        );
        let local_delta = rotation.inverse() * (world_delta / scale);
        if let Ok(children) = children_q.get(root) {
            for child in children.iter() {
                if let Ok(mut transform) = model_tf.get_mut(child) {
                    transform.translation.x += local_delta.x;
                    transform.translation.z += local_delta.z;
                }
            }
        }
        commands.entity(root).insert(ModelRecentered);
        commands.entity(root).remove::<ModelRecenterTracking>();
    }
}

pub(crate) fn apply_hunyuan_model_materials(
    mut commands: Commands,
    mut cache: ResMut<HunyuanModelMaterialCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    roots: Query<(Entity, &HunyuanModelPart), Without<HunyuanModelMaterialized>>,
    children_q: Query<&Children>,
    mut material_q: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    for (root, part) in &roots {
        let handle = cache.handle_for(part.entity_id, &mut materials);
        let mut applied = false;
        let mut stack: Vec<Entity> = children_q
            .get(root)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter());
            }
            if let Ok(mut material) = material_q.get_mut(entity) {
                material.0 = handle.clone();
                applied = true;
            }
        }
        if applied {
            commands.entity(root).insert(HunyuanModelMaterialized);
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct OverlayVfxQueries<'w, 's> {
    pub(crate) destruction: Query<'w, 's, (&'static Transform, &'static StructureDestructionVfx)>,
    pub(crate) promotion: Query<'w, 's, (&'static Transform, &'static VeterancyPromotionEffect)>,
    pub(crate) impacts: Query<'w, 's, (&'static Transform, &'static ImpactBurst)>,
    pub(crate) camera: Query<'w, 's, &'static GlobalTransform, With<MainCamera>>,
    pub(crate) time: Res<'w, Time>,
}

pub(crate) fn draw_world_overlays(
    mut gizmos: Gizmos,
    mut hud: Gizmos<HudGizmos>,
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
            Option<&HarvestOrder>,
            Option<&ResourceCargo>,
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
            Option<&HarvestOrder>,
            Option<&ResourceCargo>,
        ),
        Without<Selected>,
    >,
    pulses: Query<&ShotPulse>,
    warnings: Query<(&Transform, &SupportWarning)>,
    reveals: Query<(&Transform, &TemporarySupportReveal)>,
    orbital_strikes: Query<(&Transform, &PendingOrbitalStrike)>,
    click_markers: Query<(&Transform, &ClickMarker)>,
    vfx: OverlayVfxQueries,
    resources: Query<(
        Entity,
        &Transform,
        &Selectable,
        &ResourceNode,
        &VisibilityState,
    )>,
    supply_crates: Query<(&Transform, &Selectable, &SupplyCrate, &VisibilityState)>,
    hovered_resource: Res<HoveredResource>,
    visible_player: Res<VisiblePlayer>,
    player_colors: Res<PlayerColorSlots>,
    placement_preview: StructurePlacementPreviewParams,
) {
    let visible_team = visible_player.team;
    // Camera's horizontal right axis, so health bars draw screen-horizontal.
    let bar_right = vfx
        .camera
        .single()
        .ok()
        .map(|gt| {
            let r = gt.right();
            Vec3::new(r.x, 0.0, r.z).normalize_or(Vec3::X)
        })
        .unwrap_or(Vec3::X);

    // Highlight the resource under the cursor so the player knows a left/right
    // click will hit it (it sits exactly where the click is judged).
    if let Some(hovered) = hovered_resource.0
        && let Ok((_, transform, _, resource, _)) = resources.get(hovered)
    {
        let base = transform.translation;
        let half_width = resource_visual_half_width(resource.kind);
        let color = Color::srgb(1.0, 0.85, 0.25);
        for (height, scale) in [
            (0.06_f32, 1.15_f32),
            (resource_visual_height(resource.kind), 0.7),
        ] {
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(base.x, base.y + height, base.z),
                    Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
                ),
                half_width * scale,
                color,
            );
        }
    }

    // A faint battlefield grid so the ground reads with scale instead of as a
    // flat void — a baseline RTS readability cue.
    let cells = (MAP_HALF_EXTENT as u32) * 2;
    gizmos.grid(
        Isometry3d::new(
            Vec3::new(0.0, 0.02, 0.0),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        UVec2::new(cells, cells),
        Vec2::splat(1.0),
        Color::srgba(0.30, 0.38, 0.40, 0.14),
    );

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
        harvest_order,
        cargo,
    ) in &selected
    {
        let selected_color = Color::srgb(0.62, 0.95, 0.64);
        if structure.is_some() && unit.is_none() {
            draw_structure_selection_brackets(
                &mut gizmos,
                transform.translation,
                selectable.radius,
                selected_color,
            );
        } else {
            draw_ring(
                &mut gizmos,
                transform.translation,
                selectable.radius + 0.18,
                selected_color,
            );
        }
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
            &mut hud,
            transform.translation,
            selectable.radius,
            *health,
            bar_right,
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
        draw_harvest_and_cargo_visuals(
            &mut gizmos,
            &mut hud,
            transform.translation,
            selectable.radius,
            harvest_order,
            cargo,
            &resources,
            vfx.time.elapsed_secs(),
            bar_right,
        );
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
    for (transform, selectable, team, health, unit, structure, harvest_order, cargo) in &all {
        draw_harvest_and_cargo_visuals(
            &mut gizmos,
            &mut hud,
            transform.translation,
            selectable.radius,
            harvest_order,
            cargo,
            &resources,
            vfx.time.elapsed_secs(),
            bar_right,
        );
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
                &mut hud,
                transform.translation,
                selectable.radius,
                *health,
                bar_right,
            );
        }
    }
    for pulse in &pulses {
        // Brighten the team color toward a hot muzzle/tracer hue so shots read as
        // attacks, and draw it thick so a brief tracer is actually noticeable.
        let base = player_colors.color(pulse.team).to_srgba();
        let tracer = Color::srgb(
            (base.red * 0.4 + 0.85).min(1.0),
            (base.green * 0.4 + 0.78).min(1.0),
            (base.blue * 0.4 + 0.30).min(1.0),
        );
        hud.line(pulse.from, pulse.to, tracer);
    }
    for (transform, burst) in &vfx.impacts {
        draw_impact_burst(
            &mut gizmos,
            &mut hud,
            transform.translation,
            burst,
            &player_colors,
        );
    }
    for (transform, marker) in &click_markers {
        match marker.kind {
            ClickMarkerKind::Move => draw_ring(
                &mut gizmos,
                transform.translation,
                marker.radius,
                Color::srgba(1.0, 1.0, 1.0, 0.55),
            ),
            ClickMarkerKind::Harvest => {
                // gold ring + a little planted flag, so "go mine here" reads as a
                // deploy order rather than a plain move.
                let gold = Color::srgba(0.98, 0.78, 0.28, 0.9);
                draw_ring(&mut gizmos, transform.translation, marker.radius, gold);
                let base = transform.translation;
                let top = base + Vec3::Y * 1.1;
                gizmos.line(base, top, gold);
                gizmos.line(top, top + Vec3::new(0.5, -0.2, 0.0), gold);
                gizmos.line(top + Vec3::new(0.5, -0.2, 0.0), base + Vec3::Y * 0.7, gold);
            }
            ClickMarkerKind::Attack => {
                // Red ring + crosshair X — "attack here".
                let red = Color::srgba(1.0, 0.28, 0.22, 0.95);
                draw_ring(&mut gizmos, transform.translation, marker.radius, red);
                let c = Vec3::new(transform.translation.x, 0.06, transform.translation.z);
                let r = marker.radius;
                gizmos.line(c + Vec3::new(-r, 0.0, -r), c + Vec3::new(r, 0.0, r), red);
                gizmos.line(c + Vec3::new(-r, 0.0, r), c + Vec3::new(r, 0.0, -r), red);
            }
        }
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
    for (transform, effect) in &vfx.destruction {
        draw_structure_destruction_vfx(&mut gizmos, transform.translation, effect, &player_colors);
    }
    for (transform, effect) in &vfx.promotion {
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
            (!cursor_is_over_hud(window, &placement_preview.hud_zones))
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

pub(crate) fn should_draw_team_marker_for_entity(
    _unit: Option<&Unit>,
    _structure: Option<&Structure>,
) -> bool {
    // Team-colored ground rings removed — the concentric rings cluttered units and
    // looked bad. Friend/foe reads from the units themselves + selection rings.
    false
}

pub(crate) fn draw_structure_selection_brackets(
    gizmos: &mut Gizmos,
    position: Vec3,
    radius: f32,
    color: Color,
) {
    let center = terrain_overlay_point(position);
    let half_extent = (radius + 0.22).max(0.7);
    let bracket = (half_extent * 0.28).clamp(0.28, 0.82);
    for (sx, sz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let corner = Vec3::new(
            center.x + sx * half_extent,
            center.y,
            center.z + sz * half_extent,
        );
        let x_leg = Vec3::new(
            center.x + sx * (half_extent - bracket),
            center.y,
            center.z + sz * half_extent,
        );
        let z_leg = Vec3::new(
            center.x + sx * half_extent,
            center.y,
            center.z + sz * (half_extent - bracket),
        );
        gizmos.line(corner, x_leg, color);
        gizmos.line(corner, z_leg, color);
    }
}

pub(crate) fn draw_structure_placement_footprint(
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

pub(crate) fn structure_placement_preview_color(validity: StructurePlacementValidity) -> Color {
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

pub(crate) fn draw_terrain_order_path(gizmos: &mut Gizmos, start: Vec3, targets: &[Vec3]) {
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

pub(crate) fn terrain_overlay_point(position: Vec3) -> Vec3 {
    Vec3::new(position.x, 0.08, position.z)
}

pub(crate) fn should_draw_air_to_terrain_marker(domain: MovementDomain) -> bool {
    domain == MovementDomain::Air
}

pub(crate) fn draw_air_to_terrain_marker(
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

pub(crate) fn air_to_terrain_marker_color(team: Team, visible_team: Team) -> Option<Color> {
    if team == Team::Neutral {
        return None;
    }
    if team == visible_team {
        Some(Color::srgba(0.3, 0.95, 0.65, 0.8))
    } else {
        Some(Color::srgba(1.0, 0.28, 0.2, 0.8))
    }
}

/// Draws a rally flag at each selected production structure's rally point (plus a
/// line from the building to it), so setting a rally with right-click gives the
/// player clear feedback — a planted flag — instead of nothing.
pub(crate) fn draw_selected_rally_flags(
    mut gizmos: Gizmos<HudGizmos>,
    visible_player: Res<VisiblePlayer>,
    selected: Query<(&Transform, &Team, &RallyPoint), (With<Selected>, With<Structure>)>,
) {
    let color = Color::srgb(0.55, 0.95, 0.62);
    let faint = Color::srgba(0.55, 0.95, 0.62, 0.4);
    for (transform, team, rally) in &selected {
        if *team != visible_player.team {
            continue;
        }
        let Some(target) = rally.target else {
            continue;
        };
        let base = transform.translation + Vec3::Y * 0.2;
        let foot = Vec3::new(target.x, 0.05, target.z);
        // Tether from the building to the rally point.
        gizmos.line(base, foot + Vec3::Y * 0.15, faint);
        // Flag: a pole with a small triangular banner at the top.
        let pole_top = foot + Vec3::Y * 1.5;
        gizmos.line(foot, pole_top, color);
        let banner_out = Vec3::new(0.7, 0.0, 0.0);
        let banner_mid = pole_top - Vec3::Y * 0.22 + banner_out;
        gizmos.line(pole_top, banner_mid, color);
        gizmos.line(banner_mid, pole_top - Vec3::Y * 0.44, color);
        // Ground ring marking the rally spot.
        gizmos.circle(
            Isometry3d::new(foot, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            0.45,
            color,
        );
    }
}

pub(crate) fn draw_ring(gizmos: &mut Gizmos, position: Vec3, radius: f32, color: Color) {
    gizmos.circle(
        Isometry3d::new(
            Vec3::new(position.x, 0.05, position.z),
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        radius,
        color,
    );
}

pub(crate) fn draw_team_marker(
    gizmos: &mut Gizmos,
    position: Vec3,
    radius: f32,
    team: Team,
    player_colors: &PlayerColorSlots,
) {
    // A bold, team-colored ground ring (friend/foe at a glance). Drawn as a few
    // concentric circles so it reads as a thick band even with thin gizmo lines.
    let color = player_colors.color(team);
    let center = Vec3::new(position.x, position.y + 0.06, position.z);
    let rotation = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for ring in [0.60_f32, 0.68, 0.76] {
        gizmos.circle(
            Isometry3d::new(center, rotation),
            (radius * ring).max(0.25),
            color,
        );
    }
}

pub(crate) fn map_contains_ground_point_in_bounds(point: Vec3, bounds: MapBounds) -> bool {
    bounds.contains_ground_point(point)
}

pub(crate) fn validated_terrain_target_in_bounds(point: Vec3, bounds: MapBounds) -> Option<Vec3> {
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

pub(crate) fn cursor_minimap_local(window: &Window) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    if !minimap_contains_cursor(window, cursor) {
        return None;
    }
    Some(cursor - minimap_screen_min(window))
}

pub(crate) fn xz_distance(a: Vec3, b: Vec3) -> f32 {
    xz_distance_squared(a, b).sqrt()
}

pub(crate) fn xz_distance_squared(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length_squared()
}

pub(crate) fn distance_point_to_xz_segment(point: Vec3, start: Vec3, end: Vec3) -> f32 {
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

pub(crate) fn point_is_on_screen(
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

pub(crate) fn formation_offset(index: usize, count: usize) -> Vec3 {
    if count <= 1 {
        return Vec3::ZERO;
    }
    let side = (count as f32).sqrt().ceil() as usize;
    let x = (index % side) as f32 - (side as f32 - 1.0) * 0.5;
    let z = (index / side) as f32 - (side as f32 - 1.0) * 0.5;
    Vec3::new(x * 0.9, 0.0, z * 0.9)
}

pub(crate) fn free_position(origin: Vec3, seed: u32, radius: f32) -> Vec3 {
    free_position_in_bounds(origin, seed, radius, MapBounds::default())
}

pub(crate) fn free_position_in_bounds(
    origin: Vec3,
    seed: u32,
    radius: f32,
    bounds: MapBounds,
) -> Vec3 {
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
    fn rts_cursor_tracks_command_mode_and_hud() {
        let mut window = Window {
            resolution: WindowResolution::new(1280, 720),
            ..default()
        };
        window.set_cursor_position(None);

        let mut command_mode = CommandMode::default();
        let hud_zones = HudHitZones::default();
        assert_eq!(
            desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
            RtsCursorKind::Default
        );

        command_mode.attack_move = true;
        assert_eq!(
            desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
            RtsCursorKind::Attack
        );

        command_mode.attack_move = false;
        command_mode.pending_structure_placement =
            Some(PendingStructurePlacement::new("CommandCenter"));
        assert_eq!(
            desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
            RtsCursorKind::Build
        );

        window.set_cursor_position(Some(Vec2::new(640.0, 10.0)));
        assert_eq!(
            desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
            RtsCursorKind::Default
        );
    }

    #[test]
    fn power_readout_shows_used_before_capacity() {
        let mut econ = TeamEconomy::new(0, 0);
        econ.power_used = 26;
        econ.power_capacity = 13;
        assert_eq!(power_readout_text(&econ), "26/13");
        assert!(econ.low_power());
    }

    #[test]
    fn shift_right_click_queues_unit_waypoints_only_when_allowed() {
        let existing = OrderQueue {
            orders: VecDeque::from([UnitQueuedOrder::Move(Vec3::new(1.0, 0.0, 1.0))]),
        };
        assert!(
            should_queue_selected_order(true, true, true, None),
            "shift + active order should append a waypoint"
        );
        assert!(
            should_queue_selected_order(true, true, false, Some(&existing)),
            "shift + existing queued orders should append another waypoint"
        );
        assert!(
            !should_queue_selected_order(true, false, true, Some(&existing)),
            "callers must explicitly allow queueing"
        );
        assert!(
            !should_queue_selected_order(false, true, true, Some(&existing)),
            "plain right-click should replace the current order"
        );
    }

    #[test]
    fn rally_point_tracks_move_and_attack_move_modes() {
        let mut rally = RallyPoint {
            target: None,
            target_unit: None,
            mode: RallyMode::Move,
        };
        let move_target = Vec3::new(3.0, 0.0, 4.0);
        assert!(apply_rally_point_command_in_bounds(
            &mut rally,
            move_target,
            None,
            RallyMode::Move,
            MapBounds::default(),
        ));
        assert_eq!(rally.target, Some(move_target));
        assert_eq!(rally.mode, RallyMode::Move);

        let attack_target = Vec3::new(8.0, 0.0, -2.0);
        assert!(apply_rally_point_command_in_bounds(
            &mut rally,
            attack_target,
            None,
            RallyMode::AttackMove,
            MapBounds::default(),
        ));
        assert_eq!(rally.target, Some(attack_target));
        assert_eq!(rally.mode, RallyMode::AttackMove);
    }

    #[test]
    fn attack_rally_spawns_combat_units_with_attack_move_only() {
        let target = Vec3::new(6.0, 0.0, -3.0);
        let rally = RallyPoint {
            target: Some(target),
            target_unit: None,
            mode: RallyMode::AttackMove,
        };
        let tank = registry::entity("Tank").expect("tank definition should exist");
        assert!(matches!(
            spawned_unit_rally_order(tank, target, Some(rally)),
            UnitQueuedOrder::AttackMove(destination) if destination == target
        ));

        let worker = registry::entity("Worker").expect("worker definition should exist");
        assert!(matches!(
            spawned_unit_rally_order(worker, target, Some(rally)),
            UnitQueuedOrder::Move(destination) if destination == target
        ));
    }

    // Allies share vision: an enemy unit standing next to an ally's unit (but far
    // from the viewing player's own units) must be revealed through the ally, and
    // must stay fogged when the same teams are NOT allied. Mirrors godot's
    // FogOfWar revealing units `is_allied_with(visible_player)`.
    // W/S keyboard pan must match the edge-pan sign convention (pan.y<0 = view up,
    // matching cursor at the top edge). Guards against the recurring inversion.
    #[test]
    fn allied_vision_is_shared_through_allies() {
        fn enemy_visible_with_alliance(allied: bool) -> bool {
            let mut app = App::new();
            app.insert_resource(VisiblePlayer::per_player(Team::Player(0)));
            let mut relations = TeamRelations::default();
            relations.set_allied(Team::Player(0), Team::Player(1), allied);
            app.insert_resource(relations);
            app.add_systems(Update, update_visibility);

            // Viewing player's own unit, far from the action (does not reveal it).
            app.world_mut().spawn((
                Team::Player(0),
                Transform::from_xyz(0.0, 0.0, 0.0),
                VisionRadius(8.0),
                VisibilityState { visible: true },
                Visibility::Visible,
            ));
            // Ally unit parked next to the enemy, far from the viewer.
            app.world_mut().spawn((
                Team::Player(1),
                Transform::from_xyz(100.0, 0.0, 0.0),
                VisionRadius(12.0),
                VisibilityState { visible: true },
                Visibility::Visible,
            ));
            // Enemy unit beside the ally — only the ally can see it.
            let enemy = app
                .world_mut()
                .spawn((
                    Team::Player(2),
                    Transform::from_xyz(101.0, 0.0, 0.0),
                    VisionRadius(8.0),
                    VisibilityState { visible: false },
                    Visibility::Hidden,
                ))
                .id();

            app.update();
            app.world().get::<VisibilityState>(enemy).unwrap().visible
        }

        assert!(
            enemy_visible_with_alliance(true),
            "allied unit should reveal the enemy beside it for the viewing player"
        );
        assert!(
            !enemy_visible_with_alliance(false),
            "without an alliance the enemy beside a neutral team must stay fogged"
        );
    }

    #[test]
    fn edge_pan_is_disabled_by_options_overlays_and_interactive_ui_only() {
        let options = MenuOptionsState::default();
        assert_eq!(
            effective_camera_edge_pan_width(
                &options,
                &MatchMenuState::default(),
                &MatchBriefingState::default(),
                false,
            ),
            CAMERA_EDGE_PAN_WIDTH
        );

        let mut edge_pan_disabled = options;
        edge_pan_disabled.camera_edge_pan = false;
        assert_eq!(
            effective_camera_edge_pan_width(
                &edge_pan_disabled,
                &MatchMenuState::default(),
                &MatchBriefingState::default(),
                false,
            ),
            0.0
        );

        assert_eq!(
            effective_camera_edge_pan_width(
                &options,
                &MatchMenuState { visible: true },
                &MatchBriefingState::default(),
                false,
            ),
            0.0
        );
        assert_eq!(
            effective_camera_edge_pan_width(
                &options,
                &MatchMenuState::default(),
                &MatchBriefingState {
                    visible: true,
                    ..default()
                },
                false,
            ),
            CAMERA_EDGE_PAN_WIDTH,
            "the opening battle briefing is passive; it must not block edge pan"
        );
        assert_eq!(
            effective_camera_edge_pan_width(
                &options,
                &MatchMenuState::default(),
                &MatchBriefingState::default(),
                true,
            ),
            0.0
        );

        let mut window = Window {
            resolution: WindowResolution::new(1280, 720),
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(640.0, 10.0)));
        assert!(cursor_is_over_hud(&window, &HudHitZones::default()));
        assert_eq!(
            effective_camera_edge_pan_width(
                &options,
                &MatchMenuState::default(),
                &MatchBriefingState::default(),
                false,
            ),
            CAMERA_EDGE_PAN_WIDTH,
            "passive HUD strips must not block top/bottom edge pan"
        );
    }

    #[test]
    fn support_power_hud_specs_match_godot_strip() {
        let specs = support_power_button_specs();
        assert_eq!(
            specs.iter().map(|spec| spec.kind).collect::<Vec<_>>(),
            vec![
                SupportPowerKind::RadarSweep,
                SupportPowerKind::OrbitalStrike,
                SupportPowerKind::EmpPulse,
                SupportPowerKind::ChronoRelay,
                SupportPowerKind::ShieldOverdrive,
                SupportPowerKind::NaniteRepairSwarm,
                SupportPowerKind::WeatherStorm,
                SupportPowerKind::StrategicMissile,
                SupportPowerKind::Paradrop,
            ]
        );
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.hotkey_label)
                .collect::<Vec<_>>(),
            vec!["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"]
        );
        for spec in specs {
            let path = std::path::Path::new("assets").join(spec.icon_path);
            assert!(
                path.exists(),
                "support power icon must exist for {:?}: {}",
                spec.kind,
                path.display()
            );
        }
    }

    #[test]
    fn support_power_button_state_matches_lock_cooldown_and_low_power_rules() {
        let ready =
            support_power_button_state(SupportPowerKind::RadarSweep, true, false, 0.0, false);
        assert!(ready.enabled);
        assert!(ready.unlocked);
        assert!(!ready.active);
        assert_eq!(ready.cooldown_seconds, None);
        assert_eq!(ready.badge_text, "");

        let active =
            support_power_button_state(SupportPowerKind::RadarSweep, true, false, 0.0, true);
        assert!(active.enabled);
        assert!(active.active);

        let locked =
            support_power_button_state(SupportPowerKind::RadarSweep, false, false, 0.0, false);
        assert!(!locked.enabled);
        assert!(!locked.unlocked);
        assert_eq!(
            locked.badge_text, "",
            "locked support powers are hidden instead of rendered with a lock badge"
        );

        let cooling =
            support_power_button_state(SupportPowerKind::RadarSweep, true, false, 12.2, false);
        assert!(!cooling.enabled);
        assert_eq!(cooling.cooldown_seconds, Some(13));
        assert_eq!(cooling.badge_text, "13");

        let low_power =
            support_power_button_state(SupportPowerKind::RadarSweep, true, true, 0.0, false);
        assert!(!low_power.enabled);
        assert!(low_power.low_power);
        assert_eq!(low_power.badge_text, "");
    }

    #[test]
    fn support_powers_unlock_only_after_required_structures_are_constructed() {
        let mut world = World::new();
        let team = Team::Player(0);
        let other_team = Team::Player(1);

        {
            let mut query = world.query::<StructurePrereqItem<'_>>();
            let structures = query.query(&world);
            assert_eq!(visible_support_power_count(team, &structures), 0);
            assert!(!support_power_unlocked(
                team,
                SupportPowerKind::RadarSweep,
                &structures
            ));
        }

        world.spawn((Structure { id: "RadarUplink" }, team, Transform::default()));
        {
            let mut query = world.query::<StructurePrereqItem<'_>>();
            let structures = query.query(&world);
            assert!(support_power_unlocked(
                team,
                SupportPowerKind::RadarSweep,
                &structures
            ));
            assert_eq!(visible_support_power_count(team, &structures), 1);
        }

        world.spawn((
            Structure {
                id: "WeatherControlSpire",
            },
            team,
            Transform::default(),
            UnderConstruction {
                remaining: 8.0,
                total: 8.0,
                cost: registry::Cost::default(),
                free_worker_origin: None,
            },
        ));
        {
            let mut query = world.query::<StructurePrereqItem<'_>>();
            let structures = query.query(&world);
            assert_eq!(
                visible_support_power_count(team, &structures),
                1,
                "under-construction tech must not reveal support buttons"
            );
            assert!(!support_power_unlocked(
                team,
                SupportPowerKind::StrategicMissile,
                &structures
            ));
        }

        world.spawn((Structure { id: "TechAirport" }, team, Transform::default()));
        world.spawn((
            Structure {
                id: "WeatherControlSpire",
            },
            team,
            Transform::default(),
        ));
        world.spawn((
            Structure { id: "RoboticsBay" },
            other_team,
            Transform::default(),
        ));
        {
            let mut query = world.query::<StructurePrereqItem<'_>>();
            let structures = query.query(&world);
            assert!(support_power_unlocked(
                team,
                SupportPowerKind::Paradrop,
                &structures
            ));
            assert!(support_power_unlocked(
                team,
                SupportPowerKind::StrategicMissile,
                &structures
            ));
            assert_eq!(
                visible_support_power_count(team, &structures),
                4,
                "radar + airport + weather spire should reveal radar, paradrop, and two superweapons only"
            );
        }
    }

    #[test]
    fn support_power_panel_hit_rect_is_tight() {
        let mut window = Window {
            resolution: WindowResolution::new(1280, 720),
            ..default()
        };
        assert_eq!(
            support_power_panel_width_for_visible_count(0),
            0.0,
            "no unlocked support powers should leave no top-right hit rect"
        );
        assert_eq!(
            support_power_panel_width_for_visible_count(SupportPowerKind::ALL.len()),
            SUPPORT_POWER_PANEL_WIDTH_PX
        );

        let inside = Vec2::new(1278.0 - SUPPORT_POWER_PANEL_RIGHT_PX, 16.0);
        assert!(!support_power_panel_contains_cursor(&window, inside, 0));
        assert!(support_power_panel_contains_cursor(&window, inside, 3));
        window.set_cursor_position(Some(inside));
        let zones = HudHitZones {
            world_rects: hud_world_input_rects(1280.0, 720.0, 3, 0, 0, 0, false),
        };
        assert!(cursor_is_over_hud(&window, &zones));

        let left_of_panel = Vec2::new(
            1280.0
                - SUPPORT_POWER_PANEL_RIGHT_PX
                - support_power_panel_width_for_visible_count(3)
                - 8.0,
            SUPPORT_POWER_PANEL_TOP_PX + 20.0,
        );
        assert!(!support_power_panel_contains_cursor(
            &window,
            left_of_panel,
            3
        ));
        assert!(
            !cursor_blocks_world_order_controls(left_of_panel, &zones),
            "support panel hit rect should not consume the whole top strip"
        );

        let below_panel = Vec2::new(
            1280.0 - SUPPORT_POWER_PANEL_RIGHT_PX - 12.0,
            SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX + 4.0,
        );
        assert!(!support_power_panel_contains_cursor(
            &window,
            below_panel,
            3
        ));
    }

    #[test]
    fn objective_tracker_sits_below_support_power_strip() {
        assert!(
            OBJECTIVE_TRACKER_TOP_PX
                >= SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX + 6.0,
            "the top-center objective HUD must not overlap the Godot-style support power strip"
        );
    }

    // Every command-panel action must resolve to an icon asset that actually
    // exists on disk, so the HUD renders godot-style command icons instead of
    // falling back to a blank button. Locks the action->icon mapping.
    #[test]
    fn command_actions_resolve_to_existing_icon_assets() {
        let standing_orders = [
            BuildAction::SellStructure,
            BuildAction::RepairStructure,
            BuildAction::ToggleDeployMode,
            BuildAction::SetRallyPoint,
            BuildAction::HoldPosition,
            BuildAction::AttackMove,
            BuildAction::Patrol,
            BuildAction::GuardArea,
            BuildAction::StopSelected,
            BuildAction::ScatterSelected,
        ];
        for (idx, action) in standing_orders.into_iter().enumerate() {
            let path = command_action_icon_path(action)
                .unwrap_or_else(|| panic!("no icon for standing order #{idx}"));
            assert!(
                std::path::Path::new("assets").join(path).exists(),
                "missing icon asset {path} for standing order #{idx}"
            );
        }
        // Train/Build pull the produced entity's registry icon (e.g. Worker).
        let worker_icon = command_action_icon_path(BuildAction::Train("Worker"))
            .expect("Worker should have a registry icon");
        assert!(
            std::path::Path::new("assets").join(worker_icon).exists(),
            "missing Worker icon asset {worker_icon}"
        );
        assert_eq!(command_action_icon_path(BuildAction::None), None);
    }

    #[test]
    fn worker_is_the_only_resource_collector_unit() {
        let worker = registry::entity("Worker").expect("Worker must stay in the registry");
        assert!(
            worker.resource_capacity > 0,
            "Worker must carry resources now that separate vehicle collectors are removed"
        );
        for entity in registry::ENTITY_DEFS {
            assert!(
                !entity.is_worker || entity.id == "Worker",
                "{} must not share the worker classification",
                entity.id
            );
            assert!(
                entity.resource_capacity <= 0 || entity.id == "Worker",
                "{} must not share the resource collector role",
                entity.id
            );
        }
        assert!(
            registry::entity("OreHarvester").is_none(),
            "OreHarvester should not exist in the playable registry"
        );
        assert!(
            registry::entity("MobileConstructionVehicle").is_none(),
            "MobileConstructionVehicle should not exist in the playable registry"
        );
        for faction_id in ["alliance", "demon", "chaos"] {
            let faction = registry::faction(faction_id).expect("registered skirmish faction");
            let command_center = faction
                .production
                .iter()
                .find(|production| production.producer == "CommandCenter")
                .expect("CommandCenter production list");
            assert!(
                command_center.products.contains(&"Worker"),
                "{faction_id} CommandCenter must train Worker"
            );
            for production in faction.production {
                assert!(
                    !production.products.contains(&"OreHarvester"),
                    "{faction_id} {} must not expose OreHarvester",
                    production.producer
                );
                assert!(
                    !production.products.contains(&"MobileConstructionVehicle"),
                    "{faction_id} {} must not expose MobileConstructionVehicle",
                    production.producer
                );
            }
        }
    }

    #[test]
    fn critical_units_do_not_share_model_signatures() {
        fn model_signature(id: &str) -> Vec<&'static str> {
            registry::entity(id)
                .unwrap_or_else(|| panic!("{id} must stay in the registry"))
                .render_parts
                .iter()
                .map(|part| part.model)
                .collect()
        }

        let worker = registry::entity("Worker").expect("Worker must stay in the registry");
        let scout = registry::entity("ScoutRover").expect("ScoutRover must stay in the registry");
        assert_eq!(worker.render_parts.len(), 2);
        assert_eq!(scout.render_parts.len(), 1);
        assert!(
            worker
                .render_parts
                .iter()
                .any(|part| part.model == "models/kenney-spacekit/astronautB.glb"),
            "Worker should read as an engineer/infantry unit, not another rover"
        );
        assert!(
            !worker
                .render_parts
                .iter()
                .any(|part| part.model == "models/kenney-spacekit/rover.glb"),
            "Worker must not share ScoutRover's rover mesh"
        );
        assert_eq!(
            scout.render_parts[0].model,
            "models/kenney-spacekit/rover.glb"
        );
        assert_eq!(scout.render_parts[0].translation, [-3.3, 0.0, -2.475]);
        assert_eq!(scout.render_parts[0].scale, [1.65, 1.65, 1.65]);

        for (left, right) in [
            ("Worker", "ScoutRover"),
            ("ScoutRover", "RocketInfantry"),
            ("ScoutRover", "ShieldTrooper"),
            ("RocketInfantry", "ShieldTrooper"),
            ("GrenadierTrooper", "RocketInfantry"),
            ("GrenadierTrooper", "RocketTrooperRobot"),
            ("RocketInfantry", "RocketTrooperRobot"),
        ] {
            assert_ne!(
                model_signature(left),
                model_signature(right),
                "{left} and {right} must have distinct model signatures"
            );
        }
    }

    #[test]
    fn registry_render_part_models_exist_on_disk() {
        for entity in registry::ENTITY_DEFS {
            for part in entity.render_parts {
                assert!(
                    std::path::Path::new("assets").join(part.model).exists(),
                    "{} references missing model asset {}",
                    entity.id,
                    part.model
                );
            }
        }
    }

    #[test]
    fn hunyuan_render_parts_have_forward_rotation_and_material_fallback() {
        let mut count = 0;
        for entity in registry::ENTITY_DEFS {
            for part in entity.render_parts {
                if !is_hunyuan_model_path(part.model) {
                    continue;
                }
                count += 1;
                assert_eq!(
                    part.rotation,
                    [0.0, 1.0, 0.0, 0.0],
                    "{} must rotate its Hunyuan mesh 180 degrees around Y so the generated front faces the RTS forward axis",
                    entity.id
                );
                let material = hunyuan_model_material(entity.id);
                assert!(
                    material.metallic > 0.5,
                    "{} must use the runtime material fallback for mesh-only Hunyuan GLBs",
                    entity.id
                );
            }
        }
        assert_eq!(
            count, 14,
            "expected every Hunyuan replacement to be guarded"
        );
    }

    #[test]
    fn hunyuan_material_system_applies_to_loaded_scene_children() {
        let mut app = App::new();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<HunyuanModelMaterialCache>();
        app.add_systems(Update, apply_hunyuan_model_materials);

        let initial = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.02, 0.02, 0.02),
                ..default()
            });
        let root = app
            .world_mut()
            .spawn(HunyuanModelPart {
                entity_id: "FlameAssaultBuggy",
            })
            .id();
        let child = app
            .world_mut()
            .spawn((ChildOf(root), MeshMaterial3d(initial.clone())))
            .id();

        app.update();

        let assigned = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(child)
            .expect("child mesh material")
            .0
            .clone();
        assert_ne!(assigned, initial);
        assert!(app.world().get::<HunyuanModelMaterialized>(root).is_some());
        let material = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&assigned)
            .expect("assigned Hunyuan material");
        let color = material.base_color.to_srgba();
        assert!(
            color.red > color.green && color.red > color.blue,
            "FlameAssaultBuggy fallback should visibly read as a flame vehicle"
        );
    }

    #[test]
    fn model_harness_ids_cover_registry_with_compiler_checked_length() {
        assert_eq!(
            MODEL_HARNESS_ENTITY_IDS.len(),
            registry::ENTITY_DEFS.len(),
            "MODEL_HARNESS_ENTITY_IDS has a const length check against ENTITY_DEFS; this runtime assertion keeps the failure message clear"
        );
        let mut seen = std::collections::BTreeSet::new();
        for id in MODEL_HARNESS_ENTITY_IDS {
            assert!(seen.insert(id), "duplicate model harness id {id}");
            assert!(
                registry::entity(id).is_some(),
                "model harness id {id} must resolve to an EntityDef"
            );
        }
    }

    #[test]
    fn model_harness_roots_are_kind_typed_instances() {
        let mut app = App::new();
        let def = model_harness_entity_def(MODEL_HARNESS_ENTITY_IDS[0]);
        let root = spawn_model_harness_root(app.world_mut(), 0, def, Vec3::ZERO);

        let mut query = app.world_mut().query::<Instance<ModelHarnessRoot>>();
        let roots = query.iter(app.world()).collect::<Vec<_>>();

        assert_eq!(roots, vec![root]);
        let marker = app
            .world()
            .get::<ModelHarnessRoot>(root.entity())
            .expect("typed harness root marker");
        assert_eq!(marker.index, 0);
        assert_eq!(marker.id, def.id);
    }

    #[test]
    fn headless_loading_state_skips_render_asset_preload() {
        let mut app = build_game_app(GameAppMode::Headless);
        for _ in 0..4 {
            app.update();
        }

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::MainMenu
        );
        assert!(
            app.world()
                .resource::<StartupLoadingAssets>()
                .handles
                .is_empty(),
            "headless tests should not preload the full render asset set"
        );
    }

    #[test]
    fn startup_loading_state_tracks_real_asset_handles_when_enabled() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(StartupLoadingPolicy {
            preload_assets: true,
        });

        app.update();

        let retained = app.world().resource::<StartupLoadingAssets>();
        assert!(
            retained.handles.len() > registry::ENTITY_DEFS.len(),
            "iyes_progress startup loading should track UI assets, icons, cursor assets, model-map data, and migrated model scenes"
        );
        let loading = app.world().resource::<AssetsLoading<AppScreen>>();
        assert!(
            !loading.allow_failures,
            "startup loading should fail loudly instead of silently skipping missing critical assets"
        );
        assert!(
            loading.track_dependencies,
            "startup loading should wait for GLB scene dependencies, not only top-level handles"
        );
    }

    #[test]
    fn impact_burst_scales_with_damage_and_structures() {
        let infantry_hit = impact_burst_power(4.0, 0.45, false);
        let heavy_vehicle_hit = impact_burst_power(18.0, 0.8, false);
        let structure_hit = impact_burst_power(18.0, 1.4, true);

        assert!(
            infantry_hit >= 0.45,
            "small hits should still generate a readable impact burst"
        );
        assert!(
            heavy_vehicle_hit > infantry_hit,
            "higher damage should produce a larger impact burst"
        );
        assert!(
            structure_hit > heavy_vehicle_hit,
            "structure impacts should read heavier than same-damage unit impacts"
        );
        assert!(
            impact_burst_lifetime(structure_hit) > impact_burst_lifetime(infantry_hit),
            "larger bursts should stay visible for more frames"
        );
    }

    #[test]
    fn impact_burst_kind_tracks_weapon_family() {
        let rifle = Weapon::new(4.0, 4.0, 0.8, 0.0, 0.0, 1.0, false, true);
        let rocket = Weapon::new(6.0, 10.0, 1.4, 1.0, 0.45, 1.2, true, true);

        assert_eq!(
            impact_burst_kind_for_entity_id("HeavyMachinegunTrooper", &rifle, false),
            ImpactBurstKind::Ballistic
        );
        assert_eq!(
            impact_burst_kind_for_entity_id("RocketInfantry", &rocket, false),
            ImpactBurstKind::Explosive
        );
        assert_eq!(
            impact_burst_kind_for_entity_id("TeslaCrawlerMk2", &rifle, false),
            ImpactBurstKind::Electric
        );
        assert_eq!(
            impact_burst_kind_for_entity_id("FlameAssaultBuggy", &rifle, false),
            ImpactBurstKind::Fire
        );
        assert_eq!(
            impact_burst_kind_for_entity_id("RailCannonBunker", &rifle, true),
            ImpactBurstKind::Heavy
        );
    }

    #[test]
    fn worker_build_menu_orders_opening_before_late_tech() {
        let faction = faction_def(SkirmishFaction::Alliance).expect("alliance faction");
        let ordered = sorted_worker_build_structures(faction);
        let position = |id: &str| {
            ordered
                .iter()
                .position(|candidate| *candidate == id)
                .unwrap_or_else(|| panic!("missing build structure {id}"))
        };

        assert_eq!(ordered.first(), Some(&"PowerReactor"));
        assert!(position("PowerReactor") < position("Refinery"));
        assert!(position("Refinery") < position("Barracks"));
        assert!(position("Barracks") < position("VehicleFactory"));
        assert!(position("VehicleFactory") < position("TechLab"));
        assert!(position("TechLab") < position("WeatherControlSpire"));
    }

    #[test]
    fn worker_build_menu_splits_production_and_defense_tabs() {
        assert_eq!(
            build_structure_tab_for("PowerReactor"),
            BuildStructureTab::Production
        );
        assert_eq!(
            build_structure_tab_for("Refinery"),
            BuildStructureTab::Production
        );
        assert_eq!(
            build_structure_tab_for("Barracks"),
            BuildStructureTab::Production
        );
        assert_eq!(
            build_structure_tab_for("VehicleFactory"),
            BuildStructureTab::Production
        );
        assert_eq!(
            build_structure_tab_for("WeatherControlSpire"),
            BuildStructureTab::Production
        );
        assert_eq!(
            build_structure_tab_for("AntiGroundTurret"),
            BuildStructureTab::Defense
        );
        assert_eq!(
            build_structure_tab_for("AntiAirTurret"),
            BuildStructureTab::Defense
        );
        assert_eq!(
            build_structure_tab_for("TeslaFenceSegment"),
            BuildStructureTab::Defense
        );
        assert_eq!(
            build_structure_tab_for("RailCannonBunker"),
            BuildStructureTab::Defense
        );
    }

    #[test]
    fn worker_cargo_visual_slots_show_loaded_resources() {
        let empty = ResourceCargo {
            capacity: 6,
            ore: 0,
            crystal: 0,
        };
        assert!(harvest_cargo_visual_slots(empty).is_empty());

        let mixed = ResourceCargo {
            capacity: 8,
            ore: 4,
            crystal: 3,
        };
        assert_eq!(
            harvest_cargo_visual_slots(mixed),
            vec![
                ResourceKind::Ore,
                ResourceKind::Ore,
                ResourceKind::Ore,
                ResourceKind::Ore,
                ResourceKind::Crystal,
                ResourceKind::Crystal,
            ],
            "cargo visuals should show both resource kinds and cap to visible slots"
        );
    }

    #[test]
    fn right_click_resource_orders_only_workers_to_harvest() {
        let resource = Entity::from_raw_u32(7).unwrap();
        let choices = OrderTargetChoices {
            supply_crate_position: None,
            resource_target: Some(resource),
            resource_dropoff_target: None,
            enemy_target: None,
            repair_target: None,
            construct_target: None,
            garrison_target: None,
            follow_target: None,
        };
        let context = UnitOrderContext {
            force_move: false,
            enemy_target_capturable: false,
            attack_move: false,
            patrol: false,
            origin: Vec3::ZERO,
            point: Vec3::new(4.0, 0.0, 5.0),
            offset: Vec3::ZERO,
        };
        let worker = Unit {
            id: "Worker",
            speed: 2.0,
            can_crush: false,
            can_be_crushed: true,
        };
        let tank = Unit {
            id: "Tank",
            speed: 2.0,
            can_crush: true,
            can_be_crushed: false,
        };

        assert!(matches!(
            desired_order_for_selected_unit(&worker, choices, context),
            Some(UnitQueuedOrder::Harvest {
                target,
                state: HarvestState::MovingToResource
            }) if target == resource
        ));
        assert!(
            !matches!(
                desired_order_for_selected_unit(&tank, choices, context),
                Some(UnitQueuedOrder::Harvest { .. })
            ),
            "combat units should not become resource collectors when right-clicking ore"
        );
    }

    // An idle defense structure (weapon off cooldown) sweeps over time; one that
    // is mid-engagement (cooldown_left > 0) or still under construction stays put.
    #[test]
    fn idle_defense_tower_scans_when_not_engaging() {
        fn final_yaw(cooldown_left: f32, under_construction: bool) -> f32 {
            let mut app = App::new();
            app.insert_resource(Time::<()>::default());
            app.add_systems(Update, update_idle_tower_scan);
            let mut entity = app.world_mut().spawn((
                Structure {
                    id: "AntiGroundTurret",
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                Weapon {
                    range: 10.0,
                    damage: 5.0,
                    cooldown: 1.0,
                    splash_radius: 0.0,
                    splash_damage_multiplier: 0.0,
                    structure_damage_multiplier: 1.0,
                    cooldown_left,
                    can_attack_air: false,
                    can_attack_ground: true,
                },
            ));
            if under_construction {
                entity.insert(UnderConstruction {
                    remaining: 5.0,
                    total: 5.0,
                    cost: registry::Cost { ore: 0, crystal: 0 },
                    free_worker_origin: None,
                });
            }
            let id = entity.id();
            // Advance several frames of simulated time so the scan accumulates.
            for _ in 0..30 {
                let mut time = app.world_mut().resource_mut::<Time<()>>();
                time.advance_by(std::time::Duration::from_secs_f32(0.1));
                app.update();
            }
            app.world()
                .get::<Transform>(id)
                .unwrap()
                .rotation
                .to_euler(EulerRot::YXZ)
                .0
        }

        assert!(
            final_yaw(0.0, false).abs() > 0.01,
            "idle tower should sweep (yaw should change)"
        );
        assert!(
            final_yaw(0.5, false).abs() < 1e-6,
            "engaging tower (on cooldown) should not sweep"
        );
        assert!(
            final_yaw(0.0, true).abs() < 1e-6,
            "tower under construction should not sweep"
        );
    }

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
    fn ai_drone_scouting_moves_idle_ai_drones() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..20 {
            app.update();
        }

        let drone_def = registry::entity("Drone").expect("Drone must stay in the registry");
        let before = Vec3::new(12.0, 0.0, 12.0);
        let drone = app
            .world_mut()
            .spawn((
                Unit {
                    id: "Drone",
                    speed: drone_def.speed,
                    can_crush: drone_def.can_crush,
                    can_be_crushed: drone_def.can_be_crushed,
                },
                Team::Player(1),
                Transform::from_translation(before),
                Selectable {
                    radius: drone_def.radius,
                },
                Health::new(drone_def.health),
                VisionRadius(unit_vision_radius(drone_def)),
                MovementDomain::from_registry(drone_def.domain),
                VisibilityState { visible: true },
                Visibility::Visible,
                MatchScopedEntity,
            ))
            .id();

        for _ in 0..180 {
            app.update();
        }

        let world = app.world();
        let after = world
            .get::<Transform>(drone)
            .expect("AI Drone should still exist")
            .translation;
        let distance = xz_distance(before, after);
        eprintln!("[diag] AI Drone scout moved {distance:.2}m");
        assert!(
            distance > 0.5,
            "AI Drone did not leave its spawn point for scouting"
        );
        assert!(
            world
                .get::<AiDroneScout>(drone)
                .is_some_and(|scout| scout.last_target.is_some()),
            "AI Drone should remember the current scout target"
        );
    }

    #[test]
    fn ai_defense_profile_limits_match_godot_difficulty_targets() {
        let easy = faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Easy);
        assert!(
            easy.defense_priority.is_empty(),
            "Easy AI should not inherit the Normal defense build queue"
        );
        assert!(
            !easy.active_offense_enabled,
            "Easy AI should give the player a build-up window instead of launching attack waves"
        );
        assert_eq!(
            ai_battlegroup_target_units(&easy),
            2,
            "Easy AI should still train a small defensive force"
        );
        assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &easy), 0);
        assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &easy), 0);

        let normal =
            faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Normal);
        assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &normal), 1);
        assert_eq!(ai_structure_profile_limit("AntiAirTurret", &normal), 1);
        assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &normal), 2);
        assert_eq!(
            ai_structure_profile_limit("ArcCoilDefenseTower", &normal),
            1
        );
        assert_eq!(
            ai_structure_profile_limit("PrismDefenseObelisk", &normal),
            1
        );
        assert_eq!(ai_structure_profile_limit("RailCannonBunker", &normal), 1);

        let hard = faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Hard);
        assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &hard), 2);
        assert_eq!(ai_structure_profile_limit("AntiAirTurret", &hard), 2);
        assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &hard), 4);
        assert_eq!(ai_structure_profile_limit("ArcCoilDefenseTower", &hard), 2);
        assert_eq!(ai_structure_profile_limit("PrismDefenseObelisk", &hard), 2);
        assert_eq!(ai_structure_profile_limit("RailCannonBunker", &hard), 2);

        let chaos_normal =
            faction_ai_profile_for_difficulty(SkirmishFaction::Chaos, AiDifficulty::Normal);
        let chaos_hard =
            faction_ai_profile_for_difficulty(SkirmishFaction::Chaos, AiDifficulty::Hard);
        assert_eq!(
            ai_structure_profile_limit("TeslaFenceSegment", &chaos_normal),
            2
        );
        assert_eq!(
            ai_structure_profile_limit("TeslaFenceSegment", &chaos_hard),
            4
        );
    }

    #[test]
    fn godot_skirmish_startup_uses_worker_only_economy() {
        let mut flavor_units = Vec::new();
        for faction in SkirmishFaction::ALL {
            let startup = faction_startup_for_loadout(faction, StartupLoadoutMode::GodotSkirmish);
            assert_eq!(
                startup.structures,
                &[SpawnSpec {
                    id: "CommandCenter",
                    offset: (0.0, 0.0),
                }],
                "{} should keep the minimal one-base skirmish opening",
                faction.label()
            );
            assert_eq!(
                startup
                    .units
                    .iter()
                    .filter(|spec| spec.id == "Worker")
                    .count(),
                2,
                "{} should still start with two worker economy units",
                faction.label()
            );
            assert!(
                startup
                    .units
                    .iter()
                    .all(|spec| !matches!(spec.id, "OreHarvester" | "MobileConstructionVehicle")),
                "{} must not reintroduce a separate vehicle collector/builder economy start",
                faction.label()
            );
            let flavor = startup
                .units
                .iter()
                .find(|spec| spec.id != "Worker")
                .map(|spec| spec.id)
                .expect("each faction should have a visible faction-specific starter");
            flavor_units.push(flavor);
        }
        flavor_units.sort_unstable();
        flavor_units.dedup();
        assert_eq!(
            flavor_units.len(),
            SkirmishFaction::ALL.len(),
            "each faction should have a distinct starter unit in the default cargo-run opening"
        );
    }

    #[test]
    fn start_camera_focus_prefers_command_center_over_worker() {
        let startup = faction_startup_for_loadout(
            SkirmishFaction::Alliance,
            StartupLoadoutMode::GodotSkirmish,
        );
        assert_eq!(
            startup_camera_focus_offset(startup),
            Vec3::ZERO,
            "default cargo-run camera should start over the CommandCenter, not the worker offset"
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
    }

    #[test]
    fn runtime_team_ids_from_relations_do_not_clamp_after_lobby_slot_count() {
        let active = vec![true; 12];
        let mut relations = TeamRelations::default();
        relations.ensure_player_count(active.len());

        let team_ids = skirmish_team_ids_from_relations(&active, &relations);

        assert_eq!(team_ids, (0..12).collect::<Vec<_>>());

        let relation_ids = (0..12).collect::<Vec<_>>();
        let relations = skirmish_team_relations_from_team_ids(&active, &relation_ids);
        assert!(relations.are_enemies(Team::Player(0), Team::Player(8)));
        assert!(relations.are_enemies(Team::Player(8), Team::Player(11)));

        let mut allied_ids = (0..12).collect::<Vec<_>>();
        allied_ids[11] = 8;
        let allied_relations = skirmish_team_relations_from_team_ids(&active, &allied_ids);
        assert!(allied_relations.are_allied(Team::Player(8), Team::Player(11)));
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

    #[test]
    fn virtual_spawn_positions_exist_after_map_spawn_slots() {
        let map = largest_skirmish_map();
        let bounds = MapBounds::from_map(map);
        let first_extra = team_start_position_for_spawn_slot(map, map.players);
        let second_extra = team_start_position_for_spawn_slot(map, map.players + 1);

        assert!(bounds.contains_ground_point(first_extra));
        assert!(bounds.contains_ground_point(second_extra));
        assert!(xz_distance(first_extra, second_extra) > 1.0);
        assert!(xz_distance(first_extra, team_start_position_for_spawn_slot(map, 0)) > 1.0);
    }

    // Fast (no-render) diagnostic for the human core loop: what units does the
    // player start with, and do they actually move when ordered to attack-move?
    #[test]
    fn diag_player_units_exist_and_move() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..20 {
            app.update();
        }

        let player = Team::Player(0);
        let snapshot = |app: &mut App| -> Vec<(Entity, &'static str, Vec3)> {
            let world = app.world_mut();
            let mut q = world.query_filtered::<(Entity, &Unit, &Team, &Transform), ()>();
            q.iter(world)
                .filter(|(_, _, team, _)| **team == player)
                .map(|(e, unit, _, tf)| (e, unit.id, tf.translation))
                .collect()
        };

        let before = snapshot(&mut app);
        eprintln!("[diag] player starts with {} units:", before.len());
        for (_, id, pos) in &before {
            let can_attack = registry::entity(id)
                .map(|d| d.weapon.is_some())
                .unwrap_or(false);
            eprintln!("  {id}  weapon={can_attack}  @ ({:.1},{:.1})", pos.x, pos.z);
        }

        assert!(!before.is_empty(), "player should start with units");
    }

    // Fast (no-render) observation of a full default match (Human P0 vs Easy AI
    // P1): does the economy grow, does the AI build an army and attack, does the
    // match progress toward a result? Uses a fixed timestep so game-time is real.
    #[test]
    fn diag_match_economy_and_ai_progress() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..20 {
            app.update();
        }
        // Minimal start has workers only — you build a Refinery for one. Spectate
        // so the AI drives P0 too: it builds a refinery, the free P0 worker
        // spawns, and (per the auto-harvest fix) that player-team worker
        // auto-harvests so P0 ore grows. Guards the player-team auto-harvest path.
        app.world_mut()
            .insert_resource(VisiblePlayer::spectator_per_player(Team::Player(0)));

        let sample = |app: &mut App| {
            let world = app.world_mut();
            let mut units = world.query::<(&Team, &Unit)>();
            let (mut p0, mut p1) = (0u32, 0u32);
            let mut p0_battle = 0u32;
            for (team, unit) in units.iter(world) {
                match *team {
                    Team::Player(0) => {
                        p0 += 1;
                        if ai_battle_unit_id(unit.id) {
                            p0_battle += 1;
                        }
                    }
                    Team::Player(1) => p1 += 1,
                    _ => {}
                }
            }
            let _ = p0_battle;
            let econ = world.resource::<Economies>();
            let (ore0, ore1) = (econ.get(Team::Player(0)).ore, econ.get(Team::Player(1)).ore);
            let phase = world.resource::<MatchState>().phase;
            (p0, p1, ore0, ore1, phase, p0_battle)
        };

        eprintln!("[diag] t(s) | P0_units P1_units | P0_ore P1_ore | phase");
        let mut start_ore = None;
        let mut peak_ore = 0;
        for step in 0..=24 {
            // 5 game-seconds per step at 1/30s per tick = 150 ticks.
            if step > 0 {
                for _ in 0..150 {
                    app.update();
                }
            }
            let (p0, p1, o0, o1, phase, p0_battle) = sample(&mut app);
            start_ore.get_or_insert(o0);
            peak_ore = peak_ore.max(o0);
            eprintln!(
                "[diag] {:>4} | P0 {p0:>2} (army {p0_battle:>2}) P1 {p1:>2} | ore {o0:>4} {o1:>4} | {phase:?}",
                step * 5
            );
            if !matches!(phase, MatchPhase::Running) {
                break;
            }
        }
        // Regression guard: a player-team worker (from a built Refinery) must
        // auto-harvest, so P0 ore rises above its start at some point even though
        // the AI also spends it. Guards the auto-harvest fix that previously
        // excluded the player team (which left the human economy dead).
        assert!(
            peak_ore > start_ore.unwrap(),
            "player ore never grew (start {}, peak {peak_ore}): worker not auto-harvesting",
            start_ore.unwrap()
        );
    }

    // End-to-end loop: with both sides AI (spectator), does a full match actually
    // resolve to a victory/defeat (economy -> army -> combat -> result)?
    #[test]
    fn diag_ai_vs_ai_match_resolves() {
        let resolved = capture_run_ai_match_until_resolved(240);
        match resolved {
            Some((secs, phase)) => eprintln!("[diag] AI-vs-AI resolved at ~{secs}s: {phase}"),
            None => eprintln!("[diag] AI-vs-AI still Running after 240s (possible stalemate)"),
        }
        assert!(
            resolved.is_some(),
            "AI-vs-AI did not resolve within 240s; economy/combat loop is not a completed match"
        );
    }

    // Each of the 3 factions must be fully playable: from its own base, its
    // production chain must build an army. Same-faction AI-vs-AI per faction.
    #[test]
    fn diag_all_three_factions_playable() {
        for faction in SkirmishFaction::ALL {
            let mut app = build_game_app(GameAppMode::Headless);
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(1.0 / 30.0),
            ));
            {
                let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
                for slot in settings.player_factions.iter_mut() {
                    *slot = faction;
                }
            }
            app.world_mut()
                .resource_mut::<NextState<AppScreen>>()
                .set(AppScreen::InMatch);
            for _ in 0..20 {
                app.update();
            }
            app.world_mut()
                .insert_resource(VisiblePlayer::spectator_per_player(Team::Player(0)));

            let count_units = |app: &mut App, team: Team| {
                let world = app.world_mut();
                let mut q = world.query_filtered::<&Team, With<Unit>>();
                q.iter(world).filter(|t| **t == team).count()
            };
            let start_units = count_units(&mut app, Team::Player(0));
            let mut peak = start_units;
            let mut resolved = false;
            for _ in 0..72 {
                for _ in 0..150 {
                    app.update();
                }
                peak = peak.max(count_units(&mut app, Team::Player(0)));
                if !matches!(
                    app.world().resource::<MatchState>().phase,
                    MatchPhase::Running
                ) {
                    resolved = true;
                    break;
                }
            }
            eprintln!(
                "[diag] {:>8}: start {start_units} units, peak {peak}, resolved={resolved}",
                faction.label()
            );
            assert!(
                peak > start_units,
                "{} produced no army (production chain broken)",
                faction.label()
            );
        }
    }

    // Lobby slots must be closable in 1-2 clicks (RA2/Warcraft style): the
    // per-slot controller cycles 关闭 -> 我方 -> 电脑 -> 关闭.
    #[test]
    fn lobby_slot_closes_in_one_cycle_from_ai() {
        let mut sel = SkirmishMenuSelection::default();
        let slot = 1; // default slot 1 is an AI slot, within every map.
        sel.set_lobby_slot_controller(slot, SkirmishPlayerController::Ai(AiDifficulty::Easy));
        sel.cycle_lobby_slot_controller(slot); // AI -> 关闭
        assert_eq!(
            sel.lobby_controllers[slot],
            SkirmishPlayerController::None,
            "an AI slot must close in a single cycle"
        );
        sel.cycle_lobby_slot_controller(slot); // 关闭 -> 我方
        assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::Human);
        sel.cycle_lobby_slot_controller(slot); // 我方 -> 电脑
        assert!(matches!(
            sel.lobby_controllers[slot],
            SkirmishPlayerController::Ai(_)
        ));
        sel.cycle_lobby_slot_controller(slot); // 电脑 -> 关闭 (closeable again)
        assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::None);
    }

    // Manual harvesting must work for the human player (who no longer auto-harvests):
    // issuing a harvest order to a player Worker should make it gather ore and
    // deposit at the CommandCenter, so P0 ore grows.
    #[test]
    fn manual_harvest_grows_player_ore() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..20 {
            app.update();
        }
        let player = Team::Player(0);

        // Find a player worker and the nearest resource node.
        let worker = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<(Entity, &Team, &Unit), ()>();
            q.iter(world)
                .find(|(_, t, u)| **t == player && can_unit_collect_resources(u))
                .map(|(e, _, _)| e)
        }
        .expect("player should have a collector-capable worker");
        let (node, node_pos) = {
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &Transform, &ResourceNode)>();
            q.iter(world)
                .find(|(_, _, n)| n.amount > 0)
                .map(|(e, tf, _)| (e, tf.translation))
        }
        .expect("map should have a resource node");
        // Make sure the node is visible so it's a legal manual target.
        let _ = node_pos;

        let ore_before = app.world().resource::<Economies>().get(player).ore;
        app.world_mut().entity_mut(worker).insert(HarvestOrder {
            resource: Some(node),
            state: HarvestState::MovingToResource,
            collect_remaining: 0.0,
            last_kind: None,
        });
        for _ in 0..1200 {
            app.update();
        }
        let ore_after = app.world().resource::<Economies>().get(player).ore;
        eprintln!("[diag] manual harvest: P0 ore {ore_before} -> {ore_after}");
        assert!(
            ore_after > ore_before,
            "manual harvest did not grow ore ({ore_before} -> {ore_after})"
        );
    }

    // Lobby controller dropdown: toggling opens the option list, picking an option
    // sets the controller and closes the dropdown (no cycling).
    #[test]
    fn lobby_controller_dropdown_opens_and_sets() {
        let mut sel = SkirmishMenuSelection::default();
        let slot = 1;
        assert_eq!(sel.controller_dropdown_open, None);
        sel.toggle_controller_dropdown(slot);
        assert_eq!(
            sel.controller_dropdown_open,
            Some(slot),
            "toggle should open"
        );
        sel.set_lobby_slot_controller_choice(slot, SkirmishPlayerController::None);
        assert_eq!(
            sel.lobby_controllers[slot],
            SkirmishPlayerController::None,
            "picking 关闭 should close the slot"
        );
        assert_eq!(
            sel.controller_dropdown_open, None,
            "picking an option should close the dropdown"
        );
        sel.toggle_controller_dropdown(slot);
        sel.set_lobby_slot_controller_choice(slot, SkirmishPlayerController::Human);
        assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::Human);
    }

    #[test]
    fn map_and_resource_choices_close_dropdowns() {
        let mut sel = SkirmishMenuSelection::default();
        sel.toggle_map_dropdown();
        assert!(sel.map_dropdown_open);
        sel.set_map_choice(1);
        assert_eq!(sel.map_index, 1);
        assert!(!sel.map_dropdown_open);
        assert!(!sel.resources_dropdown_open);

        sel.toggle_resources_dropdown();
        assert!(sel.resources_dropdown_open);
        sel.set_starting_resource_choice(0);
        assert_eq!(sel.starting_resource_index, 0);
        assert!(!sel.map_dropdown_open);
        assert!(!sel.resources_dropdown_open);
    }

    #[test]
    fn setup_menu_rebuilds_map_and_resource_dropdown_options() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::SkirmishSetup);
        for _ in 0..8 {
            app.update();
        }

        let count_buttons = |app: &mut App, predicate: fn(MainMenuAction) -> bool| -> usize {
            let world = app.world_mut();
            let mut buttons = world.query::<&MainMenuButton>();
            buttons
                .iter(world)
                .filter(|button| predicate(button.action))
                .count()
        };

        assert_eq!(
            count_buttons(&mut app, |action| matches!(
                action,
                MainMenuAction::SelectMap(_)
            )),
            0,
            "closed map dropdown should not render stale map options"
        );
        app.world_mut()
            .resource_mut::<SkirmishMenuSelection>()
            .toggle_map_dropdown();
        app.update();
        assert_eq!(
            count_buttons(&mut app, |action| matches!(
                action,
                MainMenuAction::SelectMap(_)
            )),
            SKIRMISH_MAPS.len() + 1,
            "opening map dropdown should render every map plus random"
        );

        app.world_mut()
            .resource_mut::<SkirmishMenuSelection>()
            .toggle_resources_dropdown();
        app.update();
        assert_eq!(
            count_buttons(&mut app, |action| matches!(
                action,
                MainMenuAction::SelectMap(_)
            )),
            0,
            "opening resources dropdown should close map options"
        );
        assert_eq!(
            count_buttons(&mut app, |action| matches!(
                action,
                MainMenuAction::SelectStartingResources(_)
            )),
            GODOT_STARTING_RESOURCE_OPTIONS.len(),
            "opening resources dropdown should render every resource preset"
        );
    }

    // Faction dropdown: toggling opens it (and closes the controller dropdown);
    // picking a faction sets it and closes the dropdown.
    #[test]
    fn lobby_faction_dropdown_opens_and_sets() {
        let mut sel = SkirmishMenuSelection::default();
        let slot = 1;
        sel.toggle_controller_dropdown(slot);
        sel.toggle_faction_dropdown(slot);
        assert_eq!(
            sel.faction_dropdown_open,
            Some(slot),
            "faction toggle opens"
        );
        assert_eq!(
            sel.controller_dropdown_open, None,
            "opening faction closes the controller dropdown"
        );
        sel.set_lobby_slot_faction_choice(slot, SkirmishFaction::Chaos);
        assert_eq!(sel.lobby_factions[slot], SkirmishFaction::Chaos);
        assert_eq!(
            sel.faction_dropdown_open, None,
            "picking closes the dropdown"
        );
    }

    #[test]
    fn empty_battle_log_does_not_swallow_world_clicks() {
        // Mid-screen point inside the old fixed battle-log band (the harvest
        // harness clicked (619, 170) and lost the worker selection to it).
        let point = Vec2::new(619.0, 170.0);
        let empty = HudHitZones {
            world_rects: hud_world_input_rects(1280.0, 720.0, 0, 0, 0, 0, false),
        };
        assert!(!empty.blocks_world(point));
        let with_log = HudHitZones {
            world_rects: hud_world_input_rects(1280.0, 720.0, 0, 2, 0, 0, false),
        };
        assert!(with_log.blocks_world(point));
        assert!(
            !with_log.blocks_world(Vec2::new(619.0, 300.0)),
            "hit rect must stay proportional to visible rows"
        );
    }

    #[test]
    fn bottom_world_clicks_pass_outside_actual_hud_panels() {
        // The harvest harness right-clicked ore at (671.5, 584.6); the old
        // full-width bottom band swallowed it. With one command row + selection
        // panel visible, that point is open world and must NOT be blocked.
        let zones = HudHitZones {
            world_rects: hud_world_input_rects(1280.0, 720.0, 0, 0, 4, 0, true),
        };
        let ore_click = Vec2::new(671.5, 584.6);
        assert!(!zones.blocks_world(ore_click));
        // Inside the actual command card (bottom-right) it must block.
        assert!(zones.blocks_world(Vec2::new(1000.0, 700.0)));
        // Inside the minimap (bottom-left) it must block.
        assert!(zones.blocks_world(Vec2::new(80.0, 650.0)));
        // Inside the selection panel (bottom-center) it must block.
        assert!(zones.blocks_world(Vec2::new(400.0, 700.0)));
    }
}
