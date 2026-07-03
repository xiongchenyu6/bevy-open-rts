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

#[derive(Resource)]
pub(crate) struct UnitGroups {
    pub(crate) slots: [Vec<Entity>; 9],
    pub(crate) last_accessed: Option<usize>,
}

impl Default for UnitGroups {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| Vec::new()),
            last_accessed: None,
        }
    }
}

#[derive(Resource, Debug)]
pub(crate) struct DoubleClickState {
    pub(crate) last_click_time: f32,
    pub(crate) last_unit: Option<Entity>,
    pub(crate) last_unit_type: Option<&'static str>,
}

#[derive(Resource, Default)]
pub(crate) struct IdleWorkerCycleState {
    pub(crate) request_for: Option<Team>,
    pub(crate) last_selected: Option<Entity>,
}

impl Default for DoubleClickState {
    fn default() -> Self {
        Self {
            last_click_time: -1000.0,
            last_unit: None,
            last_unit_type: None,
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandHotkey {
    pub(crate) display: &'static str,
    pub(crate) key_code: KeyCode,
}

impl CommandHotkey {
    pub(crate) const fn new(display: &'static str, key_code: KeyCode) -> Self {
        Self { display, key_code }
    }
}

#[derive(Component)]
pub(crate) struct CommandTooltip;

#[derive(Component)]
pub(crate) struct CommandTooltipText;

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

pub(crate) fn setup_command_tooltip(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            CommandTooltip,
            Visibility::Hidden,
            GlobalZIndex(55),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(COMMAND_TOOLTIP_WIDTH_PX),
                border: UiRect::all(px(1)),
                padding: UiRect::all(px(10)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.34, 0.46, 0.50)),
            BackgroundColor(Color::srgba(0.02, 0.035, 0.045, 0.94)),
            MatchScopedEntity,
        ))
        .with_children(|tooltip| {
            tooltip.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.95, 0.94)),
                TextLayout::justify(Justify::Left),
                CommandTooltipText,
            ));
        });
}

pub(crate) fn select_entities(
    mut commands: Commands,
    terrain: Res<TerrainHeightField>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    visible_player: Res<VisiblePlayer>,
    mut command_mode: ResMut<CommandMode>,
    hud_zones: Res<HudHitZones>,
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

    disarm_support_power_on_left_click(
        &mut command_mode,
        &mouse,
        cursor_is_over_hud(window, &hud_zones),
    );

    if mouse.just_pressed(MouseButton::Left) {
        drag_state.active = true;
        drag_state.dragging = false;
        drag_state.start = cursor;
        drag_state.started_in_hud = cursor_is_over_hud(window, &hud_zones);
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
        if std::env::var_os("RTS_SELECT_DIAG").is_some() {
            eprintln!("[select-diag] click swallowed: started_in_hud at {cursor:?}");
        }
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

    let Some(point) = pointer_ground(window, &camera_q, &terrain) else {
        if std::env::var_os("RTS_SELECT_DIAG").is_some() {
            eprintln!("[select-diag] pointer_ground=None at {cursor:?}");
        }
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    };
    if std::env::var_os("RTS_SELECT_DIAG").is_some() {
        eprintln!(
            "[select-diag] click at {cursor:?} ground=({:.2},{:.2})",
            point.x, point.z
        );
    }
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
        // Resources are clicked on their visible (tall) model, so use the same
        // screen-capsule hit-test as harvest targeting instead of a circle around
        // the ground point — otherwise clicking the crystal body misses.
        let (screen_distance, screen_pick) = if let Some(resource) = resource_node {
            match resource_cursor_pick_distance(
                cursor,
                camera,
                camera_transform,
                transform.translation,
                resource.kind,
                RESOURCE_ORDER_SCREEN_PICK_MAX_RADIUS_PX,
            ) {
                Some((distance, pick_radius)) => (Some(distance), distance <= pick_radius),
                None => (None, false),
            }
        } else {
            let screen_distance = camera
                .world_to_viewport(camera_transform, transform.translation)
                .ok()
                .map(|screen_position| screen_position.distance(cursor));
            let screen_pick = screen_distance.is_some_and(|distance| {
                distance <= single_click_selection_screen_radius(selectable.radius)
            });
            (screen_distance, screen_pick)
        };
        // Ground-proximity fallback only for non-resource units (resources rely on
        // the model-capsule test; their ground raycast lands behind the crystal).
        let ground_pick = resource_node.is_none() && ground_distance <= selectable.radius + 0.35;
        if std::env::var_os("RTS_SELECT_DIAG").is_some() && unit.is_some() && ground_distance < 3.0
        {
            eprintln!(
                "[select-diag] cand unit={:?} vis={} gdist={ground_distance:.2} sdist={screen_distance:?} gpick={ground_pick} spick={screen_pick}",
                unit.map(|u| u.id),
                visibility.visible
            );
        }
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

pub(crate) fn single_click_selection_screen_radius(selectable_radius: f32) -> f32 {
    (SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PX
        + selectable_radius.max(0.0) * SINGLE_CLICK_SELECTION_SCREEN_RADIUS_PER_METER_PX)
        .clamp(24.0, 72.0)
}

pub(crate) fn cancel_selection_drag(drag_state: &mut SelectionDragState) {
    drag_state.active = false;
    drag_state.dragging = false;
}

pub(crate) fn selection_drag_should_interrupt(
    drag_state: &SelectionDragState,
    cursor: Vec2,
    window_size: Vec2,
) -> bool {
    drag_state.active
        && !drag_state.started_in_hud
        && selection_drag_hits_screen_margin(cursor, window_size)
}

pub(crate) fn selection_drag_hits_screen_margin(cursor: Vec2, window_size: Vec2) -> bool {
    cursor.x <= SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.x >= window_size.x - SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.y <= SELECTION_DRAG_INTERRUPT_MARGIN_PX
        || cursor.y >= window_size.y - SELECTION_DRAG_INTERRUPT_MARGIN_PX
}

pub(crate) fn active_selection_drag_box_rect(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    drag_state: &SelectionDragState,
) -> Option<ScreenRect> {
    if !drag_state.active || !drag_state.dragging || drag_state.started_in_hud {
        return None;
    }
    let window = window_q.single().ok()?;
    selection_drag_box_rect(drag_state.start, window.cursor_position()?)
}

pub(crate) fn selection_drag_box_rect(start: Vec2, end: Vec2) -> Option<ScreenRect> {
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
pub(crate) enum SingleClickSelectionAction {
    Select,
    ToggleDeselect,
}

pub(crate) fn single_click_selection_action(
    additive: bool,
    already_selected: bool,
) -> SingleClickSelectionAction {
    if additive && already_selected {
        SingleClickSelectionAction::ToggleDeselect
    } else {
        SingleClickSelectionAction::Select
    }
}

pub(crate) fn issue_orders(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    economies: Res<Economies>,
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

pub(crate) fn command_queue_controls(
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

    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    // Plain S stops; Ctrl+S is the quicksave hotkey.
    if keyboard.just_pressed(KeyCode::KeyS) && !ctrl {
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

pub(crate) fn update_command_tooltip(
    build_queue: Res<BuildQueue>,
    visible_player: Res<VisiblePlayer>,
    player_factions: Res<PlayerFactions>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    command_mode: Res<CommandMode>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    selected_structures: Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: Query<StructureEntityItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
    slot_q: Query<(
        &CommandSlot,
        &BuildAction,
        &CommandSlotAvailability,
        &Interaction,
    )>,
    support_button_q: Query<(&SupportPowerButton, &Interaction)>,
    mut tooltip_q: Query<(&mut Node, &mut Visibility), With<CommandTooltip>>,
    mut text_q: Query<&mut Text, With<CommandTooltipText>>,
) {
    let Ok((mut tooltip_node, mut tooltip_visibility)) = tooltip_q.single_mut() else {
        return;
    };
    let Ok(mut tooltip_text) = text_q.single_mut() else {
        return;
    };
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        *tooltip_visibility = Visibility::Hidden;
        return;
    };
    let Ok(window) = window_q.single() else {
        *tooltip_visibility = Visibility::Hidden;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *tooltip_visibility = Visibility::Hidden;
        return;
    };
    if let Some((button, _)) = support_button_q
        .iter()
        .filter(|(_, interaction)| {
            matches!(interaction, Interaction::Hovered | Interaction::Pressed)
        })
        .min_by_key(|(button, _)| button.kind.idx())
    {
        let missing_requirements = support_power_missing_requirement_labels(
            visible_team,
            button.kind.definition().requirements,
            &structures,
        );
        let low_power = economies.get(visible_team).low_power();
        let state = support_power_button_state(
            button.kind,
            missing_requirements.is_empty(),
            low_power,
            support_cooldowns.remaining_for(visible_team, button.kind),
            command_mode.support_power == Some(button.kind),
        );
        **tooltip_text = support_power_tooltip(button.kind, &state, &missing_requirements);
        position_command_tooltip(&mut tooltip_node, window, cursor);
        *tooltip_visibility = Visibility::Inherited;
        return;
    }
    let Some((slot, action, availability, _)) = slot_q
        .iter()
        .filter(|(_, action, _, interaction)| {
            !matches!(action, BuildAction::None)
                && matches!(interaction, Interaction::Hovered | Interaction::Pressed)
        })
        .min_by_key(|(slot, ..)| slot.0)
    else {
        *tooltip_visibility = Visibility::Hidden;
        return;
    };

    let faction = player_factions.slot_faction(visible_team);
    **tooltip_text = command_action_tooltip(
        slot.0,
        *action,
        availability.enabled,
        visible_team,
        faction,
        &selected_structures,
        &producer_structures,
        &structures,
        &build_queue,
    );
    position_command_tooltip(&mut tooltip_node, window, cursor);
    *tooltip_visibility = Visibility::Inherited;
}

pub(crate) fn position_command_tooltip(tooltip_node: &mut Node, window: &Window, cursor: Vec2) {
    let max_left = (window.width() - COMMAND_TOOLTIP_WIDTH_PX - 8.0).max(8.0);
    let left = (cursor.x + COMMAND_TOOLTIP_OFFSET_X_PX).clamp(8.0, max_left);
    let raw_top = if cursor.y > COMMAND_TOOLTIP_OFFSET_Y_PX + 12.0 {
        cursor.y - COMMAND_TOOLTIP_OFFSET_Y_PX
    } else {
        cursor.y + 24.0
    };
    tooltip_node.left = px(left);
    tooltip_node.top = px(raw_top.clamp(8.0, (window.height() - 120.0).max(8.0)));
}

#[allow(dead_code)]
pub(crate) fn current_command_actions(
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
        BuildStructureTab::Production,
        false,
    )
}

pub(crate) fn current_command_actions_for_faction(
    team: Team,
    faction: SkirmishFaction,
    selected_units: &Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    _structures: &Query<StructurePrereqItem<'_>>,
    build_structure_tab: BuildStructureTab,
    has_idle_worker: bool,
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

    if has_idle_worker {
        push_action_unique(&mut actions, BuildAction::SelectIdleWorker);
    }

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
        push_action_unique(&mut actions, BuildAction::AttackMove);
        push_action_unique(&mut actions, BuildAction::SellStructure);
        if has_repairable_structure {
            push_action_unique(&mut actions, BuildAction::RepairStructure);
        }
    } else {
        if has_selected_team_structure {
            push_action_unique(&mut actions, BuildAction::SellStructure);
        }
        if show_worker_construction_menu {
            push_action_unique(
                &mut actions,
                BuildAction::SelectBuildTab(BuildStructureTab::Production),
            );
            push_action_unique(
                &mut actions,
                BuildAction::SelectBuildTab(BuildStructureTab::Defense),
            );
            for structure in sorted_worker_build_structures(faction)
                .into_iter()
                .filter(|structure| build_structure_tab_for(structure) == build_structure_tab)
            {
                push_action_unique(&mut actions, BuildAction::Build(structure));
            }
        }
    }
    actions.truncate(COMMAND_SLOT_COUNT);
    actions
}

pub(crate) fn command_action_enabled_for_panel(
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
        BuildAction::SelectBuildTab(_) => true,
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
        BuildAction::SellStructure
        | BuildAction::RepairStructure
        | BuildAction::ToggleDeployMode
        | BuildAction::SetRallyPoint
        | BuildAction::SelectIdleWorker
        | BuildAction::HoldPosition
        | BuildAction::AttackMove
        | BuildAction::Patrol
        | BuildAction::GuardArea
        | BuildAction::ScatterSelected => true,
    }
}

#[cfg(test)]
pub(crate) fn command_label(index: usize, action: Option<BuildAction>) -> String {
    command_label_with_queue(index, action, None)
}

// Asset path of the command-button icon for an action, mirroring godot's command
// icon mosaic. Train/Build pull the produced entity's registry icon; standing
// orders use the matching `ui/icons/<Name>.png` mirrored from the godot project.
pub(crate) fn command_action_icon_path(action: BuildAction) -> Option<&'static str> {
    match action {
        BuildAction::Train(id) | BuildAction::Build(id) => {
            registry::entity(id).and_then(|def| def.icon)
        }
        BuildAction::SellStructure => Some("ui/icons/SellStructure.png"),
        BuildAction::RepairStructure => Some("ui/icons/Repair.png"),
        BuildAction::ToggleDeployMode => Some("ui/icons/DeployMode.png"),
        BuildAction::SetRallyPoint => Some("ui/icons/RallyPoint.png"),
        BuildAction::SelectIdleWorker => registry::entity("Worker").and_then(|def| def.icon),
        BuildAction::HoldPosition => Some("ui/icons/HoldPosition.png"),
        BuildAction::AttackMove => Some("ui/icons/AttackMove.png"),
        BuildAction::Patrol => Some("ui/icons/Patrol.png"),
        BuildAction::GuardArea => Some("ui/icons/GuardArea.png"),
        BuildAction::StopSelected => Some("ui/icons/StopCommand.png"),
        BuildAction::ScatterSelected => Some("ui/icons/Scatter.png"),
        BuildAction::SelectBuildTab(_) | BuildAction::None => None,
    }
}

pub(crate) fn command_grid_hotkey(index: usize) -> Option<CommandHotkey> {
    COMMAND_SLOT_HOTKEYS.get(index).copied()
}

pub(crate) fn command_action_hotkey(index: usize, action: BuildAction) -> Option<CommandHotkey> {
    match action {
        BuildAction::None => None,
        BuildAction::SelectIdleWorker => Some(CommandHotkey::new("I", KeyCode::KeyI)),
        BuildAction::GuardArea => Some(CommandHotkey::new("G", KeyCode::KeyG)),
        BuildAction::StopSelected => Some(CommandHotkey::new("S", KeyCode::KeyS)),
        BuildAction::ScatterSelected => Some(CommandHotkey::new("X", KeyCode::KeyX)),
        _ => command_grid_hotkey(index),
    }
}

pub(crate) fn command_action_display_key(index: usize, action: BuildAction) -> &'static str {
    command_action_hotkey(index, action)
        .map(|hotkey| hotkey.display)
        .unwrap_or(" ")
}

pub(crate) fn command_label_with_queue(
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
            let prefix = match (current_language(), action) {
                (Language::Zh, BuildAction::Build(_)) => "建",
                (Language::Zh, BuildAction::Train(_)) => "训",
                (_, BuildAction::Build(_)) => "B",
                _ => "T",
            };
            let queue_badge = queue_state
                .filter(|state| state.count > 0 || state.full)
                .map(queue_button_badge_text)
                .unwrap_or_default();
            format!(
                "{key} {prefix} {} {}/{}{queue_badge}",
                localized_compact_entity_label(id),
                cost.ore,
                cost.crystal
            )
        }
        BuildAction::SellStructure => format!("{key} {}", t("出售建筑", "Sell")),
        BuildAction::RepairStructure => format!("{key} {}", t("维修建筑", "Repair")),
        BuildAction::ToggleDeployMode => format!("{key} {}", t("切换部署", "Toggle Deploy")),
        BuildAction::SetRallyPoint => format!("{key} {}", t("设置集结", "Rally Point")),
        BuildAction::SelectIdleWorker => format!("{key} {}", t("闲置工人", "Idle Worker")),
        BuildAction::HoldPosition => format!("{key} {}", t("坚守", "Hold")),
        BuildAction::AttackMove => format!("{key} {}", t("攻击移动", "Attack-Move")),
        BuildAction::Patrol => format!("{key} {}", t("巡逻", "Patrol")),
        BuildAction::GuardArea => format!("{key} {}", t("守卫区域", "Guard")),
        BuildAction::StopSelected => format!("{key} {}", t("停止", "Stop")),
        BuildAction::ScatterSelected => format!("{key} {}", t("散开", "Scatter")),
        BuildAction::SelectBuildTab(tab) => format!("{key} {}", tab.label()),
        BuildAction::None => String::new(),
    }
}

pub(crate) fn command_action_tooltip(
    index: usize,
    action: BuildAction,
    enabled: bool,
    team: Team,
    faction: SkirmishFaction,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    build_queue: &BuildQueue,
) -> String {
    let key = command_action_display_key(index, action);
    let mut lines = Vec::new();
    match action {
        BuildAction::Train(id) | BuildAction::Build(id) => {
            let verb = match action {
                BuildAction::Train(_) => t("训练", "Train"),
                BuildAction::Build(_) => t("建造", "Build"),
                _ => "",
            };
            lines.push(format!("{key} {verb} {}", localized_entity_label(id)));
            if let Some(def) = registry::entity(id) {
                lines.push(format!(
                    "{}: {} {} / {} {}   {}: {:.0}s",
                    t("成本", "Cost"),
                    t("矿石", "Ore"),
                    def.cost.ore,
                    t("水晶", "Crystal"),
                    def.cost.crystal,
                    t("用时", "Time"),
                    def.build_seconds
                ));
                if def.power_delta != 0 {
                    lines.push(format!(
                        "{}: {}",
                        t("电力", "Power"),
                        signed_number(def.power_delta)
                    ));
                }
                if let Some(weapon) = def.weapon {
                    lines.push(format!(
                        "{}: {:.0}   {}: {:.1}   {}: {:.1}s",
                        t("攻击", "Damage"),
                        weapon.damage,
                        t("射程", "Range"),
                        weapon.range,
                        t("冷却", "Cooldown"),
                        weapon.cooldown
                    ));
                } else if def.resource_capacity > 0 {
                    lines.push(format!(
                        "{}: {}",
                        t("采集载货", "Cargo"),
                        def.resource_capacity
                    ));
                } else if def.is_resource_producer {
                    lines.push(format!(
                        "{}: +{}/+{}",
                        t("资源收入", "Income"),
                        def.resource_income_ore,
                        def.resource_income_crystal
                    ));
                }
                if !def.requirements.is_empty() {
                    let requirements = def
                        .requirements
                        .iter()
                        .map(|requirement| localized_compact_entity_label(requirement))
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("{}: {requirements}", t("需求", "Requires")));
                }
                let missing = missing_requirement_labels(def, team, structures);
                if !missing.is_empty() {
                    lines.push(format!("{}: {}", t("缺少", "Missing"), missing.join(", ")));
                }
                if let Some(queue_state) = command_queue_button_state_for_action(
                    team,
                    faction,
                    action,
                    selected_structures,
                    producer_structures,
                    build_queue,
                ) {
                    lines.push(format!(
                        "{}: {}/{}{}",
                        t("队列", "Queue"),
                        queue_state.count,
                        PRODUCTION_QUEUE_LIMIT,
                        if queue_state.full {
                            t(" 已满", " full")
                        } else {
                            ""
                        }
                    ));
                }
            }
        }
        BuildAction::SellStructure => {
            lines.push(format!("{key} {}", t("出售建筑", "Sell structure")));
            lines.push(
                t(
                    "返还部分资源并移除当前选中建筑。",
                    "Refunds part of the cost and removes the selected structure.",
                )
                .to_string(),
            );
        }
        BuildAction::RepairStructure => {
            lines.push(format!("{key} {}", t("维修建筑", "Repair structure")));
            lines.push(
                t(
                    "消耗资源修复当前受损建筑。",
                    "Spends resources to repair damaged selected structures.",
                )
                .to_string(),
            );
        }
        BuildAction::ToggleDeployMode => {
            lines.push(format!("{key} {}", t("切换部署", "Toggle deploy")));
            lines.push(
                t(
                    "在机动和架设火力模式之间切换。",
                    "Switches between mobile and deployed fire mode.",
                )
                .to_string(),
            );
        }
        BuildAction::SetRallyPoint => {
            lines.push(format!("{key} {}", t("设置集结点", "Set rally point")));
            lines.push(
                t(
                    "下一次点地面或目标会设置生产建筑集结点。",
                    "The next terrain or target click sets the production rally point.",
                )
                .to_string(),
            );
        }
        BuildAction::SelectIdleWorker => {
            lines.push(format!("{key} {}", t("闲置工人", "Idle worker")));
            lines.push(
                t(
                    "选择并跳转到下一个没有命令的工人。",
                    "Selects and jumps to the next worker with no active order.",
                )
                .to_string(),
            );
        }
        BuildAction::HoldPosition => {
            lines.push(format!("{key} {}", t("坚守", "Hold position")));
            lines.push(
                t(
                    "单位保持阵位，只攻击进入射程的敌人。",
                    "Units hold position and only fire at enemies in range.",
                )
                .to_string(),
            );
        }
        BuildAction::AttackMove => {
            lines.push(format!("{key} {}", t("攻击移动", "Attack move")));
            lines.push(
                t(
                    "单位移动途中主动攻击；生产建筑选中时，下一次点地面会设置攻击集结点。",
                    "Units engage while moving; with a production structure selected, the next terrain click sets an attack rally point.",
                )
                .to_string(),
            );
        }
        BuildAction::Patrol => {
            lines.push(format!("{key} {}", t("巡逻", "Patrol")));
            lines.push(
                t(
                    "在当前位置和指定地点之间巡逻。",
                    "Patrol between current position and target point.",
                )
                .to_string(),
            );
        }
        BuildAction::GuardArea => {
            lines.push(format!("{key} {}", t("守卫区域", "Guard area")));
            lines.push(
                t(
                    "守卫附近区域并响应敌人。",
                    "Guard the nearby area and react to enemies.",
                )
                .to_string(),
            );
        }
        BuildAction::StopSelected => {
            lines.push(format!("{key} {}", t("停止", "Stop")));
            lines.push(
                t(
                    "取消当前命令和未完成动作。",
                    "Cancels active orders and pending actions.",
                )
                .to_string(),
            );
        }
        BuildAction::ScatterSelected => {
            lines.push(format!("{key} {}", t("散开", "Scatter")));
            lines.push(
                t(
                    "让选中单位短距离分散，减少溅射伤害。",
                    "Spreads selected units to reduce splash damage.",
                )
                .to_string(),
            );
        }
        BuildAction::SelectBuildTab(tab) => {
            lines.push(format!("{key} {}", tab.label()));
            lines.push(
                match tab {
                    BuildStructureTab::Production => t(
                        "显示电力、经济、生产、科技和后期建筑。",
                        "Shows power, economy, production, tech, and late-game structures.",
                    ),
                    BuildStructureTab::Defense => t(
                        "显示炮塔、围栏、方尖塔和地堡等防御建筑。",
                        "Shows turrets, fences, obelisks, bunkers, and other defenses.",
                    ),
                }
                .to_string(),
            );
        }
        BuildAction::None => {}
    }
    if !enabled {
        lines.push(format!(
            "{}: {}",
            t("状态", "Status"),
            command_action_unavailable_reason(
                action,
                team,
                faction,
                selected_structures,
                producer_structures,
                structures,
                build_queue,
            )
        ));
    }
    lines.join("\n")
}

pub(crate) fn command_action_unavailable_reason(
    action: BuildAction,
    team: Team,
    faction: SkirmishFaction,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    build_queue: &BuildQueue,
) -> String {
    match action {
        BuildAction::Train(id) | BuildAction::Build(id) => {
            let Some(def) = registry::entity(id) else {
                return t("目标不存在", "Missing target").to_string();
            };
            let missing = missing_requirement_labels(def, team, structures);
            if !missing.is_empty() {
                return format!("{} {}", t("缺少", "Missing"), missing.join(", "));
            }
            if matches!(action, BuildAction::Build(_))
                && faction_def(faction).is_none_or(|faction| !faction.can_construct(id))
            {
                return t("当前阵营无法建造", "Faction cannot build this").to_string();
            }
            if matches!(action, BuildAction::Train(_)) {
                let producers = command_queue_producers_for_action(
                    team,
                    faction,
                    action,
                    selected_structures,
                    producer_structures,
                );
                if producers.is_empty() {
                    return t("没有可用生产建筑", "No available producer").to_string();
                }
                if producers
                    .iter()
                    .all(|producer| !build_queue_has_capacity(build_queue, *producer))
                {
                    return t("生产队列已满", "Production queue full").to_string();
                }
            }
            t("暂不可用", "Unavailable").to_string()
        }
        BuildAction::None => t("无命令", "No command").to_string(),
        BuildAction::SelectBuildTab(_) => t("可用", "Available").to_string(),
        _ => t("当前选择不支持", "Not supported by current selection").to_string(),
    }
}

pub(crate) fn selection_hotkeys(
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
    mut unit_groups: ResMut<UnitGroups>,
    mut bookmarks: ResMut<CameraBookmarks>,
    mut camera_state: ResMut<RtsCamera>,
    mut battle_log: ResMut<BattleLog>,
    mut idle_worker_cycle: ResMut<IdleWorkerCycleState>,
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

    if alt && keyboard.just_pressed(KeyCode::KeyI) {
        idle_worker_cycle.request_for = Some(visible_team);
    }
}

pub(crate) fn process_idle_worker_selection_requests(
    mut commands: Commands,
    visible_player: Res<VisiblePlayer>,
    map_bounds: Res<MapBounds>,
    mut idle_worker_cycle: ResMut<IdleWorkerCycleState>,
    mut camera_state: ResMut<RtsCamera>,
    selectable_q: Query<
        (Entity, &Transform, &Team, Option<&Unit>, Option<&Structure>),
        With<Selectable>,
    >,
    idle_workers: Query<IdleWorkerSelectionItem<'_>, With<Unit>>,
    unit_transforms: Query<&Transform, With<Unit>>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
) {
    let Some(requested_team) = idle_worker_cycle.request_for.take() else {
        return;
    };
    if controlled_player_team(Some(&*visible_player)) != Some(requested_team) {
        return;
    }

    let mut candidates = idle_workers
        .iter()
        .filter_map(|item| {
            if !is_idle_worker_item(requested_team, item) {
                return None;
            }
            let (entity, ..) = item;
            unit_transforms
                .get(entity)
                .ok()
                .map(|transform| (entity, transform.translation))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(entity, _)| entity.index());
    if candidates.is_empty() {
        idle_worker_cycle.last_selected = None;
        record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::Error);
        push_battle_log(
            &mut battle_log,
            t("没有闲置工人", "No idle workers").to_string(),
            None,
        );
        return;
    }

    let next_index = idle_worker_cycle
        .last_selected
        .and_then(|last| candidates.iter().position(|(entity, _)| *entity == last))
        .map(|index| (index + 1) % candidates.len())
        .unwrap_or(0);
    let (target, position) = candidates[next_index];
    idle_worker_cycle.last_selected = Some(target);
    apply_selected_from_ids(
        &mut commands,
        &selectable_q,
        &[target],
        false,
        requested_team,
    );
    set_camera_focus_safely(&mut camera_state, position, *map_bounds);
    record_selection_audio_feedback(&mut audio_feedback, true, true);
    push_battle_log(
        &mut battle_log,
        t("闲置工人", "Idle worker").to_string(),
        Some(position),
    );
}

pub(crate) fn valid_group_entities(
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

pub(crate) fn focus_entities(
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

pub(crate) fn selected_entities_focus(
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

pub(crate) fn is_unit_idle(
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

pub(crate) fn is_idle_worker_item(team: Team, item: IdleWorkerSelectionItem<'_>) -> bool {
    let (
        _entity,
        unit_team,
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
    ) = item;
    *unit_team == team
        && visibility.visible
        && is_builder_worker_selection_unit(unit)
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
}

pub(crate) fn has_idle_worker_for_team(
    team: Team,
    idle_workers: &Query<IdleWorkerSelectionItem<'_>, With<Unit>>,
) -> bool {
    idle_workers
        .iter()
        .any(|item| is_idle_worker_item(team, item))
}

pub(crate) fn apply_selected_from_ids(
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

pub(crate) fn command_shortcuts(
    mut commands: Commands,
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
            visible_team,
            action_resources.player_factions.slot_faction(visible_team),
            *action,
            &mut action_resources.build_structure_tab,
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
            &mut action_resources.idle_worker_cycle,
            production_batch_modifier_pressed(&keyboard),
        );
        return;
    }
}

pub(crate) fn execute_command_action(
    commands: &mut Commands,
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    build_structure_tab: &mut BuildStructureTab,
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
    idle_worker_cycle: &mut IdleWorkerCycleState,
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
        BuildAction::SelectIdleWorker => {
            clear_targeting_modes(command_mode);
            idle_worker_cycle.request_for = Some(team);
            true
        }
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
            }) || selected_structures.iter().any(
                |(_, structure, structure_team, _, under_construction)| {
                    *structure_team == team
                        && structure_is_constructed(under_construction)
                        && is_rally_point_structure(structure.id)
                },
            ),
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
        BuildAction::SelectBuildTab(tab) => {
            *build_structure_tab = tab;
            clear_targeting_modes(command_mode);
            true
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
    } else if handled
        && !matches!(
            action,
            BuildAction::Build(_) | BuildAction::SelectBuildTab(_) | BuildAction::SelectIdleWorker
        )
    {
        record_build_action_audio_feedback(audio_feedback, team, team, action);
    }
    handled
}

pub(crate) fn request_selected_deploy_toggle(
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

pub(crate) fn command_queue_button_state_for_action(
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

pub(crate) fn command_queue_producers_for_action(
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

pub(crate) fn update_deploy_mode_requests(
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

pub(crate) fn siege_drill_can_toggle_deploy_mode(
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

pub(crate) fn apply_siege_drill_deploy_mode(
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

pub(crate) fn command_origins_for(
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

pub(crate) fn exact_control_group_slot(
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

pub(crate) fn pointer_ground(
    window: &Window,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    terrain: &TerrainHeightField,
) -> Option<Vec3> {
    let cursor = window.cursor_position()?;
    let (camera, camera_transform) = camera_q.single().ok()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    terrain.raycast(ray.origin, *ray.direction)
}

pub(crate) fn screen_polygon_for_drag(start: Vec2, end: Vec2) -> Option<Vec<Vec2>> {
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

pub(crate) fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
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

/// Remembers the full mixed selection while Tab cycles its unit-type subgroups
/// (Tab: full selection -> type A only -> type B only -> ... -> full selection).
#[derive(Resource, Default)]
pub(crate) struct TabSubgroupState {
    pub(crate) full: Vec<Entity>,
    pub(crate) cursor: Option<usize>,
}

/// Cursor sequence: None (full selection) -> 0 -> 1 -> ... -> count-1 -> None.
pub(crate) fn next_subgroup_cursor(cursor: Option<usize>, type_count: usize) -> Option<usize> {
    match cursor {
        None if type_count > 0 => Some(0),
        Some(index) if index + 1 < type_count => Some(index + 1),
        _ => None,
    }
}

/// Stable (alphabetical) distinct unit-type order for subgroup cycling.
pub(crate) fn distinct_subgroup_types(ids: &[&str]) -> Vec<String> {
    let mut types: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    types.sort();
    types.dedup();
    types
}

pub(crate) fn cycle_selection_subgroup(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut state: ResMut<TabSubgroupState>,
    selected_units: Query<(Entity, &Team, &Unit), With<Selected>>,
    all_units: Query<(&Team, &Unit, &Health), With<Selectable>>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }
    let Some(team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    let current: Vec<Entity> = selected_units
        .iter()
        .filter(|(_, unit_team, _)| **unit_team == team)
        .map(|(entity, _, _)| entity)
        .collect();

    // The remembered full set, dropping anything dead/despawned since last Tab.
    let full_alive: Vec<(Entity, &'static str)> = state
        .full
        .iter()
        .filter_map(|&entity| {
            all_units
                .get(entity)
                .ok()
                .filter(|(unit_team, _, health)| **unit_team == team && health.current > 0.0)
                .map(|(_, unit, _)| (entity, unit.id))
        })
        .collect();

    // Continue cycling only while the selection is still a subset of the
    // remembered set; any fresh player selection restarts the cycle from it.
    let continuing = !current.is_empty()
        && !full_alive.is_empty()
        && current.iter().all(|entity| {
            full_alive
                .iter()
                .any(|(full_entity, _)| full_entity == entity)
        });
    let (full_pairs, cursor) = if continuing {
        (full_alive, state.cursor)
    } else {
        let fresh: Vec<(Entity, &'static str)> = selected_units
            .iter()
            .filter(|(_, unit_team, _)| **unit_team == team)
            .map(|(entity, _, unit)| (entity, unit.id))
            .collect();
        (fresh, None)
    };
    if full_pairs.is_empty() {
        return;
    }

    let ids: Vec<&str> = full_pairs.iter().map(|(_, id)| *id).collect();
    let types = distinct_subgroup_types(&ids);
    let next = next_subgroup_cursor(cursor, types.len());

    for (entity, id) in &full_pairs {
        let keep = match next {
            None => true,
            Some(index) => *id == types[index],
        };
        if keep {
            commands.entity(*entity).try_insert(Selected);
        } else {
            commands.entity(*entity).try_remove::<Selected>();
        }
    }
    state.full = full_pairs.iter().map(|(entity, _)| *entity).collect();
    state.cursor = next;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_cursor_cycles_full_then_each_type_then_full() {
        assert_eq!(next_subgroup_cursor(None, 3), Some(0));
        assert_eq!(next_subgroup_cursor(Some(0), 3), Some(1));
        assert_eq!(
            next_subgroup_cursor(Some(2), 3),
            None,
            "wraps back to the full selection"
        );
        assert_eq!(
            next_subgroup_cursor(None, 0),
            None,
            "empty selection never enters a subgroup"
        );
    }

    #[test]
    fn subgroup_type_order_is_stable_and_deduped() {
        let types = distinct_subgroup_types(&["Tank", "Worker", "Tank", "Drone"]);
        assert_eq!(
            types,
            vec![
                "Drone".to_string(),
                "Tank".to_string(),
                "Worker".to_string()
            ]
        );
    }
}
