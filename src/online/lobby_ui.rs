//! Scrollable lobby content and a persistent action footer; no network actions here.
use super::*;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

#[derive(Component)]
pub(super) struct ScrollBody;
#[derive(Component)]
pub(super) struct ActionFooter;

pub(super) fn panel() -> impl Bundle {
    let (mut node, border, background) = online_panel_node(1240.0);
    node.overflow = Overflow::clip_y();
    (node, border, background)
}

pub(super) fn body() -> impl Bundle {
    (
        ScrollBody,
        ScrollPosition::default(),
        Node {
            width: Val::Percent(100.0),
            min_height: px(0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    )
}

pub(super) fn footer() -> impl Bundle {
    (
        ActionFooter,
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            ..default()
        },
    )
}

pub(super) fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    mut areas: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollBody>>,
) {
    let mut delta = 0.0;
    for event in wheel.read() {
        let lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y * 0.05,
        };
        delta -= lines * 48.0;
    }
    if delta == 0.0 {
        return;
    }
    for (mut position, node) in &mut areas {
        let maximum = (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0)
            * node.inverse_scale_factor();
        position.y = (position.y + delta).clamp(0.0, maximum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn spawn_fixture(mut commands: Commands, session: Res<OnlineSession>) {
        commands
            .spawn(Node::default())
            .with_children(|root| spawn_online_lobby(root, &session, Handle::<Font>::default()));
    }
    #[test]
    fn four_and_eight_slot_lobbies_keep_actions_outside_scrollable_player_content() {
        for map_id in ["plain_and_simple", "big_arena"] {
            let mut app = build_game_app(GameAppMode::Headless);
            let mut lobby = OnlineLobbySnapshot::new("LOCAL-QA".into(), "Host".into());
            resize_lobby_for_map(
                &mut lobby,
                SKIRMISH_MAPS.iter().find(|m| m.id == map_id).unwrap(),
            );
            let expected = lobby.slots.len();
            let mut session = app.world_mut().resource_mut::<OnlineSession>();
            session.is_host = true;
            session.local_player_id = Some(1);
            session.lobby = Some(lobby);
            drop(session);
            app.world_mut().run_system_once(spawn_fixture).unwrap();
            let bodies = app
                .world_mut()
                .query_filtered::<Entity, With<ScrollBody>>()
                .iter(app.world())
                .collect::<Vec<_>>();
            assert_eq!(
                bodies.len(),
                1,
                "actual lobby requires its own scrollable content: {map_id}"
            );
            let footers = app
                .world_mut()
                .query_filtered::<Entity, With<ActionFooter>>()
                .iter(app.world())
                .collect::<Vec<_>>();
            assert_eq!(footers.len(), 1);
            let body = bodies[0];
            let footer = footers[0];
            assert!(app.world().get::<ScrollPosition>(body).is_some());
            assert_eq!(app.world().get::<Node>(body).unwrap().min_height, px(0));
            assert_eq!(app.world().get::<Node>(footer).unwrap().flex_shrink, 0.0);
            assert_eq!(
                app.world().get::<ChildOf>(body).unwrap().parent(),
                app.world().get::<ChildOf>(footer).unwrap().parent()
            );
            let buttons = app
                .world_mut()
                .query::<(&OnlineButton, &ChildOf)>()
                .iter(app.world())
                .collect::<Vec<_>>();
            for action in [
                OnlineUiAction::ToggleReady,
                OnlineUiAction::StartMatch,
                OnlineUiAction::Disconnect,
            ] {
                let (_, parent) = buttons
                    .iter()
                    .find(|(b, _)| {
                        std::mem::discriminant(&b.action) == std::mem::discriminant(&action)
                    })
                    .expect("actual footer action");
                assert_eq!(
                    parent.parent(),
                    footer,
                    "footer action cannot be clipped inside body"
                );
            }
            assert_eq!(
                buttons
                    .iter()
                    .filter(|(b, _)| matches!(b.action, OnlineUiAction::CycleOccupant(_)))
                    .count(),
                expected
            );
        }
    }
    #[test]
    fn actual_wheel_events_scroll_lobby_and_reverse_immediately_after_bottom_clamp() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<MouseWheel>()
            .add_systems(Update, scroll);
        let body = app
            .world_mut()
            .spawn((
                ScrollBody,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(1000.0, 400.0),
                    content_size: Vec2::new(1000.0, 800.0),
                    ..default()
                },
            ))
            .id();
        let wheel = |y| MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        };
        app.world_mut().write_message(wheel(-100.0));
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(body).unwrap().y,
            400.0,
            "wheel must move actual body and clamp stored offset"
        );
        app.world_mut().write_message(wheel(1.0));
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(body).unwrap().y,
            352.0,
            "single up event must immediately return from bottom"
        );
    }
}
