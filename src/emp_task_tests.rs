//! Temporary EMP must pause paid worker tasks without discarding them.
use super::*;

fn app() -> App {
    // Use the live shared match schedule, including every EMP/order system.
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.1),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    app
}

fn step(app: &mut App) {
    app.update();
}

fn worker() -> Unit {
    Unit {
        id: "Worker",
        speed: 3.0,
        can_crush: false,
        can_be_crushed: true,
    }
}

#[test]
fn emp_pauses_construction_then_completes_after_actual_status_expiry() {
    let mut app = app();
    let target = app
        .world_mut()
        .spawn((
            Structure {
                id: "AntiAirTurret",
            },
            Team::Player(0),
            Transform::default(),
            Selectable { radius: 0.6 },
            Health::new(8.0),
            UnderConstruction {
                remaining: 0.5,
                total: 0.5,
                cost: registry::Cost::default(),
                free_worker_origin: None,
            },
        ))
        .id();
    let builder = app
        .world_mut()
        .spawn((
            worker(),
            Team::Player(0),
            Transform::from_xyz(1.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            ConstructOrder { target },
            OrderQueue {
                orders: std::collections::VecDeque::from([UnitQueuedOrder::Move(Vec3::new(
                    4.0, 0.0, 0.0,
                ))]),
            },
            MoveOrder { target: Vec3::X },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    for _ in 0..3 {
        step(&mut app);
        assert_eq!(
            app.world().get::<ConstructOrder>(builder).unwrap().target,
            target
        );
        assert!(app.world().get::<MoveOrder>(builder).is_some());
        assert_eq!(
            app.world().get::<OrderQueue>(builder).unwrap().orders.len(),
            1
        );
        assert_eq!(
            app.world()
                .get::<UnderConstruction>(target)
                .unwrap()
                .remaining,
            0.5
        );
    }
    for _ in 0..20 {
        step(&mut app);
    }
    assert!(app.world().get::<EmpDisabled>(builder).is_none());
    assert!(
        app.world().get::<UnderConstruction>(target).is_none(),
        "retained task should finish after real EMP expiry"
    );
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 8.0);
    assert!(app.world().get::<ConstructOrder>(builder).is_none());
}

#[test]
fn emp_preserves_harvest_target_cargo_and_progress_then_collects_after_expiry() {
    let mut app = app();
    let mineral = app
        .world_mut()
        .spawn((
            ResourceNode {
                kind: ResourceKind::Ore,
                amount: 10,
            },
            Transform::default(),
            Selectable { radius: 0.6 },
        ))
        .id();
    let collector = app
        .world_mut()
        .spawn((
            worker(),
            Team::Player(0),
            Transform::from_xyz(1.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            OrderQueue {
                orders: std::collections::VecDeque::from([UnitQueuedOrder::Move(Vec3::new(
                    4.0, 0.0, 0.0,
                ))]),
            },
            ResourceCargo {
                capacity: 6,
                ore: 2,
                crystal: 0,
            },
            HarvestOrder {
                resource: Some(mineral),
                state: HarvestState::Collecting,
                collect_remaining: 0.15,
                last_kind: Some(ResourceKind::Ore),
            },
            MoveOrder { target: Vec3::X },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    for _ in 0..3 {
        step(&mut app);
        let order = app.world().get::<HarvestOrder>(collector).unwrap();
        assert_eq!(order.resource, Some(mineral));
        assert_eq!(order.state, HarvestState::Collecting);
        assert_eq!(order.collect_remaining, 0.15);
        assert_eq!(order.last_kind, Some(ResourceKind::Ore));
        assert_eq!(app.world().get::<ResourceCargo>(collector).unwrap().ore, 2);
        assert_eq!(app.world().get::<ResourceNode>(mineral).unwrap().amount, 10);
        assert!(app.world().get::<MoveOrder>(collector).is_some());
        assert_eq!(
            app.world()
                .get::<OrderQueue>(collector)
                .unwrap()
                .orders
                .len(),
            1
        );
    }
    for _ in 0..10 {
        step(&mut app);
    }
    assert!(app.world().get::<EmpDisabled>(collector).is_none());
    assert!(app.world().get::<HarvestOrder>(collector).is_some());
    assert!(app.world().get::<ResourceCargo>(collector).unwrap().ore > 2);
    assert!(app.world().get::<ResourceNode>(mineral).unwrap().amount < 10);
}

#[test]
fn live_schedule_pauses_capture_garrison_repair_and_idle_queue_until_emp_expires() {
    let mut app = app();
    let enemy_hq = app
        .world_mut()
        .spawn((
            Structure {
                id: "CommandCenter",
            },
            Team::Player(1),
            Transform::from_xyz(8.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
            Health::new(20.0),
        ))
        .id();
    let engineer = Unit {
        id: "EngineerDrone",
        speed: 4.0,
        can_crush: false,
        can_be_crushed: false,
    };
    let elapsed = capture_time_for_unit(&engineer) - 0.05;
    let capturer = app
        .world_mut()
        .spawn((
            engineer,
            Team::Player(0),
            Transform::from_xyz(9.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
            Health::new(5.0),
            CaptureOrder {
                target: enemy_hq,
                elapsed,
            },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    let bunker = app
        .world_mut()
        .spawn((
            Structure { id: "TechBunker" },
            Team::Player(0),
            Transform::from_xyz(0.0, 0.0, 8.0),
            Selectable { radius: 0.6 },
            Health::new(12.0),
            Garrison {
                capacity: 3,
                damage_per_unit: 1.0,
                count: 0,
            },
        ))
        .id();
    let infantry = app
        .world_mut()
        .spawn((
            Unit {
                id: "HeavyMachinegunTrooper",
                speed: 3.0,
                can_crush: false,
                can_be_crushed: true,
            },
            Team::Player(0),
            Transform::from_xyz(1.0, 0.0, 8.0),
            Selectable { radius: 0.6 },
            Health::new(7.0),
            GarrisonOrder { target: bunker },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    let damaged = app
        .world_mut()
        .spawn((
            Structure { id: "PowerReactor" },
            Team::Player(0),
            Transform::from_xyz(0.0, 0.0, -8.0),
            Selectable { radius: 0.6 },
            Health {
                current: 4.0,
                max: 8.0,
            },
            // A real recent hit suppresses Alliance's ambient field repair,
            // so only the crawler's paused/resumed repair can change this HP.
            RecentDamage::fresh(),
        ))
        .id();
    let repair_unit = Unit {
        id: "MobileRepairCrawler",
        speed: 3.0,
        can_crush: false,
        can_be_crushed: true,
    };
    assert!(repair_capability(&repair_unit).is_some());
    let repairer = app
        .world_mut()
        .spawn((
            repair_unit,
            Team::Player(0),
            Transform::from_xyz(1.0, 0.0, -8.0),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            RepairOrder { target: damaged },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    let queued = app
        .world_mut()
        .spawn((
            worker(),
            Team::Player(0),
            Transform::from_xyz(-5.0, 0.0, 0.0),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            OrderQueue {
                orders: std::collections::VecDeque::from([UnitQueuedOrder::Move(Vec3::new(
                    5.0, 0.0, 0.0,
                ))]),
            },
            EmpDisabled { remaining: 0.5 },
        ))
        .id();
    for _ in 0..3 {
        step(&mut app);
        assert_eq!(
            app.world().get::<CaptureOrder>(capturer).unwrap().elapsed,
            elapsed
        );
        assert_eq!(*app.world().get::<Team>(enemy_hq).unwrap(), Team::Player(1));
        assert_eq!(app.world().get::<Garrison>(bunker).unwrap().count, 0);
        assert!(app.world().get::<GarrisonOrder>(infantry).is_some());
        assert!(app.world().get::<RepairOrder>(repairer).is_some());
        assert_eq!(app.world().get::<Health>(damaged).unwrap().current, 4.0);
        assert_eq!(
            app.world().get::<OrderQueue>(queued).unwrap().orders.len(),
            1
        );
        assert!(app.world().get::<MoveOrder>(queued).is_none());
    }
    for _ in 0..10 {
        step(&mut app);
    }
    assert_eq!(*app.world().get::<Team>(enemy_hq).unwrap(), Team::Player(0));
    assert_eq!(app.world().get::<Garrison>(bunker).unwrap().count, 1);
    assert!(app.world().get::<Health>(damaged).unwrap().current > 4.0);
    assert!(app.world().get::<OrderQueue>(queued).is_none());
    assert!(app.world().get::<MoveOrder>(queued).is_some());
}
