//! Orders and selection: click/drag selection, the command card action logic,
//! unit orders (+queue), rally points, control groups and idle-worker cycling.
//!
//! Pure move out of lib.rs (module split); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

#[derive(Resource)]
pub(crate) struct CommandMode {
    pub(crate) attack_move: bool,
    pub(crate) patrol: bool,
    pub(crate) rally_point: bool,
    pub(crate) support_power: Option<SupportPowerKind>,
    pub(crate) pending_structure_placement: Option<PendingStructurePlacement>,
}

impl CommandMode {
    pub(crate) fn has_pending_interaction(&self) -> bool {
        self.attack_move
            || self.patrol
            || self.rally_point
            || self.support_power.is_some()
            || self.pending_structure_placement.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RallyMode {
    Move,
    AttackMove,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct RallyPoint {
    pub(crate) target: Option<Vec3>,
    pub(crate) target_unit: Option<Entity>,
    pub(crate) mode: RallyMode,
}

#[derive(Component)]
pub(crate) struct DeployModeToggleRequest;

#[derive(Component)]
pub(crate) struct ClickMarker {
    pub(crate) ttl: f32,
    pub(crate) radius: f32,
    pub(crate) kind: ClickMarkerKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClickMarkerKind {
    Move,
    Harvest,
    Attack,
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

#[derive(Component, Clone, Copy)]
pub(crate) struct MoveOrder {
    pub(crate) target: Vec3,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct FollowOrder {
    pub(crate) target: Entity,
    pub(crate) allow_enemy: bool,
    pub(crate) offset: Vec3,
}

#[derive(Clone)]
pub(crate) enum UnitQueuedOrder {
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
pub(crate) struct OrderTargetChoices {
    pub(crate) supply_crate_position: Option<Vec3>,
    pub(crate) resource_target: Option<Entity>,
    pub(crate) resource_dropoff_target: Option<Entity>,
    pub(crate) enemy_target: Option<Entity>,
    pub(crate) repair_target: Option<Entity>,
    pub(crate) construct_target: Option<Entity>,
    pub(crate) garrison_target: Option<Entity>,
    pub(crate) follow_target: Option<Entity>,
}

impl OrderTargetChoices {
    pub(crate) fn force_follow_target(self) -> Option<Entity> {
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
pub(crate) struct UnitOrderContext {
    pub(crate) force_move: bool,
    pub(crate) enemy_target_capturable: bool,
    pub(crate) attack_move: bool,
    pub(crate) patrol: bool,
    pub(crate) origin: Vec3,
    pub(crate) point: Vec3,
    pub(crate) offset: Vec3,
}

#[derive(Component)]
pub(crate) struct OrderQueue {
    pub(crate) orders: VecDeque<UnitQueuedOrder>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct HoldPosition {
    pub(crate) enabled: bool,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct AttackOrder {
    pub(crate) target: Entity,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct CaptureOrder {
    pub(crate) target: Entity,
    pub(crate) elapsed: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct GarrisonOrder {
    pub(crate) target: Entity,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct AttackMoveOrder {
    pub(crate) destination: Vec3,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct PatrolOrder {
    pub(crate) origin: Vec3,
    pub(crate) destination: Vec3,
    pub(crate) moving_to_destination: bool,
}

pub(crate) fn is_rally_point_structure(id: &str) -> bool {
    matches!(
        id,
        "CommandCenter" | "Barracks" | "VehicleFactory" | "AircraftFactory"
    )
}

pub(crate) fn can_repair_order_target(
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

pub(crate) fn update_click_markers(
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

pub(crate) fn issue_orders(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,

    visible_player: Res<VisiblePlayer>,
    hud_zones: Res<HudHitZones>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut order_resources: OrderResources,
    mut selected_params: ParamSet<(
        Query<SelectedOrderUnitItem<'_>, SelectedOrderUnitFilter>,
        Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
    )>,
    selectable_q: Query<SelectableOrderTargetItem<'_>>,

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
    if cursor_blocks_world_order_controls(cursor, &hud_zones) {
        if std::env::var_os("RTS_SELECT_DIAG").is_some() {
            eprintln!("[select-diag] right-click swallowed by HUD at {cursor:?}");
        }
        return;
    }
    let Some(raw_point) = pointer_ground(window, &camera_q, &order_resources.terrain) else {
        return;
    };
    let Some(point) = validated_terrain_target_in_bounds(raw_point, *order_resources.map_bounds)
    else {
        return;
    };

    if order_resources.command_mode.support_power.is_some() {
        // Right-click cancels an armed support power (left-click fires it —
        // see fire_support_power_on_left_click).
        order_resources.command_mode.support_power = None;
        return;
    }

    if order_resources.command_mode.rally_point {
        let rally_unit_target = rally_target_at(point, visible_team, &selectable_q);
        let set_any = apply_selected_rally_points(
            visible_team,
            point,
            rally_unit_target,
            RallyMode::Move,
            *order_resources.map_bounds,
            &mut selected_params.p1(),
        );
        if set_any {
            commands.spawn((
                Transform::from_translation(point + Vec3::Y * 0.04),
                ClickMarker {
                    ttl: CLICK_MARKER_TTL_SECONDS,
                    radius: CLICK_MARKER_RADIUS_START,
                    kind: ClickMarkerKind::Move,
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
    let selected_unit_count = selected.len();
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
            true,
            has_active,
            queue,
        );
        commands
            .entity(entity)
            .try_insert(HoldPosition { enabled: false });
    }

    let set_attack_rally_any = if selected_unit_count == 0
        && order_resources.command_mode.attack_move
        && terrain_target_only
    {
        apply_selected_rally_points(
            visible_team,
            point,
            None,
            RallyMode::AttackMove,
            *order_resources.map_bounds,
            &mut selected_params.p1(),
        )
    } else {
        false
    };

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
                RallyMode::Move,
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
    if issued_any || set_rally_any || set_attack_rally_any {
        // A harvest order plants its "deploy-to-mine" flag ON the targeted ore,
        // not on the empty click point; everything else gets the white move ring.
        let harvest_pos = if has_selected_resource_collector && enemy_target.is_none() {
            resource_target
                .and_then(|entity| resource_targets.get(entity).ok())
                .map(|(_, transform, ..)| transform.translation)
        } else {
            None
        };
        // Right-clicking an enemy plants a red attack marker on it so the order
        // reads as "attack", not a plain move.
        let enemy_pos = enemy_target
            .and_then(|entity| selectable_q.get(entity).ok())
            .map(|item| item.1.translation);
        let (marker_pos, marker_kind) = if set_attack_rally_any {
            (point, ClickMarkerKind::Attack)
        } else if let Some(enemy) = enemy_pos {
            (enemy, ClickMarkerKind::Attack)
        } else if let Some(ore) = harvest_pos {
            (ore, ClickMarkerKind::Harvest)
        } else {
            (point, ClickMarkerKind::Move)
        };
        commands.spawn((
            Transform::from_translation(marker_pos + Vec3::Y * 0.03),
            ClickMarker {
                ttl: CLICK_MARKER_TTL_SECONDS,
                radius: CLICK_MARKER_RADIUS_START,
                kind: marker_kind,
            },
            MatchScopedEntity,
        ));
    }
}

pub(crate) fn should_set_terrain_rally_points(
    queue_mode: bool,
    attack_move: bool,
    patrol: bool,
) -> bool {
    !queue_mode && !attack_move && !patrol
}

pub(crate) fn apply_selected_terrain_rally_points(
    visible_team: Team,
    target: Vec3,
    bounds: MapBounds,
    rally_points: &mut Query<(&Team, &mut RallyPoint), SelectedRallyPointFilter>,
) -> bool {
    apply_selected_rally_points(
        visible_team,
        target,
        None,
        RallyMode::Move,
        bounds,
        rally_points,
    )
}

pub(crate) fn apply_selected_rally_points(
    visible_team: Team,
    target: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
    mode: RallyMode,
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
                mode,
                bounds,
            )
        {
            set_any = true;
        }
    }
    set_any
}

pub(crate) fn desired_order_for_selected_unit(
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
        // Only armed units attack. A worker right-clicking an enemy
        // should fall through to a plain move, not uselessly chase a unit it
        // cannot damage.
        if registry::entity(unit.id).is_some_and(|def| def.weapon.is_some()) {
            return Some(UnitQueuedOrder::Attack(target));
        }
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
pub(crate) fn apply_rally_point_command(
    rally_point: &mut RallyPoint,
    point: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
) -> bool {
    apply_rally_point_command_in_bounds(
        rally_point,
        point,
        rally_unit_target,
        RallyMode::Move,
        MapBounds::default(),
    )
}

pub(crate) fn apply_rally_point_command_in_bounds(
    rally_point: &mut RallyPoint,
    point: Vec3,
    rally_unit_target: Option<(Entity, Vec3)>,
    mode: RallyMode,
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
    rally_point.mode = mode;
    true
}

pub(crate) fn update_rally_point_targets(
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

pub(crate) fn refresh_rally_point_target(
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

pub(crate) fn rally_target_at(
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

pub(crate) fn nearest_repair_order_target(
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

pub(crate) fn nearest_construct_order_target(
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

pub(crate) fn nearest_follow_order_target(
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

pub(crate) fn nearest_garrison_target(
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

pub(crate) fn can_garrison_structure_target(
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

pub(crate) fn nearest_enemy_order_target(
    point: Vec3,
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    visible_team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Option<Entity> {
    enemy_target_at_cursor(cursor, camera_q, visible_team, selectable_q)
        .or_else(|| nearest_enemy_target_with_snap_radius(point, visible_team, selectable_q, 0.45))
}

pub(crate) fn enemy_target_at_cursor(
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

pub(crate) fn can_unit_garrison(unit: &Unit) -> bool {
    is_infantry_unit(unit)
}

pub(crate) fn unit_has_movement_trait(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| def.speed > 0.0)
}

pub(crate) fn can_unit_guard_area(unit: &Unit) -> bool {
    unit_has_movement_trait(unit)
        && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
}

pub(crate) fn unit_supports_hold_position(unit: &Unit) -> bool {
    can_unit_guard_area(unit)
}

pub(crate) fn unit_supports_attack_move(unit: &Unit) -> bool {
    unit.speed > 0.0 && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
}

pub(crate) fn unit_supports_patrol(unit: &Unit) -> bool {
    unit.speed > 0.0
}

pub(crate) fn update_command_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    structures: Query<StructurePrereqItem<'_>>,
    mut command_mode: ResMut<CommandMode>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
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
        if !keyboard.just_pressed(power.hotkey()) {
            continue;
        }
        if player_support_power_available(
            visible_team,
            power,
            &economies,
            &support_cooldowns,
            &structures,
        ) {
            toggle_support_power_mode(&mut command_mode, power);
        } else {
            // A silent no-op here read as "F1 does nothing" — say WHY the
            // power can't fire right now.
            let def = power.definition();
            let missing = support_power_missing_requirement_labels(
                visible_team,
                def.requirements,
                &structures,
            );
            let reason = if !missing.is_empty() {
                format!("{}: {}", t("需要", "Requires"), missing.join(" + "))
            } else if def.requires_power && economies.get(visible_team).low_power() {
                t("电力不足", "Low power").to_string()
            } else {
                t("冷却中", "Cooling down").to_string()
            };
            push_battle_log(
                &mut battle_log,
                format!(
                    "{} {}: {}",
                    power.label(),
                    t("不可用", "unavailable"),
                    reason
                ),
                None,
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::Error);
        }
        return;
    }
}

pub(crate) fn clear_targeting_modes(command_mode: &mut CommandMode) {
    command_mode.attack_move = false;
    command_mode.patrol = false;
    command_mode.rally_point = false;
    command_mode.support_power = None;
    command_mode.pending_structure_placement = None;
}

pub(crate) fn begin_attack_move_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.attack_move = true;
    true
}

pub(crate) fn toggle_attack_move_mode(command_mode: &mut CommandMode) -> bool {
    if command_mode.attack_move {
        clear_targeting_modes(command_mode);
        return false;
    }
    begin_attack_move_mode(command_mode, true)
}

pub(crate) fn begin_patrol_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.patrol = true;
    true
}

pub(crate) fn toggle_patrol_mode(command_mode: &mut CommandMode) -> bool {
    if command_mode.patrol {
        clear_targeting_modes(command_mode);
        return false;
    }
    begin_patrol_mode(command_mode, true)
}

pub(crate) fn begin_rally_point_mode(command_mode: &mut CommandMode, enabled: bool) -> bool {
    if !enabled || command_mode.support_power.is_some() {
        return false;
    }
    clear_targeting_modes(command_mode);
    command_mode.rally_point = true;
    true
}

pub(crate) fn toggle_selected_hold_position<'a>(
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

pub(crate) fn guard_selected_area<'a>(
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

pub(crate) fn stop_selected_units(
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

pub(crate) fn stop_selected_entities(
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

pub(crate) fn scatter_selected_units(
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

pub(crate) fn scatter_selected_positions(
    commands: &mut Commands,
    units: &[(Entity, Vec3)],
) -> bool {
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

pub(crate) fn scatter_target_positions(positions: &[Vec3]) -> Vec<Vec3> {
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

pub(crate) fn progress_queued_orders(
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

pub(crate) fn clear_emp_disabled_orders(
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

pub(crate) fn issue_unit_order(commands: &mut Commands, entity: Entity, order: UnitQueuedOrder) {
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
                last_kind: None,
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

pub(crate) fn issue_or_queue_unit_order(
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

pub(crate) fn should_queue_selected_order(
    queue_mode: bool,
    allow_queue: bool,
    has_active: bool,
    queue: Option<&OrderQueue>,
) -> bool {
    allow_queue
        && queue_mode
        && (has_active || queue.is_some_and(|order_queue| !order_queue.orders.is_empty()))
}

pub(crate) fn clear_order_state(commands: &mut Commands, entity: Entity) {
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

pub(crate) fn clear_non_attack_order_state(commands: &mut Commands, entity: Entity) {
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

pub(crate) fn has_active_orders_in_query(
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

pub(crate) fn has_active_orders_or_queue(
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

pub(crate) fn has_active_order_state(order_state: CommandOrderStateItem<'_>) -> bool {
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

pub(crate) fn update_deploy_mode_requests(
    mut commands: Commands,
    mut units: Query<
        (
            Entity,
            &mut Unit,
            &mut HoldPosition,
            Option<&mut Weapon>,
            &mut VisionRadius,
            &Transform,
            Option<&DeployedSiegeMode>,
            &Health,
            Option<&EmpDisabled>,
        ),
        With<DeployModeToggleRequest>,
    >,
) {
    let mut deployable_count = 0usize;
    let mut deployed_count = 0usize;
    for (_entity, unit, _hold, weapon, _vision, _transform, deployed, health, emp) in
        units.iter_mut()
    {
        if vehicle_can_toggle_deploy_mode(&unit, weapon.is_some(), health, emp) {
            deployable_count += 1;
            if deployed.is_some() {
                deployed_count += 1;
            }
        }
    }
    let desired_deployed = deployable_count > 0 && deployed_count != deployable_count;

    for (entity, mut unit, mut hold, weapon, mut vision, transform, deployed, health, emp) in
        &mut units
    {
        commands
            .entity(entity)
            .try_remove::<DeployModeToggleRequest>();
        if !vehicle_can_toggle_deploy_mode(&unit, weapon.is_some(), health, emp) {
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
        apply_vehicle_deploy_mode(
            &mut commands,
            entity,
            &mut unit,
            &mut hold,
            &mut weapon,
            &mut vision,
            transform.translation,
            deployed,
            desired_deployed,
            true,
        );
    }
}

/// Vehicles that can trade all mobility for extra range and firepower.
/// The SiegeDrillTank keeps its bespoke burrow numbers; the rest use the
/// generic VEHICLE_DEPLOY_* multipliers on their own registry stats.
pub(crate) const DEPLOYABLE_VEHICLE_IDS: [&str; 7] = [
    "SiegeDrillTank",
    "HammerSiegeTank",
    "SiegeArtilleryVehicle",
    "LongbowMissileCrawler",
    "RailArtilleryWalker",
    "HeavySiegeWalker",
    "RailgunTank",
];

pub(crate) fn is_deployable_vehicle(unit_id: &str) -> bool {
    DEPLOYABLE_VEHICLE_IDS.contains(&unit_id)
}

/// Attack range the unit would have while deployed. If it is already
/// deployed the current weapon range IS the deployed range.
pub(crate) fn deployed_attack_range(
    unit_id: &str,
    current_range: f32,
    currently_deployed: bool,
) -> f32 {
    if currently_deployed {
        current_range
    } else if unit_id == "SiegeDrillTank" {
        SIEGE_DRILL_DEPLOYED_ATTACK_RANGE
    } else {
        current_range * VEHICLE_DEPLOY_RANGE_MULTIPLIER
    }
}

pub(crate) fn vehicle_can_toggle_deploy_mode(
    unit: &Unit,
    has_weapon: bool,
    health: &Health,
    emp: Option<&EmpDisabled>,
) -> bool {
    is_deployable_vehicle(unit.id)
        && has_weapon
        && health.current > 0.0
        && !emp.is_some_and(|emp| emp.remaining > 0.0)
}

pub(crate) fn apply_vehicle_deploy_mode(
    commands: &mut Commands,
    entity: Entity,
    unit: &mut Unit,
    hold: &mut HoldPosition,
    weapon: &mut Weapon,
    vision: &mut VisionRadius,
    position: Vec3,
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
            weapon.damage = deployed.base_attack_damage;
            weapon.cooldown = deployed.base_attack_interval;
            weapon.structure_damage_multiplier = deployed.base_structure_damage_multiplier;
            vision.0 = deployed.base_sight_range;
            commands.entity(entity).try_remove::<DeployedSiegeMode>();
            spawn_landing_dust(commands, position);
        }
        (None, true) => {
            commands.entity(entity).try_insert(DeployedSiegeMode {
                previous_hold_position: hold.enabled,
                base_speed: unit.speed,
                base_attack_range: weapon.range,
                base_attack_damage: weapon.damage,
                base_attack_interval: weapon.cooldown,
                base_structure_damage_multiplier: weapon.structure_damage_multiplier,
                base_sight_range: vision.0,
            });
            unit.speed = 0.0;
            hold.enabled = true;
            if unit.id == "SiegeDrillTank" {
                weapon.range = SIEGE_DRILL_DEPLOYED_ATTACK_RANGE;
                weapon.cooldown = SIEGE_DRILL_DEPLOYED_ATTACK_INTERVAL;
                weapon.structure_damage_multiplier =
                    SIEGE_DRILL_DEPLOYED_STRUCTURE_DAMAGE_MULTIPLIER;
                vision.0 = SIEGE_DRILL_DEPLOYED_SIGHT_RANGE;
            } else {
                weapon.range *= VEHICLE_DEPLOY_RANGE_MULTIPLIER;
                weapon.damage *= VEHICLE_DEPLOY_DAMAGE_MULTIPLIER;
                weapon.cooldown *= VEHICLE_DEPLOY_COOLDOWN_MULTIPLIER;
                weapon.structure_damage_multiplier *= VEHICLE_DEPLOY_STRUCTURE_DAMAGE_MULTIPLIER;
                vision.0 += VEHICLE_DEPLOY_SIGHT_BONUS;
            }
            spawn_landing_dust(commands, position);
        }
        _ => {}
    }
}

pub(crate) fn issue_spawned_unit_rally_order(
    commands: &mut Commands,
    spawned: Entity,
    def: &registry::EntityDef,
    rally_target: Vec3,
    rally_point: Option<RallyPoint>,
) {
    match spawned_unit_rally_order(def, rally_target, rally_point) {
        UnitQueuedOrder::AttackMove(destination) => {
            commands
                .entity(spawned)
                .try_insert(AttackMoveOrder { destination });
        }
        UnitQueuedOrder::Move(target) => {
            commands.entity(spawned).try_insert(MoveOrder { target });
        }
        _ => {}
    }
}

pub(crate) fn spawned_unit_rally_order(
    def: &registry::EntityDef,
    rally_target: Vec3,
    rally_point: Option<RallyPoint>,
) -> UnitQueuedOrder {
    if rally_point.is_some_and(|rally| rally.mode == RallyMode::AttackMove)
        && def.weapon.is_some()
        && def.speed > 0.0
    {
        UnitQueuedOrder::AttackMove(rally_target)
    } else {
        UnitQueuedOrder::Move(rally_target)
    }
}

pub(crate) fn update_capture_orders(
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
        let structure_label = localized_entity_label(structure.id);
        if *capturer_team == player_team {
            push_battle_log(
                &mut battle_log,
                format!("{} {structure_label}", t("已占领", "Captured")),
                Some(target_transform.translation),
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::StructureCaptured);
        } else if relations.are_allied(victim_team, player_team) {
            push_battle_log(
                &mut battle_log,
                format!("{} {structure_label}", t("失去", "Lost")),
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

pub(crate) fn update_garrison_orders(
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

pub(crate) fn update_follow_orders(
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

pub(crate) fn follow_order_reference_position(target_position: Vec3, follow: &FollowOrder) -> Vec3 {
    target_position + follow.offset
}

pub(crate) fn follow_order_move_target_position(
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

pub(crate) fn unit_contact_move_target_position(
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

pub(crate) fn contact_action_entry_range(
    source_radius: f32,
    target_radius: f32,
    margin: f32,
) -> f32 {
    source_radius + target_radius + margin + CONTACT_ACTION_REACHED_TOLERANCE_M
}

pub(crate) fn move_order_targets_contact(
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

pub(crate) fn follow_order_desired_distance(
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

pub(crate) fn update_repair_orders(
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
        if heal_sparkle_due(
            time.elapsed_secs(),
            time.delta_secs(),
            repairer.entity.to_bits(),
        ) {
            spawn_heal_sparkle(&mut commands, target_transform.translation);
        }
        if target_health.current >= target_health.max {
            commands.entity(repairer.entity).try_remove::<RepairOrder>();
        }
    }
}

pub(crate) fn clear_attack_chase_order(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .try_remove::<AttackOrder>()
        .try_remove::<MoveOrder>();
}

pub(crate) fn attack_order_target_valid(
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

pub(crate) fn update_attack_move_and_patrol_orders(
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

pub(crate) fn selected_terrain_order_path_points(
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

pub(crate) fn active_terrain_order_target(
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

pub(crate) fn queued_terrain_order_target(order: &UnitQueuedOrder) -> Option<Vec3> {
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

pub(crate) fn should_draw_action_queue_path(team: Team, visible_team: Team) -> bool {
    team == visible_team
}
