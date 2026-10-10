//! Recorded normal TechDivide0684 contact routing regression; no World injection.
use super::*;

#[test]
fn constructor_can_leave_recorded_clearance_pocket_without_crossing_obstacles() {
    let source = Vec3::new(-29.62732, 0.05, 23.755636);
    let target = Vec3::new(-35.185074, 0.0, 17.689447);
    let radius = 0.6;
    let obstacles = [
        (Vec3::new(-29.16797, 0.0, 21.879368), 1.45), // completed PowerReactor
        (Vec3::new(-32.0, 0.0, 26.0), 2.0),           // HQ
        (Vec3::new(-28.0, 0.0, 24.0), 0.6),           // resource node
        (target, 1.5),                                // paid VehicleFactory foundation
    ];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 42.0,
            half_depth: 42.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let escape = Vec3::new(-29.25, source.y, 24.75);
    assert!(
        grid.line_clear(source, escape),
        "short escape preserves base terrain/static restrictions"
    );
    assert!(contact_position_is_clear(escape, radius, &obstacles, &[]));
    for (position, obstacle_radius) in obstacles {
        let minimum = distance_point_to_xz_segment(position, source, escape);
        let original = xz_distance(source, position);
        let clearance = radius + obstacle_radius + MOVEMENT_OBSTACLE_CLEARANCE_M;
        assert!(
            minimum + 1.0e-4 >= original.min(clearance),
            "escape worsens penetration or crosses a new footprint"
        );
    }
    let approach = reachable_unit_contact_approach(
        source,
        radius,
        target,
        1.5,
        &grid,
        &obstacles,
        &[obstacles[0], obstacles[1], obstacles[3]],
        &[],
    )
    .expect("recorded pocket has a clear outward escape and valid contact route");
    assert!(contact_position_is_clear(
        approach.target,
        radius,
        &obstacles,
        &[]
    ));
    assert!(
        !approach.waypoints.is_empty(),
        "worker follows real approach waypoints"
    );
    assert!(move_order_targets_contact(
        Some(&MoveOrder {
            target: approach.target
        }),
        target,
        radius,
        1.5
    ));
}

#[test]
fn contact_escape_does_not_cross_a_blocked_terrain_wall() {
    let source = Vec3::new(-4.0, 0.05, 0.0);
    let target = Vec3::new(4.0, 0.0, 0.0);
    let obstacles = [(target, 1.5)];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 10.0,
            half_depth: 10.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    for z in 0..grid.height {
        grid.blocked[(z * grid.width + grid.width / 2) as usize] = true;
        grid.terrain_blocked[(z * grid.width + grid.width / 2) as usize] = true;
    }
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            1.5,
            &grid,
            &obstacles,
            &obstacles,
            &[]
        )
        .is_none(),
        "contact escape must respect the continuous blocked terrain barrier"
    );
}

#[test]
fn constructor_can_escape_recorded_finished_refinery_overlap() {
    let source = Vec3::new(-28.186312, 0.05, 28.825842);
    let target = Vec3::new(-29.16797, 0.0, 21.879368);
    let obstacles = [
        (Vec3::new(-27.52068, 0.0, 28.22164), 1.5),
        (Vec3::new(-32.0, 0.0, 26.0), 2.0),
        (Vec3::new(-27.0, 0.0, 30.0), 0.6),
        (Vec3::new(-24.0, 0.0, 27.0), 0.6),
        (target, 1.45),
    ];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 42.0,
            half_depth: 42.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let approach = reachable_unit_contact_approach(
        source,
        0.6,
        target,
        1.45,
        &grid,
        &obstacles,
        &[obstacles[0], obstacles[1], obstacles[4]],
        &[],
    )
    .expect("already-overlapped completed Refinery needs a continuous outward escape");
    assert!(contact_position_is_clear(
        approach.target,
        0.6,
        &obstacles,
        &[]
    ));
    let first = *approach.waypoints.first().expect("real escape waypoint");
    let minimum = distance_point_to_xz_segment(obstacles[0].0, source, first);
    assert!(
        minimum + 1e-4 >= xz_distance(source, obstacles[0].0),
        "first leg worsens existing penetration"
    );
}

#[test]
fn normal_paid_factory_constructor_escapes_and_finishes_in_full_schedule() {
    let mut app = recorded_tech_divide_app();
    let mut factory = None;
    let mut tracked_worker = None;
    let mut previous_position = None;
    let mut previous_remaining = None;
    let mut paid = false;
    let mut initial_overlap = false;
    let mut movement_distance = 0.0;
    let mut contact_progress = false;
    let mut completed = false;
    for _ in 0..3600 {
        let before = {
            let bank = app.world().resource::<Economies>().get(Team::Player(1));
            (bank.ore, bank.crystal)
        };
        app.update();
        let world = app.world_mut();
        let after = {
            let bank = world.resource::<Economies>().get(Team::Player(1));
            (bank.ore, bank.crystal)
        };
        if factory.is_none() {
            let mut sites = world.query::<(Entity, &Team, &Structure, &UnderConstruction)>();
            if let Some((entity, _, _, site)) = sites.iter(world).find(|(_, team, structure, _)| {
                **team == Team::Player(1) && structure.id == "VehicleFactory"
            }) {
                factory = Some(entity);
                let cost = registry::entity("VehicleFactory").unwrap().cost;
                assert_eq!((site.cost.ore, site.cost.crystal), (cost.ore, cost.crystal));
                assert!(site.free_worker_origin.is_none());
                assert!(before.0 >= cost.ore && before.1 >= cost.crystal);
                paid = after.0 < before.0 || after.1 < before.1;
            }
        }
        let Some(factory) = factory else { continue };
        if tracked_worker.is_none() {
            let mut builders = world.query::<(Entity, &ConstructOrder)>();
            tracked_worker = builders
                .iter(world)
                .find(|(_, order)| order.target == factory)
                .map(|(entity, _)| entity);
        }
        if let Some(worker) = tracked_worker {
            let position = world.get::<Transform>(worker).unwrap().translation;
            let speed = world.get::<Unit>(worker).unwrap().speed;
            let radius = world.get::<Selectable>(worker).unwrap().radius;
            if let Some(previous) = previous_position {
                movement_distance += xz_distance(previous, position);
                assert!(
                    xz_distance(previous, position)
                        <= (speed + UNIT_SEPARATION_MAX_SPEED) / 30.0 + 0.002,
                    "constructor teleported instead of following normal movement"
                );
            }
            previous_position = Some(position);
            let mut structures = world.query::<(
                &Structure,
                &Transform,
                &Selectable,
                Option<&UnderConstruction>,
            )>();
            initial_overlap |=
                structures
                    .iter(world)
                    .any(|(structure, transform, selectable, site)| {
                        structure.id == "PowerReactor"
                            && site.is_none()
                            && xz_distance(position, transform.translation)
                                < radius + selectable.radius + MOVEMENT_OBSTACLE_CLEARANCE_M
                    });
            let remaining = world
                .get::<UnderConstruction>(factory)
                .map(|site| site.remaining);
            if previous_remaining
                .zip(remaining)
                .is_some_and(|(before, after)| after < before - 1e-5)
            {
                let target = world.get::<Transform>(factory).unwrap().translation;
                let target_radius = world.get::<Selectable>(factory).unwrap().radius;
                assert!(
                    xz_distance(position, target)
                        <= contact_action_entry_range(
                            radius,
                            target_radius,
                            CONSTRUCTION_ENTRY_MARGIN_M
                        ) + (speed + UNIT_SEPARATION_MAX_SPEED) / 30.0
                            + 1e-4
                );
                contact_progress = true;
            }
            previous_remaining = remaining;
        }
        if world.get::<Structure>(factory).is_some()
            && world.get::<UnderConstruction>(factory).is_none()
        {
            completed = true;
            break;
        }
    }
    {
        let world = app.world_mut();
        let state = world.resource::<MatchState>();
        eprintln!(
            "[constructor-phase] clock={} phase={:?} reason={:?}",
            state.start_time_sec, state.phase, state.result_reason
        );
        let mut builders = world.query::<(
            Entity,
            &ConstructOrder,
            &Transform,
            Option<&MoveOrder>,
            Option<&PlannedPath>,
        )>();
        for (entity, order, position, movement, path) in builders.iter(world) {
            eprintln!(
                "[constructor-active] worker={entity:?} position={:?} target={:?} id={:?} movement={:?} path={:?}",
                position.translation,
                order.target,
                world
                    .get::<Structure>(order.target)
                    .map(|structure| structure.id),
                movement.map(|movement| movement.target),
                path.map(|path| (path.goal, path.next, path.waypoints.len()))
            );
        }
        let mut sites =
            world.query::<(Entity, &Team, &Structure, &Transform, &UnderConstruction)>();
        for (entity, team, structure, position, site) in sites.iter(world) {
            eprintln!(
                "[constructor-site] entity={entity:?} team={team:?} id={} position={:?} remaining={}",
                structure.id, position.translation, site.remaining
            );
        }
    }
    assert!(paid, "normal factory never debited its authored price");
    eprintln!(
        "[constructor-paid] factory={factory:?} worker={tracked_worker:?} paid={paid} old_overlap={initial_overlap} movement={movement_distance:.3} contact={contact_progress} completed={completed}"
    );
    assert!(
        movement_distance > 0.1,
        "constructor never followed normal movement to the site"
    );
    assert!(
        contact_progress,
        "worker never reached and worked at the actual paid site"
    );
    assert!(
        completed,
        "paid VehicleFactory never completed in the full normal schedule"
    );
}

fn recorded_tech_divide_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings.map_path = SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == "tech_divide")
            .unwrap()
            .godot_path;
        settings.starting_resources = StartingResources::new(32, 16);
        settings.startup_loadout = StartupLoadoutMode::GodotSkirmish;
        settings.victory_condition = VictoryCondition::Annihilation;
        settings.visible_player = VisiblePlayer::all_players(Team::Player(0));
        settings.active_teams = vec![false; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_controllers =
            vec![SkirmishPlayerController::None; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_factions = vec![SkirmishFaction::Alliance; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_spawn_slots = default_player_spawn_slots();
        settings.team_relations = TeamRelations::default();
        for (player, difficulty, spawn) in
            [(0, AiDifficulty::Beginner, 0), (1, AiDifficulty::Easy, 2)]
        {
            settings.active_teams[player] = true;
            settings.player_controllers[player] = SkirmishPlayerController::Ai(difficulty);
            settings.player_spawn_slots[player] = spawn;
            settings
                .ai_difficulties
                .set_difficulty(Team::Player(player), difficulty);
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    app
}

#[test]
fn normal_paid_refinery_finishes_without_enclosing_its_constructor() {
    let mut app = recorded_tech_divide_app();
    let mut refinery = None;
    let mut worker = None;
    let mut paid = false;
    let mut work_finished_clear = false;
    for _ in 0..900 {
        let before = {
            let bank = app.world().resource::<Economies>().get(Team::Player(1));
            (bank.ore, bank.crystal)
        };
        app.update();
        let world = app.world_mut();
        let after = {
            let bank = world.resource::<Economies>().get(Team::Player(1));
            (bank.ore, bank.crystal)
        };
        if refinery.is_none() {
            let mut sites = world.query::<(Entity, &Team, &Structure, &UnderConstruction)>();
            if let Some((entity, _, _, site)) = sites.iter(world).find(|(_, team, structure, _)| {
                **team == Team::Player(1) && structure.id == "Refinery"
            }) {
                refinery = Some(entity);
                let cost = registry::entity("Refinery").unwrap().cost;
                assert_eq!((site.cost.ore, site.cost.crystal), (cost.ore, cost.crystal));
                paid = before.0 >= cost.ore
                    && before.1 >= cost.crystal
                    && (after.0 < before.0 || after.1 < before.1);
            }
        }
        let Some(refinery) = refinery else { continue };
        if worker.is_none() {
            let mut builders = world.query::<(Entity, &ConstructOrder)>();
            worker = builders
                .iter(world)
                .find(|(_, order)| order.target == refinery)
                .map(|(entity, _)| entity);
        }
        if world
            .get::<UnderConstruction>(refinery)
            .is_some_and(|site| site.remaining <= 0.0)
        {
            let worker = worker.expect("normal Worker assigned to paid site");
            let position = world.get::<Transform>(worker).unwrap().translation;
            let target = world.get::<Transform>(refinery).unwrap().translation;
            let clearance = world.get::<Selectable>(worker).unwrap().radius
                + world.get::<Selectable>(refinery).unwrap().radius
                + MOVEMENT_OBSTACLE_CLEARANCE_M;
            let distance = xz_distance(position, target);
            eprintln!(
                "[constructor-work-finished] clock={} distance={distance} clearance={clearance}",
                world.resource::<MatchState>().start_time_sec
            );
            assert!(
                distance + 1e-4 >= clearance,
                "last construction work enclosed its constructor"
            );
            work_finished_clear = true;
        }
        if world.get::<UnderConstruction>(refinery).is_none() {
            assert!(paid, "Refinery did not use the normal price");
            let worker = worker.expect("normal Worker assigned to paid site");
            let position = world.get::<Transform>(worker).unwrap().translation;
            let target = world.get::<Transform>(refinery).unwrap().translation;
            let clearance = world.get::<Selectable>(worker).unwrap().radius
                + world.get::<Selectable>(refinery).unwrap().radius
                + MOVEMENT_OBSTACLE_CLEARANCE_M;
            let distance = xz_distance(position, target);
            eprintln!(
                "[constructor-finish-clearance] clock={} distance={distance} clearance={clearance}",
                world.resource::<MatchState>().start_time_sec
            );
            assert!(
                work_finished_clear,
                "actual zero-remaining construction completion was not observed"
            );
            let body_clearance = world.get::<Selectable>(worker).unwrap().radius
                + world.get::<Selectable>(refinery).unwrap().radius;
            assert!(
                distance + 1e-4 >= body_clearance,
                "normal post-completion movement trapped the Worker inside the finished building"
            );
            return;
        }
    }
    panic!("normal paid Refinery did not finish");
}

#[test]
fn building_escape_never_crosses_a_real_terrain_cliff() {
    const PLATEAUS: &[TerrainPlateauSpec] = &[TerrainPlateauSpec {
        min: (10.0, 0.0),
        max: (20.0, 20.0),
        level: 1,
    }];
    let map = SkirmishMapDef {
        size: (20.0, 20.0),
        terrain_plateaus: PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
        ..SKIRMISH_MAPS[0]
    };
    let mut terrain = TerrainHeightField::default();
    terrain.rebuild(&map);
    let source = Vec3::new(-4.0, 0.05, 0.0);
    let target = Vec3::new(4.0, 1.4, 0.0);
    let obstacles = [(Vec3::new(-4.0, 0.0, 0.2), 1.5), (target, 1.5)];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 10.0,
            half_depth: 10.0,
        },
        &obstacles,
        &terrain,
    );
    assert!(!grid.terrain_line_clear(source, target));
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            1.5,
            &grid,
            &obstacles,
            &obstacles,
            &[]
        )
        .is_none()
    );
}

#[test]
fn building_escape_never_crosses_a_sealed_static_wall() {
    let source = Vec3::new(-4.0, 0.05, 0.0);
    let target = Vec3::new(4.0, 0.0, 0.0);
    let buildings = [(Vec3::new(-4.0, 0.0, 0.2), 1.5), (target, 1.5)];
    let mut obstacles = buildings.to_vec();
    for z in (-9..=9).step_by(2) {
        obstacles.push((Vec3::new(0.0, 0.0, z as f32), 1.1));
    }
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 10.0,
            half_depth: 10.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            1.5,
            &grid,
            &obstacles,
            &buildings,
            &[]
        )
        .is_none()
    );
}

#[test]
fn overlap_escape_exception_excludes_resources_and_mobile_units() {
    let source = Vec3::new(-4.0, 0.05, 0.0);
    let target = Vec3::new(4.0, 0.0, 0.0);
    let resource = (Vec3::new(-4.0, 0.0, 0.2), 1.5);
    let buildings = [(target, 1.5)];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 10.0,
            half_depth: 10.0,
        },
        &[resource, buildings[0]],
        &TerrainHeightField::default(),
    );
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            1.5,
            &grid,
            &[resource, buildings[0]],
            &buildings,
            &[]
        )
        .is_none()
    );
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 10.0,
            half_depth: 10.0,
        },
        &buildings,
        &TerrainHeightField::default(),
    );
    assert!(
        reachable_unit_contact_approach(
            source,
            0.6,
            target,
            1.5,
            &grid,
            &buildings,
            &buildings,
            &[(source, 0.6)]
        )
        .is_none()
    );
}

#[test]
fn recorded_resource_padding_overlap_recovers_only_outward() {
    let resource = [(Vec3::ZERO, 0.6)];
    let source = Vec3::new(1.32, 0.05, 0.0);
    assert!(
        contact_escape_leg_is_clear(source, Vec3::new(1.6, 0.05, 0.0), 0.6, &resource, &[], &[]),
        "outside body but already inside navigation padding can move outward"
    );
    assert!(
        !contact_escape_leg_is_clear(source, Vec3::new(1.28, 0.05, 0.0), 0.6, &resource, &[], &[]),
        "moving deeper into existing resource padding is invalid"
    );
    assert!(
        !contact_escape_leg_is_clear(
            Vec3::new(1.1, 0.05, 0.0),
            Vec3::new(1.6, 0.05, 0.0),
            0.6,
            &resource,
            &[],
            &[]
        ),
        "actual resource body overlap is never an escape exception"
    );
}

#[test]
fn recorded_building_egress_preserves_mobile_bodies_and_all_later_gap() {
    let source = Vec3::new(36.33197, 0.05, 23.208853);
    let building = (Vec3::new(37.569324, 0.0, 24.369417), 1.45);
    let resource = (Vec3::new(34.742104, 0.0, 23.747704), 0.6);
    let peer = (Vec3::new(35.500843, 0.05, 22.275192), 0.6);
    let end = Vec3::new(38.75, 0.05, 20.25);
    assert!(contact_position_is_clear(
        end,
        0.6,
        &[building, resource],
        &[peer]
    ));
    assert!(distance_point_to_xz_segment(peer.0, source, end) > 1.2);
    assert!(
        contact_escape_leg_is_clear(
            source,
            end,
            0.6,
            &[building, resource],
            &[building],
            &[peer]
        ),
        "short measured building egress can traverse the optional mobile gap without touching its body"
    );
    assert!(
        !contact_escape_leg_is_clear(
            source,
            Vec3::new(34.0, 0.05, 20.0),
            0.6,
            &[building, resource],
            &[building],
            &[peer]
        ),
        "mobile body crossing remains forbidden"
    );
    assert!(
        !contact_escape_leg_is_clear(
            source,
            Vec3::new(40.0, 0.05, 26.0),
            0.6,
            &[building, resource],
            &[building],
            &[peer]
        ),
        "building escape cannot deepen the initial overlap"
    );
    let new_obstacle = (Vec3::new(37.5, 0.0, 21.8), 0.6);
    assert!(
        !contact_escape_leg_is_clear(
            source,
            end,
            0.6,
            &[building, resource, new_obstacle],
            &[building],
            &[peer]
        ),
        "a new resource obstruction cannot be tunneled through"
    );
}

#[test]
fn mobile_gap_remains_required_after_the_initial_building_exit() {
    let source = Vec3::new(1.0, 0.05, 0.0);
    let end = Vec3::new(5.0, 0.05, 0.0);
    let building = (Vec3::ZERO, 1.5);
    let peer = (Vec3::new(4.0, 0.05, 1.225), 0.6);
    assert!(contact_position_is_clear(end, 0.6, &[building], &[peer]));
    assert!(distance_point_to_xz_segment(peer.0, source, end) > 1.2);
    assert!(
        !contact_escape_leg_is_clear(source, end, 0.6, &[building], &[building], &[peer]),
        "post-building-exit0.05 mobile gap cannot be reduced to body-only clearance"
    );
}

#[test]
fn recorded_deep_techlab_builder_has_a_curved_monotonic_exit() {
    let source = Vec3::new(36.084114, 0.05, 22.689596);
    let target = Vec3::new(35.603214, 0.0, 22.380812);
    let obstacles = [
        (target, 1.6),
        (Vec3::new(37.569324, 0.0, 24.369417), 1.45),
        (Vec3::new(34.742104, 0.0, 23.747704), 0.6),
        (Vec3::new(36.059, 0.0, 26.025597), 0.6),
        (Vec3::new(38.9349, 0.0, 25.4003), 0.6),
        (Vec3::new(40.0, 0.0, 20.0), 2.0),
    ];
    let buildings = [obstacles[0], obstacles[1], obstacles[5]];
    let occupied = [(Vec3::new(34.08079, 0.05, 20.764206), 0.6)];
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 48.0,
            half_depth: 48.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let approach = reachable_unit_contact_approach(
        source, 0.6, target, 1.6, &grid, &obstacles, &buildings, &occupied,
    )
    .expect("measured builder can leave along a curved continuously outward route");
    assert!(contact_position_is_clear(
        approach.target,
        0.6,
        &obstacles,
        &occupied
    ));
    let mut previous = source;
    for waypoint in &approach.waypoints {
        for &(position, radius) in &obstacles {
            let before = xz_distance(previous, position);
            let minimum = distance_point_to_xz_segment(position, previous, *waypoint);
            let full = 0.6 + radius + MOVEMENT_OBSTACLE_CLEARANCE_M;
            assert!(minimum + 1e-4 >= before.min(full));
        }
        assert!(grid.terrain_line_clear(previous, *waypoint));
        previous = *waypoint;
    }
}
#[test]
fn normal_separation_releases_recorded_pair_without_entering_static_bodies() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.1),
    ));
    app.insert_resource(TeamRelations::default());
    app.insert_resource(MapBounds {
        half_width: 48.0,
        half_depth: 48.0,
    });
    app.insert_resource(TerrainHeightField::default());
    app.add_systems(Update, separate_units);
    let positions = [
        Vec3::new(-36.16629, 0.05, -26.76077),
        Vec3::new(-36.68934, 0.05, -26.28649),
    ];
    let ids = positions.map(|position| {
        app.world_mut()
            .spawn((
                Team::Player(0),
                Unit {
                    id: "Worker",
                    speed: 3.0,
                    can_crush: false,
                    can_be_crushed: true,
                },
                MovementDomain::Terrain,
                Selectable { radius: 0.6 },
                Transform::from_translation(position),
                Health::new(6.0),
            ))
            .id()
    });
    let obstacles = [
        (Vec3::new(-37.05493, 0.0, -28.39860), 1.5),
        (Vec3::new(-37.99236, 0.0, -24.57923), 1.5),
        (Vec3::new(-37.16797, 0.0, -24.12063), 1.45),
    ];
    for (position, radius) in obstacles {
        app.world_mut().spawn((
            Structure {
                id: "VehicleFactory",
            },
            Transform::from_translation(position),
            Selectable { radius },
            Health::new(12.0),
        ));
    }
    for _ in 0..20 {
        let before = ids.map(|id| app.world().get::<Transform>(id).unwrap().translation);
        app.update();
        for (id, previous) in ids.into_iter().zip(before) {
            let next = app.world().get::<Transform>(id).unwrap().translation;
            assert!(xz_distance(previous, next) <= UNIT_SEPARATION_MAX_SPEED * 0.1 + 1e-4);
            for (position, radius) in obstacles {
                let original = xz_distance(previous, position);
                assert!(
                    distance_point_to_xz_segment(position, previous, next) + 1e-4
                        >= original.min(radius + 0.6)
                );
            }
        }
    }
    let final_positions = ids.map(|id| app.world().get::<Transform>(id).unwrap().translation);
    assert!(
        xz_distance(final_positions[0], final_positions[1]) >= 1.2,
        "normal separation must release the recorded overlapping pair"
    );
}

#[test]
fn recorded_factory_padding_deficit_has_a_curved_outward_contact_route() {
    let source = Vec3::new(-38.107796, 0.05, -27.120518);
    let target = Vec3::new(-36.224319, 0.0, -28.059418);
    let obstacles = [
        (target, 1.5),
        (Vec3::new(-37.992355, 0.0, -24.579231), 1.5),
        (Vec3::new(-37.167973, 0.0, -24.120632), 1.45),
        (Vec3::new(-37.655800, 0.0, -28.613300), 0.6),
        (Vec3::new(-36.444401, 0.0, -23.515352), 1.4),
    ];
    let buildings = [obstacles[0], obstacles[1], obstacles[2], obstacles[4]];
    let occupied = [
        (Vec3::new(-39.143517, 0.05, -27.820362), 0.6),
        (Vec3::new(-35.879921, 0.05, -25.958956), 0.6),
    ];
    assert!(xz_distance(source, target) > 2.1);
    assert!(xz_distance(source, target) < 2.1 + MOVEMENT_OBSTACLE_CLEARANCE_M);
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(
        MapBounds {
            half_width: 48.0,
            half_depth: 48.0,
        },
        &obstacles,
        &TerrainHeightField::default(),
    );
    let approach = reachable_unit_contact_approach(
        source, 0.6, target, 1.5, &grid, &obstacles, &buildings, &occupied,
    )
    .expect("existing structure padding deficit can leave along a curved outward path");
    assert!(contact_position_is_clear(
        approach.target,
        0.6,
        &obstacles,
        &occupied
    ));
    let mut previous = source;
    for point in &approach.waypoints {
        for (position, radius) in obstacles {
            let before = xz_distance(previous, position);
            assert!(
                distance_point_to_xz_segment(position, previous, *point) + 1e-4
                    >= before.min(0.6 + radius + MOVEMENT_OBSTACLE_CLEARANCE_M)
            );
        }
        for (position, radius) in occupied {
            assert!(
                distance_point_to_xz_segment(position, previous, *point) + 1e-4 >= 0.6 + radius
            );
        }
        assert!(grid.terrain_line_clear(previous, *point));
        previous = *point;
    }
}
