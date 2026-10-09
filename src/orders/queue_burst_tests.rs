use super::*;

fn normal_worker() -> (App, Entity, [Vec3; 3]) {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    let (worker, origin) = {
        let world = app.world_mut();
        let mut units = world.query::<(Entity, &Unit, &Team, &Transform, &Health)>();
        let (entity, _, _, position, _) = units
            .iter(world)
            .find(|(_, unit, team, _, health)| {
                unit.id == "Worker" && **team == Team::Player(0) && health.current > 0.0
            })
            .unwrap();
        (entity, position.translation)
    };
    (
        app,
        worker,
        [
            origin + Vec3::X * 6.0,
            origin + Vec3::X * 6.0 + Vec3::Z * 6.0,
            origin,
        ],
    )
}

fn assert_fifo(app: &App, worker: Entity, goals: [Vec3; 3]) {
    assert_eq!(
        app.world().get::<MoveOrder>(worker).unwrap().target,
        goals[0]
    );
    let queue = app
        .world()
        .get::<OrderQueue>(worker)
        .expect("both queued commands retained");
    let actual = queue
        .orders
        .iter()
        .map(|order| match order {
            UnitQueuedOrder::Move(target) => *target,
            _ => panic!("normal move burst changed order kind"),
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, goals[1..]);
}

#[test]
fn two_same_frame_queued_moves_preserve_existing_active_move() {
    let (mut app, worker, goals) = normal_worker();
    let mut commands = bevy::ecs::world::CommandQueue::default();
    {
        let mut writer = Commands::new(&mut commands, app.world());
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[0]),
            false,
            true,
        );
    }
    commands.apply(app.world_mut());
    {
        let mut writer = Commands::new(&mut commands, app.world());
        for target in goals[1..].iter().copied() {
            issue_or_queue_unit_order(
                &mut writer,
                worker,
                UnitQueuedOrder::Move(target),
                true,
                true,
            );
        }
    }
    // One normal deferred application for the complete input-frame burst.
    commands.apply(app.world_mut());
    assert_fifo(&app, worker, goals);
}

#[test]
fn fresh_replace_then_same_frame_queued_moves_preserve_fifo() {
    let (mut app, worker, goals) = normal_worker();
    let mut commands = bevy::ecs::world::CommandQueue::default();
    {
        let mut writer = Commands::new(&mut commands, app.world());
        for (index, target) in goals.iter().copied().enumerate() {
            issue_or_queue_unit_order(
                &mut writer,
                worker,
                UnitQueuedOrder::Move(target),
                index != 0,
                true,
            );
        }
    }
    commands.apply(app.world_mut());
    assert_fifo(&app, worker, goals);
}

#[test]
fn normal_replace_cancels_earlier_same_frame_queue() {
    let (mut app, worker, goals) = normal_worker();
    let mut commands = bevy::ecs::world::CommandQueue::default();
    {
        let mut writer = Commands::new(&mut commands, app.world());
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[0]),
            false,
            true,
        );
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[1]),
            true,
            true,
        );
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[2]),
            false,
            true,
        );
    }
    commands.apply(app.world_mut());
    assert_eq!(
        app.world().get::<MoveOrder>(worker).unwrap().target,
        goals[2]
    );
    assert!(app.world().get::<OrderQueue>(worker).is_none());
}

#[test]
fn disallowed_queue_still_replaces_and_despawned_actor_is_ignored() {
    let (mut app, worker, goals) = normal_worker();
    let mut commands = bevy::ecs::world::CommandQueue::default();
    {
        let mut writer = Commands::new(&mut commands, app.world());
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[0]),
            false,
            true,
        );
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[1]),
            true,
            true,
        );
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[2]),
            true,
            false,
        );
    }
    commands.apply(app.world_mut());
    assert_eq!(
        app.world().get::<MoveOrder>(worker).unwrap().target,
        goals[2]
    );
    assert!(app.world().get::<OrderQueue>(worker).is_none());
    {
        let mut writer = Commands::new(&mut commands, app.world());
        writer.entity(worker).despawn();
        issue_or_queue_unit_order(
            &mut writer,
            worker,
            UnitQueuedOrder::Move(goals[0]),
            true,
            true,
        );
    }
    commands.apply(app.world_mut());
    assert!(app.world().get_entity(worker).is_err());
}
