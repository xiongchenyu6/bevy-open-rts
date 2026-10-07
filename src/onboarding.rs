//! Optional boot camp observes normal player commands; it never executes them.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TutorialStage {
    #[default]
    SelectWorker,
    Gather,
    BuildPower,
    BuildBarracks,
    Train,
    Move,
    Attack,
    Complete,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct TutorialState {
    pub(crate) active: bool,
    pub(crate) stage: TutorialStage,
    pub(crate) initial_ore: i32,
    pub(crate) initial_units: usize,
    pub(crate) initial_kills: u32,
    pub(crate) last_cargo: i32,
    pub(crate) move_origin: [f32; 3],
}

impl TutorialStage {
    pub(crate) fn number(self) -> usize {
        self as usize + 1
    }
    fn title(self) -> &'static str {
        match self {
            Self::SelectWorker => t("选中工人", "Select a worker"),
            Self::Gather => t("采集资源", "Gather resources"),
            Self::BuildPower => t("为基地供电", "Power your base"),
            Self::BuildBarracks => t("建设兵营", "Build a barracks"),
            Self::Train => t("训练部队", "Train your first troops"),
            Self::Move => t("移动部队", "Move your troops"),
            Self::Attack => t("攻击敌方基地", "Attack the enemy base"),
            Self::Complete => t("训练完成", "Training complete"),
        }
    }
    fn instruction(self) -> &'static str {
        match self {
            Self::SelectWorker => t(
                "左键点击基地附近的工人。",
                "Left-click a worker near your base.",
            ),
            Self::Gather => t(
                "右键点击矿石。工人会采集并自动送回资源。",
                "Right-click a mineral deposit. Your worker gathers and returns resources automatically.",
            ),
            Self::BuildPower => t(
                "选中工人，在指令面板选择发电站，再点击空地放置。",
                "Select a worker, choose Power Reactor in the command card, then place it on clear ground.",
            ),
            Self::BuildBarracks => t(
                "用工人在空地建造兵营，等待施工完成。",
                "Use a worker to place a Barracks on clear ground and wait for construction to finish.",
            ),
            Self::Train => t(
                "选中兵营，训练步兵。资源和队列进度显示在界面上。",
                "Select your Barracks and train infantry. Costs and queue progress appear in the command card.",
            ),
            Self::Move => t(
                "左键选中步兵，再右键点击空地。",
                "Left-click your infantry, then right-click clear ground to move it.",
            ),
            Self::Attack => t(
                "用 T 进入攻击移动模式，点击敌方基地。摧毁敌方指挥中心。",
                "Press T for attack-move, then click toward the enemy base. Destroy its Command Center to graduate.",
            ),
            Self::Complete => t(
                "你已经掌握了基地建设与作战的基础。",
                "You are ready to build a base and command an army.",
            ),
        }
    }
}

pub(crate) fn tutorial_match_settings() -> MatchSetupSettings {
    let mut settings = MatchSetupSettings::default();
    settings.active_teams = vec![true, true, false];
    settings.player_factions = vec![SkirmishFaction::Alliance; 3];
    settings.ai_difficulties.players = vec![AiDifficulty::Beginner; 3];
    settings.player_controllers = vec![
        SkirmishPlayerController::Human,
        SkirmishPlayerController::Ai(AiDifficulty::Beginner),
        SkirmishPlayerController::None,
    ];
    settings.starting_resources = StartingResources {
        ore: 64,
        crystal: 32,
    };
    settings.victory_condition = VictoryCondition::Headquarters;
    settings
}

pub(crate) fn tutorial_enemy_ai_enabled(tutorial: Res<TutorialState>) -> bool {
    !tutorial.active
}

#[derive(Component)]
pub(crate) struct TutorialPanel;
#[derive(Component)]
pub(crate) struct TutorialTitle;
#[derive(Component)]
pub(crate) struct TutorialInstruction;
#[derive(Component)]
pub(crate) struct SkipTutorial;

pub(crate) fn setup_tutorial_panel(mut commands: Commands, assets: Res<AssetServer>) {
    let font = assets.load(UI_FONT_PATH);
    commands
        .spawn((
            TutorialPanel,
            MatchScopedEntity,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(68),
                width: px(300),
                height: px(190),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.06, 0.96)),
            BorderColor::all(Color::srgb(0.28, 0.56, 0.6)),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                TutorialTitle,
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.96, 1.0)),
            ));
            panel.spawn((
                TutorialInstruction,
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.94, 0.96)),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            panel
                .spawn((
                    Button,
                    SkipTutorial,
                    Node {
                        height: px(28),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.07, 0.1, 0.12)),
                ))
                .with_children(|button| {
                    button.spawn((
                        localized_text("跳过训练", "Skip training"),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.82, 0.86)),
                    ));
                });
        });
}

pub(crate) fn update_tutorial(
    mut tutorial: ResMut<TutorialState>,
    economies: Res<Economies>,
    mut state: ResMut<MatchState>,
    mut flow: ResMut<MatchFlow>,
    units: Query<(
        &Unit,
        &Team,
        &Transform,
        Has<Selected>,
        Option<&ResourceCargo>,
        Option<&MoveOrder>,
        Option<&AttackMoveOrder>,
    )>,
    buildings: Query<(&Structure, &Team, Has<UnderConstruction>)>,
    mut panels: Query<&mut Visibility, With<TutorialPanel>>,
    mut titles: Query<&mut Text, (With<TutorialTitle>, Without<TutorialInstruction>)>,
    mut instructions: Query<&mut Text, (With<TutorialInstruction>, Without<TutorialTitle>)>,
) {
    for mut visible in &mut panels {
        *visible = if tutorial.active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !tutorial.active {
        return;
    }
    let player = Team::Player(0);
    let cargo: i32 = units
        .iter()
        .filter(|(_, team, ..)| **team == player)
        .filter_map(|(_, _, _, _, cargo, ..)| cargo)
        .map(|c| c.ore + c.crystal)
        .sum();
    let armed = |unit: &Unit| registry::entity(unit.id).is_some_and(|def| def.weapon.is_some());
    let completed = match tutorial.stage {
        TutorialStage::SelectWorker => units
            .iter()
            .any(|(u, t, _, s, ..)| *t == player && u.id == "Worker" && s),
        TutorialStage::Gather => {
            economies.get(player).ore > tutorial.initial_ore
                || (tutorial.last_cargo > cargo && tutorial.last_cargo > 0)
        }
        TutorialStage::BuildPower => buildings
            .iter()
            .any(|(b, t, c)| *t == player && b.id == "PowerReactor" && !c),
        TutorialStage::BuildBarracks => buildings
            .iter()
            .any(|(b, t, c)| *t == player && b.id == "Barracks" && !c),
        TutorialStage::Train => {
            units
                .iter()
                .filter(|(u, t, ..)| **t == player && armed(u))
                .count()
                > tutorial.initial_units
        }
        TutorialStage::Move => units
            .iter()
            .any(|(u, t, transform, s, _, move_order, attack)| {
                *t == player
                    && s
                    && armed(u)
                    && (move_order.is_some() || attack.is_some())
                    && transform
                        .translation
                        .distance(Vec3::from_array(tutorial.move_origin))
                        > 2.0
            }),
        TutorialStage::Attack => {
            state.phase == MatchPhase::HumanVictory
                && state.enemy_units_destroyed + state.enemy_structures_destroyed
                    > tutorial.initial_kills
        }
        TutorialStage::Complete => false,
    };
    tutorial.last_cargo = cargo;
    if completed {
        tutorial.stage = match tutorial.stage {
            TutorialStage::SelectWorker => {
                tutorial.initial_ore = economies.get(player).ore;
                TutorialStage::Gather
            }
            TutorialStage::Gather => TutorialStage::BuildPower,
            TutorialStage::BuildPower => TutorialStage::BuildBarracks,
            TutorialStage::BuildBarracks => {
                tutorial.initial_units = units
                    .iter()
                    .filter(|(u, t, ..)| **t == player && armed(u))
                    .count();
                TutorialStage::Train
            }
            TutorialStage::Train => {
                tutorial.move_origin = units
                    .iter()
                    .find(|(u, t, ..)| **t == player && armed(u))
                    .map(|(_, _, tf, ..)| tf.translation.to_array())
                    .unwrap_or_default();
                TutorialStage::Move
            }
            TutorialStage::Move => TutorialStage::Attack,
            _ => TutorialStage::Complete,
        };
    }
    if tutorial.stage == TutorialStage::Complete {
        tutorial.active = false;
        finalize_match(
            &mut state,
            &mut flow,
            MatchPhase::HumanVictory,
            t("基础训练完成", "Boot camp complete"),
        );
    }
    for mut title in &mut titles {
        title.0 = format!(
            "{} / 7 · {}",
            tutorial.stage.number().min(7),
            tutorial.stage.title()
        );
    }
    for mut instruction in &mut instructions {
        instruction.0 = tutorial.stage.instruction().to_string();
    }
}

pub(crate) fn tutorial_skip(
    mouse: Res<ButtonInput<MouseButton>>,
    buttons: Query<&Interaction, With<SkipTutorial>>,
    mut tutorial: ResMut<TutorialState>,
    mut next: ResMut<NextState<AppScreen>>,
) {
    if tutorial.active
        && mouse.just_pressed(MouseButton::Left)
        && buttons.iter().any(|i| *i == Interaction::Pressed)
    {
        *tutorial = TutorialState::default();
        next.set(AppScreen::CampaignMenu);
    }
}
