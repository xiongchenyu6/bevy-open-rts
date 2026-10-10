use super::*;

fn fixture() -> (App, Entity, Entity) {
    let mut app = App::new();
    let mut setup = MatchSetupSettings::default();
    setup.player_controllers = vec![
        SkirmishPlayerController::Human,
        SkirmishPlayerController::Human,
        SkirmishPlayerController::Ai(AiDifficulty::Easy),
    ];
    app.insert_resource(setup)
        .insert_resource(VisiblePlayer::per_player(Team::Player(0)))
        .insert_resource(PlayerFactions(vec![SkirmishFaction::Chaos; 3]))
        .insert_resource(Economies {
            players: vec![
                TeamEconomy::new(32, 16),
                TeamEconomy::new(32, 16),
                TeamEconomy::new(32, 0),
            ],
        })
        .add_systems(Update, auto_assign_idle_resource_collectors);
    for id in [
        "CommandCenter",
        "Refinery",
        "PowerReactor",
        "RadarUplink",
        "VehicleFactory",
        "RoboticsBay",
    ] {
        app.world_mut().spawn((
            Structure { id },
            Team::Player(2),
            Transform::from_xyz(-4.0, 0.0, 0.0),
            Selectable { radius: 1.0 },
            Health::new(20.0),
        ));
    }
    let ore = app
        .world_mut()
        .spawn((
            ResourceNode {
                kind: ResourceKind::Ore,
                amount: 100,
            },
            Transform::from_xyz(1.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
        ))
        .id();
    let crystal = app
        .world_mut()
        .spawn((
            ResourceNode {
                kind: ResourceKind::Crystal,
                amount: 100,
            },
            Transform::from_xyz(8.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
        ))
        .id();
    (app, ore, crystal)
}
fn worker(app: &mut App, team: Team, resource: Option<Entity>) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            Unit {
                id: "Worker",
                speed: 2.8,
                can_crush: false,
                can_be_crushed: true,
            },
            team,
            Transform::default(),
            Health::new(6.0),
            ResourceCargo {
                capacity: 6,
                ore: 0,
                crystal: 0,
            },
        ))
        .id();
    if let Some(resource) = resource {
        app.world_mut().entity_mut(entity).insert(HarvestOrder {
            resource: Some(resource),
            state: HarvestState::Collecting,
            collect_remaining: 0.7,
            last_kind: Some(ResourceKind::Ore),
        });
    }
    entity
}
#[test]
fn crystal_demand_reassigns_one_empty_ai_ore_collector_without_touching_manual_orders() {
    let (mut app, ore, crystal) = fixture();
    let ai = worker(&mut app, Team::Player(2), Some(ore));
    let ai_second = worker(&mut app, Team::Player(2), Some(ore));
    let human = worker(&mut app, Team::Player(1), Some(ore));
    let queued = worker(&mut app, Team::Player(2), Some(ore));
    app.world_mut().entity_mut(queued).insert(OrderQueue {
        orders: std::collections::VecDeque::from([UnitQueuedOrder::Move(Vec3::X * 12.0)]),
    });
    let constructor = worker(&mut app, Team::Player(2), Some(ore));
    app.world_mut()
        .entity_mut(constructor)
        .insert(ConstructOrder { target: ore });
    let repairer = worker(&mut app, Team::Player(2), Some(ore));
    app.world_mut()
        .entity_mut(repairer)
        .insert(RepairOrder { target: ore });
    let mover = worker(&mut app, Team::Player(2), None);
    app.world_mut().entity_mut(mover).insert(MoveOrder {
        target: Vec3::Z * 11.0,
    });
    app.update();
    let changed = [ai, ai_second]
        .into_iter()
        .filter(|e| app.world().get::<HarvestOrder>(*e).unwrap().resource == Some(crystal))
        .count();
    assert_eq!(
        changed, 1,
        "one crystal collector must service actual available Robot crystal cost; the other retains Ore income"
    );
    for entity in [human, queued, constructor, repairer] {
        let order = app.world().get::<HarvestOrder>(entity).unwrap();
        assert_eq!(order.resource, Some(ore));
        assert_eq!(order.collect_remaining, 0.7);
    }
    assert_eq!(
        app.world().get::<OrderQueue>(queued).unwrap().orders.len(),
        1
    );
    assert_eq!(
        app.world()
            .get::<ConstructOrder>(constructor)
            .unwrap()
            .target,
        ore
    );
    assert_eq!(
        app.world().get::<RepairOrder>(repairer).unwrap().target,
        ore
    );
    assert_eq!(
        app.world().get::<MoveOrder>(mover).unwrap().target,
        Vec3::Z * 11.0
    );
    assert!(app.world().get::<HarvestOrder>(mover).is_none());
    assert_eq!(
        app.world()
            .resource::<Economies>()
            .get(Team::Player(2))
            .crystal,
        0
    );
    assert_eq!(
        app.world().get::<ResourceNode>(crystal).unwrap().amount,
        100
    );
}
#[test]
fn idle_ai_prefers_demanded_crystal_and_retains_nearest_ore_when_cost_is_already_affordable() {
    let (mut app, ore, crystal) = fixture();
    let ai = worker(&mut app, Team::Player(2), None);
    app.update();
    assert_eq!(
        app.world().get::<HarvestOrder>(ai).unwrap().resource,
        Some(crystal)
    );
    let (mut app, ore2, _) = fixture();
    app.world_mut()
        .resource_mut::<Economies>()
        .get_mut(Team::Player(2))
        .crystal = 16;
    let ai = worker(&mut app, Team::Player(2), None);
    app.update();
    assert_eq!(
        app.world().get::<HarvestOrder>(ai).unwrap().resource,
        Some(ore2)
    );
    let _ = ore;
}
