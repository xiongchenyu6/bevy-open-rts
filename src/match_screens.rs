//! Match briefing (opening objective card) and match-end overlay
//! (victory/defeat title, stats + sparkline charts, rematch buttons).

use bevy::prelude::*;

use crate::*;

pub(crate) const MATCH_END_TITLE_COLOR: Color = Color::srgb(0.98, 0.96, 0.42);

pub(crate) const MATCH_END_BG_COLOR: Color = Color::srgba(0.04, 0.05, 0.08, 0.86);

pub(crate) const MATCH_END_TITLE_FONT_SIZE: f32 = 34.0;

pub(crate) const MATCH_END_TEXT_FONT_SIZE: f32 = 19.0;

pub(crate) const MATCH_BRIEFING_AUTO_HIDE_SECONDS: f32 = 14.0;

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
    ContinueCampaign,
}

#[derive(Component)]
pub(crate) struct CampaignContinueButton;
#[derive(Component)]
pub(crate) struct CampaignContinueLabel;

#[derive(Component)]
pub(crate) struct MatchReturnLabel;

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
    active_mission: Res<ActiveMission>,
    tutorial: Res<TutorialState>,
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

    if tutorial.active {
        briefing.dismiss();
    }
    let panel_visibility = if briefing.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let reopen_visibility = if briefing.visible || tutorial.active {
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
        if let Some(mission) = active_mission.0.and_then(mission_by_index) {
            **text = format!(
                "{}\n\n{}",
                t(mission.name_zh, mission.name_en),
                t(mission.briefing_zh, mission.briefing_en)
            );
        }
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
        match settings.victory_condition {
            VictoryCondition::Headquarters => t(
                "目标：摧毁所有敌方指挥中心，保护你的指挥中心",
                "Objective: destroy enemy Command Centers and protect your own"
            ),
            VictoryCondition::Annihilation => t(
                "目标：消灭敌方基地和工人，保护你的基地",
                "Objective: destroy enemy bases and workers while protecting your own base"
            ),
        },
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

pub(crate) fn match_end_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    match_state: Res<MatchState>,
    mut match_menu: ResMut<MatchMenuState>,
    online_session: Option<Res<OnlineSession>>,
    mut online_lifecycle: Option<ResMut<OnlineLifecycleControl>>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut active_mission: ResMut<ActiveMission>,
    mut trigger_state: ResMut<MissionTriggerState>,
    mut setup: ResMut<MatchSetupSettings>,
    mut selected_map: ResMut<SelectedSkirmishMap>,
    mut tutorial: ResMut<TutorialState>,
    mut buttons: Query<(
        &Interaction,
        &MatchEndButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let match_finished = !match_state.is_running();
    let online_match = online_match_uses_command_transport(online_session.as_deref());
    for (interaction, button, mut background, mut border) in &mut buttons {
        let enabled = match_end_action_enabled(button.action, match_finished, online_match)
            && (button.action != MatchEndAction::ContinueCampaign
                || (match_state.phase == MatchPhase::HumanVictory
                    && (active_mission.0.is_some() || tutorial.stage == TutorialStage::Complete)));
        let clicked = enabled
            && *interaction == Interaction::Pressed
            && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match_menu.visible = false;
            match button.action {
                MatchEndAction::Restart => {
                    *trigger_state = MissionTriggerState::default();
                    if tutorial.active || tutorial.stage == TutorialStage::Complete {
                        *tutorial = TutorialState {
                            active: true,
                            ..default()
                        };
                    }
                    next_state.set(AppScreen::RestartingMatch);
                }
                MatchEndAction::ContinueCampaign => {
                    if let Some(index) = active_mission.0.map(|index| index + 1)
                        && let Some(mission) = mission_by_index(index)
                        && let Some(settings) = match_settings_for_mission(mission)
                    {
                        selected_map.godot_path = settings.map_path;
                        *setup = settings;
                        active_mission.0 = Some(index);
                        *trigger_state = MissionTriggerState::default();
                        *tutorial = TutorialState::default();
                        next_state.set(AppScreen::RestartingMatch);
                    } else {
                        *tutorial = TutorialState::default();
                        next_state.set(AppScreen::CampaignMenu);
                    }
                }
                MatchEndAction::ReturnToSetup => {
                    if online_match {
                        if let Some(lifecycle) = online_lifecycle.as_deref_mut() {
                            lifecycle.request_return_to_lobby();
                        }
                    } else {
                        next_state.set(if active_mission.0.is_some() {
                            AppScreen::CampaignMenu
                        } else {
                            AppScreen::SkirmishSetup
                        });
                    }
                }
                MatchEndAction::ExitToMenu => {
                    if online_match {
                        if let Some(lifecycle) = online_lifecycle.as_deref_mut() {
                            lifecycle.request_leave_session();
                        }
                    } else {
                        next_state.set(AppScreen::MainMenu);
                    }
                }
            }
        }

        let (bg, border_color) = match_end_button_visual(*interaction, enabled);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

pub(crate) fn match_end_action_enabled(
    action: MatchEndAction,
    match_finished: bool,
    online_match: bool,
) -> bool {
    match_finished
        && (!online_match
            || !matches!(
                action,
                MatchEndAction::Restart | MatchEndAction::ContinueCampaign
            ))
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
    tutorial: Res<TutorialState>,
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    mut overlay_q: Query<
        (&mut Visibility, &Children),
        (With<MatchEndOverlay>, Without<CommandStack>),
    >,
    mut command_stacks: Query<&mut Visibility, (With<CommandStack>, Without<MatchEndOverlay>)>,
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
    for mut visibility in &mut command_stacks {
        *visibility = if match_state.is_running() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
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
        **title_text = if tutorial.stage == TutorialStage::Complete {
            t("基础训练完成", "Boot Camp Complete")
        } else {
            t("对局结算", "Match Results")
        }
        .to_string();
    }
    if let Ok(mut reason_text) = reason_text_q.single_mut() {
        **reason_text = if tutorial.stage == TutorialStage::Complete {
            t(
                "你已准备好迎接第一场战役。",
                "You are ready for your first campaign mission.",
            )
        } else {
            match_state.result_reason
        }
        .to_string();
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
            "{}: {}  {}: {:02}:{:02}\n{enemy_losses}\n{human_losses}\n{resources}",
            t("剩余阵营", "Teams left"),
            match_state.remaining_teams,
            t("用时", "Time"),
            minutes,
            seconds
        );
    }
}

pub(crate) fn update_campaign_continue_button(
    state: Res<MatchState>,
    active: Res<ActiveMission>,
    tutorial: Res<TutorialState>,
    mut buttons: Query<&mut Node, With<CampaignContinueButton>>,
    mut labels: Query<&mut Text, With<CampaignContinueLabel>>,
    mut return_labels: Query<&mut Text, (With<MatchReturnLabel>, Without<CampaignContinueLabel>)>,
) {
    let available = state.phase == MatchPhase::HumanVictory
        && (active.0.is_some() || tutorial.stage == TutorialStage::Complete);
    for mut button in &mut buttons {
        button.display = if available {
            Display::Flex
        } else {
            Display::None
        };
    }
    let text = if active.0.is_some_and(|i| i + 1 < CAMPAIGN_MISSIONS.len()) {
        t("下一关", "Next Mission")
    } else {
        t("战役地图", "Campaign Map")
    };
    for mut label in &mut labels {
        label.0 = text.to_string();
    }
    for mut label in &mut return_labels {
        label.0 = if active.0.is_some() {
            t("战役地图", "Campaign Map")
        } else {
            t("遭遇战设置", "Skirmish Setup")
        }
        .to_string();
    }
}
