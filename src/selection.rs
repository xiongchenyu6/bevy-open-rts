//! Selection: click/box select, control groups, Tab subgroup cycling,
//! idle-worker cycling, double-click select-same-type, and the pointer→terrain
//! helpers the selection/drag code shares with order issuing.

use bevy::prelude::*;

use crate::*;

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

pub(crate) fn select_entities(
    mut commands: Commands,
    terrain: Res<TerrainHeightField>,
    support_fire_guard: Res<SupportFireClickGuard>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    visible_player: Res<VisiblePlayer>,
    command_mode: ResMut<CommandMode>,
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

    // A left-click that just fired an armed support power is consumed here so
    // it doesn't also start a selection.
    if support_fire_guard.0 && mouse.just_pressed(MouseButton::Left) {
        drag_state.active = false;
        drag_state.dragging = false;
        return;
    }

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
