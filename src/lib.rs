#[cfg(feature = "audio")]
use bevy::audio::Volume;
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin, UntypedHandle},
    camera::primitives::Aabb,
    ecs::query::Or,
    ecs::system::SystemParam,
    gizmos::config::GizmoConfigStore,
    input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
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
mod campaign;
pub(crate) use campaign::*;
mod terrain;
pub(crate) use terrain::*;
mod maps;
pub(crate) use maps::*;
mod support_powers;
pub(crate) use support_powers::*;
mod spawn;
pub(crate) use spawn::*;
mod selection;
pub(crate) use selection::*;
mod command_card;
pub(crate) use command_card::*;
mod fog;
pub(crate) use fog::*;
mod match_screens;
pub(crate) use match_screens::*;
mod combat_vfx;
pub(crate) use combat_vfx::*;
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
    CampaignMenu,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpawnSpec {
    pub(crate) id: &'static str,
    pub(crate) offset: (f32, f32),
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
    pub(crate) victory_condition: VictoryCondition,
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
            victory_condition: VictoryCondition::default(),
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
    pub(crate) victory_condition_index: usize,
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
    pub(crate) victory_dropdown_open: bool,
}

impl Default for SkirmishMenuSelection {
    fn default() -> Self {
        Self {
            map_index: 0,
            starting_resource_index: DEFAULT_STARTING_RESOURCE_INDEX,
            victory_condition_index: 0,
            match_mode: SkirmishMatchMode::OneVsOne,
            ai_difficulty: AiDifficulty::Easy,
            lobby_controllers: DEFAULT_LOBBY_CONTROLLERS,
            controller_dropdown_open: None,
            faction_dropdown_open: None,
            team_dropdown_open: None,
            color_dropdown_open: None,
            map_dropdown_open: false,
            resources_dropdown_open: false,
            victory_dropdown_open: false,
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
            victory_condition_index: VictoryCondition::ALL
                .iter()
                .position(|mode| *mode == settings.victory_condition)
                .unwrap_or(0),
            victory_dropdown_open: false,
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
            victory_condition: VictoryCondition::ALL
                .get(self.victory_condition_index % VictoryCondition::ALL.len())
                .copied()
                .unwrap_or_default(),
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
        self.victory_dropdown_open = false;
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

    pub(crate) fn toggle_victory_dropdown(&mut self) {
        let was_open = self.victory_dropdown_open;
        self.close_all_lobby_dropdowns();
        self.victory_dropdown_open = !was_open;
    }

    pub(crate) fn set_victory_condition_choice(&mut self, index: usize) {
        if index < VictoryCondition::ALL.len() {
            self.victory_condition_index = index;
        }
        self.close_all_lobby_dropdowns();
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

pub(crate) fn clear_active_mission(mut active: ResMut<ActiveMission>) {
    active.0 = None;
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

pub(crate) fn skirmish_active_teams_from_controllers(
    controllers: &[SkirmishPlayerController],
) -> Vec<bool> {
    controllers
        .iter()
        .copied()
        .map(SkirmishPlayerController::is_active)
        .collect()
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

#[derive(Clone, Copy)]
pub(crate) struct TeamStartup {
    pub(crate) structures: &'static [SpawnSpec],
    pub(crate) units: &'static [SpawnSpec],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VictoryCondition {
    /// godot default: a team stays alive while any command center OR worker lives.
    #[default]
    Annihilation,
    /// Headquarters mode: lose every command center and you are out.
    Headquarters,
}

impl VictoryCondition {
    pub(crate) const ALL: [Self; 2] = [Self::Annihilation, Self::Headquarters];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Annihilation => t("歼灭", "Annihilation"),
            Self::Headquarters => t("斩首(摧毁指挥中心)", "Headquarters"),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupLoadoutMode {
    PlaytestExpanded,
    GodotSkirmish,
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
        .init_resource::<ActiveMission>()
        .init_resource::<TerrainHeightField>()
        .init_resource::<MissionTriggerState>()
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
        .init_resource::<TacticalPause>()
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
    .add_systems(OnEnter(AppScreen::SkirmishSetup), clear_active_mission)
    .add_systems(OnEnter(AppScreen::CampaignMenu), setup_campaign_menu)
    .add_systems(
        Update,
        campaign_menu_buttons.run_if(in_state(AppScreen::CampaignMenu)),
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
    // godot's WorldEnvironment: white ambient over a procedural sky.
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 340.0,
        affects_lightmapped_meshes: true,
    });
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
    mut pause: ResMut<TacticalPause>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    *match_speed = MatchSpeed::default();
    pause.0 = false;
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
                tactical_pause_hotkey,
                clear_tactical_pause_on_speed_change,
                run_mission_triggers,
                check_mission_victory,
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
                settle_new_entities_on_terrain.before(rebuild_nav_grid),
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
            (
                update_construction_work_pulses,
                animate_construction_workers,
                animate_structure_construction,
                apply_construction_ghost_material,
                update_combat_wreckage,
            )
                .chain()
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
            update_damage_numbers,
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
    pub(crate) terrain: Res<'w, TerrainHeightField>,
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
    pub(crate) terrain: Res<'w, TerrainHeightField>,
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
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
}

#[derive(Component)]
pub(crate) struct ButtonLabel;

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
    mut terrain_field: ResMut<TerrainHeightField>,
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

    // Sun direction and warm ambient mirror godot's Match.tscn
    // (DirectionalLight3D basis -Z, white ambient, procedural sky).
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 16_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 38.0, 0.0).looking_to(Vec3::new(0.534, -0.784, -0.318), Vec3::Y),
        MatchScopedEntity,
    ));

    terrain_field.rebuild(skirmish_map);
    let terrain_mesh_handle = if terrain_field.is_flat() {
        meshes.add(
            Plane3d::default()
                .mesh()
                .size(skirmish_map.size.0, skirmish_map.size.1),
        )
    } else {
        meshes.add(terrain_mesh(&terrain_field))
    };
    commands.spawn((
        Name::new(format!("{} Terrain", skirmish_map.name)),
        Mesh3d(terrain_mesh_handle),
        // godot terrain.material.tres: warm sand albedo (0.96, 0.745, 0.655).
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.96, 0.745, 0.655),
            perceptual_roughness: 0.95,
            ..default()
        })),
        bevy_rts_camera::Ground,
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
            terrain_field.max_height() + FOG_OVERLAY_Y,
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
    spawn_terrain_walls(&mut commands, &asset_server, skirmish_map);
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

pub(crate) const DEMON_STRUCTURE_WEAPON_DAMAGE_MULTIPLIER: f32 = 1.12;
pub(crate) const CHAOS_INCOMING_WEAPON_DAMAGE_SCALE: f32 = 0.9;

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

pub(crate) fn update_match_clock(mut match_state: ResMut<MatchState>, time: Res<Time>) {
    if match_state.is_running() {
        match_state.start_time_sec += time.delta_secs();
    }
}

pub(crate) fn evaluate_match_end(
    mut match_state: ResMut<MatchState>,
    mut match_flow: ResMut<MatchFlow>,
    mut audio_feedback: ResMut<AudioFeedback>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    setup_settings: Res<MatchSetupSettings>,
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

    // Headquarters mode: only command centers keep a team alive; surviving
    // workers cannot rebuild you back into the game.
    if setup_settings.victory_condition == VictoryCondition::Annihilation {
        for (unit, team, health) in &units {
            if is_worker_elimination_anchor(unit) && health.current > 0.0 {
                record_active_elimination_anchor(
                    *team,
                    &mut active_anchor_team,
                    &mut active_anchor_count,
                );
            }
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

pub(crate) fn selected_query_has_owned_voice_unit(
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
    team: Team,
) -> bool {
    selected_units
        .iter()
        .any(|(_, unit, unit_team, ..)| *unit_team == team && is_voice_unit(unit))
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
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
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

pub(crate) fn window_size(window: &Window) -> Vec2 {
    Vec2::new(window.width(), window.height())
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

pub(crate) fn entity_pair_hash(a: Entity, b: Entity) -> u64 {
    let mut x = a.to_bits().wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ b.to_bits().wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
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
    pub(crate) construction: Query<
        'w,
        's,
        (
            &'static Transform,
            &'static Selectable,
            &'static Team,
            &'static ConstructionWorkPulse,
            Option<&'static VisibilityState>,
        ),
        With<Unit>,
    >,
    pub(crate) construction_targets: Query<
        'w,
        's,
        (
            &'static Transform,
            &'static Selectable,
            Option<&'static UnderConstruction>,
            Option<&'static VisibilityState>,
        ),
        With<Structure>,
    >,
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
    for (transform, selectable, team, pulse, visibility) in &vfx.construction {
        if !visibility.is_none_or(|visibility| visibility.visible) {
            continue;
        }
        let Ok((target_transform, target_selectable, under_construction, target_visibility)) =
            vfx.construction_targets.get(pulse.target)
        else {
            continue;
        };
        if under_construction.is_none()
            || !target_visibility.is_none_or(|visibility| visibility.visible)
        {
            continue;
        }
        draw_construction_work_visuals(
            &mut gizmos,
            &mut hud,
            transform.translation,
            selectable.radius,
            target_transform.translation,
            target_selectable.radius,
            *team,
            pulse,
            &player_colors,
            vfx.time.elapsed_secs(),
        );
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
                .then(|| {
                    pointer_ground(
                        window,
                        &placement_preview.camera_q,
                        &placement_preview.terrain,
                    )
                })
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
            &placement_preview.terrain,
            &placement_preview.economies,
            &placement_preview.structures,
            &placement_preview.occupiers,
        );
    }
}

pub(crate) fn draw_construction_work_visuals(
    gizmos: &mut Gizmos,
    hud: &mut Gizmos<HudGizmos>,
    worker_position: Vec3,
    worker_radius: f32,
    target_position: Vec3,
    target_radius: f32,
    team: Team,
    pulse: &ConstructionWorkPulse,
    player_colors: &PlayerColorSlots,
    elapsed_secs: f32,
) {
    let mut to_target = Vec3::new(
        target_position.x - worker_position.x,
        0.0,
        target_position.z - worker_position.z,
    );
    if to_target.length_squared() <= f32::EPSILON {
        to_target = Vec3::Z;
    } else {
        to_target = to_target.normalize();
    }
    let side = Vec3::new(-to_target.z, 0.0, to_target.x).normalize_or(Vec3::X);
    let active = if pulse.total > 0.0 {
        (pulse.remaining / pulse.total).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let team_color = player_colors.color(team).to_srgba();
    let core = Color::srgba(
        (0.58 + team_color.red * 0.30).min(1.0),
        (0.88 + team_color.green * 0.10).min(1.0),
        (0.95 + team_color.blue * 0.05).min(1.0),
        0.32 + active * 0.52,
    );
    let hot = Color::srgba(1.0, 0.88, 0.30, 0.36 + active * 0.54);
    let smoke = Color::srgba(0.15, 0.22, 0.20, 0.12 + active * 0.18);
    let source = worker_position + to_target * (worker_radius + 0.16) + Vec3::Y * 0.42;
    let contact = target_position - to_target * (target_radius * 0.68) + Vec3::Y * 0.34;
    hud.line(source, contact, core);
    hud.line(
        source + side * 0.07 + Vec3::Y * 0.05,
        contact + side * 0.04 + Vec3::Y * 0.03,
        hot,
    );
    let phase = elapsed_secs * 13.0 + pulse.seed;
    for i in 0..7 {
        let seed = i as f32 * 2.27 + pulse.seed;
        let burst = (phase + seed).sin() * 0.5 + 0.5;
        let travel = 0.22 + 0.55 * ((phase * 0.53 + seed).cos() * 0.5 + 0.5);
        let center = source.lerp(contact, travel)
            + side * ((phase + seed).cos() * 0.10)
            + Vec3::Y * (0.05 + burst * 0.18);
        let spark_color = if i % 2 == 0 { hot } else { core };
        hud.line(
            center - side * (0.04 + burst * 0.04),
            center + side * (0.08 + burst * 0.05) + Vec3::Y * (0.03 + burst * 0.05),
            spark_color,
        );
        gizmos.circle(
            Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            0.025 + burst * 0.035,
            spark_color,
        );
    }
    gizmos.circle(
        Isometry3d::new(
            contact - Vec3::Y * 0.27,
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        (0.28 + 0.10 * (phase * 0.8).sin().abs()).min(target_radius * 0.8),
        smoke,
    );
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
        StructurePlacementValidity::OutOfMap
        | StructurePlacementValidity::CollidesWithObject
        | StructurePlacementValidity::UnevenTerrain => Color::srgba(1.0, 0.2, 0.16, 0.9),
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
mod match_tests;
