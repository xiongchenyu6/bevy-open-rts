//! The in-match HUD: resource bar, minimap, battle log, objective tracker,
//! selection panel, command card + production queue, support-power strip,
//! match menu/briefing overlays, HUD hit zones and the RTS cursor.
//!
//! Pure move out of lib.rs (module-split Stage 5); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

#[derive(Resource)]
pub(crate) struct RtsCursorAssetHandle(pub(crate) Handle<StaticCursor>);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppliedRtsCursor {
    pub(crate) index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RtsCursorKind {
    Default,
    Move,
    Attack,
    Build,
}

impl RtsCursorKind {
    pub(crate) fn atlas_index(self) -> usize {
        match self {
            Self::Default => 0,
            Self::Move => 1,
            Self::Attack => 2,
            Self::Build => 3,
        }
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchMenuState {
    pub(crate) visible: bool,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatchBriefingState {
    pub(crate) visible: bool,
    pub(crate) elapsed_seconds: f32,
    pub(crate) auto_hide_seconds: f32,
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
    pub(crate) fn show(&mut self) {
        self.visible = true;
        self.elapsed_seconds = 0.0;
    }

    pub(crate) fn dismiss(&mut self) {
        self.visible = false;
        self.elapsed_seconds = 0.0;
    }
}

#[derive(Component)]
pub(crate) struct MatchMenuOverlay;

#[derive(Component)]
pub(crate) struct MatchMenuStatusText;

#[derive(Component)]
pub(crate) struct MatchMenuFullscreenText;

#[derive(Component)]
pub(crate) struct MatchBriefingPanel;

#[derive(Component)]
pub(crate) struct MatchBriefingText;

#[derive(Component)]
pub(crate) struct MatchBriefingReopenButton;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchBriefingButton {
    pub(crate) action: MatchBriefingAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatchBriefingAction {
    Show,
    Dismiss,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchMenuButton {
    pub(crate) action: MatchMenuAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatchMenuAction {
    Resume,
    SetSpeed(MatchSpeedPreset),
    PreviousPerspective,
    NextPerspective,
    ToggleFullscreen,
    Restart,
    ReturnToSetup,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MinimapContentRect {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) fn load_rts_cursor(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RtsCursorAssetHandle(
        asset_server.load(RTS_CURSOR_ASSET_PATH),
    ));
}

pub(crate) fn update_rts_cursor(
    mut commands: Commands,
    cursor_handle: Option<Res<RtsCursorAssetHandle>>,
    static_cursors: Res<Assets<StaticCursor>>,
    command_mode: Option<Res<CommandMode>>,
    hovered_resource: Option<Res<HoveredResource>>,
    hud_zones: Res<HudHitZones>,
    window_q: Query<(Entity, Option<&AppliedRtsCursor>, &Window), With<PrimaryWindow>>,
) {
    let Some(cursor_handle) = cursor_handle else {
        return;
    };
    let Some(cursor_asset) = static_cursors.get(&cursor_handle.0) else {
        return;
    };
    let Ok((window_entity, applied, window)) = window_q.single() else {
        return;
    };
    let index = desired_rts_cursor_kind(
        command_mode.as_deref(),
        hovered_resource.as_deref(),
        window,
        &hud_zones,
    )
    .atlas_index();
    if applied.is_some_and(|applied| applied.index == index) {
        return;
    }
    commands.entity(window_entity).insert((
        CursorIcon::Custom(
            CustomCursorImageBuilder::from_static_cursor(cursor_asset, Some(index)).build(),
        ),
        AppliedRtsCursor { index },
    ));
}

pub(crate) fn desired_rts_cursor_kind(
    command_mode: Option<&CommandMode>,
    hovered_resource: Option<&HoveredResource>,
    window: &Window,
    hud_zones: &HudHitZones,
) -> RtsCursorKind {
    if cursor_is_over_hud(window, hud_zones) {
        return RtsCursorKind::Default;
    }
    let Some(command_mode) = command_mode else {
        return RtsCursorKind::Default;
    };
    if command_mode.pending_structure_placement.is_some() {
        RtsCursorKind::Build
    } else if command_mode.attack_move || command_mode.support_power.is_some() {
        RtsCursorKind::Attack
    } else if command_mode.patrol || command_mode.rally_point {
        RtsCursorKind::Move
    } else if hovered_resource.is_some_and(|hovered| hovered.0.is_some()) {
        RtsCursorKind::Build
    } else {
        RtsCursorKind::Default
    }
}

/// Gizmo group for thick world-space HUD lines (health bars, tracers).
#[derive(Default, Reflect, GizmoConfigGroup)]
pub(crate) struct HudGizmos;

#[derive(Resource, Default, Debug)]
pub(crate) struct SelectionDragState {
    pub(crate) active: bool,
    pub(crate) dragging: bool,
    pub(crate) start: Vec2,
    pub(crate) started_in_hud: bool,
}

#[derive(Component)]
pub(crate) struct SelectionDragBox;

#[derive(Clone)]
pub(crate) struct BattleLogEntry {
    pub(crate) message: String,
    pub(crate) remaining: f32,
    pub(crate) focus: Option<Vec3>,
    pub(crate) ping_kind: BattleEventPingKind,
    pub(crate) minimap_ping_active: bool,
    pub(crate) minimap_ping_remaining: f32,
}

#[derive(Resource, Default)]
pub(crate) struct BattleLog {
    pub(crate) entries: VecDeque<BattleLogEntry>,
    pub(crate) under_attack_cooldown: f32,
}

#[derive(Component)]
pub(crate) struct StatsText;

/// Top-left resource/power bar (godot ResourcesBar): per-resource count label.
#[derive(Component)]
pub(crate) struct HudResourceCount(pub(crate) ResourceKind);

/// The "used/supply" power readout in the resource bar (color-coded).
#[derive(Component)]
pub(crate) struct HudPowerText;

/// The "low power" warning shown only when underpowered.
#[derive(Component)]
pub(crate) struct HudLowPowerText;

#[derive(Component)]
pub(crate) struct SelectionText;

#[derive(Component)]
pub(crate) struct SelectionPortrait;

#[derive(Component)]
pub(crate) struct BattleLogRoot {
    pub(crate) font: Handle<Font>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct BattleLogEntryButton(pub(crate) usize);

#[derive(Component)]
pub(crate) struct ObjectiveTrackerText;

/// The fill node of the top-center objective progress bar (godot MissionProgressBar).
#[derive(Component)]
pub(crate) struct ObjectiveProgressFill;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ObjectiveTrackerState {
    pub(crate) max_enemy_anchors_seen: u32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ProductionQueueSlot(pub(crate) usize);

#[derive(Component, Clone, Copy)]
pub(crate) struct ProductionQueueSlotLabel(pub(crate) usize);

/// The "×N" count badge in a queued slot's bottom-right corner (aggregated units).
#[derive(Component, Clone, Copy)]
pub(crate) struct ProductionQueueSlotCount(pub(crate) usize);

#[derive(Component, Clone, Copy, Default)]
pub(crate) struct ProductionQueueSlotTarget {
    pub(crate) producer_entity: Option<Entity>,
    pub(crate) local_index: usize,
}

#[derive(Component)]
pub(crate) struct MinimapRoot;

#[derive(Component)]
pub(crate) struct MinimapContent;

#[derive(Component)]
pub(crate) struct MinimapStatusText;

#[derive(Component)]
pub(crate) struct MinimapMarker;

#[derive(Component, Clone, Copy)]
pub(crate) struct CommandSlot(pub(crate) usize);

#[derive(Component, Clone, Copy)]
pub(crate) struct CommandSlotLabel(pub(crate) usize);

#[derive(Component, Clone, Copy)]
pub(crate) struct CommandSlotIcon(pub(crate) usize);

#[derive(Component, Clone, Copy)]
pub(crate) struct CommandSlotAvailability {
    pub(crate) enabled: bool,
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SupportPowerPanelState {
    pub(crate) visible_count: usize,
}

impl Default for CommandSlotAvailability {
    fn default() -> Self {
        Self { enabled: false }
    }
}

pub(crate) fn match_menu_fullscreen_button_text(fullscreen: bool) -> &'static str {
    if fullscreen {
        t("窗口模式", "Windowed")
    } else {
        t("全屏", "Fullscreen")
    }
}

pub(crate) fn setup_match_end_overlay(commands: &mut Commands, font: Handle<Font>) {
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
                        localized_text("结算", "Results"),
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
                    for (chart, zh, en) in [
                        (MatchEndChart::Army, "兵力曲线", "Army over time"),
                        (MatchEndChart::Economy, "经济曲线", "Economy over time"),
                    ] {
                        panel.spawn((
                            localized_text(zh, en),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.62, 0.72, 0.7, 0.95)),
                        ));
                        panel.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: px(46),
                                flex_direction: FlexDirection::Row,
                                column_gap: px(2),
                                align_items: AlignItems::FlexEnd,
                                padding: UiRect::all(px(3)),
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.2, 0.24, 0.26, 0.9)),
                            BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.85)),
                            chart,
                        ));
                    }
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
                                    button.spawn(match_end_button_label(
                                        "重开对局",
                                        "Restart Match",
                                        font.clone(),
                                    ));
                                });
                            row.spawn(match_end_button(MatchEndAction::ReturnToSetup))
                                .with_children(|button| {
                                    button.spawn(match_end_button_label(
                                        "返回设置",
                                        "Back to Setup",
                                        font.clone(),
                                    ));
                                });
                            row.spawn(match_end_button(MatchEndAction::ExitToMenu))
                                .with_children(|button| {
                                    button.spawn(match_end_button_label(
                                        "退出菜单",
                                        "Exit to Menu",
                                        font.clone(),
                                    ));
                                });
                        });
                });
        });
}

pub(crate) fn setup_match_menu_overlay(commands: &mut Commands, font: Handle<Font>) {
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
                        min_height: px(370),
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
                        localized_text("对局菜单", "Match Menu"),
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
                            button.spawn(match_menu_button_label(
                                "继续战斗",
                                "Resume Battle",
                                font.clone(),
                            ));
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
                                    button.spawn(match_menu_button_label(
                                        "上一视角",
                                        "Prev View",
                                        font.clone(),
                                    ));
                                });
                            row.spawn(match_menu_button(MatchMenuAction::NextPerspective))
                                .with_children(|button| {
                                    button.spawn(match_menu_button_label(
                                        "下一视角",
                                        "Next View",
                                        font.clone(),
                                    ));
                                });
                        });
                    panel
                        .spawn(match_menu_button(MatchMenuAction::ToggleFullscreen))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(17.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.92, 0.96, 0.98)),
                                MatchMenuFullscreenText,
                            ));
                        });
                    panel
                        .spawn(match_menu_button(MatchMenuAction::Restart))
                        .with_children(|button| {
                            button.spawn(match_menu_button_label(
                                "重开对局",
                                "Restart Match",
                                font.clone(),
                            ));
                        });
                    panel
                        .spawn(match_menu_button(MatchMenuAction::ReturnToSetup))
                        .with_children(|button| {
                            button.spawn(match_menu_button_label(
                                "返回设置",
                                "Back to Setup",
                                font.clone(),
                            ));
                        });
                });
        });
}

pub(crate) fn match_menu_speed_row(font: Handle<Font>) -> impl Bundle {
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
                localized_text("游戏速度", "Game Speed"),
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

pub(crate) fn match_menu_speed_button(preset: MatchSpeedPreset, font: Handle<Font>) -> impl Bundle {
    (
        match_menu_button(MatchMenuAction::SetSpeed(preset)),
        children![match_menu_button_label(
            preset.label(),
            preset.label(),
            font
        )],
    )
}

pub(crate) fn match_menu_button(action: MatchMenuAction) -> impl Bundle {
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

pub(crate) fn match_menu_button_label(
    zh: &'static str,
    en: &'static str,
    font: Handle<Font>,
) -> impl Bundle {
    (
        localized_text(zh, en),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

pub(crate) fn setup_match_briefing(commands: &mut Commands, font: Handle<Font>) {
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
                localized_text("目标", "Objectives"),
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
                        localized_text("战斗简报", "Briefing"),
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

/// One resource group in the top-left bar: a colored swatch (the mineral color) +
/// a count label that `update_resource_bar` keeps current.
pub(crate) fn spawn_hud_resource_group(
    bar: &mut ChildSpawnerCommands,
    kind: ResourceKind,
    font: Handle<Font>,
) {
    bar.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(6),
        ..default()
    })
    .with_children(|group| {
        group.spawn((
            Node {
                width: px(14),
                height: px(14),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            BackgroundColor(kind.color()),
        ));
        group.spawn((
            Text::new("0"),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.97, 1.0)),
            HudResourceCount(kind),
        ));
    });
}

pub(crate) fn update_resource_bar(
    economies: Res<Economies>,
    visible_player: Res<VisiblePlayer>,
    mut counts: Query<
        (&HudResourceCount, &mut Text),
        (Without<HudPowerText>, Without<HudLowPowerText>),
    >,
    mut power_text: Query<
        &mut Text,
        (
            With<HudPowerText>,
            Without<HudResourceCount>,
            Without<HudLowPowerText>,
        ),
    >,
    mut power_color: Query<&mut TextColor, With<HudPowerText>>,
    mut low_power: Query<&mut Visibility, With<HudLowPowerText>>,
) {
    let econ = economies.get(visible_player.team);
    for (count, mut text) in &mut counts {
        let value = match count.0 {
            ResourceKind::Ore => econ.ore,
            ResourceKind::Crystal => econ.crystal,
        };
        let next = value.to_string();
        if text.0 != next {
            text.0 = next;
        }
    }
    let low = econ.low_power();
    let pwr = power_readout_text(econ);
    for mut text in &mut power_text {
        if text.0 != pwr {
            text.0 = pwr.clone();
        }
    }
    let color = if low {
        Color::srgb(1.0, 0.42, 0.32)
    } else {
        Color::srgb(0.72, 1.0, 0.74)
    };
    for mut text_color in &mut power_color {
        text_color.0 = color;
    }
    for mut visibility in &mut low_power {
        *visibility = if low {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Hide the status / selected-unit panels when their text is empty, so their
/// background boxes don't linger (the top-left strip is empty whenever no command
/// mode is active, leaving just the resource bar like godot).
pub(crate) fn update_selection_text_visibility(
    mut query: Query<(&Text, &mut Visibility), Or<(With<SelectionText>, With<StatsText>)>>,
) {
    for (text, mut visibility) in &mut query {
        let wanted = if text.0.trim().is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

pub(crate) fn setup_ui(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load(UI_FONT_PATH);

    // Top-left resource/power bar (godot ResourcesBar): colored swatch + count per
    // resource, then a color-coded power readout + a low-power warning.
    commands
        .spawn((
            Name::new("Resource Bar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(10),
                left: px(12),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(16),
                padding: UiRect::new(px(12), px(14), px(6), px(6)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.26, 0.32, 0.32)),
            BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.82)),
            MatchScopedEntity,
        ))
        .with_children(|bar| {
            spawn_hud_resource_group(bar, ResourceKind::Ore, font.clone());
            spawn_hud_resource_group(bar, ResourceKind::Crystal, font.clone());
            // Power group.
            bar.spawn((
                localized_text("电力", "PWR"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.86, 0.72)),
            ));
            bar.spawn((
                Text::new("0/0"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 1.0, 0.74)),
                HudPowerText,
            ));
            bar.spawn((
                localized_text("低电力", "LOW POWER"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.42, 0.32)),
                Visibility::Hidden,
                HudLowPowerText,
            ));
        });

    // Global status (team / units / AI / mode) — a styled strip just under the
    // resource bar.
    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.92, 0.96)),
        Node {
            position_type: PositionType::Absolute,
            top: px(52),
            left: px(12),
            padding: UiRect::new(px(8), px(10), px(3), px(3)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.22, 0.28, 0.28)),
        BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.7)),
        StatsText,
        MatchScopedEntity,
    ));

    // Selected-unit details — bottom-left, just above the portrait/command card so
    // the unit's text sits with its icon (like godot's unit panel) instead of
    // overlapping the top-left status.
    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.92, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            // Bottom-center, to the right of the portrait (godot SelectionInfo panel).
            bottom: px(12),
            left: px(272),
            max_width: px(372),
            padding: UiRect::new(px(8), px(10), px(3), px(3)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.24, 0.3, 0.32)),
        BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.7)),
        SelectionText,
        MatchScopedEntity,
    ));

    commands.spawn((
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            // godot: selection/unit panel sits bottom-center (between minimap and command grid).
            left: px(200),
            bottom: px(12),
            width: px(64),
            height: px(64),
            border: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.32, 0.4, 0.46)),
        Visibility::Hidden,
        SelectionPortrait,
        MatchScopedEntity,
    ));

    // godot: objective tracker (+ progress bar) centered near the top.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(OBJECTIVE_TRACKER_TOP_PX),
                left: px(0),
                right: px(0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            },
            MatchScopedEntity,
        ))
        .with_children(|center| {
            center.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.98, 0.94)),
                TextLayout::justify(Justify::Center),
                Node {
                    max_width: px(460),
                    // Dark rounded backing so the objective reads over the bright
                    // sand terrain instead of washing out.
                    padding: UiRect::axes(px(12.0), px(6.0)),
                    border_radius: BorderRadius::all(px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.74)),
                ObjectiveTrackerText,
            ));
            center
                .spawn((
                    Node {
                        width: px(300),
                        height: px(8),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.3, 0.4, 0.42)),
                    BackgroundColor(Color::srgba(0.02, 0.05, 0.04, 0.7)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.8, 0.42)),
                        ObjectiveProgressFill,
                    ));
                });
        });

    // godot: battle notifications/log centered, just below the objective tracker.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(BATTLE_LOG_TOP_PX),
            left: px(0),
            right: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            align_items: AlignItems::Center,
            ..default()
        },
        BattleLogRoot { font: font.clone() },
        MatchScopedEntity,
    ));

    setup_minimap(commands, font.clone());
    setup_selection_drag_box(commands);
    setup_match_end_overlay(commands, font.clone());
    setup_match_menu_overlay(commands, font.clone());
    setup_match_briefing(commands, font.clone());
    setup_support_power_panel(commands, font.clone(), asset_server);

    // godot: production queue stacked directly above the command grid, both pinned to
    // the bottom-right corner. A column container so the queue always hugs the top of
    // the grid (no floating) and both size to their content.
    let queue_font = font.clone();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                bottom: px(12),
                width: px(612),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(8),
                ..default()
            },
            MatchScopedEntity,
        ))
        .with_children(|stack| {
            // Production queue row (above the command grid).
            stack
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(6),
                    row_gap: px(6),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|parent| {
                    for index in 0..PRODUCTION_QUEUE_HUD_SLOT_COUNT {
                        parent
                            .spawn(production_queue_slot(index))
                            .with_children(|slot| {
                                slot.spawn(production_queue_slot_label(index, queue_font.clone()));
                                slot.spawn(production_queue_slot_count(index, queue_font.clone()));
                            });
                    }
                });
            // Command/action grid (below).
            stack
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(8),
                    row_gap: px(8),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|parent| {
                    for index in 0..COMMAND_SLOT_COUNT {
                        parent.spawn(command_button(index)).with_children(|button| {
                            button.spawn(command_button_icon(index));
                            button.spawn(command_button_label(index, font.clone()));
                        });
                    }
                });
        });
    setup_command_tooltip(commands, font);
}

pub(crate) fn setup_selection_drag_box(commands: &mut Commands) {
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

pub(crate) fn setup_minimap(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(MINIMAP_LEFT_PX),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ObjectiveTrackerSnapshot {
    pub(crate) enemy_teams: u32,
    pub(crate) remaining_anchors: u32,
    pub(crate) total_anchors: u32,
    pub(crate) structures: u32,
    pub(crate) workers: u32,
    pub(crate) completion_percent: u32,
}

pub(crate) fn update_battle_log(
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

pub(crate) fn spawn_battle_log_entry(
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

pub(crate) fn battle_log_entry_text(font: Handle<Font>, entry: &BattleLogEntry) -> impl Bundle {
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

pub(crate) fn battle_log_entry_button_color(interaction: Interaction) -> Color {
    match interaction {
        Interaction::Pressed => Color::srgba(0.18, 0.24, 0.16, 0.62),
        Interaction::Hovered => Color::srgba(0.13, 0.17, 0.12, 0.5),
        Interaction::None => Color::srgba(0.0, 0.0, 0.0, 0.0),
    }
}

pub(crate) fn push_battle_log(
    battle_log: &mut BattleLog,
    message: impl Into<String>,
    focus: Option<Vec3>,
) {
    push_battle_log_with_kind(battle_log, message, focus, BattleEventPingKind::Generic);
}

/// Promotions fire in bursts (alliance doubles veterancy XP); collapse
/// consecutive promotion entries into one "×N" line so the battle log
/// doesn't flood mid-screen during a big fight.
pub(crate) fn push_promotion_battle_log(
    battle_log: &mut BattleLog,
    unit_label: &str,
    rank: u8,
    focus: Option<Vec3>,
) {
    let prefix = t("单位晋升", "Unit promoted");
    let detail = format!("{unit_label} {}{rank}", t("等级", "Lv"));
    if let Some(last) = battle_log.entries.back_mut()
        && last.message.starts_with(prefix)
    {
        let count = last
            .message
            .split('×')
            .nth(1)
            .and_then(|rest| rest.split(':').next())
            .and_then(|n| n.trim().parse::<u32>().ok())
            .unwrap_or(1)
            + 1;
        last.message = format!("{prefix} ×{count}: {detail}");
        last.remaining = BATTLE_LOG_ENTRY_TTL_SECONDS;
        if focus.is_some() {
            last.focus = focus;
            last.minimap_ping_active = true;
            last.minimap_ping_remaining = BATTLE_EVENT_PING_LIFETIME_SECONDS;
        }
        return;
    }
    push_battle_log(battle_log, format!("{prefix}: {detail}"), focus);
}

pub(crate) fn push_battle_log_with_kind(
    battle_log: &mut BattleLog,
    message: impl Into<String>,
    focus: Option<Vec3>,
    ping_kind: BattleEventPingKind,
) {
    let message = message.into();
    // Collapse repeats (e.g. spamming "资源不足"): if the newest entry says the
    // same thing, just refresh its lifetime instead of stacking duplicates.
    if let Some(last) = battle_log.entries.back_mut()
        && last.message == message
    {
        last.remaining = BATTLE_LOG_ENTRY_TTL_SECONDS;
        if focus.is_some() {
            last.focus = focus;
            last.minimap_ping_active = true;
            last.minimap_ping_remaining = BATTLE_EVENT_PING_LIFETIME_SECONDS;
        }
        return;
    }
    battle_log.entries.push_back(BattleLogEntry {
        message,
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

pub(crate) fn minimap_entity_marker_style(
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

pub(crate) fn minimap_marker_bundle(
    world: Vec3,
    size: f32,
    color: Color,
    bounds: MapBounds,
) -> impl Bundle {
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

pub(crate) fn minimap_camera_marker_bundle(world: Vec3, bounds: MapBounds) -> impl Bundle {
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

pub(crate) fn minimap_ping_bundle(
    world: Vec3,
    size: f32,
    color: Color,
    bounds: MapBounds,
) -> impl Bundle {
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

pub(crate) fn minimap_ping_size(kind: BattleEventPingKind) -> f32 {
    match kind {
        BattleEventPingKind::Generic => 18.0,
        BattleEventPingKind::SupportPower => 22.0,
        BattleEventPingKind::EnemySupportPower => 24.0,
        BattleEventPingKind::EnemySuperweapon => 31.0,
    }
}

pub(crate) fn minimap_ping_progress(entry: &BattleLogEntry) -> f32 {
    1.0 - (entry.minimap_ping_remaining / BATTLE_EVENT_PING_LIFETIME_SECONDS).clamp(0.0, 1.0)
}

pub(crate) fn minimap_ping_size_at_progress(kind: BattleEventPingKind, progress: f32) -> f32 {
    let min = match kind {
        BattleEventPingKind::Generic | BattleEventPingKind::SupportPower => 5.0,
        BattleEventPingKind::EnemySupportPower => 6.0,
        BattleEventPingKind::EnemySuperweapon => 7.0,
    };
    min.lerp(minimap_ping_size(kind), progress.clamp(0.0, 1.0))
}

pub(crate) fn minimap_ping_color_at_progress(kind: BattleEventPingKind, progress: f32) -> Color {
    let alpha_scale = 1.0 - progress.clamp(0.0, 1.0);
    match kind {
        BattleEventPingKind::Generic => Color::srgba(1.0, 0.92, 0.32, 0.9 * alpha_scale),
        BattleEventPingKind::SupportPower => Color::srgba(0.35, 0.82, 1.0, 0.95 * alpha_scale),
        BattleEventPingKind::EnemySupportPower => Color::srgba(1.0, 0.44, 0.18, 0.96 * alpha_scale),
        BattleEventPingKind::EnemySuperweapon => Color::srgba(1.0, 0.12, 0.08, alpha_scale),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MinimapRadarState {
    Online,
    MissingRadar,
    LowPower,
}

impl MinimapRadarState {
    pub(crate) fn status_text(self) -> &'static str {
        match self {
            Self::Online => "",
            Self::MissingRadar => t(
                "雷达离线\n建造雷达站",
                "Radar offline\nBuild a Radar Uplink",
            ),
            Self::LowPower => t("雷达离线\n电力不足", "Radar offline\nNot enough power"),
        }
    }
}

pub(crate) fn minimap_radar_state(has_radar: bool, low_power: bool) -> MinimapRadarState {
    if !has_radar {
        MinimapRadarState::MissingRadar
    } else if low_power {
        MinimapRadarState::LowPower
    } else {
        MinimapRadarState::Online
    }
}

pub(crate) fn command_button(index: usize) -> impl Bundle {
    (
        Button,
        BuildAction::None,
        CommandSlot(index),
        CommandSlotAvailability::default(),
        Node {
            width: px(146),
            height: px(46),
            border: UiRect::all(px(1)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            column_gap: px(6),
            padding: UiRect::horizontal(px(6)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.34, 0.39)),
        BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.78)),
    )
}

pub(crate) fn command_button_icon(index: usize) -> impl Bundle {
    (
        ImageNode::default(),
        Node {
            width: px(36),
            height: px(36),
            ..default()
        },
        Visibility::Hidden,
        CommandSlotIcon(index),
    )
}

pub(crate) fn command_button_label(index: usize, font: Handle<Font>) -> impl Bundle {
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

pub(crate) fn setup_support_power_panel(
    commands: &mut Commands,
    font: Handle<Font>,
    asset_server: &AssetServer,
) {
    commands
        .spawn((
            Name::new("Support Powers"),
            SupportPowersPanel,
            GlobalZIndex(18),
            Node {
                position_type: PositionType::Absolute,
                top: px(SUPPORT_POWER_PANEL_TOP_PX),
                right: px(SUPPORT_POWER_PANEL_RIGHT_PX),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(SUPPORT_POWER_BUTTON_GAP_PX),
                padding: UiRect::all(px(SUPPORT_POWER_PANEL_PADDING_PX)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.25, 0.38, 0.38)),
            BackgroundColor(Color::srgba(0.02, 0.035, 0.04, 0.82)),
            MatchScopedEntity,
        ))
        .with_children(|panel| {
            for spec in support_power_button_specs() {
                panel
                    .spawn(support_power_button(spec.kind))
                    .with_children(|button| {
                        button.spawn((
                            ImageNode::new(asset_server.load(spec.icon_path)),
                            Node {
                                width: px(58),
                                height: px(58),
                                ..default()
                            },
                        ));
                        button.spawn((
                            Text::new(spec.hotkey_label),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.96, 0.92)),
                            TextLayout::justify(Justify::Center),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(3),
                                top: px(2),
                                width: px(28),
                                ..default()
                            },
                            SupportPowerHotkeyLabel { kind: spec.kind },
                        ));
                        button.spawn((
                            Text::new(""),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.98, 0.84, 0.42)),
                            TextLayout::justify(Justify::Center),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                right: px(0),
                                bottom: px(2),
                                ..default()
                            },
                            SupportPowerCooldownLabel { kind: spec.kind },
                        ));
                    });
            }
        });
}

pub(crate) fn production_queue_slot(index: usize) -> impl Bundle {
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

pub(crate) fn production_queue_slot_label(index: usize, font: Handle<Font>) -> impl Bundle {
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

/// The "×N" count badge anchored to a queued slot's bottom-right corner.
pub(crate) fn production_queue_slot_count(index: usize, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.86, 0.42)),
        Node {
            position_type: PositionType::Absolute,
            right: px(3),
            bottom: px(1),
            ..default()
        },
        ProductionQueueSlotCount(index),
    )
}

pub(crate) fn cursor_is_over_interactive_button(
    window: &Window,
    buttons: &Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<Button>,
    >,
) -> bool {
    let Some(cursor) = window.physical_cursor_position() else {
        return false;
    };
    buttons.iter().any(|(node, transform, visibility)| {
        visibility.is_none_or(|visibility| visibility.get())
            && !node.is_empty()
            && node.contains_point(*transform, cursor)
    })
}

pub(crate) fn minimap_input(
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
    disarm_support_power_on_left_click(&mut order_resources.command_mode, &mouse, false);

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
                        RallyMode::Move,
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
                        kind: ClickMarkerKind::Move,
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
                    kind: ClickMarkerKind::Move,
                },
                MatchScopedEntity,
            ));
        }
    }
}

pub(crate) fn battle_log_entry_buttons(
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

pub(crate) fn update_selection_drag_box(
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

pub(crate) fn match_menu_input(
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

pub(crate) fn match_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut options: ResMut<MenuOptionsState>,
    mut match_menu: ResMut<MatchMenuState>,
    mut match_speed: ResMut<MatchSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
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
                MatchMenuAction::ToggleFullscreen => {
                    if let Ok(mut window) = windows.single_mut() {
                        options.fullscreen = toggle_window_fullscreen(&mut window);
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

pub(crate) fn match_menu_action_enabled(
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
        | MatchMenuAction::ToggleFullscreen
        | MatchMenuAction::Restart
        | MatchMenuAction::ReturnToSetup => true,
    }
}

pub(crate) fn match_menu_button_visual(
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

pub(crate) fn refresh_command_panel(
    build_queue: Res<BuildQueue>,
    build_structure_tab: Res<BuildStructureTab>,
    visible_player: Res<VisiblePlayer>,
    player_factions: Res<PlayerFactions>,
    selected_units: Query<CommandPanelUnitItem<'_>, With<Selected>>,
    selected_structures: Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    idle_workers: Query<IdleWorkerSelectionItem<'_>, With<Unit>>,
    producer_structures: Query<StructureEntityItem<'_>>,
    structures: Query<StructurePrereqItem<'_>>,
    mut slot_q: Query<(
        &CommandSlot,
        &mut BuildAction,
        &mut CommandSlotAvailability,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
    )>,
    mut label_q: Query<(&CommandSlotLabel, &mut Text, &mut TextColor)>,
    asset_server: Res<AssetServer>,
    mut icon_q: Query<(&CommandSlotIcon, &mut ImageNode, &mut Visibility)>,
) {
    let set_slot_icon =
        |slot_index: usize,
         action: Option<BuildAction>,
         icon_q: &mut Query<(&CommandSlotIcon, &mut ImageNode, &mut Visibility)>| {
            for (icon, mut image_node, mut visibility) in icon_q.iter_mut() {
                if icon.0 != slot_index {
                    continue;
                }
                match action.and_then(command_action_icon_path) {
                    Some(path) => {
                        image_node.image = asset_server.load(path);
                        *visibility = Visibility::Inherited;
                    }
                    None => *visibility = Visibility::Hidden,
                }
            }
        };
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        for (
            slot,
            mut action,
            mut availability,
            interaction,
            mut background,
            mut border,
            mut node,
        ) in &mut slot_q
        {
            let _ = slot;
            *action = BuildAction::None;
            availability.enabled = false;
            // No controlled selection -> collapse every slot so no empty grid shows.
            node.display = Display::None;
            let (bg, border_color) = command_button_colors(BuildAction::None, false, *interaction);
            *background = BackgroundColor(bg);
            *border = BorderColor::all(border_color);
        }
        for (_, mut text, mut text_color) in &mut label_q {
            **text = String::new();
            *text_color = command_button_text_color(BuildAction::None, false);
        }
        for (_, _, mut visibility) in &mut icon_q {
            *visibility = Visibility::Hidden;
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
        *build_structure_tab,
        has_idle_worker_for_team(visible_team, &idle_workers),
    );
    for (slot, mut action, mut availability, interaction, mut background, mut border, mut node) in
        &mut slot_q
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
        // Collapse empty slots so the grid only shows the unit's actual commands
        // (combat units have a few; workers fill many).
        node.display = if matches!(next_action, BuildAction::None) {
            Display::None
        } else {
            Display::Flex
        };
        let (mut bg, mut border_color) = command_button_colors(next_action, enabled, *interaction);
        if matches!(next_action, BuildAction::SelectBuildTab(tab) if tab == *build_structure_tab) {
            bg = Color::srgba(0.18, 0.13, 0.05, 0.96);
            border_color = Color::srgb(0.82, 0.58, 0.18);
        }
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
        set_slot_icon(slot.0, action, &mut icon_q);
    }
}

pub(crate) fn command_button_colors(
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

pub(crate) fn command_button_text_color(action: BuildAction, enabled: bool) -> TextColor {
    if action == BuildAction::None {
        TextColor(Color::srgba(0.48, 0.54, 0.58, 0.55))
    } else if enabled {
        TextColor(Color::srgb(0.9, 0.94, 0.96))
    } else {
        TextColor(Color::srgba(0.62, 0.68, 0.72, 0.68))
    }
}

pub(crate) fn command_buttons(
    mut commands: Commands,
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
        let (mut bg, mut border_color) =
            command_button_colors(*action, availability.enabled, effective_interaction);
        if matches!(*action, BuildAction::SelectBuildTab(tab) if tab == *action_resources.build_structure_tab)
        {
            bg = Color::srgba(0.18, 0.13, 0.05, 0.96);
            border_color = Color::srgb(0.82, 0.58, 0.18);
        }
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
}

pub(crate) fn production_queue_slot_buttons(
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

pub(crate) fn update_objective_tracker_hud(
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    active_mission: Res<ActiveMission>,
    match_state: Res<MatchState>,
    mut objective_tracker: ResMut<ObjectiveTrackerState>,
    structures: Query<(&Structure, &Team, &Health)>,
    units: Query<(&Unit, &Team, &Health)>,
    mut objective_text: Query<&mut Text, With<ObjectiveTrackerText>>,
    mut objective_fill: Query<&mut Node, With<ObjectiveProgressFill>>,
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
    if let Ok(mut fill) = objective_fill.single_mut() {
        fill.width = Val::Percent(snapshot.completion_percent as f32);
    }
    **text = objective_tracker_text(snapshot);
    // Survive missions show the countdown instead of the anchor objective.
    if let Some(remaining) = mission_survive_remaining(&active_mission, &match_state) {
        **text = format!(
            "{}: {}:{:02}",
            t("目标: 坚守", "Objective: hold out"),
            (remaining / 60.0) as u32,
            remaining as u32 % 60
        );
    }
}

pub(crate) fn update_hud(
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
    support_cooldowns: Res<SupportCooldowns>,
    mut stats_text: Query<
        &mut Text,
        (
            With<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
        ),
    >,
    mut selection_text: Query<
        &mut Text,
        (
            With<SelectionText>,
            Without<StatsText>,
            Without<ObjectiveTrackerText>,
        ),
    >,
    mut production_queue_slots: Query<(
        &ProductionQueueSlot,
        &mut ProductionQueueSlotTarget,
        &mut BackgroundColor,
        &mut Visibility,
        &mut Node,
    )>,
    mut production_queue_slot_labels: Query<
        (&ProductionQueueSlotLabel, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueSlotCount>,
        ),
    >,
    mut production_queue_slot_counts: Query<
        (&ProductionQueueSlotCount, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueSlotLabel>,
        ),
    >,
    command_mode: Res<CommandMode>,
    placement_feedback: Res<StructurePlacementFeedback>,
    unit_groups: Res<UnitGroups>,
) {
    let visible_team = visible_player.team;
    if let Ok(mut text) = stats_text.single_mut() {
        // godot's top-left is just the resource bar — show only transient command
        // feedback (placement / attack-move / patrol / rally / support); empty (and
        // hidden) when idle. No permanent player / unit / AI status line.
        let mode_text = if let Some(pending) = command_mode.pending_structure_placement {
            let label = localized_entity_label(pending.id);
            let feedback = placement_feedback
                .validity
                .and_then(structure_placement_feedback_text)
                .map(|message| format!(" {message}"))
                .unwrap_or_default();
            format!(
                "{}:{label}{feedback} {}",
                t("摆放", "Place"),
                t("R旋转 右键取消", "R rotate / right-click cancel")
            )
        } else if command_mode.attack_move {
            t("模式:攻击移动", "Mode: Attack-Move").to_string()
        } else if command_mode.patrol {
            t("模式:巡逻", "Mode: Patrol").to_string()
        } else if command_mode.rally_point {
            t("模式:设置集结", "Mode: Set Rally").to_string()
        } else if let Some(power) = command_mode.support_power {
            let remaining = support_cooldowns.remaining_for(visible_team, power);
            if remaining > 0.0 {
                format!(
                    "{}:{} ({}{remaining:.1}s)",
                    t("支援", "Support"),
                    power.label(),
                    t("冷却", "CD ")
                )
            } else {
                t("支援:就绪", "Support: Ready").to_string()
            }
        } else {
            String::new()
        };
        **text = mode_text.trim().to_string();
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
                .map(|unit| localized_entity_label(unit.id))
                .or_else(|| structure.map(|structure| localized_entity_label(structure.id)))
                .unwrap_or_else(|| t("实体", "Entity").to_string());
            items.push(SelectionHudItem {
                label,
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
        let observed_queue_producers = if selected_visible_count == selected_queue_producers.len() {
            selected_queue_producers.as_slice()
        } else {
            &[]
        };
        render_production_queue_slots(
            visible_team,
            &build_queue,
            &economies,
            observed_queue_producers,
            &mut production_queue_slots,
            &mut production_queue_slot_labels,
            &mut production_queue_slot_counts,
        );
    }
}

// Shows the primary selected entity's command icon as a portrait next to the
// selection readout (godot SelectionInfo portrait). Kept separate from
// `update_hud` because that system is already at Bevy's 16-param limit.
pub(crate) fn update_selection_portrait(
    visible_player: Res<VisiblePlayer>,
    selected: Query<(Option<&Unit>, Option<&Structure>, &Team), With<Selected>>,
    asset_server: Res<AssetServer>,
    mut portrait: Query<(&mut ImageNode, &mut Visibility), With<SelectionPortrait>>,
) {
    let Ok((mut image_node, mut visibility)) = portrait.single_mut() else {
        return;
    };
    let visible_team = visible_player.team;
    let mut visible_team_icon: Option<&'static str> = None;
    let mut any_icon: Option<&'static str> = None;
    for (unit, structure, team) in &selected {
        let icon = unit
            .and_then(|unit| registry::entity(unit.id))
            .or_else(|| structure.and_then(|structure| registry::entity(structure.id)))
            .and_then(|def| def.icon);
        if any_icon.is_none() {
            any_icon = icon;
        }
        if *team == visible_team && visible_team_icon.is_none() {
            visible_team_icon = icon;
        }
    }
    match visible_team_icon.or(any_icon) {
        Some(path) => {
            image_node.image = asset_server.load(path);
            *visibility = Visibility::Inherited;
        }
        None => *visibility = Visibility::Hidden,
    }
}

pub(crate) fn selection_hud_text(
    items: &[SelectionHudItem],
    control_group: Option<usize>,
) -> String {
    if items.is_empty() {
        return String::new();
    }
    let group_text = control_group
        .map(|slot| format!("  {} {slot}", t("编组", "Group")))
        .unwrap_or_default();
    if items.len() == 1 {
        let item = &items[0];
        let attack_text = item
            .attack
            .map(|(damage, range)| {
                format!(
                    "{} {damage:.1} {} {range:.1}",
                    t("攻击", "ATK"),
                    t("射程", "RNG")
                )
            })
            .unwrap_or_else(|| format!("{} -", t("攻击", "ATK")));
        let mut parts = vec![
            format!("{}  {}", item.team.label(), item.label),
            format!(
                "{} {:.0}/{:.0}",
                t("生命", "HP"),
                item.health_current,
                item.health_max
            ),
            attack_text,
            format!("{}: {}", t("军阶", "Rank"), veterancy_rank_label(item.rank)),
        ];
        if let Some(badge) = veterancy_rank_badge(item.rank) {
            parts.push(format!("{} {badge}", t("徽章", "Badge")));
        }
        if let Some((count, capacity)) = item.garrison {
            parts.push(format!("{} {count}/{capacity}", t("驻军", "Garrison")));
        }
        if let Some((total, capacity, ore, crystal)) = item.cargo {
            parts.push(format!(
                "{} {total}/{capacity} ({ore}/{crystal})",
                t("载货", "Cargo")
            ));
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
        t("军阶: 新兵", "Rank: Rookie").to_string()
    } else {
        format!(
            "{}: {}",
            t("军阶", "Rank"),
            rank_counts
                .iter()
                .map(|(rank, count)| format!("{rank} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "{} {}{}  {}: {}  {}",
        t("已选择", "Selected"),
        items.len(),
        group_text,
        t("类型", "Type"),
        type_text,
        rank_text
    )
}

#[derive(Clone, Copy)]
pub(crate) struct ProductionQueueHudEntry {
    pub(crate) producer_entity: Entity,
    pub(crate) local_index: usize,
    pub(crate) action: BuildAction,
    pub(crate) progress: f32,
    pub(crate) active: bool,
    /// How many consecutive same-type jobs this slot aggregates (shown as ×N).
    pub(crate) count: usize,
}

pub(crate) fn render_production_queue_slots(
    team: Team,
    build_queue: &BuildQueue,
    economies: &Economies,
    producer_entities: &[Entity],
    slots: &mut Query<(
        &ProductionQueueSlot,
        &mut ProductionQueueSlotTarget,
        &mut BackgroundColor,
        &mut Visibility,
        &mut Node,
    )>,
    labels: &mut Query<
        (&ProductionQueueSlotLabel, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueSlotCount>,
        ),
    >,
    counts: &mut Query<
        (&ProductionQueueSlotCount, &mut Text),
        (
            Without<StatsText>,
            Without<SelectionText>,
            Without<ObjectiveTrackerText>,
            Without<ProductionQueueSlotLabel>,
        ),
    >,
) {
    let entries = production_queue_hud_entries(team, build_queue, producer_entities);
    for (slot, mut target, mut color, mut visibility, mut node) in slots {
        if let Some(entry) = entries.get(slot.0).copied() {
            target.producer_entity = Some(entry.producer_entity);
            target.local_index = entry.local_index;
            *visibility = Visibility::Visible;
            node.display = Display::Flex;
            *color = BackgroundColor(production_queue_slot_color(team, entry, economies));
        } else {
            *target = ProductionQueueSlotTarget::default();
            *visibility = Visibility::Hidden;
            // Collapse empty slots so the queue row shrinks to the queued items.
            node.display = Display::None;
            *color = BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.9));
        }
    }
    for (label, mut text) in labels {
        **text = entries
            .get(label.0)
            .map(|entry| production_queue_slot_text(team, label.0, *entry, economies))
            .unwrap_or_default();
    }
    for (count, mut text) in counts {
        **text = entries
            .get(count.0)
            .map(|entry| production_queue_slot_count_text(*entry))
            .unwrap_or_default();
    }
}

pub(crate) fn production_queue_slot_text(
    team: Team,
    _display_index: usize,
    entry: ProductionQueueHudEntry,
    economies: &Economies,
) -> String {
    let label =
        build_action_target_label(entry.action).unwrap_or_else(|| t("无效", "Invalid").to_string());
    let status = if entry.active && entry.progress >= 100.0 {
        t("就绪", "Ready")
    } else if !entry.active {
        t("等待", "Waiting")
    } else if economies.get(team).low_power() {
        t("低电", "Low Pwr")
    } else {
        t("生产", "Producing")
    };
    format!(
        "{} {:.0}%\n{}",
        compact_label(&label),
        entry.progress,
        status
    )
}

/// The ×N badge text for an aggregated slot (empty when only one is queued).
pub(crate) fn production_queue_slot_count_text(entry: ProductionQueueHudEntry) -> String {
    if entry.count > 1 {
        format!("×{}", entry.count)
    } else {
        String::new()
    }
}

pub(crate) fn production_queue_slot_color(
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

/// Screen rects (min..max, window px) of HUD panels that consume world input,
/// rebuilt every frame from what is ACTUALLY rendered. The old scheme dead-zoned a
/// fixed full-width bottom band, so once the command card stopped being full-width
/// a right-click on ore in the lower half of the screen silently did nothing.
#[derive(Resource, Default)]
pub(crate) struct HudHitZones {
    pub(crate) world_rects: Vec<(Vec2, Vec2)>,
}

impl HudHitZones {
    pub(crate) fn blocks_world(&self, cursor: Vec2) -> bool {
        self.world_rects.iter().any(|(min, max)| {
            cursor.x >= min.x && cursor.x <= max.x && cursor.y >= min.y && cursor.y <= max.y
        })
    }
}

/// Pure geometry for the HUD input rects (testable without a world).
pub(crate) fn hud_world_input_rects(
    width: f32,
    height: f32,
    support_visible_count: usize,
    battle_log_rows: usize,
    command_slots: usize,
    queue_slots: usize,
    selection_panel_visible: bool,
) -> Vec<(Vec2, Vec2)> {
    let mut rects = Vec::new();
    // Minimap (bottom-left).
    rects.push((
        Vec2::new(0.0, height - MINIMAP_BOTTOM_PX - MINIMAP_SIZE_PX - 2.0),
        Vec2::new(MINIMAP_LEFT_PX + MINIMAP_SIZE_PX + 2.0, height),
    ));
    // Battle log is passive toast text (top-center) — it must NOT block world
    // clicks, or it carves a dead zone out of the upper playfield where the
    // player can't issue orders. `battle_log_rows` is kept for call-site
    // compatibility but no longer contributes a hit rect.
    let _ = battle_log_rows;
    // Support power strip (top-right), scaled to unlocked powers.
    let support_width = support_power_panel_width_for_visible_count(support_visible_count);
    if support_width > 0.0 {
        let right = width - SUPPORT_POWER_PANEL_RIGHT_PX;
        rects.push((
            Vec2::new((right - support_width).max(0.0), SUPPORT_POWER_PANEL_TOP_PX),
            Vec2::new(
                right,
                SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX,
            ),
        ));
    }
    // Command card (bottom-right): visible command rows + queue rows above them.
    if command_slots > 0 || queue_slots > 0 {
        let rows = command_slots.div_ceil(4) + queue_slots.div_ceil(6);
        let card_height = rows as f32 * COMMAND_CARD_ROW_HIT_PX + 8.0;
        rects.push((
            Vec2::new(
                width - 12.0 - COMMAND_CARD_WIDTH_PX - 2.0,
                height - 12.0 - card_height,
            ),
            Vec2::new(width, height),
        ));
    }
    // Selection panel (bottom-center portrait + text) while something is selected.
    if selection_panel_visible {
        rects.push((Vec2::new(196.0, height - 88.0), Vec2::new(652.0, height)));
    }
    rects
}

/// Rebuilds [`HudHitZones`] from live HUD state. Consumers read last frame's rects
/// (a one-frame lag on panel growth is imperceptible for input hit-testing).
pub(crate) fn refresh_hud_hit_zones(
    mut zones: ResMut<HudHitZones>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    support_panel: Res<SupportPowerPanelState>,
    battle_log: Res<BattleLog>,
    command_slots_q: Query<&Node, With<CommandSlot>>,
    queue_slots_q: Query<&Node, (With<ProductionQueueSlot>, Without<CommandSlot>)>,
    selection_text_q: Query<&Visibility, With<SelectionText>>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let command_slots = command_slots_q
        .iter()
        .filter(|node| node.display != Display::None)
        .count();
    let queue_slots = queue_slots_q
        .iter()
        .filter(|node| node.display != Display::None)
        .count();
    let selection_visible = selection_text_q
        .iter()
        .any(|visibility| *visibility != Visibility::Hidden);
    zones.world_rects = hud_world_input_rects(
        window.width(),
        window.height(),
        support_panel.visible_count,
        battle_log.entries.len(),
        command_slots,
        queue_slots,
        selection_visible,
    );
}

pub(crate) fn cursor_is_over_hud(window: &Window, zones: &HudHitZones) -> bool {
    let Some(cursor) = window.cursor_position() else {
        return false;
    };
    cursor_is_over_top_status_hud(cursor) || zones.blocks_world(cursor)
}

pub(crate) fn cursor_is_over_top_status_hud(cursor: Vec2) -> bool {
    cursor.y < 76.0
}

pub(crate) fn cursor_blocks_world_order_controls(cursor: Vec2, zones: &HudHitZones) -> bool {
    zones.blocks_world(cursor)
}

pub(crate) fn support_power_panel_width_for_visible_count(visible_count: usize) -> f32 {
    let visible_count = visible_count.min(SupportPowerKind::ALL.len());
    if visible_count == 0 {
        return 0.0;
    }
    if visible_count == SupportPowerKind::ALL.len() {
        return SUPPORT_POWER_PANEL_WIDTH_PX;
    }
    SUPPORT_POWER_PANEL_PADDING_PX * 2.0
        + SUPPORT_POWER_BUTTON_SIZE_PX * visible_count as f32
        + SUPPORT_POWER_BUTTON_GAP_PX * visible_count.saturating_sub(1) as f32
}

#[cfg(test)]
pub(crate) fn support_power_panel_contains_cursor(
    window: &Window,
    cursor: Vec2,
    visible_count: usize,
) -> bool {
    let width = support_power_panel_width_for_visible_count(visible_count);
    if width <= 0.0 {
        return false;
    }
    let left = (window.width() - SUPPORT_POWER_PANEL_RIGHT_PX - width).max(0.0);
    let right = window.width() - SUPPORT_POWER_PANEL_RIGHT_PX;
    cursor.x >= left
        && cursor.x <= right
        && cursor.y >= SUPPORT_POWER_PANEL_TOP_PX
        && cursor.y <= SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX
}

pub(crate) fn minimap_contains_cursor(window: &Window, cursor: Vec2) -> bool {
    let min = minimap_screen_min(window);
    cursor.x >= min.x
        && cursor.x <= min.x + MINIMAP_SIZE_PX
        && cursor.y >= min.y
        && cursor.y <= min.y + MINIMAP_SIZE_PX
}

pub(crate) fn minimap_screen_min(window: &Window) -> Vec2 {
    Vec2::new(
        MINIMAP_LEFT_PX,
        window.height() - MINIMAP_BOTTOM_PX - MINIMAP_SIZE_PX,
    )
}

#[cfg(test)]
pub(crate) fn minimap_local_position(world: Vec3) -> Vec2 {
    minimap_local_position_in_bounds(world, MapBounds::default())
}

pub(crate) fn minimap_local_position_in_bounds(world: Vec3, bounds: MapBounds) -> Vec2 {
    bounds.minimap_local_position(world)
}

#[cfg(test)]
pub(crate) fn minimap_world_position(local: Vec2) -> Vec3 {
    minimap_world_position_in_bounds(local, MapBounds::default())
}

pub(crate) fn minimap_world_position_from_local_in_bounds(
    local: Vec2,
    bounds: MapBounds,
) -> Option<Vec3> {
    bounds.minimap_world_position_checked(local)
}

#[cfg(test)]
pub(crate) fn minimap_world_position_in_bounds(local: Vec2, bounds: MapBounds) -> Vec3 {
    bounds.minimap_world_position(local)
}

/// Fills the match-end sparklines from the replay keyframes (plus a live final
/// point), one colored bar per team per keyframe; cleared while the match runs
/// so restarts rebuild fresh.
pub(crate) fn update_match_end_charts(
    mut commands: Commands,
    match_state: Res<MatchState>,
    timeline: Res<ReplayTimeline>,
    economies: Res<Economies>,
    live_units: Query<(&Team, &Health), With<Unit>>,
    charts: Query<(Entity, &MatchEndChart, Option<&Children>)>,
) {
    if match_state.is_running() {
        for (entity, _, children) in &charts {
            if children.is_some_and(|children| !children.is_empty()) {
                commands.entity(entity).despawn_children();
            }
        }
        return;
    }
    let team_count = economies.players.len();
    if team_count == 0 {
        return;
    }
    for (entity, chart, children) in &charts {
        if children.is_some_and(|children| !children.is_empty()) {
            continue;
        }
        // Series: per keyframe, one value per team; append the live end state.
        let mut points: Vec<Vec<f32>> = Vec::new();
        for frame in &timeline.frames {
            let mut row = vec![0.0f32; team_count];
            match chart {
                MatchEndChart::Army => {
                    for unit in &frame.units {
                        if let Some(value) = row.get_mut(unit.team) {
                            *value += 1.0;
                        }
                    }
                }
                MatchEndChart::Economy => {
                    for (index, economy) in frame.economies.iter().enumerate().take(team_count) {
                        row[index] = (economy.ore + economy.crystal).max(0) as f32;
                    }
                }
            }
            points.push(row);
        }
        let mut live_row = vec![0.0f32; team_count];
        match chart {
            MatchEndChart::Army => {
                for (team, health) in &live_units {
                    if health.current <= 0.0 {
                        continue;
                    }
                    if let Team::Player(index) = team
                        && let Some(value) = live_row.get_mut(*index)
                    {
                        *value += 1.0;
                    }
                }
            }
            MatchEndChart::Economy => {
                for (index, economy) in economies.players.iter().enumerate().take(team_count) {
                    live_row[index] = (economy.ore + economy.crystal).max(0) as f32;
                }
            }
        }
        points.push(live_row);
        // Keep the most recent keyframes if the match ran very long.
        const MAX_POINTS: usize = 40;
        if points.len() > MAX_POINTS {
            let skip = points.len() - MAX_POINTS;
            points.drain(0..skip);
        }
        let max_value = points
            .iter()
            .flatten()
            .fold(1.0f32, |max, value| max.max(*value));
        commands.entity(entity).with_children(|parent| {
            for row in &points {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(1),
                        align_items: AlignItems::FlexEnd,
                        ..default()
                    })
                    .with_children(|group| {
                        for (team_index, value) in row.iter().enumerate() {
                            let height = (value / max_value * 38.0).max(2.0);
                            group.spawn((
                                Node {
                                    width: px(3),
                                    height: px(height),
                                    ..default()
                                },
                                BackgroundColor(player_color(team_index)),
                            ));
                        }
                    });
            }
        });
    }
}

/// Tactical pause (F10): the simulation freezes (virtual time scale 0) while
/// selection and order input keep working — orders are event-driven, so you can
/// line up commands and they execute on resume. The camera plugin also runs on
/// virtual time, so use the minimap to jump the view while paused.
#[derive(Resource, Default)]
pub(crate) struct TacticalPause(pub(crate) bool);

pub(crate) fn tactical_pause_hotkey(
    keyboard: Res<ButtonInput<KeyCode>>,
    match_speed: Res<MatchSpeed>,
    mut pause: ResMut<TacticalPause>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut battle_log: ResMut<BattleLog>,
) {
    if !keyboard.just_pressed(KeyCode::F10) && !keyboard.just_pressed(KeyCode::Pause) {
        return;
    }
    pause.0 = !pause.0;
    if pause.0 {
        virtual_time.set_relative_speed(0.0);
        push_battle_log(
            &mut battle_log,
            t(
                "战术暂停 — 可继续下达指令 (F10 恢复)",
                "Tactical pause — orders still accepted (F10 resumes)",
            ),
            None,
        );
    } else {
        virtual_time.set_relative_speed(match_speed.preset.scale());
        push_battle_log(&mut battle_log, t("已恢复", "Resumed"), None);
    }
}

/// Any explicit speed selection from the match menu clears a tactical pause.
pub(crate) fn clear_tactical_pause_on_speed_change(
    match_speed: Res<MatchSpeed>,
    mut pause: ResMut<TacticalPause>,
) {
    if match_speed.is_changed() && pause.0 {
        pause.0 = false;
    }
}

#[cfg(test)]
mod tactical_pause_tests {
    use super::*;

    #[test]
    fn f10_freezes_the_clock_but_orders_still_land() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..40 {
            app.update();
        }
        // Headless apps run MinimalPlugins (no InputPlugin), so drive
        // ButtonInput directly and clear just_pressed by hand.
        let tap_f10 = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::F10);
            app.update();
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.clear_just_pressed(KeyCode::F10);
            input.release(KeyCode::F10);
            input.clear_just_released(KeyCode::F10);
        };
        tap_f10(&mut app);
        assert!(app.world().resource::<TacticalPause>().0, "F10 pauses");
        let clock_before = app.world().resource::<MatchState>().start_time_sec;
        // While paused, issue a move order to a unit; the component must stick.
        let unit = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<(Entity, &Team), With<Unit>>();
            q.iter(world)
                .find(|(_, team)| **team == Team::Player(0))
                .map(|(entity, _)| entity)
                .expect("player unit exists")
        };
        app.world_mut().entity_mut(unit).insert(MoveOrder {
            target: Vec3::new(3.0, 0.0, 3.0),
        });
        for _ in 0..30 {
            app.update();
        }
        let clock_after = app.world().resource::<MatchState>().start_time_sec;
        assert!(
            (clock_after - clock_before).abs() < f32::EPSILON,
            "match clock frozen while paused"
        );
        assert!(
            app.world().get::<MoveOrder>(unit).is_some(),
            "order accepted during pause (executes on resume)"
        );

        tap_f10(&mut app);
        for _ in 0..30 {
            app.update();
        }
        assert!(!app.world().resource::<TacticalPause>().0, "F10 resumes");
        assert!(
            app.world().resource::<MatchState>().start_time_sec > clock_after,
            "clock runs again after resume"
        );
    }
}

/// Data marker spawned at every damage application; consumed by the floater
/// system (rendered only when the 伤害数字 option is on).
#[derive(Component)]
pub(crate) struct PendingDamageNumber {
    pub(crate) position: Vec3,
    pub(crate) amount: f32,
}

/// A live on-screen damage floater rising above its world anchor.
#[derive(Component)]
pub(crate) struct DamageNumber {
    pub(crate) world: Vec3,
    pub(crate) remaining: f32,
}

pub(crate) const DAMAGE_NUMBER_LIFETIME_SEC: f32 = 0.8;

pub(crate) fn update_damage_numbers(
    mut commands: Commands,
    time: Res<Time>,
    options: Res<MenuOptionsState>,
    asset_server: Res<AssetServer>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    pending: Query<(Entity, &PendingDamageNumber)>,
    mut floaters: Query<(Entity, &mut DamageNumber, &mut Node, &mut TextColor)>,
) {
    for (entity, request) in &pending {
        if options.damage_numbers {
            commands.spawn((
                Text::new(format!("-{:.0}", request.amount.max(1.0))),
                TextFont {
                    font: asset_server.load(UI_FONT_PATH).into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.42, 0.3, 1.0)),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                GlobalZIndex(40),
                DamageNumber {
                    world: request.position,
                    remaining: DAMAGE_NUMBER_LIFETIME_SEC,
                },
                MatchScopedEntity,
            ));
        }
        commands.entity(entity).despawn();
    }
    if floaters.is_empty() {
        return;
    }
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    for (entity, mut floater, mut node, mut color) in &mut floaters {
        floater.remaining -= time.delta_secs();
        if floater.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = 1.0 - floater.remaining / DAMAGE_NUMBER_LIFETIME_SEC;
        let anchor = floater.world + Vec3::Y * (0.8 + progress * 0.9);
        match camera.world_to_viewport(camera_transform, anchor) {
            Ok(screen) => {
                node.left = px(screen.x - 8.0);
                node.top = px(screen.y);
                color.0.set_alpha(1.0 - progress);
            }
            Err(_) => {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[cfg(test)]
mod damage_number_tests {
    use super::*;

    #[test]
    fn combat_damage_spawns_floaters_when_enabled() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<MenuOptionsState>()
            .damage_numbers = true;
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..30 {
            app.update();
        }
        // Two enemies face to face; the auto-acquire + combat loop does the rest.
        let asset_server = app.world().resource::<AssetServer>().clone();
        let mut next_id = NextSpawnId(app.world().resource::<NextSpawnId>().0);
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            for (team, x) in [(Team::Player(0), -1.0f32), (Team::Player(1), 1.0f32)] {
                spawn_unit(
                    &mut commands,
                    &asset_server,
                    &mut next_id,
                    "HeavyMachinegunTrooper",
                    team,
                    Vec3::new(x, 0.0, 10.0),
                    0,
                    Team::Player(0),
                );
            }
        }
        queue.apply(app.world_mut());
        let mut saw_floater = false;
        for _ in 0..90 {
            app.update();
            let world = app.world_mut();
            if world
                .query_filtered::<(), With<DamageNumber>>()
                .iter(world)
                .next()
                .is_some()
            {
                saw_floater = true;
                break;
            }
        }
        assert!(
            saw_floater,
            "combat with the option on must spawn damage floaters"
        );
        // Markers must not leak.
        for _ in 0..60 {
            app.update();
        }
        let world = app.world_mut();
        let pending = world
            .query_filtered::<(), With<PendingDamageNumber>>()
            .iter(world)
            .count();
        assert_eq!(pending, 0, "markers are consumed every frame");
    }
}

pub(crate) const MINIMAP_SIZE_PX: f32 = 158.0;

// godot anchors the minimap/radar in the bottom-LEFT corner.
pub(crate) const MINIMAP_LEFT_PX: f32 = 12.0;

pub(crate) const MINIMAP_BOTTOM_PX: f32 = 12.0;

pub(crate) const MINIMAP_ENTITY_MARKER_PX: f32 = 4.6;

pub(crate) const MINIMAP_STRUCTURE_MARKER_PX: f32 = 6.2;

pub(crate) const MINIMAP_RESOURCE_MARKER_PX: f32 = 3.8;

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

pub(crate) fn cursor_minimap_local(window: &Window) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    if !minimap_contains_cursor(window, cursor) {
        return None;
    }
    Some(cursor - minimap_screen_min(window))
}
