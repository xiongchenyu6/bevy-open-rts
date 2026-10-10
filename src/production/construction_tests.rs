use super::*;

fn construction_test_app() -> App {
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
    app
}

fn captured_construction_app() -> (App, Entity, Entity, Vec3) {
    let mut app = construction_test_app();
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
    let mut source = Vec3::new(-23.93772, 0.0, -17.399445);
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
    assert!(
        reachable_unit_contact_approach(
            source, 0.6, target, 1.5, &grid, &obstacles, &obstacles, &occupied
        )
        .is_none(),
        "already-overlapped mobile Worker must resolve through normal separation"
    );
    let away = (source - occupied[0].0).normalize();
    source += away
        * ((0.6 + occupied[0].1 + UNIT_SEPARATION_GAP_M) - xz_distance(source, occupied[0].0)
            + 1e-4);
    let approach = reachable_unit_contact_approach(
        source, 0.6, target, 1.5, &grid, &obstacles, &obstacles, &occupied,
    )
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
        reachable_unit_contact_approach(
            source, 0.6, target, 1.5, &grid, &obstacles, &obstacles, &occupied
        )
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
    // Close every physical exit with real hard walls. Aggregate raster cells
    // alone cannot stand in for terrain when exact circles leave a clear leg.
    let walls = (0..8)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / 8.0;
            let position = start + Vec3::new(angle.cos(), 0.0, angle.sin()) * 1.2;
            app.world_mut()
                .spawn((
                    TerrainWall,
                    Transform::from_translation(position),
                    Selectable { radius: 0.6 },
                ))
                .id()
        })
        .collect::<Vec<_>>();
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
    for wall in walls {
        app.world_mut().entity_mut(wall).despawn();
    }
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

fn captured_army_contact_geometry() -> (Vec3, Vec3, [(Vec3, f32); 6], [(Vec3, f32); 3]) {
    let source = Vec3::new(-21.072931, 0.05, -18.432116);
    let target = Vec3::new(-21.999998, 0.0, -13.000001);
    let obstacles = [
        (Vec3::new(-24.0, 0.0, -16.0), 0.6),
        (Vec3::new(-22.6, 0.0, -14.4), 0.6),
        (Vec3::new(-26.0, 0.0, -19.500002), 1.45),
        (Vec3::new(-22.0, 0.0, -22.0), 2.0),
        (Vec3::new(-18.000002, 0.0, -19.499998), 1.45),
        (target, 0.6),
    ];
    let occupied = [
        (Vec3::new(-17.898895, 0.05, -14.627776), 0.9),
        (Vec3::new(-16.060225, 0.05, -15.886452), 0.9),
        (Vec3::new(-20.37863, 0.05, -14.580074), 0.48),
    ];
    (source, target, obstacles, occupied)
}

#[test]
fn allied_army_does_not_permanently_cut_off_an_open_construction_contact() {
    // Minimized from campaign-contact-fixture-2/failure-contact.json. The
    // resources are alive (204/240), terrain is flat and the source cell is open.
    // Six static footprints and three allied units reproduce the hard-routing
    // pocket; removing either the first Tank or infantry restores a route.
    let (source, target, obstacles, occupied) = captured_army_contact_geometry();
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 30.0,
            half_depth: 30.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let source_cell = grid.cell_of(source);
    assert!(!grid.is_blocked(source_cell.0, source_cell.1));
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            0.6,
            &grid,
            &obstacles,
            &[obstacles[2], obstacles[3], obstacles[4], obstacles[5]],
            &[]
        )
        .is_some()
    );
    let approach = reachable_unit_contact_approach(
        source,
        0.6,
        target,
        0.6,
        &grid,
        &obstacles,
        &[obstacles[2], obstacles[3], obstacles[4], obstacles[5]],
        &occupied,
    )
    .expect(
        "mobile allied troops must not permanently isolate a constructor with free target contacts",
    );
    assert!(contact_position_is_clear(
        approach.target,
        0.6,
        &obstacles,
        &occupied
    ));
    assert_eq!(approach.waypoints.last(), Some(&approach.target));
}

#[test]
fn constructor_walks_past_allied_army_and_completes_without_crossing_static_footprints() {
    let mut app = construction_test_app();
    app.insert_resource(MapBounds {
        half_width: 30.0,
        half_depth: 30.0,
    });
    let (source, target, obstacles, occupied) = captured_army_contact_geometry();
    for (index, &(position, radius)) in obstacles[..5].iter().enumerate() {
        if index < 2 {
            app.world_mut().spawn((
                ResourceNode {
                    kind: ResourceKind::Ore,
                    amount: [204, 240][index],
                },
                Transform::from_translation(position),
                Selectable { radius },
            ));
        } else {
            app.world_mut().spawn((
                Structure {
                    id: ["PowerReactor", "CommandCenter", "Barracks"][index - 2],
                },
                Team::Player(0),
                Transform::from_translation(position),
                Selectable { radius },
                Health::new(20.0),
            ));
        }
    }
    let paid_cost = registry::Cost { ore: 8, crystal: 6 };
    let foundation = app
        .world_mut()
        .spawn((
            Structure {
                id: "AntiAirTurret",
            },
            Team::Player(0),
            Transform::from_translation(target),
            Selectable { radius: 0.6 },
            Health {
                current: 1.0,
                max: 8.0,
            },
            UnderConstruction {
                remaining: 1.0,
                total: 1.0,
                cost: paid_cost,
                free_worker_origin: None,
            },
        ))
        .id();
    for (index, &(position, radius)) in occupied.iter().enumerate() {
        app.world_mut().spawn((
            Unit {
                id: if index < 2 {
                    "Tank"
                } else {
                    "HeavyMachinegunTrooper"
                },
                speed: registry::entity(if index < 2 {
                    "Tank"
                } else {
                    "HeavyMachinegunTrooper"
                })
                .unwrap()
                .speed,
                can_crush: false,
                can_be_crushed: index == 2,
            },
            Team::Player(0),
            MovementDomain::Terrain,
            Transform::from_translation(position),
            Selectable { radius },
            Health::new(10.0),
        ));
    }
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
            Transform::from_translation(source),
            Selectable { radius: 0.6 },
            Health::new(6.0),
            ConstructOrder { target: foundation },
        ))
        .id();
    let mut previous = source;
    let mut walked = false;
    for _ in 0..900 {
        app.update();
        let world = app.world();
        let position = world.get::<Transform>(worker).unwrap().translation;
        assert!(
            xz_distance(position, previous) <= (2.5 + UNIT_SEPARATION_MAX_SPEED) / 30.0 + 0.001,
            "constructor must walk and yield normally; no teleporting"
        );
        assert!(
            obstacles
                .iter()
                .all(|(center, radius)| xz_distance(position, *center) + 1.0e-4 >= 0.6 + radius),
            "constructor penetrated a static footprint at {position:?}"
        );
        walked |= xz_distance(position, source) > 1.0;
        previous = position;
        if let Some(construction) = world.get::<UnderConstruction>(foundation) {
            assert_eq!(
                (construction.cost.ore, construction.cost.crystal),
                (paid_cost.ore, paid_cost.crystal)
            );
        } else {
            break;
        }
    }
    assert!(
        walked,
        "the captured constructor never acquired a real movement route"
    );
    assert!(
        app.world().get::<UnderConstruction>(foundation).is_none(),
        "mobile allied troops permanently stalled the captured foundation"
    );
    assert!(app.world().get::<ConstructOrder>(worker).is_none());
    assert_eq!(app.world().get::<Health>(foundation).unwrap().current, 8.0);
}

#[test]
fn stalled_foundation_retains_damage_and_does_not_revive() {
    let mut app = construction_test_app();
    let mut sites = Vec::new();
    for current in [4.0, 0.0] {
        sites.push(
            app.world_mut()
                .spawn((
                    Structure { id: "PowerReactor" },
                    Team::Player(0),
                    Transform::default(),
                    Health { current, max: 12.0 },
                    UnderConstruction {
                        remaining: 0.5,
                        total: 1.0,
                        cost: registry::Cost { ore: 3, crystal: 1 },
                        free_worker_origin: None,
                    },
                ))
                .id(),
        );
    }
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().get::<Health>(sites[0]).unwrap().current, 4.0);
    assert_eq!(app.world().get::<Health>(sites[1]).unwrap().current, 0.0);
    for site in sites {
        assert_eq!(
            app.world()
                .get::<UnderConstruction>(site)
                .unwrap()
                .remaining,
            0.5
        );
    }
}

#[test]
fn construction_growth_and_completion_preserve_received_damage() {
    let mut construction = UnderConstruction {
        remaining: 0.5,
        total: 1.0,
        cost: registry::Cost { ore: 3, crystal: 1 },
        free_worker_origin: None,
    };
    // Half-built cap is 6.5 HP. A previous hit removed 2.5 HP.
    let mut health = Health {
        current: 4.0,
        max: 12.0,
    };
    apply_structure_construction_progress(&mut construction, &mut health, 10.0);
    assert_eq!(construction.remaining, 0.0);
    assert_eq!(health.current, 9.5);
    let mut app = construction_test_app();
    let site = app
        .world_mut()
        .spawn((
            Structure { id: "PowerReactor" },
            Team::Player(0),
            Transform::default(),
            health,
            construction,
        ))
        .id();
    app.update();
    assert!(app.world().get::<UnderConstruction>(site).is_none());
    assert_eq!(app.world().get::<Health>(site).unwrap().current, 9.5);
}

#[test]
fn live_weapons_destroy_an_unworked_foundation() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<MatchSetupSettings>()
        .player_controllers
        .fill(SkirmishPlayerController::Human);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    let site = app
        .world_mut()
        .spawn((
            Structure { id: "PowerReactor" },
            Team::Player(1),
            Transform::default(),
            Selectable { radius: 1.45 },
            MovementDomain::Terrain,
            Health {
                current: 6.5,
                max: 12.0,
            },
            UnderConstruction {
                remaining: 0.5,
                total: 1.0,
                cost: registry::Cost { ore: 3, crystal: 1 },
                free_worker_origin: None,
            },
        ))
        .id();
    let def = registry::entity("ScoutRover").unwrap();
    let w = def.weapon.unwrap();
    app.world_mut().spawn((
        Unit {
            id: def.id,
            speed: def.speed,
            can_crush: def.can_crush,
            can_be_crushed: def.can_be_crushed,
        },
        Team::Player(0),
        Transform::from_xyz(3.0, 0.05, 0.0),
        Selectable { radius: def.radius },
        MovementDomain::Terrain,
        Health::new(def.health),
        VisionRadius(8.0),
        Weapon::new(
            w.range,
            w.damage,
            w.cooldown,
            w.splash_radius,
            w.splash_damage_multiplier,
            w.structure_damage_multiplier,
            w.can_attack_air,
            w.can_attack_ground,
        ),
        HoldPosition { enabled: true },
        AttackOrder { target: site },
    ));
    for _ in 0..600 {
        app.update();
        if app.world().get::<Health>(site).is_none() {
            return;
        }
    }
    panic!(
        "ordinary repeated weapon hits did not destroy the idle foundation: HP {:?}",
        app.world().get::<Health>(site).map(|health| health.current)
    );
}
