//! Native presentation fixture. No gameplay or paid-use acceptance.
use super::*;
use bevy::ecs::system::RunSystemOnce;

#[derive(Component)]
struct VisualIconIdentity {
    name: String,
    path: String,
}

pub fn capture_setup_semantic_ui(app: &mut App) {
    let world = app.world_mut();
    let team = world.resource::<VisiblePlayer>().team;
    world.resource_mut::<Economies>().get_mut(team);
    let roots: Vec<_> = world
        .query_filtered::<Entity, (With<Node>, Without<ChildOf>)>()
        .iter(world)
        .collect();
    for root in roots {
        world.entity_mut(root).despawn();
    }
    let server = world.resource::<AssetServer>().clone();
    let font = server.load(UI_FONT_PATH);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        commands.spawn((
            Text::new("UI visual fixture — no gameplay acceptance"),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                top: px(24),
                left: px(30),
                ..default()
            },
        ));
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: px(250),
                    left: px(30),
                    column_gap: px(12),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.035, 0.04, 0.9)),
            ))
            .with_children(|row| {
                for (index, (name, action)) in [
                    ("GuardArea", BuildAction::GuardArea),
                    ("HoldPosition", BuildAction::HoldPosition),
                    ("DeployMode", BuildAction::ToggleDeployMode),
                    ("Repair", BuildAction::RepairStructure),
                ]
                .into_iter()
                .enumerate()
                {
                    let path = command_action_icon_path(action).unwrap();
                    row.spawn(command_button(index))
                        .insert((
                            action,
                            CommandSlotAvailability { enabled: true },
                            VisualIconIdentity {
                                name: name.into(),
                                path: path.into(),
                            },
                        ))
                        .with_children(|button| {
                            button
                                .spawn(command_button_icon(index))
                                .insert((ImageNode::new(server.load(path)), Visibility::Inherited));
                            button
                                .spawn(command_button_badge(
                                    CommandSlotBadge::Hotkey(index),
                                    font.clone(),
                                ))
                                .insert(Text::new(command_action_display_key(index, action)));
                        });
                }
            });
        setup_support_power_panel(&mut commands, font.clone(), &server);
        setup_command_tooltip(&mut commands, font);
    }
    queue.apply(world);
    for mut node in world
        .query_filtered::<&mut Node, With<SupportPowersPanel>>()
        .iter_mut(world)
    {
        node.top = px(110);
        node.right = px(30);
    }
    let mut query = world.query::<(&SupportPowerButton, &mut Node, &mut Visibility)>();
    for (button, mut node, mut visibility) in query.iter_mut(world) {
        if matches!(
            button.kind,
            SupportPowerKind::RadarSweep
                | SupportPowerKind::EmpPulse
                | SupportPowerKind::ShieldOverdrive
        ) {
            continue;
        }
        node.display = Display::Flex;
        *visibility = Visibility::Inherited;
    }
    let buttons: Vec<_> = world
        .query::<(Entity, &SupportPowerButton)>()
        .iter(world)
        .filter(|(_, b)| {
            !matches!(
                b.kind,
                SupportPowerKind::RadarSweep
                    | SupportPowerKind::EmpPulse
                    | SupportPowerKind::ShieldOverdrive
            )
        })
        .map(|(e, b)| (e, b.kind))
        .collect();
    for (entity, kind) in buttons {
        world.entity_mut(entity).insert(VisualIconIdentity {
            name: format!("{kind:?}"),
            path: kind.icon_path().into(),
        });
    }
}

pub fn capture_semantic_ui_targets(app: &mut App) -> Vec<(String, String, Vec2)> {
    let world = app.world_mut();
    world
        .query::<(&VisualIconIdentity, &UiGlobalTransform)>()
        .iter(world)
        .map(|(id, t)| (id.name.clone(), id.path.clone(), t.translation))
        .collect()
}

pub fn capture_semantic_ui_tooltip(app: &mut App) -> Option<(String, String)> {
    let world = app.world_mut();
    world.run_system_once(update_command_tooltip).ok()?;
    let name = world
        .query::<(&VisualIconIdentity, &Interaction)>()
        .iter(world)
        .find(|(_, interaction)| **interaction == Interaction::Hovered)
        .map(|(id, _)| id.name.clone())?;
    let text = world
        .query_filtered::<&Text, With<CommandTooltipText>>()
        .iter(world)
        .next()?
        .0
        .clone();
    Some((name, text))
}
