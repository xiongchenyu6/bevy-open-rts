//! Campaign mission cutscenes: a full-screen illustrated briefing panel shown
//! as the mission opens — keyart with a slow Ken Burns zoom, the mission
//! title, a typewriter briefing and a "click to continue" hint. The sim clock
//! is held at zero while the panel is up and released on dismissal.

use bevy::prelude::*;

use crate::*;

/// Root of the live cutscene overlay.
#[derive(Component)]
pub(crate) struct MissionCutsceneRoot;

/// The keyart node, slowly zooming for a filmic feel.
#[derive(Component)]
pub(crate) struct CutsceneArt {
    pub(crate) age: f32,
}

/// The briefing body, revealed one character at a time.
#[derive(Component)]
pub(crate) struct CutsceneTypewriter {
    pub(crate) full: String,
    pub(crate) shown: usize,
    pub(crate) timer: f32,
}

/// Shows the cutscene when a campaign mission starts (skirmish is unaffected).
pub(crate) fn spawn_mission_cutscene(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    active_mission: Res<ActiveMission>,
    locale: Res<Locale>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let Some(mission) = active_mission.0.and_then(mission_by_index) else {
        return;
    };
    let Some(asset_server) = asset_server else {
        return;
    };
    // Hold the battlefield while the player reads the briefing.
    virtual_time.set_relative_speed(0.0);
    let zh = matches!(locale.0, Language::Zh);
    let title = if zh { mission.name_zh } else { mission.name_en };
    let briefing = if zh {
        mission.briefing_zh
    } else {
        mission.briefing_en
    };
    let font: Handle<Font> = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            Name::new("Mission cutscene"),
            MissionCutsceneRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(60),
            MatchScopedEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("Cutscene art"),
                ImageNode::new(asset_server.load(mission.art)),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                Transform::from_scale(Vec3::splat(1.0)),
                CutsceneArt { age: 0.0 },
            ));
            // Bottom letterbox band holding title + briefing.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    bottom: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::new(px(48), px(48), px(20), px(26)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.66)),
            ))
            .with_children(|band| {
                band.spawn((
                    Text::new(title),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.9, 0.55)),
                ));
                band.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.94, 0.92)),
                    Node {
                        max_width: px(980),
                        ..default()
                    },
                    CutsceneTypewriter {
                        full: briefing.to_string(),
                        shown: 0,
                        timer: 0.0,
                    },
                ));
                band.spawn((
                    Text::new(t("点击 或 回车 继续", "Click or press Enter to continue")),
                    TextFont {
                        font: font.into(),
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.75, 0.8, 0.78, 0.8)),
                    Node {
                        margin: UiRect::top(px(6)),
                        ..default()
                    },
                ));
            });
        });
}

/// Ken Burns zoom + typewriter reveal. Uses real time — the sim clock is held
/// at zero while the cutscene is up.
pub(crate) fn animate_mission_cutscene(
    time: Res<Time<Real>>,
    mut art: Query<(&mut CutsceneArt, &mut Transform)>,
    mut writers: Query<(&mut CutsceneTypewriter, &mut Text)>,
) {
    let dt = time.delta_secs();
    for (mut state, mut transform) in &mut art {
        state.age += dt;
        let zoom = 1.0 + (state.age / 14.0).min(1.0) * 0.07;
        transform.scale = Vec3::splat(zoom);
    }
    for (mut writer, mut text) in &mut writers {
        if writer.shown >= writer.full.chars().count() {
            continue;
        }
        writer.timer += dt;
        // ~28 characters per second.
        let target = ((writer.timer * 28.0) as usize).min(writer.full.chars().count());
        if target != writer.shown {
            writer.shown = target;
            text.0 = writer.full.chars().take(target).collect();
        }
    }
}

/// Click / Enter / Space / Escape dismisses the cutscene and releases the sim.
pub(crate) fn dismiss_mission_cutscene(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    roots: Query<Entity, With<MissionCutsceneRoot>>,
    pause: Res<TacticalPause>,
    match_speed: Res<MatchSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if roots.is_empty() {
        return;
    }
    let confirm = mouse.just_pressed(MouseButton::Left)
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Escape);
    if !confirm {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !pause.0 {
        virtual_time.set_relative_speed(match_speed.preset.scale());
    }
}
