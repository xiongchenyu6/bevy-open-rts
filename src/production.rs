//! Production: the build queue and training, structure construction and
//! manual placement (validation, preview, foundations, sell/cancel).
//!
//! Pure move out of lib.rs (module split); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

pub(crate) const PRODUCTION_QUEUE_LIMIT: usize = 5;

pub(crate) const PRODUCTION_QUEUE_HUD_SLOT_COUNT: usize = 24;

pub(crate) const CONSTRUCTION_ENTRY_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;

pub(crate) const PRODUCTION_VETERANCY_PRODUCER_COUNT: usize = 3;

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct StructurePlacementFeedback {
    pub(crate) validity: Option<StructurePlacementValidity>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct UnderConstruction {
    pub(crate) remaining: f32,
    pub(crate) total: f32,
    pub(crate) cost: registry::Cost,
    pub(crate) free_worker_origin: Option<Vec3>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct RepairOrder {
    pub(crate) target: Entity,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ConstructOrder {
    pub(crate) target: Entity,
}

pub(crate) fn production_veterancy_slot(producer_id: &str) -> Option<usize> {
    match producer_id {
        "Barracks" => Some(0),
        "VehicleFactory" => Some(1),
        "AircraftFactory" => Some(2),
        _ => None,
    }
}

pub(crate) fn production_speed_multiplier(economy: &TeamEconomy) -> f32 {
    if economy.low_power() {
        LOW_POWER_PRODUCTION_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildStructureTab {
    Production,
    Defense,
}

impl Default for BuildStructureTab {
    fn default() -> Self {
        Self::Production
    }
}

impl BuildStructureTab {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Production => t("生产", "Production"),
            Self::Defense => t("防御", "Defense"),
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuildAction {
    None,
    Train(&'static str),
    Build(&'static str),
    SelectBuildTab(BuildStructureTab),
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
    SelectIdleWorker,
}

impl BuildAction {
    pub(crate) fn audio_command_key(self) -> Option<&'static str> {
        match self {
            Self::ToggleDeployMode => Some(COMMAND_KEY_TOGGLE_DEPLOY),
            Self::HoldPosition => Some(COMMAND_KEY_HOLD_POSITION),
            Self::GuardArea => Some(COMMAND_KEY_GUARD_AREA),
            Self::StopSelected => Some(COMMAND_KEY_CANCEL),
            Self::ScatterSelected => Some(COMMAND_KEY_SCATTER),
            Self::None
            | Self::Train(_)
            | Self::Build(_)
            | Self::SelectBuildTab(_)
            | Self::SelectIdleWorker
            | Self::SellStructure
            | Self::RepairStructure
            | Self::SetRallyPoint
            | Self::AttackMove
            | Self::Patrol => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BuildJob {
    pub(crate) team: Team,
    pub(crate) action: BuildAction,
    pub(crate) producer_entity: Entity,
    pub(crate) producer_id: &'static str,
    pub(crate) timer: f32,
    pub(crate) origin: Vec3,
}

#[derive(Resource, Default)]
pub(crate) struct BuildQueue(pub(crate) Vec<BuildJob>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnqueueBuildActionResult {
    Enqueued,
    NotEnoughResources,
    QueueFull,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StructurePlacementValidity {
    Valid,
    CollidesWithObject,
    NotEnoughResources,
    OutOfMap,
    MissingTech,
    OutOfBaseRadius,
    UnevenTerrain,
}

pub(crate) fn repair_order_range(
    capability: RepairCapability,
    source_radius: f32,
    target_radius: f32,
) -> f32 {
    if capability.radius > 0.0 {
        capability.radius + target_radius
    } else {
        source_radius + target_radius + REPAIR_ADHERENCE_MARGIN_M + REPAIR_ENTRY_MARGIN_M
    }
}

pub(crate) fn structure_placement_feedback_text(
    validity: StructurePlacementValidity,
) -> Option<&'static str> {
    match validity {
        StructurePlacementValidity::Valid => None,
        StructurePlacementValidity::CollidesWithObject => Some(t(
            "无法摆放: 与单位/建筑/资源重叠",
            "Can't place: overlaps a unit/building/resource",
        )),
        StructurePlacementValidity::NotEnoughResources => {
            Some(t("无法摆放: 资源不足", "Can't place: not enough resources"))
        }
        StructurePlacementValidity::OutOfMap => {
            Some(t("无法摆放: 超出地图边界", "Can't place: outside the map"))
        }
        StructurePlacementValidity::MissingTech => Some(t(
            "无法摆放: 缺少建造前置",
            "Can't place: missing prerequisite",
        )),
        StructurePlacementValidity::OutOfBaseRadius => {
            Some(t("无法摆放: 离基地太远", "Can't place: too far from base"))
        }
        StructurePlacementValidity::UnevenTerrain => {
            Some(t("无法摆放: 地形不平整", "Can't place: uneven terrain"))
        }
    }
}

pub(crate) fn structure_placement_input(
    mut commands: Commands,
    terrain: Res<TerrainHeightField>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut placement: StructurePlacementInputResources,
    structures: Query<StructurePrereqItem<'_>>,
    occupiers: Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
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
        (!cursor_is_over_hud(window, &placement.hud_zones))
            .then(|| pointer_ground(window, &camera_q, &terrain))
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
                &terrain,
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
        &terrain,
        &mut placement.economies,
        &structures,
        &occupiers,
    ) {
        Ok((entity, label)) => {
            placement.command_mode.pending_structure_placement = None;
            *placement.placement_feedback = StructurePlacementFeedback::default();
            if team == player_team {
                assign_selected_constructors_to_structure(
                    &mut commands,
                    team,
                    entity,
                    point,
                    &placement.selected_constructors,
                    &placement.constructors,
                );
                record_sound_audio_feedback(
                    &mut placement.audio_feedback,
                    SoundEffectKind::ConstructionStarted,
                );
                push_battle_log(
                    &mut placement.battle_log,
                    format!(
                        "{}: {}",
                        t("开始施工", "Construction started"),
                        localized_entity_label(label)
                    ),
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

pub(crate) fn assign_selected_constructors_to_structure(
    commands: &mut Commands,
    team: Team,
    target: Entity,
    target_position: Vec3,
    selected_constructors: &Query<
        (Entity, &Unit, &Team, &Health),
        (With<Selected>, With<Unit>, Without<Structure>),
    >,
    constructors: &Query<
        (Entity, &Unit, &Team, &Transform, &Health),
        (With<Unit>, Without<Structure>),
    >,
) -> bool {
    let mut assigned = false;
    for (entity, unit, unit_team, health) in selected_constructors {
        if *unit_team != team || health.current <= 0.0 || !can_unit_construct_structures(unit) {
            continue;
        }
        issue_unit_order(commands, entity, UnitQueuedOrder::Construct(target));
        assigned = true;
    }
    if assigned {
        return true;
    }

    let mut nearest = None;
    for (entity, unit, unit_team, transform, health) in constructors {
        if *unit_team != team || health.current <= 0.0 || !can_unit_construct_structures(unit) {
            continue;
        }
        let distance = xz_distance(transform.translation, target_position);
        if nearest.is_none_or(|(_, best_distance)| distance < best_distance) {
            nearest = Some((entity, distance));
        }
    }
    if let Some((entity, _)) = nearest {
        issue_unit_order(commands, entity, UnitQueuedOrder::Construct(target));
        assigned = true;
    }
    assigned
}

pub(crate) fn rotate_pending_structure_placement(command_mode: &mut CommandMode) -> bool {
    let Some(pending) = command_mode.pending_structure_placement.as_mut() else {
        return false;
    };
    pending.rotation_y_radians = normalize_structure_rotation_y(
        pending.rotation_y_radians + STRUCTURE_PLACEMENT_ROTATION_STEP_RADIANS,
    );
    true
}

pub(crate) fn begin_pending_structure_drag(pending: &mut PendingStructurePlacement, point: Vec3) {
    pending.position = Some(point);
    pending.drag_rotation_origin = Some(point);
}

#[allow(dead_code)]
pub(crate) fn begin_structure_placement_mode(
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

pub(crate) fn begin_structure_placement_mode_for_faction(
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

pub(crate) fn cancel_selected_under_construction_structure<'a>(
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

pub(crate) fn build_structure_tab_for(id: &str) -> BuildStructureTab {
    match id {
        "AntiGroundTurret"
        | "AntiAirTurret"
        | "TeslaFenceSegment"
        | "ArcCoilDefenseTower"
        | "LanceBeamDefenseTower"
        | "PrismDefenseObelisk"
        | "RailCannonBunker" => BuildStructureTab::Defense,
        _ => BuildStructureTab::Production,
    }
}

pub(crate) fn build_structure_order_compare(
    left: &&'static str,
    right: &&'static str,
) -> std::cmp::Ordering {
    build_structure_order_stage(left)
        .cmp(&build_structure_order_stage(right))
        .then_with(|| left.cmp(right))
}

pub(crate) fn build_structure_order_stage(id: &str) -> u8 {
    match id {
        // Opening: power first, then economy and core production.
        "PowerReactor" => 0,
        "Refinery" => 5,
        "Barracks" => 10,
        "RepairPad" => 14,
        // Early defense before tech expansion.
        "AntiGroundTurret" => 18,
        "AntiAirTurret" => 20,
        "TeslaFenceSegment" => 22,
        // Scouting/tech unlocks and mid-game production.
        "RadarUplink" => 26,
        "VehicleFactory" => 30,
        "OrePurifier" => 34,
        "AdvancedReactorPlant" => 36,
        "TechLab" => 40,
        "AircraftFactory" => 44,
        "RoboticsBay" => 48,
        // Late defenses and super-weapons stay at the end of the grid.
        "ArcCoilDefenseTower" => 56,
        "LanceBeamDefenseTower" => 58,
        "PrismDefenseObelisk" => 60,
        "RailCannonBunker" => 62,
        "WeatherControlSpire" => 70,
        _ => 90,
    }
}

pub(crate) fn production_structure_hotkey_select_all(
    alt: bool,
    ctrl: bool,
    shift: bool,
    just_pressed: bool,
) -> Option<bool> {
    (alt && !ctrl && just_pressed).then_some(shift)
}

pub(crate) fn production_batch_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)
}

pub(crate) fn sell_selected_structures(
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

pub(crate) fn construction_cancel_refund(cost: registry::Cost) -> (i32, i32) {
    (cost.ore, cost.crystal)
}

pub(crate) fn cancel_latest_queued_product(
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

pub(crate) fn cancel_latest_queued_product_for_producers(
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

pub(crate) fn cancel_queued_job_at_local_index(
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

pub(crate) fn production_occupied_spawn_points(
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

#[allow(dead_code)]
pub(crate) fn enqueue_build_action(
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

pub(crate) fn enqueue_build_action_for_faction(
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
        | BuildAction::None => EnqueueBuildActionResult::Unavailable,
    }
}

pub(crate) fn build_queue_has_capacity(build_queue: &BuildQueue, producer_entity: Entity) -> bool {
    producer_build_queue_len(build_queue, producer_entity) < PRODUCTION_QUEUE_LIMIT
}

pub(crate) fn enqueue_build_jobs_for_producers(
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
pub(crate) fn production_origins_for(
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

pub(crate) fn production_origins_for_faction(
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

pub(crate) fn build_target_product(action: BuildAction) -> &'static str {
    match action {
        BuildAction::Train(product) | BuildAction::Build(product) => product,
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
        | BuildAction::None => "",
    }
}

pub(crate) fn update_construct_orders(
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

pub(crate) fn production_queue_hud_entries(
    team: Team,
    build_queue: &BuildQueue,
    producer_entities: &[Entity],
) -> Vec<ProductionQueueHudEntry> {
    let mut entries: Vec<ProductionQueueHudEntry> = Vec::new();
    for producer_entity in producer_entities {
        let mut local_index = 0usize;
        for job in build_queue
            .0
            .iter()
            .filter(|job| job.team == team && job.producer_entity == *producer_entity)
        {
            // Aggregate consecutive same-type jobs into one slot with a ×N count
            // (e.g. queueing 3 workers shows one 工人 slot, not three).
            if let Some(last) = entries.last_mut() {
                if last.producer_entity == *producer_entity && last.action == job.action {
                    last.count += 1;
                    local_index += 1;
                    continue;
                }
            }
            let progress = registry::entity(build_target_product(job.action))
                .map(|def| production_job_progress(job, def))
                .unwrap_or(100.0);
            entries.push(ProductionQueueHudEntry {
                producer_entity: *producer_entity,
                local_index,
                action: job.action,
                progress,
                active: local_index == 0,
                count: 1,
            });
            local_index += 1;
        }
    }
    entries
}

pub(crate) fn structure_has_production_queue(structure_id: &str) -> bool {
    matches!(
        structure_id,
        "CommandCenter" | "Barracks" | "VehicleFactory" | "AircraftFactory"
    )
}

pub(crate) fn production_job_progress(job: &BuildJob, def: &registry::EntityDef) -> f32 {
    if def.build_seconds <= 0.0 {
        return 100.0;
    }
    ((def.build_seconds - job.timer).max(0.0) / def.build_seconds * 100.0).clamp(0.0, 100.0)
}

pub(crate) fn build_action_target_label(action: BuildAction) -> Option<String> {
    let id = match action {
        BuildAction::Train(id) | BuildAction::Build(id) => id,
        _ => return None,
    };
    registry::entity(id).map(|_| localized_compact_entity_label(id))
}

pub(crate) fn draw_structure_placement_preview(
    gizmos: &mut Gizmos,
    pending: PendingStructurePlacement,
    team: Team,
    faction: SkirmishFaction,
    point: Vec3,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
) {
    let Some(def) = registry::entity(pending.id) else {
        return;
    };
    let validity = structure_placement_validity_for_faction(
        team, faction, pending.id, point, bounds, terrain, economies, structures, occupiers,
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

#[allow(dead_code)]
pub(crate) fn spawn_structure_under_construction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_worker_origin: Option<Vec3>,
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
        free_worker_origin,
        rotation_y_radians,
        visible_team,
        default_visual_faction(team),
    )
}

pub(crate) fn spawn_structure_under_construction_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_worker_origin: Option<Vec3>,
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
        free_worker_origin,
        rotation_y_radians,
        visible_team,
        Some(faction),
    )
}

pub(crate) fn spawn_structure_under_construction_with_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    free_worker_origin: Option<Vec3>,
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
            free_worker_origin,
        },
        Health {
            current: 1.0,
            max: def.health,
        },
    ));
    entity
}

pub(crate) fn progress_under_construction_structures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
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
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for (entity, structure, team, transform, visual_faction, mut health, mut construction) in
        &mut structures
    {
        // AI-controlled structures self-construct: the AI keeps its workers
        // gathering and doesn't reliably free them to build, so advance its
        // buildings automatically (RA2-style auto-construction).
        if controlled_team != Some(*team) && construction.remaining > 0.0 {
            construction.remaining = (construction.remaining
                - STRUCTURE_CONSTRUCTION_PROGRESS_PER_SECOND * time.delta_secs())
            .max(0.0);
        }
        let progress = structure_construction_progress(*construction);
        health.current = structure_construction_health(health.max, progress);
        if construction.remaining > 0.0 {
            continue;
        }

        health.current = health.max;
        commands.entity(entity).try_remove::<UnderConstruction>();
        // A completed refinery grants a Worker for every team. Human-owned
        // workers still gather manually, so the player keeps direct control.
        if let Some(origin) = construction.free_worker_origin {
            let spawn_seed = next_id.0 + 17;
            spawn_refinery_free_worker(
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
            let label = localized_entity_label(structure.id);
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::ProductionReady);
            record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::ConstructionComplete);
            record_production_ready_battle_log(
                *team,
                player_team,
                true,
                &label,
                transform.translation,
                &mut battle_log,
            );
        }
    }
}

#[allow(dead_code)]
pub(crate) fn structure_placement_validity(
    team: Team,
    id: &'static str,
    point: Vec3,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
) -> StructurePlacementValidity {
    structure_placement_validity_for_faction(
        team,
        SkirmishFaction::from_team(team),
        id,
        point,
        bounds,
        terrain,
        economies,
        structures,
        occupiers,
    )
}

pub(crate) fn structure_placement_validity_for_faction(
    team: Team,
    faction: SkirmishFaction,
    id: &'static str,
    point: Vec3,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
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
    if !terrain_site_is_buildable(terrain, point, def.radius) {
        return StructurePlacementValidity::UnevenTerrain;
    }
    StructurePlacementValidity::Valid
}

/// Structures need level ground: every sampled footprint point must sit within
/// one climbable step of the center height (rules out cliff edges and ramps).
pub(crate) fn terrain_site_is_buildable(
    terrain: &TerrainHeightField,
    point: Vec3,
    radius: f32,
) -> bool {
    if terrain.is_flat() {
        return true;
    }
    let mut lowest = terrain.height_at(point);
    let mut highest = lowest;
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::TAU / 8.0;
        let sample = point + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
        let height = terrain.height_at(sample);
        lowest = lowest.min(height);
        highest = highest.max(height);
    }
    highest - lowest <= TERRAIN_MAX_STEP_M
}

pub(crate) fn selected_under_construction_stop_target<'a>(
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

pub(crate) fn process_build_queue(
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
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
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
            | BuildAction::SelectBuildTab(_) => {
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
                            last_kind: None,
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
                        issue_spawned_unit_rally_order(
                            &mut commands,
                            spawned,
                            def,
                            rally_target,
                            rally_point,
                        );
                    }
                } else if let Some(rally_target) =
                    rally_point.and_then(|rally_point| rally_point.target)
                {
                    issue_spawned_unit_rally_order(
                        &mut commands,
                        spawned,
                        def,
                        rally_target,
                        rally_point,
                    );
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
                        &localized_entity_label(def.id),
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
                    let free_worker_origin = (id == "Refinery").then_some(origin);
                    spawn_structure_under_construction_for_faction(
                        &mut commands,
                        &asset_server,
                        &mut next_id,
                        id,
                        team,
                        spawn_at,
                        free_worker_origin,
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
                            format!(
                                "{}: {}",
                                t("开始施工", "Construction started"),
                                localized_entity_label(def.id)
                            ),
                            Some(spawn_at),
                        );
                    }
                } else {
                    build_queue.0.remove(index);
                }
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
            | BuildAction::None => {}
        }
    }
}
