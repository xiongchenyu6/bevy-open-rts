//! Command card: the 4x4 action grid (actions per selection/faction, labels,
//! icons, hotkeys, tooltips, availability), command shortcuts, action
//! execution, and the production-queue button plumbing.

use bevy::prelude::*;

use crate::*;

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

pub(crate) fn command_queue_controls(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut command_mode: ResMut<CommandMode>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut online: OnlineGameplayCommandParams,
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
        let entities = selected
            .iter()
            .filter_map(|(entity, _, team, _, _, orders)| {
                (**team == visible_team && has_active_order_state(*orders)).then_some(*entity)
            })
            .collect::<Vec<_>>();
        let handled = if online.is_active() {
            submit_online_unit_action(&mut online, entities, OnlineUnitAction::Stop)
        } else {
            stop_selected_entities(
                &mut commands,
                selected
                    .iter()
                    .filter_map(|(entity, _, team, _, _, orders)| {
                        (**team == visible_team && has_active_order_state(*orders))
                            .then_some(*entity)
                    }),
            )
        };
        if handled {
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
        let handled = if online.is_active() {
            submit_online_unit_action(
                &mut online,
                selected
                    .iter()
                    .filter(|(_, unit, team, ..)| {
                        **team == visible_team && unit_supports_hold_position(unit)
                    })
                    .map(|(entity, ..)| *entity),
                OnlineUnitAction::ToggleHoldPosition,
            )
        } else {
            toggle_selected_hold_position(
                &mut commands,
                visible_team,
                selected
                    .iter()
                    .map(|(entity, unit, team, _, hold, ..)| (*entity, *unit, *team, *hold)),
            )
        };
        if handled {
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
        let handled = if online.is_active() {
            submit_online_unit_action(
                &mut online,
                selected
                    .iter()
                    .filter(|(_, unit, team, ..)| {
                        **team == visible_team && can_unit_guard_area(unit)
                    })
                    .map(|(entity, ..)| *entity),
                OnlineUnitAction::GuardArea,
            )
        } else {
            guard_selected_area(
                &mut commands,
                visible_team,
                selected
                    .iter()
                    .map(|(entity, unit, team, ..)| (*entity, *unit, *team)),
            )
        };
        if handled {
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
        let handled = if online.is_active() {
            submit_online_unit_action(
                &mut online,
                scatter_units.iter().map(|(entity, _)| *entity),
                OnlineUnitAction::Scatter,
            )
        } else {
            scatter_selected_positions(&mut commands, &scatter_units)
        };
        if handled {
            clear_targeting_modes(&mut command_mode);
            record_command_audio_feedback(
                &mut audio_feedback,
                has_owned_voice_unit,
                Some(COMMAND_KEY_SCATTER),
            );
        }
    }
}

fn submit_online_unit_action(
    online: &mut OnlineGameplayCommandParams,
    entities: impl IntoIterator<Item = Entity>,
    action: OnlineUnitAction,
) -> bool {
    let Some(units) = entities
        .into_iter()
        .map(|entity| online.network_id_for(entity))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    !units.is_empty() && online.submit(OnlinePlayerCommand::UnitAction { units, action })
}

fn submit_online_structure_action(
    online: &mut OnlineGameplayCommandParams,
    entities: impl IntoIterator<Item = Entity>,
    action: OnlineStructureAction,
) -> bool {
    let Some(structures) = entities
        .into_iter()
        .map(|entity| online.network_id_for(entity))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    !structures.is_empty()
        && online.submit(OnlinePlayerCommand::StructureAction { structures, action })
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
        .any(|(unit, unit_team, ..)| *unit_team == team && is_deployable_vehicle(unit.id))
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

pub(crate) fn command_shortcuts(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    visible_player: Res<VisiblePlayer>,
    mut action_resources: CommandActionResources,
    mut online: OnlineGameplayCommandParams,
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
            &mut online,
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
    online: &mut OnlineGameplayCommandParams,
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
        BuildAction::SellStructure => {
            if online.is_active() {
                submit_online_structure_action(
                    online,
                    selected_sell_structures
                        .iter()
                        .filter(|(_, _, structure_team, health, _)| {
                            **structure_team == team && health.current > 0.0
                        })
                        .map(|(entity, ..)| entity),
                    OnlineStructureAction::Sell,
                )
            } else {
                sell_selected_structures(
                    commands,
                    team,
                    selected_sell_structures,
                    economies,
                    build_queue,
                )
            }
        }
        BuildAction::RepairStructure => {
            if online.is_active() {
                submit_online_structure_action(
                    online,
                    selected_repair_structures
                        .iter()
                        .filter(|(_, _, structure_team, health, repair, construction)| {
                            **structure_team == team
                                && health.current > 0.0
                                && health.current < health.max
                                && repair.is_none()
                                && structure_is_constructed(*construction)
                        })
                        .map(|(entity, ..)| entity),
                    OnlineStructureAction::Repair,
                )
            } else {
                repair_selected_structures(commands, team, selected_repair_structures, economies)
            }
        }
        BuildAction::ToggleDeployMode => {
            if online.is_active() {
                submit_online_unit_action(
                    online,
                    selected_units
                        .iter()
                        .filter(|(_, unit, unit_team, ..)| {
                            **unit_team == team && is_deployable_vehicle(unit.id)
                        })
                        .map(|(entity, ..)| entity),
                    OnlineUnitAction::ToggleDeployMode,
                )
            } else {
                request_selected_deploy_toggle(commands, team, selected_units)
            }
        }
        BuildAction::SetRallyPoint => begin_rally_point_mode(command_mode, true),
        BuildAction::SelectIdleWorker => {
            clear_targeting_modes(command_mode);
            idle_worker_cycle.request_for = Some(team);
            true
        }
        BuildAction::HoldPosition => {
            let handled = if online.is_active() {
                submit_online_unit_action(
                    online,
                    selected_units
                        .iter()
                        .filter(|(_, unit, unit_team, ..)| {
                            **unit_team == team && unit_supports_hold_position(unit)
                        })
                        .map(|(entity, ..)| entity),
                    OnlineUnitAction::ToggleHoldPosition,
                )
            } else {
                toggle_selected_hold_position(
                    commands,
                    team,
                    selected_units
                        .iter()
                        .map(|(entity, unit, unit_team, _, hold, ..)| {
                            (entity, unit, unit_team, hold)
                        }),
                )
            };
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
            if online.is_active() {
                submit_online_unit_action(
                    online,
                    selected_units
                        .iter()
                        .filter(|(_, unit, unit_team, ..)| {
                            **unit_team == team && can_unit_guard_area(unit)
                        })
                        .map(|(entity, ..)| entity),
                    OnlineUnitAction::GuardArea,
                )
            } else {
                guard_selected_area(
                    commands,
                    team,
                    selected_units
                        .iter()
                        .map(|(entity, unit, unit_team, ..)| (entity, unit, unit_team)),
                )
            }
        }
        BuildAction::StopSelected => {
            clear_targeting_modes(command_mode);
            if online.is_active() {
                let selected_unit_count = selected_units
                    .iter()
                    .filter(|(_, _, unit_team, ..)| **unit_team == team)
                    .count();
                if let Some((entity, _)) = selected_under_construction_stop_target(
                    team,
                    selected_unit_count,
                    selected_sell_structures.iter().map(
                        |(entity, _, structure_team, health, under_construction)| {
                            (entity, structure_team, health, under_construction)
                        },
                    ),
                ) {
                    submit_online_structure_action(
                        online,
                        [entity],
                        OnlineStructureAction::CancelConstruction,
                    )
                } else {
                    submit_online_unit_action(
                        online,
                        selected_units
                            .iter()
                            .filter_map(|(entity, _, unit_team, _, _, orders)| {
                                (*unit_team == team && has_active_order_state(orders))
                                    .then_some(entity)
                            }),
                        OnlineUnitAction::Stop,
                    )
                }
            } else {
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
        }
        BuildAction::ScatterSelected => {
            clear_targeting_modes(command_mode);
            if online.is_active() {
                submit_online_unit_action(
                    online,
                    selected_units
                        .iter()
                        .filter(|(_, unit, unit_team, ..)| {
                            **unit_team == team && unit_supports_patrol(unit)
                        })
                        .map(|(entity, ..)| entity),
                    OnlineUnitAction::Scatter,
                )
            } else {
                scatter_selected_units(commands, team, selected_units)
            }
        }
        BuildAction::Train(_) => {
            let result = if online.is_active() {
                submit_online_train_action(
                    online,
                    team,
                    faction,
                    action,
                    selected_structures,
                    producer_structures,
                    structures,
                    economies,
                    build_queue,
                    batch_to_limit,
                )
            } else {
                enqueue_build_action_for_faction(
                    team,
                    faction,
                    action,
                    selected_structures,
                    producer_structures,
                    structures,
                    economies,
                    build_queue,
                    batch_to_limit,
                )
            };
            match result {
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

#[allow(clippy::too_many_arguments)]
fn submit_online_train_action(
    online: &mut OnlineGameplayCommandParams,
    team: Team,
    faction: SkirmishFaction,
    action: BuildAction,
    selected_structures: &Query<StructureEntityItem<'_>, With<Selected>>,
    producer_structures: &Query<StructureEntityItem<'_>>,
    structures: &Query<StructurePrereqItem<'_>>,
    economies: &Economies,
    build_queue: &BuildQueue,
    batch_to_limit: bool,
) -> EnqueueBuildActionResult {
    let BuildAction::Train(product_id) = action else {
        return EnqueueBuildActionResult::Unavailable;
    };
    let Some(def) = registry::entity(product_id) else {
        return EnqueueBuildActionResult::Unavailable;
    };
    if !requirements_met(def, team, structures) {
        return EnqueueBuildActionResult::Unavailable;
    }
    let charged_cost = faction_unit_cost(Some(faction), def.cost);
    if !economies.get(team).can_afford(charged_cost) {
        return EnqueueBuildActionResult::NotEnoughResources;
    }
    let producers = match production_origins_for_faction(
        team,
        faction,
        product_id,
        selected_structures,
        producer_structures,
        build_queue,
    ) {
        Ok(producers) => producers,
        Err(result) => return result,
    };
    let producers = producers
        .into_iter()
        .map(|(entity, _, _)| online.network_id_for(entity))
        .collect::<Option<Vec<_>>>();
    let Some(producers) = producers else {
        return EnqueueBuildActionResult::Unavailable;
    };
    if online.submit(OnlinePlayerCommand::TrainUnits {
        producers,
        unit_id: product_id.to_string(),
        batch_to_limit,
    }) {
        EnqueueBuildActionResult::Enqueued
    } else {
        EnqueueBuildActionResult::Unavailable
    }
}

pub(crate) fn request_selected_deploy_toggle(
    commands: &mut Commands,
    team: Team,
    selected_units: &Query<SelectedCommandUnitItem<'_>, SelectedCommandUnitFilter>,
) -> bool {
    let mut requested_any = false;
    for (entity, unit, unit_team, ..) in selected_units {
        if *unit_team == team && is_deployable_vehicle(unit.id) {
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
