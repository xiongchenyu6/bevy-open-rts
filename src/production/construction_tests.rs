use super::*;

fn captured_construction_app() -> (App, Entity, Entity, Vec3) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.insert_resource(MapBounds {
        half_width: 40.0,
        half_depth: 40.0,
    });
    app.insert_resource(VisiblePlayer::per_player(Team::Player(0)));
    app.init_resource::<TerrainHeightField>()
        .init_resource::<NavGrid>()
        .init_resource::<TeamRelations>()
        .init_resource::<KillCredits>()
        .init_resource::<MatchState>()
        .init_resource::<LatestBattleEvent>()
        .init_resource::<NextSpawnId>()
        .init_resource::<AudioFeedback>()
        .init_resource::<BattleLog>();
    app.add_systems(
        Update,
        (
            rebuild_nav_grid,
            update_construct_orders,
            plan_unit_paths,
            move_units,
            separate_units,
            progress_under_construction_structures,
        )
            .chain(),
    );
    let factory_position = Vec3::new(-28.0, 0.0, -16.0);
    let factory = app
        .world_mut()
        .spawn((
            Structure {
                id: "VehicleFactory",
            },
            Team::Player(0),
            Transform::from_translation(factory_position),
            Selectable { radius: 1.5 },
            Health::new(40.0),
            UnderConstruction {
                remaining: 1.0,
                total: 30.0,
                cost: registry::Cost {
                    ore: 16,
                    crystal: 8,
                },
                free_worker_origin: None,
            },
        ))
        .id();
    app.world_mut().spawn((
        Structure { id: "PowerReactor" },
        Team::Player(0),
        Transform::from_xyz(-26.0, 0.0, -19.5),
        Selectable { radius: 1.5 },
        Health::new(40.0),
    ));
    app.world_mut().spawn((
        Unit {
            id: "Worker",
            speed: 2.5,
            can_crush: false,
            can_be_crushed: true,
        },
        Team::Player(0),
        MovementDomain::Terrain,
        Transform::from_xyz(-25.110014, 0.05, -17.64278),
        Selectable { radius: 0.6 },
        Health::new(6.0),
    ));
    let start = Vec3::new(-23.93772, 0.05, -17.399445);
    let blocked_goal = Vec3::new(-26.043524, 0.0, -17.390038);
    let worker = app
        .world_mut()
        .spawn((
            Unit {
                id: "Worker",
                speed: 2.5,
                can_crush: false,
                can_be_crushed: true,
            },
            Team::Player(0),
            MovementDomain::Terrain,
            Transform::from_translation(start),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            ConstructOrder { target: factory },
            MoveOrder {
                target: blocked_goal,
            },
            PlannedPath {
                goal: blocked_goal,
                waypoints: vec![Vec3::new(-26.25, 0.0, -17.25)],
                next: 0,
            },
        ))
        .id();
    (app, factory, worker, start)
}

#[test]
fn blocked_construction_contact_uses_an_alternate_side_and_finishes_by_walking() {
    let (mut app, factory, worker, start) = captured_construction_app();
    let mut previous = start;
    let mut chosen_approach = None;
    for _ in 0..240 {
        app.update();
        let world = app.world();
        if chosen_approach.is_none() {
            chosen_approach = world.get::<MoveOrder>(worker).map(|order| order.target);
        }
        let position = world.get::<Transform>(worker).unwrap().translation;
        assert!(
            xz_distance(position, previous) <= (2.5 + UNIT_SEPARATION_MAX_SPEED) / 30.0 + 0.001,
            "worker must walk; no teleporting to contact"
        );
        previous = position;
        if world.get::<UnderConstruction>(factory).is_none() {
            break;
        }
        let cost = world.get::<UnderConstruction>(factory).unwrap().cost;
        assert_eq!((cost.ore, cost.crystal), (16, 8));
    }
    let approach = chosen_approach.expect("construction chooses a move approach");
    assert!(
        approach.z > -16.5,
        "neighboring Power/worker block the south-east side; approach from north, got {approach:?}"
    );
    assert!(
        app.world().get::<UnderConstruction>(factory).is_none(),
        "real movement and construction systems must finish the previously stuck foundation"
    );
    assert!(app.world().get::<ConstructOrder>(worker).is_none());
    assert!(xz_distance(start, previous) > 1.0);
    assert_eq!(app.world().get::<Health>(factory).unwrap().current, 40.0);
}

#[test]
fn contact_selection_rejects_occupied_endpoints_and_unreachable_ring_cells() {
    let source = Vec3::new(-23.93772, 0.0, -17.399445);
    let target = Vec3::new(-28.0, 0.0, -16.0);
    let obstacles = [(target, 1.5), (Vec3::new(-26.0, 0.0, -19.5), 1.5)];
    let occupied = [(Vec3::new(-25.110014, 0.0, -17.64278), 0.6)];
    let blocked_goal = Vec3::new(-26.043524, 0.0, -17.390038);
    assert!(!contact_position_is_clear(
        blocked_goal,
        0.6,
        &obstacles,
        &occupied
    ));
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 40.0,
            half_depth: 40.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let approach =
        reachable_unit_contact_approach(source, 0.6, target, 1.5, &grid, &obstacles, &occupied)
            .unwrap();
    assert!(contact_position_is_clear(
        approach.target,
        0.6,
        &obstacles,
        &occupied
    ));
    assert!((xz_distance(approach.target, target) - 2.4).abs() < 0.001);
    assert_eq!(approach.waypoints.last(), Some(&approach.target));
    assert!(
        xz_distance(*approach.waypoints.last().unwrap(), target)
            <= contact_action_entry_range(0.6, 1.5, CONSTRUCTION_ENTRY_MARGIN_M)
    );
    grid.blocked.fill(true);
    assert!(
        reachable_unit_contact_approach(source, 0.6, target, 1.5, &grid, &obstacles, &occupied)
            .is_none(),
        "an inaccessible ring must not snap to an unrelated open cell"
    );
}

#[test]
fn construction_waits_for_a_real_route_without_moving_or_advancing_work() {
    let (mut app, factory, worker, _) = captured_construction_app();
    let idle: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, (With<Unit>, Without<ConstructOrder>)>()
        .iter(app.world())
        .collect();
    for entity in idle {
        app.world_mut().entity_mut(entity).despawn();
    }
    app.update();
    let start = app.world().get::<Transform>(worker).unwrap().translation;
    app.world_mut().resource_mut::<NavGrid>().blocked.fill(true);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Transform>(worker).unwrap().translation,
        start
    );
    assert_eq!(
        app.world()
            .get::<UnderConstruction>(factory)
            .unwrap()
            .remaining,
        1.0
    );
    assert!(app.world().get::<MoveOrder>(worker).is_none());
    assert!(app.world().get::<PlannedPath>(worker).is_none());
    assert!(app.world().get::<ConstructOrder>(worker).is_some());
    let obstacles = [
        (Vec3::new(-28.0, 0.0, -16.0), 1.5),
        (Vec3::new(-26.0, 0.0, -19.5), 1.5),
    ];
    app.world_mut()
        .resource_mut::<NavGrid>()
        .rebuild_with_terrain(
            MapBounds {
                half_width: 40.0,
                half_depth: 40.0,
            },
            &obstacles,
            &TerrainHeightField::default(),
        );
    for _ in 0..240 {
        app.update();
        if app.world().get::<UnderConstruction>(factory).is_none() {
            break;
        }
    }
    assert!(app.world().get::<UnderConstruction>(factory).is_none());
}

#[test]
fn contact_clearance_accepts_tangent_roundoff_without_accepting_overlap() {
    let obstacle = [(Vec3::new(-28.0, 0.0, -16.0), 1.5)];
    let tangent = obstacle[0].0 + Vec3::X * (0.6 + 1.5 + MOVEMENT_OBSTACLE_CLEARANCE_M);
    assert!(contact_position_is_clear(tangent, 0.6, &obstacle, &[]));
    assert!(!contact_position_is_clear(
        tangent - Vec3::X * 0.01,
        0.6,
        &obstacle,
        &[]
    ));
}
