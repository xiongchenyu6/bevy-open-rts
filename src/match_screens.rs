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
