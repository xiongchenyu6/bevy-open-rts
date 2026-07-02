//! Out-of-match screens: the front command menu, options, credits, and the
//! skirmish lobby (map preview, player-slot dropdowns), plus their widgets.
//!
//! Pure move out of lib.rs (module-split Stage 4); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

pub(crate) fn random_map_index() -> usize {
    SKIRMISH_MAPS.len()
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainMenuAction {
    SelectMap(usize),
    SelectStartingResources(usize),
    ToggleLobbySlotController(usize),
    SetLobbySlotController(usize, SkirmishPlayerController),
    ToggleLobbySlotFaction(usize),
    SetLobbySlotFaction(usize, SkirmishFaction),
    ToggleLobbySlotTeam(usize),
    SetLobbySlotTeam(usize, usize),
    ToggleLobbySlotColor(usize),
    SetLobbySlotColor(usize, usize),
    ToggleMapDropdown,
    ToggleResourcesDropdown,
    BackToMainMenu,
    StartMatch,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrontMenuAction {
    Play,
    Options,
    Credits,
    QuitOrFullscreen,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrontMenuButton {
    pub(crate) action: FrontMenuAction,
}

#[derive(Component)]
pub(crate) struct FrontMenuRosterPreview;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OptionsMenuAction {
    ToggleFullscreen,
    ToggleLanguage,
    ToggleMouseRestricted,
    MasterVolumeUp,
    MasterVolumeDown,
    MusicVolumeUp,
    MusicVolumeDown,
    SfxVolumeUp,
    SfxVolumeDown,
    VoiceVolumeUp,
    VoiceVolumeDown,
    CameraTiltUp,
    CameraTiltDown,
    CameraPanSpeedUp,
    CameraPanSpeedDown,
    ToggleEdgePan,
    Back,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OptionsMenuButton {
    pub(crate) action: OptionsMenuAction,
}

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct MenuOptionsState {
    pub(crate) fullscreen: bool,
    pub(crate) language: Language,
    pub(crate) mouse_restricted: bool,
    pub(crate) master_volume: f32,
    pub(crate) music_volume: f32,
    pub(crate) sfx_volume: f32,
    pub(crate) voice_volume: f32,
    /// Camera tilt in radians (plugin convention: 0 = top-down, larger = oblique).
    pub(crate) camera_tilt: f32,
    /// Keyboard/edge pan speed for `bevy_rts_camera`.
    pub(crate) camera_pan_speed: f32,
    /// Whether moving the cursor to the screen edge pans the camera.
    pub(crate) camera_edge_pan: bool,
}

impl Default for MenuOptionsState {
    fn default() -> Self {
        Self {
            fullscreen: true,
            language: Language::Zh,
            mouse_restricted: false,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            camera_tilt: CAMERA_RTS_ANGLE,
            camera_pan_speed: CAMERA_RTS_PAN_SPEED,
            camera_edge_pan: true,
        }
    }
}

impl MainMenuAction {
    fn is_selected(self, selection: SkirmishMenuSelection) -> bool {
        match self {
            MainMenuAction::SelectMap(index) => index == selection.map_index,
            MainMenuAction::SelectStartingResources(index) => {
                index == selection.starting_resource_index
            }
            MainMenuAction::ToggleLobbySlotController(slot) => {
                selection.controller_dropdown_open == Some(slot)
            }
            MainMenuAction::SetLobbySlotController(slot, controller) => {
                selection.lobby_controllers.get(slot).copied() == Some(controller)
            }
            MainMenuAction::ToggleLobbySlotFaction(slot) => {
                selection.faction_dropdown_open == Some(slot)
            }
            MainMenuAction::SetLobbySlotFaction(slot, faction) => {
                selection.lobby_factions.get(slot).copied() == Some(faction)
            }
            MainMenuAction::ToggleLobbySlotTeam(slot) => selection.team_dropdown_open == Some(slot),
            MainMenuAction::SetLobbySlotTeam(slot, team_index) => {
                selection.lobby_team_ids.get(slot).map(|id| *id as usize) == Some(team_index)
            }
            MainMenuAction::ToggleLobbySlotColor(slot) => {
                selection.color_dropdown_open == Some(slot)
            }
            MainMenuAction::SetLobbySlotColor(slot, color_index) => {
                selection.lobby_color_slots.get(slot).copied() == Some(color_index)
            }
            MainMenuAction::ToggleMapDropdown => selection.map_dropdown_open,
            MainMenuAction::ToggleResourcesDropdown => selection.resources_dropdown_open,
            MainMenuAction::BackToMainMenu => false,
            MainMenuAction::StartMatch => false,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainMenuButton {
    pub(crate) action: MainMenuAction,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainMenuButtonLabel {
    pub(crate) action: MainMenuAction,
}

#[derive(Component)]
pub(crate) struct MainMenuSummaryText;

#[derive(Component)]
pub(crate) struct MainMenuBriefStatusText;

#[derive(Component)]
pub(crate) struct MainMenuFactionInfoText;

#[derive(Component)]
pub(crate) struct MainMenuScrollArea;

#[derive(Component)]
pub(crate) struct MainMenuLobbySlotRow;

#[derive(Component)]
pub(crate) struct MainMenuLobbyListRoot {
    pub(crate) font: Handle<Font>,
    /// Faction emblems indexed by SkirmishFaction::index() (Alliance/Demon/Chaos).
    pub(crate) faction_emblems: [Handle<Image>; 3],
}

#[derive(Component)]
pub(crate) struct MainMenuMapResourceControlsRoot {
    pub(crate) font: Handle<Font>,
}

#[derive(Component)]
pub(crate) struct MainMenuMapResourceControlElement;

#[derive(Component)]
pub(crate) struct SkirmishMapPreviewRoot;

#[derive(Component)]
pub(crate) struct SkirmishMapPreviewElement;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkirmishMapPreviewMarker {
    pub(crate) kind: SkirmishMapPreviewMarkerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkirmishMapPreviewMarkerKind {
    Spawn,
    Ore,
    Crystal,
    NeutralTech,
    SupplyCrate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkirmishMapPreviewRect {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) fn setup_menu_backdrop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    screen: AppScreen,
    tint: Color,
) {
    commands
        .spawn((
            Name::new("Godot Main Menu Background"),
            DespawnOnExit(screen),
            ImageNode::new(asset_server.load("ui/background.png")),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(-1),
        ))
        .with_children(|bg| {
            bg.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(tint),
            ));
        });
}

pub(crate) fn setup_front_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        Name::new("Main Menu Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::MainMenu),
    ));
    setup_menu_backdrop(
        &mut commands,
        &asset_server,
        AppScreen::MainMenu,
        Color::srgba(0.0, 0.025, 0.022, 0.48),
    );

    commands
        .spawn((
            Name::new("Godot Style Command Menu"),
            DespawnOnExit(AppScreen::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(36),
                padding: UiRect::new(px(48), px(48), px(40), px(40)),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(front_briefing_column_node())
                .with_children(|column| {
                    column.spawn((
                        Text::new("Open RTS"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(72.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 1.0, 0.94)),
                    ));
                    column.spawn((
                        localized_text("前线指挥", "Frontline Command"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.54, 0.93, 0.85)),
                    ));
                    column
                        .spawn(front_intel_panel_node(148.0, None))
                        .with_children(|panel| {
                            panel.spawn((
                                localized_text("行动：遭遇战指挥", "Operation: Skirmish Command"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(28.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.86, 1.0, 0.95)),
                            ));
                            panel.spawn((
                                localized_text(
                                    "扩展武备已上线。选择战区并部署。",
                                    "Expanded arsenal online. Choose a theater and deploy.",
                                ),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(18.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.74, 0.9, 0.86)),
                            ));
                        });
                    column
                        .spawn(front_intel_panel_node(0.0, Some(1.0)))
                        .with_children(|panel| {
                            panel.spawn((
                                localized_text("可用战斗群", "Available Battle Group"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(18.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.76, 0.96, 0.9)),
                            ));
                            panel
                                .spawn(Node {
                                    width: Val::Percent(100.0),
                                    min_height: px(326),
                                    flex_grow: 1.0,
                                    align_self: AlignSelf::Stretch,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|preview| {
                                    preview.spawn((
                                        ImageNode::new(
                                            asset_server.load("ui/icons/RosterPreview.png"),
                                        ),
                                        FrontMenuRosterPreview,
                                        Node {
                                            width: px(326),
                                            height: px(326),
                                            ..default()
                                        },
                                    ));
                                });
                        });
                });

            root.spawn(front_command_panel_node())
                .with_children(|panel| {
                    panel.spawn((
                        localized_text("指挥菜单", "Command Menu"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.79, 1.0, 0.94)),
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));
                    panel.spawn(front_divider_node());
                    for (action, zh, en, height) in [
                        (FrontMenuAction::Play, "开始游戏", "Play", 62.0),
                        (FrontMenuAction::Options, "设置", "Options", 58.0),
                        (FrontMenuAction::Credits, "制作人员", "Credits", 58.0),
                        (
                            FrontMenuAction::QuitOrFullscreen,
                            "全屏",
                            "Fullscreen",
                            58.0,
                        ),
                    ] {
                        panel
                            .spawn(front_menu_button(action, height))
                            .with_children(|button| {
                                button.spawn((
                                    localized_text(zh, en),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::Px(22.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.88, 0.9, 0.9)),
                                ));
                            });
                    }
                    panel.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    panel.spawn((
                        localized_text("系统：在线", "Systems: Online"),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.48, 0.76, 0.7)),
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));
                });
        });
}

pub(crate) fn front_briefing_column_node() -> impl Bundle {
    Node {
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        row_gap: px(16),
        min_width: px(320),
        margin: UiRect::top(px(26)),
        ..default()
    }
}

pub(crate) fn front_intel_panel_node(min_height: f32, flex_grow: Option<f32>) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            min_height: if min_height > 0.0 {
                px(min_height)
            } else {
                px(0)
            },
            flex_grow: flex_grow.unwrap_or(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            padding: UiRect::new(px(18), px(18), px(14), px(14)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.22, 0.58, 0.53, 0.48)),
        BackgroundColor(Color::srgba(0.02, 0.055, 0.052, 0.64)),
    )
}

pub(crate) fn front_command_panel_node() -> impl Bundle {
    (
        Node {
            width: px(384),
            min_width: px(320),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(14),
            padding: UiRect::new(px(24), px(24), px(26), px(26)),
            border: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.35, 0.82, 0.74, 0.62)),
        BackgroundColor(Color::srgba(0.015, 0.029, 0.028, 0.82)),
    )
}

pub(crate) fn front_divider_node() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: px(1),
            margin: UiRect::vertical(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.55, 0.72, 0.68, 0.52)),
    )
}

pub(crate) fn front_menu_button(action: FrontMenuAction, height: f32) -> impl Bundle {
    (
        Button,
        FrontMenuButton { action },
        Node {
            width: Val::Percent(100.0),
            height: px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.082, 0.082, 0.92)),
    )
}

pub(crate) fn resize_front_menu_roster_preview(
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut preview_q: Query<&mut Node, With<FrontMenuRosterPreview>>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let vertical_room = (window.height() - 454.0).clamp(260.0, 860.0);
    let horizontal_room = (window.width() - 552.0).clamp(260.0, 860.0);
    let preview_size = vertical_room.min(horizontal_room);
    for mut node in preview_q.iter_mut() {
        node.width = px(preview_size);
        node.height = px(preview_size);
    }
}

pub(crate) fn front_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut options: ResMut<MenuOptionsState>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut buttons: Query<(&Interaction, &FrontMenuButton, &mut BackgroundColor)>,
) {
    for (interaction, button, mut background) in &mut buttons {
        let clicked = *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match button.action {
                FrontMenuAction::Play => next_state.set(AppScreen::SkirmishSetup),
                FrontMenuAction::Options => next_state.set(AppScreen::OptionsMenu),
                FrontMenuAction::Credits => next_state.set(AppScreen::CreditsMenu),
                FrontMenuAction::QuitOrFullscreen => {
                    if let Ok(mut window) = windows.single_mut() {
                        options.fullscreen = toggle_window_fullscreen(&mut window);
                    }
                }
            }
        }
        *background = BackgroundColor(match interaction {
            Interaction::Pressed => Color::srgba(0.13, 0.18, 0.17, 0.96),
            Interaction::Hovered => Color::srgba(0.105, 0.13, 0.125, 0.94),
            Interaction::None => Color::srgba(0.08, 0.082, 0.082, 0.92),
        });
    }
}

pub(crate) fn setup_options_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    options: Res<MenuOptionsState>,
) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        Name::new("Options Menu Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::OptionsMenu),
    ));
    setup_menu_backdrop(
        &mut commands,
        &asset_server,
        AppScreen::OptionsMenu,
        Color::srgba(0.05, 0.04, 0.035, 0.58),
    );

    commands
        .spawn((
            Name::new("Godot Style Options Menu"),
            DespawnOnExit(AppScreen::OptionsMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(options_panel_node()).with_children(|panel| {
                spawn_options_group(
                    panel,
                    &font,
                    "视频",
                    "Video",
                    &[(
                        OptionsMenuAction::ToggleFullscreen,
                        if options.fullscreen {
                            "全屏"
                        } else {
                            "窗口"
                        },
                        if options.fullscreen {
                            "Fullscreen"
                        } else {
                            "Window"
                        },
                    )],
                );
                spawn_options_group(
                    panel,
                    &font,
                    "语言",
                    "Language",
                    &[(
                        OptionsMenuAction::ToggleLanguage,
                        options.language.short_label(),
                        options.language.short_label(),
                    )],
                );
                panel.spawn(options_group_node()).with_children(|group| {
                    group.spawn(options_group_header("音频", "Audio", font.clone()));
                    for (label_zh, label_en, down, up, value) in [
                        (
                            "主音量",
                            "Master",
                            OptionsMenuAction::MasterVolumeDown,
                            OptionsMenuAction::MasterVolumeUp,
                            options.master_volume,
                        ),
                        (
                            "音乐",
                            "Music",
                            OptionsMenuAction::MusicVolumeDown,
                            OptionsMenuAction::MusicVolumeUp,
                            options.music_volume,
                        ),
                        (
                            "音效",
                            "SFX",
                            OptionsMenuAction::SfxVolumeDown,
                            OptionsMenuAction::SfxVolumeUp,
                            options.sfx_volume,
                        ),
                        (
                            "语音",
                            "Voice",
                            OptionsMenuAction::VoiceVolumeDown,
                            OptionsMenuAction::VoiceVolumeUp,
                            options.voice_volume,
                        ),
                    ] {
                        group.spawn(options_volume_row_node()).with_children(|row| {
                            row.spawn((
                                localized_text(label_zh, label_en),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.88, 0.88, 0.86)),
                                Node {
                                    width: px(92),
                                    ..default()
                                },
                            ));
                            row.spawn(options_small_button(down))
                                .with_children(|button| {
                                    button.spawn(options_button_text("-", font.clone(), 16.0));
                                });
                            row.spawn(options_slider_bar_node(value));
                            row.spawn(options_small_button(up)).with_children(|button| {
                                button.spawn(options_button_text("+", font.clone(), 16.0));
                            });
                            row.spawn((
                                Text::new(format!("{:.0}%", value * 100.0)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.88, 0.88, 0.86)),
                                Node {
                                    width: px(50),
                                    justify_content: JustifyContent::FlexEnd,
                                    ..default()
                                },
                            ));
                        });
                    }
                });
                panel.spawn(options_group_node()).with_children(|group| {
                    group.spawn(options_group_header("镜头", "Camera", font.clone()));
                    for (label_zh, label_en, down, up, value, display) in [
                        (
                            "倾斜",
                            "Tilt",
                            OptionsMenuAction::CameraTiltDown,
                            OptionsMenuAction::CameraTiltUp,
                            (options.camera_tilt - CAMERA_TILT_MIN)
                                / (CAMERA_TILT_MAX - CAMERA_TILT_MIN),
                            format!("{:.0}\u{00b0}", options.camera_tilt.to_degrees()),
                        ),
                        (
                            "平移速度",
                            "Pan Speed",
                            OptionsMenuAction::CameraPanSpeedDown,
                            OptionsMenuAction::CameraPanSpeedUp,
                            (options.camera_pan_speed - CAMERA_PAN_SPEED_MIN)
                                / (CAMERA_PAN_SPEED_MAX - CAMERA_PAN_SPEED_MIN),
                            format!("{:.0}", options.camera_pan_speed),
                        ),
                    ] {
                        group.spawn(options_volume_row_node()).with_children(|row| {
                            row.spawn((
                                localized_text(label_zh, label_en),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.88, 0.88, 0.86)),
                                Node {
                                    width: px(92),
                                    ..default()
                                },
                            ));
                            row.spawn(options_small_button(down))
                                .with_children(|button| {
                                    button.spawn(options_button_text("-", font.clone(), 16.0));
                                });
                            row.spawn(options_slider_bar_node(value));
                            row.spawn(options_small_button(up)).with_children(|button| {
                                button.spawn(options_button_text("+", font.clone(), 16.0));
                            });
                            row.spawn((
                                Text::new(display),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.88, 0.88, 0.86)),
                                Node {
                                    width: px(50),
                                    justify_content: JustifyContent::FlexEnd,
                                    ..default()
                                },
                            ));
                        });
                    }
                    group
                        .spawn(options_button(OptionsMenuAction::ToggleEdgePan, 32.0))
                        .with_children(|button| {
                            button.spawn((
                                localized_text(
                                    if options.camera_edge_pan {
                                        "边缘平移 开启"
                                    } else {
                                        "边缘平移 关闭"
                                    },
                                    if options.camera_edge_pan {
                                        "Edge Pan On"
                                    } else {
                                        "Edge Pan Off"
                                    },
                                ),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.88, 0.88, 0.86)),
                            ));
                        });
                });
                spawn_options_group(
                    panel,
                    &font,
                    "鼠标",
                    "Mouse",
                    &[(
                        OptionsMenuAction::ToggleMouseRestricted,
                        if options.mouse_restricted {
                            "开启 将鼠标限制在游戏窗口内"
                        } else {
                            "关闭 将鼠标限制在游戏窗口内"
                        },
                        if options.mouse_restricted {
                            "On Confine mouse to game window"
                        } else {
                            "Off Confine mouse to game window"
                        },
                    )],
                );
                panel
                    .spawn(options_button(OptionsMenuAction::Back, 48.0))
                    .with_children(|button| {
                        button.spawn((
                            localized_text("返回", "Back"),
                            TextFont {
                                font: font.into(),
                                font_size: FontSize::Px(24.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.88, 0.88, 0.86)),
                        ));
                    });
            });
        });
}

pub(crate) fn options_panel_node() -> impl Bundle {
    (
        Node {
            width: px(449),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(20)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.055, 0.052, 0.048, 0.88)),
    )
}

pub(crate) fn options_group_node() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            padding: UiRect::all(px(5)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.034, 0.032, 0.84)),
    )
}

pub(crate) fn options_group_header(
    zh: &'static str,
    en: &'static str,
    font: Handle<Font>,
) -> impl Bundle {
    (
        localized_text(zh, en),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.88, 0.86)),
        Node {
            width: Val::Percent(100.0),
            min_height: px(28),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.065, 0.065, 0.06, 0.88)),
    )
}

pub(crate) fn options_button(action: OptionsMenuAction, height: f32) -> impl Bundle {
    (
        Button,
        OptionsMenuButton { action },
        Node {
            width: Val::Percent(100.0),
            height: px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.048, 0.92)),
    )
}

pub(crate) fn options_small_button(action: OptionsMenuAction) -> impl Bundle {
    (
        Button,
        OptionsMenuButton { action },
        Node {
            width: px(26),
            height: px(24),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.075, 0.075, 0.07, 0.94)),
    )
}

pub(crate) fn options_button_text(
    label: &'static str,
    font: Handle<Font>,
    font_size: f32,
) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.88)),
    )
}

pub(crate) fn options_volume_row_node() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        min_height: px(28),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(7),
        ..default()
    }
}

pub(crate) fn options_slider_bar_node(value: f32) -> impl Bundle {
    (
        Node {
            width: px(148),
            height: px(8),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(999)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.76)),
        children![(
            Node {
                width: Val::Percent((value.clamp(0.0, 1.0) * 100.0).max(2.0)),
                height: px(8),
                border_radius: BorderRadius::all(px(999)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.72, 0.72, 0.72)),
        )],
    )
}

pub(crate) fn options_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut options: ResMut<MenuOptionsState>,
    mut locale: ResMut<Locale>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(&Interaction, &OptionsMenuButton, &mut BackgroundColor)>,
) {
    let mut rebuild = false;
    for (interaction, button, mut background) in &mut buttons {
        let clicked = *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match button.action {
                OptionsMenuAction::ToggleFullscreen => {
                    options.fullscreen = !options.fullscreen;
                    if let Ok(mut window) = windows.single_mut() {
                        set_window_fullscreen(&mut window, options.fullscreen);
                    }
                    rebuild = true;
                }
                OptionsMenuAction::ToggleLanguage => {
                    options.language = options.language.toggled();
                    locale.0 = options.language;
                    rebuild = true;
                }
                OptionsMenuAction::ToggleMouseRestricted => {
                    options.mouse_restricted = !options.mouse_restricted;
                    rebuild = true;
                }
                OptionsMenuAction::MasterVolumeUp => {
                    options.master_volume = (options.master_volume + 0.05).min(1.0);
                    rebuild = true;
                }
                OptionsMenuAction::MasterVolumeDown => {
                    options.master_volume = (options.master_volume - 0.05).max(0.0);
                    rebuild = true;
                }
                OptionsMenuAction::MusicVolumeUp => {
                    options.music_volume = (options.music_volume + 0.05).min(1.0);
                    rebuild = true;
                }
                OptionsMenuAction::MusicVolumeDown => {
                    options.music_volume = (options.music_volume - 0.05).max(0.0);
                    rebuild = true;
                }
                OptionsMenuAction::SfxVolumeUp => {
                    options.sfx_volume = (options.sfx_volume + 0.05).min(1.0);
                    rebuild = true;
                }
                OptionsMenuAction::SfxVolumeDown => {
                    options.sfx_volume = (options.sfx_volume - 0.05).max(0.0);
                    rebuild = true;
                }
                OptionsMenuAction::VoiceVolumeUp => {
                    options.voice_volume = (options.voice_volume + 0.05).min(1.0);
                    rebuild = true;
                }
                OptionsMenuAction::VoiceVolumeDown => {
                    options.voice_volume = (options.voice_volume - 0.05).max(0.0);
                    rebuild = true;
                }
                OptionsMenuAction::CameraTiltUp => {
                    options.camera_tilt =
                        (options.camera_tilt + CAMERA_TILT_STEP).min(CAMERA_TILT_MAX);
                    rebuild = true;
                }
                OptionsMenuAction::CameraTiltDown => {
                    options.camera_tilt =
                        (options.camera_tilt - CAMERA_TILT_STEP).max(CAMERA_TILT_MIN);
                    rebuild = true;
                }
                OptionsMenuAction::CameraPanSpeedUp => {
                    options.camera_pan_speed = (options.camera_pan_speed + CAMERA_PAN_SPEED_STEP)
                        .min(CAMERA_PAN_SPEED_MAX);
                    rebuild = true;
                }
                OptionsMenuAction::CameraPanSpeedDown => {
                    options.camera_pan_speed = (options.camera_pan_speed - CAMERA_PAN_SPEED_STEP)
                        .max(CAMERA_PAN_SPEED_MIN);
                    rebuild = true;
                }
                OptionsMenuAction::ToggleEdgePan => {
                    options.camera_edge_pan = !options.camera_edge_pan;
                    rebuild = true;
                }
                OptionsMenuAction::Back => next_state.set(AppScreen::MainMenu),
            }
        }
        *background = BackgroundColor(match interaction {
            Interaction::Pressed => Color::srgba(0.12, 0.13, 0.125, 0.96),
            Interaction::Hovered => Color::srgba(0.08, 0.085, 0.082, 0.94),
            Interaction::None => Color::srgba(0.05, 0.05, 0.048, 0.92),
        });
    }
    if rebuild {
        next_state.set(AppScreen::OptionsMenu);
    }
}

pub(crate) fn setup_credits_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        Name::new("Credits Menu Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::CreditsMenu),
    ));
    setup_menu_backdrop(
        &mut commands,
        &asset_server,
        AppScreen::CreditsMenu,
        Color::srgba(0.05, 0.04, 0.035, 0.58),
    );
    commands
        .spawn((
            Name::new("Godot Style Credits Menu"),
            DespawnOnExit(AppScreen::CreditsMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(options_panel_node()).with_children(|panel| {
                panel.spawn(options_group_node()).with_children(|group| {
                    group.spawn(options_group_header(
                        "制作人员",
                        "Credits",
                        font.clone(),
                    ));
                    group.spawn((
                        localized_text(
                            "核心贡献者：\n- Pawel Lampe (Scony) | Lampe Games\n\n素材：\n- 3D Space Kit by Kenney.nl",
                            "Core Contributors:\n- Pawel Lampe (Scony) | Lampe Games\n\nAssets:\n- 3D Space Kit by Kenney.nl",
                        ),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.88, 0.86)),
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            padding: UiRect::vertical(px(16)),
                            ..default()
                        },
                    ));
                });
                panel
                    .spawn(options_button(OptionsMenuAction::Back, 48.0))
                    .with_children(|button| {
                        button.spawn((
                            localized_text("返回", "Back"),
                            TextFont {
                                font: font.into(),
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.88, 0.88, 0.86)),
                        ));
                    });
            });
        });
}

pub(crate) fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selection: Res<SkirmishMenuSelection>,
) {
    let font = asset_server.load(UI_FONT_PATH);

    commands.spawn((
        Name::new("Skirmish Menu Camera"),
        Camera2d,
        DespawnOnExit(AppScreen::SkirmishSetup),
    ));

    // Scenic backdrop (godot's assets/ui/background.png) behind everything, with a
    // dark tactical tint so the panels stay readable on top. Spawned first so it
    // renders behind the menu root.
    commands
        .spawn((
            Name::new("Menu Background"),
            DespawnOnExit(AppScreen::SkirmishSetup),
            ImageNode::new(asset_server.load("ui/background.png")),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(-1),
        ))
        .with_children(|bg| {
            bg.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.022, 0.02, 0.55)),
            ));
        });

    commands
        .spawn((
            Name::new("Skirmish Setup Menu"),
            DespawnOnExit(AppScreen::SkirmishSetup),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                padding: UiRect::new(px(12), px(12), px(14), px(16)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|root| {
            // Centered modal dialog (godot main-menu/Play.tscn PanelContainer).
            root.spawn((
                Node {
                    width: Val::Percent(90.0),
                    max_width: px(1040),
                    min_width: px(680),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(12),
                    padding: UiRect::all(px(18)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.26, 0.32, 0.32)),
                BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.96)),
            ))
            .with_children(|modal| {
                modal
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexStart,
                        column_gap: px(14),
                        ..default()
                    })
                    .with_children(|cols| {
                        // LEFT column — 地图 (map preview + details + resources + summary).
                        cols.spawn(Node {
                            width: px(320),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(8),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn(menu_section_header("地图", "Map", font.clone()));
                            spawn_skirmish_map_preview(col, *selection);
                            col.spawn((
                                Text::new(main_menu_faction_info_text(*selection)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(15.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.78, 0.86, 0.84)),
                                Node {
                                    width: Val::Percent(100.0),
                                    ..default()
                                },
                                MainMenuFactionInfoText,
                            ));
                            col.spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Stretch,
                                    row_gap: px(8),
                                    ..default()
                                },
                                MainMenuMapResourceControlsRoot { font: font.clone() },
                            ))
                            .with_children(|controls| {
                                spawn_menu_map_resource_controls(
                                    controls,
                                    font.clone(),
                                    *selection,
                                );
                            });
                            col.spawn((
                                localized_text("行动摘要", "Operation summary"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.62, 0.72, 0.7)),
                                Node {
                                    margin: UiRect::top(px(4)),
                                    ..default()
                                },
                            ));
                            col.spawn((
                                Text::new(main_menu_summary_text(*selection)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.74, 0.82, 0.8)),
                                Node {
                                    width: Val::Percent(100.0),
                                    ..default()
                                },
                                MainMenuSummaryText,
                            ));
                        });

                        // RIGHT column — 玩家 (one dropdown row per slot).
                        cols.spawn(Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(8),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn(menu_section_header("玩家", "Players", font.clone()));
                            let faction_emblems = [
                                asset_server.load(SkirmishFaction::Alliance.emblem_path()),
                                asset_server.load(SkirmishFaction::Demon.emblem_path()),
                                asset_server.load(SkirmishFaction::Chaos.emblem_path()),
                            ];
                            col.spawn((
                                menu_lobby_list_node(),
                                MainMenuLobbyListRoot {
                                    font: font.clone(),
                                    faction_emblems: faction_emblems.clone(),
                                },
                            ))
                            .with_children(|list| {
                                for slot in 0..selection.selected_map_player_slots() {
                                    spawn_menu_lobby_slot_row(
                                        list,
                                        slot,
                                        font.clone(),
                                        &faction_emblems,
                                        *selection,
                                    );
                                }
                            });
                        });
                    });

                // Bottom — 开始 / 返回, full width, stacked (godot Play.tscn).
                modal
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        row_gap: px(6),
                        margin: UiRect::top(px(6)),
                        ..default()
                    })
                    .with_children(|bar| {
                        for action in [MainMenuAction::StartMatch, MainMenuAction::BackToMainMenu] {
                            bar.spawn((
                                Button,
                                MainMenuButton { action },
                                Node {
                                    width: Val::Percent(100.0),
                                    height: px(40),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(px(1)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.28, 0.34, 0.33)),
                                BackgroundColor(Color::srgba(0.046, 0.058, 0.06, 0.94)),
                            ))
                            .with_children(|button| {
                                button.spawn(menu_action_button_label(
                                    action,
                                    *selection,
                                    font.clone(),
                                    16.0,
                                ));
                            });
                        }
                    });
            });
        });
}

/// A panel-bar column header (godot's "地图" / "玩家" header labels).
pub(crate) fn menu_section_header(
    zh: &'static str,
    en: &'static str,
    font: Handle<Font>,
) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: px(30),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.26, 0.32, 0.32)),
        BackgroundColor(Color::srgba(0.05, 0.06, 0.065, 0.96)),
        children![(
            localized_text(zh, en),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.72, 0.38)),
        )],
    )
}

pub(crate) fn spawn_menu_map_resource_controls(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    selection: SkirmishMenuSelection,
) {
    let map_options: Vec<MainMenuAction> = (0..SKIRMISH_MAPS.len())
        .map(MainMenuAction::SelectMap)
        .chain(std::iter::once(MainMenuAction::SelectMap(
            random_map_index(),
        )))
        .collect();
    spawn_menu_inline_dropdown(
        parent,
        "地图",
        "Map",
        MainMenuAction::ToggleMapDropdown,
        selection.map_dropdown_open,
        &map_options,
        selection,
        font.clone(),
    );

    let res_options: Vec<MainMenuAction> = (0..GODOT_STARTING_RESOURCE_OPTIONS.len())
        .map(MainMenuAction::SelectStartingResources)
        .collect();
    spawn_menu_inline_dropdown(
        parent,
        "初始资源",
        "Starting resources",
        MainMenuAction::ToggleResourcesDropdown,
        selection.resources_dropdown_open,
        &res_options,
        selection,
        font,
    );
}

/// A labelled inline dropdown (toggle button showing the current value; when
/// open, the option list expands below). Used for the 地图 + 初始资源 selectors.
/// Z layer for menu dropdown popups — above all other menu UI. The menu screen and
/// the in-match HUD are never on screen together, so this won't collide with HUD
/// GlobalZIndex values.
pub(crate) const MENU_DROPDOWN_POPUP_Z: i32 = 1000;

/// Positioning context for a dropdown: a fixed-width column the floating popup
/// anchors to (absolute children resolve against it).
/// Fixed-width dropdown cell (used by the vertical left-column inline dropdowns).
pub(crate) fn menu_dropdown_cell_node(width: f32) -> Node {
    Node {
        position_type: PositionType::Relative,
        width: px(width),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// Responsive dropdown cell for the horizontal player rows: flexes to share the row
/// width evenly (so the rows never overflow the modal and scale with the window).
/// `basis` biases the natural width (the faction cell is a bit wider than team/color).
pub(crate) fn menu_dropdown_flex_cell_node(basis: f32) -> Node {
    Node {
        position_type: PositionType::Relative,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: px(basis),
        min_width: px(0.0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// Spawns a godot-OptionButton-style dropdown INTO an already-spawned cell: the
/// toggle button (current value) and, when `open`, a floating popup of options
/// absolutely positioned just below it with a high `GlobalZIndex` so it overlays the
/// rows beneath instead of pushing them down. The popup is a child of the cell, so
/// it's despawned with it on the next rebuild.
pub(crate) fn spawn_menu_dropdown_contents(
    cell: &mut ChildSpawnerCommands,
    toggle: MainMenuAction,
    open: bool,
    options: &[MainMenuAction],
    selection: SkirmishMenuSelection,
    font: Handle<Font>,
    width: f32,
    font_size: f32,
) {
    cell.spawn(menu_button(toggle, Val::Percent(100.0)))
        .with_children(|button| {
            button.spawn(menu_action_button_label(
                toggle,
                selection,
                font.clone(),
                font_size,
            ));
        });
    if !open {
        return;
    }
    cell.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(100.0),
            left: px(0),
            min_width: px(width),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(1),
            padding: UiRect::all(px(2)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.45, 0.56, 0.52)),
        BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.98)),
        GlobalZIndex(MENU_DROPDOWN_POPUP_Z),
    ))
    .with_children(|popup| {
        for option in options {
            popup
                .spawn(menu_button(*option, px(width)))
                .with_children(|button| {
                    button.spawn(menu_action_button_label(
                        *option,
                        selection,
                        font.clone(),
                        font_size,
                    ));
                });
        }
    });
}

/// A faction dropdown button: the faction emblem + its (re-translating) label, tagged
/// as a MainMenuButton so the click/highlight systems treat it like any other button.
pub(crate) fn spawn_faction_dropdown_button(
    parent: &mut ChildSpawnerCommands,
    action: MainMenuAction,
    faction: SkirmishFaction,
    faction_emblems: &[Handle<Image>; 3],
    selection: SkirmishMenuSelection,
    font: Handle<Font>,
    width: Val,
) {
    parent
        .spawn((
            Button,
            MainMenuButton { action },
            Node {
                width,
                min_height: px(32),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                column_gap: px(4),
                padding: UiRect::new(px(4), px(4), px(0), px(0)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.28, 0.34, 0.33)),
            BackgroundColor(Color::srgba(0.046, 0.058, 0.06, 0.94)),
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(faction_emblems[faction.index()].clone()),
                Node {
                    width: px(16),
                    height: px(16),
                    ..default()
                },
            ));
            button.spawn(menu_action_button_label(action, selection, font, 13.0));
        });
}

pub(crate) fn spawn_menu_inline_dropdown(
    parent: &mut ChildSpawnerCommands,
    zh: &'static str,
    en: &'static str,
    toggle: MainMenuAction,
    open: bool,
    options: &[MainMenuAction],
    selection: SkirmishMenuSelection,
    font: Handle<Font>,
) {
    parent.spawn((
        localized_text(zh, en),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.88, 0.86)),
        Node {
            margin: UiRect::top(px(4)),
            ..default()
        },
        MainMenuMapResourceControlElement,
    ));
    parent
        .spawn((
            menu_dropdown_cell_node(240.0),
            MainMenuMapResourceControlElement,
        ))
        .with_children(|cell| {
            spawn_menu_dropdown_contents(cell, toggle, open, options, selection, font, 240.0, 12.0);
        });
}

pub(crate) fn menu_action_button_label(
    action: MainMenuAction,
    selection: SkirmishMenuSelection,
    font: Handle<Font>,
    font_size: f32,
) -> impl Bundle {
    (
        menu_button_label(
            main_menu_button_label_text(action, selection),
            font,
            font_size,
        ),
        MainMenuButtonLabel { action },
    )
}

pub(crate) fn main_menu_button_label_text(
    action: MainMenuAction,
    selection: SkirmishMenuSelection,
) -> String {
    match action {
        MainMenuAction::SelectMap(index) if index == random_map_index() => {
            format!("R {}", random_map_label())
        }
        MainMenuAction::SelectMap(index) => SKIRMISH_MAPS
            .get(index)
            .map(|map| format!("{} {}", index + 1, localized_skirmish_map_name(map)))
            .unwrap_or_else(|| t("地图", "Map").to_string()),
        MainMenuAction::SelectStartingResources(index) => GODOT_STARTING_RESOURCE_OPTIONS
            .get(index)
            .map(|option| format!("{} {}", index + 5, starting_resource_option_label(option)))
            .unwrap_or_else(|| t("资源", "Resources").to_string()),
        MainMenuAction::ToggleLobbySlotController(slot) => format!(
            "{}",
            selection
                .lobby_controllers
                .get(slot)
                .copied()
                .unwrap_or(SkirmishPlayerController::None)
                .short_label()
        ),
        MainMenuAction::SetLobbySlotController(_, controller) => {
            controller.short_label().to_string()
        }
        MainMenuAction::ToggleLobbySlotFaction(slot) => format!(
            "{}",
            selection
                .lobby_factions
                .get(slot)
                .copied()
                .unwrap_or(SkirmishFaction::Alliance)
                .label()
        ),
        MainMenuAction::SetLobbySlotFaction(_, faction) => faction.label().to_string(),
        MainMenuAction::ToggleLobbySlotTeam(slot) => format!(
            "{}",
            skirmish_team_label(
                selection.lobby_team_ids.get(slot).copied().unwrap_or(0) as usize
                    % SKIRMISH_TEAM_OPTION_COUNT as usize
            )
        ),
        MainMenuAction::SetLobbySlotTeam(_, team_index) => skirmish_team_label(team_index),
        MainMenuAction::ToggleLobbySlotColor(slot) => format!(
            "{}",
            skirmish_color_label(
                selection
                    .lobby_color_slots
                    .get(slot)
                    .copied()
                    .unwrap_or(slot)
                    % PLAYER_COLOR_PALETTE.len()
            )
        ),
        MainMenuAction::SetLobbySlotColor(_, color_index) => skirmish_color_label(color_index),
        MainMenuAction::ToggleMapDropdown => SKIRMISH_MAPS
            .get(selection.map_index)
            .map(localized_skirmish_map_name)
            .unwrap_or(t("地图", "Map"))
            .to_string(),
        MainMenuAction::ToggleResourcesDropdown => format!(
            "{}",
            GODOT_STARTING_RESOURCE_OPTIONS
                .get(selection.starting_resource_index)
                .map(|option| starting_resource_option_label(option).to_string())
                .unwrap_or_else(|| t("标准", "Standard").to_string())
        ),
        MainMenuAction::BackToMainMenu => t("返回", "Back").to_string(),
        MainMenuAction::StartMatch => t("开始对战  Enter", "Start Match  Enter").to_string(),
    }
}

pub(crate) fn menu_lobby_list_node() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(6),
        ..default()
    }
}

pub(crate) fn spawn_menu_lobby_slot_row(
    parent: &mut ChildSpawnerCommands<'_>,
    slot: usize,
    font: Handle<Font>,
    faction_emblems: &[Handle<Image>; 3],
    selection: SkirmishMenuSelection,
) {
    let controller = selection
        .lobby_controllers
        .get(slot)
        .copied()
        .unwrap_or(SkirmishPlayerController::None);
    let faction = selection
        .lobby_factions
        .get(slot)
        .copied()
        .unwrap_or(SkirmishFaction::Alliance);
    let team_id =
        selection.lobby_team_ids.get(slot).copied().unwrap_or(0) % SKIRMISH_TEAM_OPTION_COUNT + 1;
    let color_slot = selection
        .lobby_color_slots
        .get(slot)
        .copied()
        .unwrap_or(slot)
        % PLAYER_COLOR_PALETTE.len()
        + 1;
    let active = controller.is_active();
    let status = if active {
        format!(
            "{} | {} | T{} | C{}",
            controller.short_label(),
            faction.label(),
            team_id,
            color_slot
        )
    } else {
        t("关闭", "Closed").to_string()
    };
    parent
        .spawn(menu_lobby_slot_row_node(slot, selection))
        .with_children(|row| {
            row.spawn(menu_lobby_slot_label_node(48.0))
                .with_children(|cell| {
                    cell.spawn((
                        Text::new(format!("{:02}", slot + 1)),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.72, 0.38)),
                    ));
                });

            let _ = (active, status);
            // Controller dropdown (floating popup): 关闭 / 我方 / 傻瓜~困难 AI.
            row.spawn(menu_dropdown_flex_cell_node(84.0))
                .with_children(|cell| {
                    let options: Vec<MainMenuAction> = [
                        SkirmishPlayerController::None,
                        SkirmishPlayerController::Human,
                        SkirmishPlayerController::Ai(AiDifficulty::Beginner),
                        SkirmishPlayerController::Ai(AiDifficulty::Easy),
                        SkirmishPlayerController::Ai(AiDifficulty::Normal),
                        SkirmishPlayerController::Ai(AiDifficulty::Hard),
                    ]
                    .into_iter()
                    .map(|c| MainMenuAction::SetLobbySlotController(slot, c))
                    .collect();
                    spawn_menu_dropdown_contents(
                        cell,
                        MainMenuAction::ToggleLobbySlotController(slot),
                        selection.controller_dropdown_open == Some(slot),
                        &options,
                        selection,
                        font.clone(),
                        84.0,
                        13.0,
                    );
                });

            // Faction dropdown (floating popup) with emblems: 苍穹联盟 / 炽炎魔军 / 混沌裂隙.
            row.spawn(menu_dropdown_flex_cell_node(96.0))
                .with_children(|cell| {
                    let current = selection
                        .lobby_factions
                        .get(slot)
                        .copied()
                        .unwrap_or(SkirmishFaction::Alliance);
                    spawn_faction_dropdown_button(
                        cell,
                        MainMenuAction::ToggleLobbySlotFaction(slot),
                        current,
                        faction_emblems,
                        selection,
                        font.clone(),
                        Val::Percent(100.0),
                    );
                    if selection.faction_dropdown_open == Some(slot) {
                        cell.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Percent(100.0),
                                left: px(0),
                                min_width: px(124.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                row_gap: px(1),
                                padding: UiRect::all(px(2)),
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.45, 0.56, 0.52)),
                            BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.98)),
                            GlobalZIndex(MENU_DROPDOWN_POPUP_Z),
                        ))
                        .with_children(|popup| {
                            for faction in SkirmishFaction::ALL {
                                spawn_faction_dropdown_button(
                                    popup,
                                    MainMenuAction::SetLobbySlotFaction(slot, faction),
                                    faction,
                                    faction_emblems,
                                    selection,
                                    font.clone(),
                                    px(124.0),
                                );
                            }
                        });
                    }
                });

            // Team dropdown (floating popup): 队1 … 队N.
            row.spawn(menu_dropdown_flex_cell_node(72.0))
                .with_children(|cell| {
                    let options: Vec<MainMenuAction> = (0..SKIRMISH_TEAM_OPTION_COUNT as usize)
                        .map(|t| MainMenuAction::SetLobbySlotTeam(slot, t))
                        .collect();
                    spawn_menu_dropdown_contents(
                        cell,
                        MainMenuAction::ToggleLobbySlotTeam(slot),
                        selection.team_dropdown_open == Some(slot),
                        &options,
                        selection,
                        font.clone(),
                        72.0,
                        13.0,
                    );
                });

            // Color dropdown (floating popup): 色1 … 色N.
            row.spawn(menu_dropdown_flex_cell_node(72.0))
                .with_children(|cell| {
                    let options: Vec<MainMenuAction> = (0..PLAYER_COLOR_PALETTE.len())
                        .map(|c| MainMenuAction::SetLobbySlotColor(slot, c))
                        .collect();
                    spawn_menu_dropdown_contents(
                        cell,
                        MainMenuAction::ToggleLobbySlotColor(slot),
                        selection.color_dropdown_open == Some(slot),
                        &options,
                        selection,
                        font.clone(),
                        72.0,
                        13.0,
                    );
                });
        });
}

pub(crate) fn menu_lobby_slot_label_node(width: f32) -> impl Bundle {
    Node {
        width: px(width),
        min_height: px(30),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexStart,
        ..default()
    }
}

pub(crate) fn menu_lobby_slot_row_node(
    slot: usize,
    selection: SkirmishMenuSelection,
) -> impl Bundle {
    let focused = selection.focus_lobby_slot() == Some(slot);
    let active = selection
        .lobby_controllers
        .get(slot)
        .copied()
        .is_some_and(SkirmishPlayerController::is_active);
    let border = if focused {
        Color::srgb(0.95, 0.72, 0.38)
    } else if active {
        Color::srgb(0.34, 0.44, 0.42)
    } else {
        Color::srgb(0.19, 0.23, 0.23)
    };
    let background = if active {
        Color::srgba(0.04, 0.055, 0.056, 0.96)
    } else {
        Color::srgba(0.028, 0.034, 0.034, 0.88)
    };

    (
        MainMenuLobbySlotRow,
        Node {
            width: Val::Percent(100.0),
            min_height: px(40),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::new(px(10), px(10), px(4), px(4)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
    )
}

pub(crate) fn menu_button(action: MainMenuAction, width: Val) -> impl Bundle {
    (
        Button,
        MainMenuButton { action },
        Node {
            width,
            min_height: px(38),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::new(px(8), px(8), px(0), px(0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.28, 0.34, 0.33)),
        BackgroundColor(Color::srgba(0.046, 0.058, 0.06, 0.94)),
    )
}

pub(crate) fn menu_button_label(
    label: impl Into<String>,
    font: Handle<Font>,
    font_size: f32,
) -> impl Bundle {
    (
        Text::new(label.into()),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 0.98)),
    )
}

pub(crate) fn main_menu_scroll(
    mut wheel_events: MessageReader<MouseWheel>,
    mut scroll_q: Query<&mut ScrollPosition, With<MainMenuScrollArea>>,
) {
    let mut delta = 0.0;
    for event in wheel_events.read() {
        let scroll_lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y * 0.05,
        };
        delta -= scroll_lines * 48.0;
    }
    if delta == 0.0 {
        return;
    }
    for mut scroll in &mut scroll_q {
        scroll.y = (scroll.y + delta).max(0.0);
    }
}

pub(crate) fn spawn_skirmish_map_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    selection: SkirmishMenuSelection,
) {
    parent
        .spawn(skirmish_map_preview_root())
        .with_children(|preview| {
            spawn_skirmish_map_preview_elements(preview, selection);
        });
}

pub(crate) fn spawn_skirmish_map_preview_elements(
    parent: &mut ChildSpawnerCommands<'_>,
    selection: SkirmishMenuSelection,
) {
    let map = selection.map();
    let rect = skirmish_map_preview_rect(map, SKIRMISH_MAP_PREVIEW_SIZE);
    parent.spawn(skirmish_map_preview_frame_node(rect));
    spawn_skirmish_map_preview_grid(parent, rect);

    for resource in map.resources {
        let kind = match resource.kind {
            ResourceKind::Ore => SkirmishMapPreviewMarkerKind::Ore,
            ResourceKind::Crystal => SkirmishMapPreviewMarkerKind::Crystal,
        };
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            resource.position,
            kind,
            7.0,
        ));
    }
    for tech in map.neutral_tech {
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            tech.position,
            SkirmishMapPreviewMarkerKind::NeutralTech,
            10.0,
        ));
    }
    for crate_spec in map.supply_crates {
        parent.spawn(skirmish_map_preview_marker_node(
            map,
            crate_spec.position,
            SkirmishMapPreviewMarkerKind::SupplyCrate,
            8.5,
        ));
    }
    for (slot, spawn_point) in map.spawn_points.iter().copied().enumerate() {
        parent.spawn(skirmish_map_preview_spawn_marker_node(
            map,
            spawn_point,
            skirmish_spawn_slot_color(selection, slot),
        ));
    }
}

pub(crate) fn spawn_skirmish_map_preview_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    rect: SkirmishMapPreviewRect,
) {
    for index in 1..SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS {
        let ratio = index as f32 / SKIRMISH_MAP_PREVIEW_GRID_DIVISIONS as f32;
        let x = rect.left + rect.width * ratio;
        let y = rect.top + rect.height * ratio;
        parent.spawn(skirmish_map_preview_grid_line_node(
            x,
            rect.top,
            1.0,
            rect.height,
        ));
        parent.spawn(skirmish_map_preview_grid_line_node(
            rect.left, y, rect.width, 1.0,
        ));
    }
}

pub(crate) fn main_menu_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<SkirmishMenuSelection>,
    mut setup_settings: ResMut<MatchSetupSettings>,
    mut random_map_cursor: ResMut<RandomMapCursor>,
    mut next_state: ResMut<NextState<AppScreen>>,
    mut buttons: Query<(
        &Interaction,
        &MainMenuButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .into_iter()
    .enumerate()
    {
        if index < SKIRMISH_MAPS.len() && keyboard.just_pressed(key) {
            selection.set_map_choice(index);
        }
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        selection.set_map_choice(random_map_index());
    }
    for (index, key) in [
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ]
    .into_iter()
    .enumerate()
    {
        if index < GODOT_STARTING_RESOURCE_OPTIONS.len() && keyboard.just_pressed(key) {
            selection.set_starting_resource_choice(index);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyH), (1, KeyCode::KeyD), (2, KeyCode::KeyC)] {
        if keyboard.just_pressed(key) {
            selection.select_lobby_slot(slot);
        }
    }
    for (mode, key) in [
        (SkirmishMatchMode::OneVsOne, KeyCode::Digit9),
        (SkirmishMatchMode::FreeForAll, KeyCode::Digit0),
        (SkirmishMatchMode::AiVsAi, KeyCode::KeyA),
        (SkirmishMatchMode::AlliedTwoVsOne, KeyCode::KeyM),
    ] {
        if keyboard.just_pressed(key) {
            selection.set_match_mode(mode);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyJ), (1, KeyCode::KeyK), (2, KeyCode::KeyL)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_team_id(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyZ), (1, KeyCode::KeyX), (2, KeyCode::KeyV)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_faction(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyQ), (1, KeyCode::KeyW), (2, KeyCode::KeyE)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_controller(slot);
        }
    }
    for (slot, key) in [(0, KeyCode::KeyU), (1, KeyCode::KeyI), (2, KeyCode::KeyO)] {
        if keyboard.just_pressed(key) {
            selection.cycle_lobby_slot_color(slot);
        }
    }
    for (difficulty, key) in [
        (AiDifficulty::Beginner, KeyCode::F1),
        (AiDifficulty::Easy, KeyCode::F2),
        (AiDifficulty::Normal, KeyCode::F3),
        (AiDifficulty::Hard, KeyCode::F4),
    ] {
        if keyboard.just_pressed(key) {
            selection.set_ai_difficulty(difficulty);
        }
    }

    let mut start_requested = keyboard.just_pressed(KeyCode::Enter);
    let mut back_requested = keyboard.just_pressed(KeyCode::Escape);
    let selection_snapshot = *selection;
    for (interaction, button, mut background, mut border) in &mut buttons {
        let clicked = *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left);
        if clicked {
            match button.action {
                MainMenuAction::SelectMap(index)
                    if index < SKIRMISH_MAPS.len() || is_random_map_index(index) =>
                {
                    selection.set_map_choice(index);
                }
                MainMenuAction::SelectStartingResources(index)
                    if index < GODOT_STARTING_RESOURCE_OPTIONS.len() =>
                {
                    selection.set_starting_resource_choice(index);
                }
                MainMenuAction::ToggleLobbySlotController(slot) => {
                    selection.toggle_controller_dropdown(slot);
                }
                MainMenuAction::SetLobbySlotController(slot, controller) => {
                    selection.set_lobby_slot_controller_choice(slot, controller);
                }
                MainMenuAction::ToggleLobbySlotFaction(slot) => {
                    selection.toggle_faction_dropdown(slot);
                }
                MainMenuAction::SetLobbySlotFaction(slot, faction) => {
                    selection.set_lobby_slot_faction_choice(slot, faction);
                }
                MainMenuAction::ToggleLobbySlotTeam(slot) => {
                    selection.toggle_team_dropdown(slot);
                }
                MainMenuAction::SetLobbySlotTeam(slot, team_index) => {
                    selection.set_lobby_slot_team_choice(slot, team_index);
                }
                MainMenuAction::ToggleLobbySlotColor(slot) => {
                    selection.toggle_color_dropdown(slot);
                }
                MainMenuAction::SetLobbySlotColor(slot, color_index) => {
                    selection.set_lobby_slot_color_choice(slot, color_index);
                }
                MainMenuAction::ToggleMapDropdown => {
                    selection.toggle_map_dropdown();
                }
                MainMenuAction::ToggleResourcesDropdown => {
                    selection.toggle_resources_dropdown();
                }
                MainMenuAction::BackToMainMenu => {
                    back_requested = true;
                }
                MainMenuAction::StartMatch => {
                    start_requested = true;
                }
                MainMenuAction::SelectMap(_) | MainMenuAction::SelectStartingResources(_) => {}
            }
        }

        let (bg, border_color) =
            main_menu_button_visual(button.action, *interaction, selection_snapshot);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }

    if back_requested {
        selection.close_all_lobby_dropdowns();
        next_state.set(AppScreen::MainMenu);
    } else if start_requested {
        start_shared_match_from_menu_selection(
            *selection,
            &mut setup_settings,
            &mut random_map_cursor,
            &mut next_state,
        );
    }
}

pub(crate) fn main_menu_button_visual(
    action: MainMenuAction,
    interaction: Interaction,
    selection: SkirmishMenuSelection,
) -> (Color, Color) {
    if matches!(action, MainMenuAction::StartMatch) {
        if !selection.can_start() {
            return match interaction {
                Interaction::Pressed => (
                    Color::srgba(0.052, 0.056, 0.056, 0.9),
                    Color::srgb(0.18, 0.22, 0.22),
                ),
                Interaction::Hovered => (
                    Color::srgba(0.06, 0.066, 0.066, 0.92),
                    Color::srgb(0.24, 0.28, 0.28),
                ),
                Interaction::None => (
                    Color::srgba(0.038, 0.044, 0.044, 0.86),
                    Color::srgb(0.16, 0.19, 0.19),
                ),
            };
        }
        return match interaction {
            Interaction::Pressed => (
                Color::srgba(0.11, 0.36, 0.2, 0.98),
                Color::srgb(0.74, 0.96, 0.62),
            ),
            Interaction::Hovered => (
                Color::srgba(0.08, 0.28, 0.18, 0.98),
                Color::srgb(0.58, 0.86, 0.5),
            ),
            Interaction::None => (
                Color::srgba(0.052, 0.18, 0.13, 0.96),
                Color::srgb(0.38, 0.64, 0.38),
            ),
        };
    }

    // Color dropdown: paint each button with the actual player color (godot shows
    // the palette swatch), and mark the current pick with a bright border.
    let swatch = match action {
        MainMenuAction::ToggleLobbySlotColor(slot) => Some(
            selection
                .lobby_color_slots
                .get(slot)
                .copied()
                .unwrap_or(slot)
                % PLAYER_COLOR_PALETTE.len(),
        ),
        MainMenuAction::SetLobbySlotColor(_, index) => Some(index % PLAYER_COLOR_PALETTE.len()),
        _ => None,
    };
    if let Some(index) = swatch {
        let c = PLAYER_COLOR_PALETTE[index];
        let bg = Color::srgb(c[0], c[1], c[2]);
        let border = if action.is_selected(selection) {
            Color::srgb(1.0, 0.95, 0.7)
        } else if interaction != Interaction::None {
            Color::WHITE
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.5)
        };
        return (bg, border);
    }

    let selected = action.is_selected(selection);
    match (selected, interaction) {
        (true, Interaction::Pressed) => (
            Color::srgba(0.32, 0.22, 0.08, 0.98),
            Color::srgb(1.0, 0.78, 0.36),
        ),
        (true, _) => (
            Color::srgba(0.22, 0.16, 0.065, 0.98),
            Color::srgb(0.88, 0.62, 0.28),
        ),
        (false, Interaction::Pressed) => (
            Color::srgba(0.08, 0.1, 0.1, 0.96),
            Color::srgb(0.42, 0.5, 0.48),
        ),
        (false, Interaction::Hovered) => (
            Color::srgba(0.062, 0.078, 0.078, 0.96),
            Color::srgb(0.42, 0.52, 0.5),
        ),
        (false, Interaction::None) => (
            Color::srgba(0.046, 0.058, 0.06, 0.94),
            Color::srgb(0.28, 0.34, 0.33),
        ),
    }
}

pub(crate) fn main_menu_brief_status_text(selection: SkirmishMenuSelection) -> String {
    let resources = selection.starting_resources();
    format!(
        "{}  |  {} {}/{}",
        selection.start_status().summary_label(),
        t("资源", "Resources"),
        resources.ore,
        resources.crystal,
    )
}

pub(crate) fn main_menu_summary_text(selection: SkirmishMenuSelection) -> String {
    let map = selection.map();
    let resources = selection.starting_resources();
    let focus_label = if selection.human_team().is_none() {
        t("观战焦点", "Spectate Focus")
    } else {
        t("我方出生槽", "My Spawn Slot")
    };
    format!(
        "{}: {}  |  {}: {}  |  {}: {}  |  AI: {}\n{}: {}  |  {}: {}\n{}: {}  |  {}: {}\n{}: {}/{}  |  {}: {}  |  {}: {}  |  {}: {}/{}  |  {}\n{}",
        t("地图", "Map"),
        selection.map_label(),
        t("模式", "Mode"),
        selection.match_mode.label(),
        focus_label,
        selection.focus_team().label(),
        selection.ai_difficulty.short_label(),
        t("控制", "Control"),
        skirmish_player_controller_text(selection),
        t("队伍", "Teams"),
        skirmish_team_setup_text(selection),
        t("种族", "Faction"),
        skirmish_player_faction_text(selection),
        t("颜色", "Color"),
        skirmish_player_color_text(selection),
        t("参战玩家", "Players"),
        selection.active_team_count(),
        selection.selected_map_player_slots(),
        t("需要出生点", "Spawns needed"),
        selection.required_player_slots(),
        t("地图出生点", "Map spawns"),
        map.players,
        t("资源", "Resources"),
        resources.ore,
        resources.crystal,
        selection.start_status().summary_label(),
        t(
            "开始: Enter/点击开始对战",
            "Start: Enter / click Start Match"
        ),
    )
}

pub(crate) fn main_menu_faction_info_text(selection: SkirmishMenuSelection) -> String {
    let faction = selection.focus_faction();
    format!(
        "{}: {}  |  {}  |  {}",
        t("对手", "Opponents"),
        skirmish_opponents_text(selection),
        skirmish_faction_roster_summary(faction),
        skirmish_faction_playstyle_summary(faction)
    )
}
