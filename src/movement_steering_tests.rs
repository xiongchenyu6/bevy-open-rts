//! Actual stored case0101 geometry; full shared movement schedule, no paid-match claim.
use super::*;
const ENDPOINT: &str = r#"[[4294966548,"Refinery","Player(0)",[14.234686851501465,0.0,-22.941082000732422],1.5,14.0,null],[352187317407,"RoboticsBay","Player(0)",[18.9823055267334,0.0,-14.97655200958252],1.5499999523162842,14.0,null],[17179868567,"PowerReactor","Player(0)",[21.181882858276367,0.0,-21.28394889831543],1.4500000476837158,12.0,null],[21474835698,"PowerReactor","Player(0)",[-20.127159118652344,0.0,-10.603227615356445],1.4500000476837158,12.0,null],[51539606782,"RadarUplink","Player(0)",[9.008344650268555,0.0,-23.600000381469727],1.399999976158142,12.0,null],[163208756131,"PowerReactor","Player(0)",[23.549999237060547,0.0,-18.247087478637695],1.4500000476837158,12.0,null],[1494648617835,"TechLab","Player(0)",[21.071468353271484,0.0,-11.356411933898926],1.600000023841858,16.0,null],[691489734441,"OrePurifier","Player(0)",[8.077125549316406,0.0,-12.249017715454102],1.600000023841858,16.0,null],[4294967118,"CommandCenter","Player(0)",[15.0,0.0,-18.0],2.0,20.0,null],[34359737635,"VehicleFactory","Player(0)",[7.001800060272217,0.0,-18.169734954833984],1.5,16.0,null],[1073741822720,"Barracks","Player(0)",[9.207566261291504,0.0,-8.78469467163086],1.4500000476837158,13.0,null],[4294967068,"resource",null,[-17.470190048217773,0.0,-9.429200172424316],0.6000000238418579,null,1000],[4294967066,"resource",null,[-15.503700256347656,0.0,-9.616700172424316],0.6000000238418579,null,1000],[4294967064,"resource",null,[-15.92648983001709,0.0,-7.863399505615234],0.6000000238418579,null,1000],[4294967042,"resource",null,[-16.539369583129883,0.0,9.359600067138672],0.6000000238418579,null,1000],[4294967038,"resource",null,[-11.240099906921387,0.0,8.992000579833984],0.6000000238418579,null,1000],[4294967044,"resource",null,[-15.369400024414062,0.0,8.192100524902344],0.6000000238418579,null,1000],[4294967056,"resource",null,[-10.742899894714355,0.0,-9.74209976196289],0.6000000238418579,null,1000],[4294967040,"resource",null,[-17.417259216308594,0.0,7.847599029541016],0.6000000238418579,null,1000],[4294967052,"resource",null,[15.685001373291016,0.0,9.30270004272461],0.6000000238418579,null,1000],[4294967050,"resource",null,[16.533000946044922,0.0,10.838298797607422],0.6000000238418579,null,1000],[4294967048,"resource",null,[17.759201049804688,0.0,9.200801849365234],0.6000000238418579,null,1000],[4294967046,"resource",null,[11.480201721191406,0.0,9.405498504638672],0.6000000238418579,null,1000],[605590387911,"RoboticsBay","Player(0)",[-23.450000762939453,0.0,-11.87308120727539],1.5499999523162842,14.0,null],[408021892504,"Tank","Player(0)",[-14.184266090393066,0.05000000074505806,-8.86388111114502],0.8999999761581421,10.0,null],[4294967074,"Worker","Player(1)",[-21.33765411376953,0.05000000074505806,-8.938756942749023],0.6000000238418579,6.0,null]]"#;
fn measured_app(endpoint: &str) -> (App, std::collections::BTreeMap<u64, Entity>) {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
        setup
            .player_controllers
            .fill(SkirmishPlayerController::Human);
        setup.player_factions.fill(SkirmishFaction::Alliance);
        setup.map_path = SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == "plain_and_simple")
            .unwrap()
            .godot_path;
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    let entities: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, (With<MatchScopedEntity>, With<Selectable>)>()
        .iter(app.world())
        .collect();
    for entity in entities {
        app.world_mut().despawn(entity);
    }
    *app.world_mut().resource_mut::<TerrainHeightField>() = TerrainHeightField::default();
    assert_eq!(app.world().resource::<MapBounds>().half_width, 25.0);
    assert_eq!(app.world().resource::<MapBounds>().half_depth, 25.0);
    let rows: Vec<(
        u64,
        String,
        Option<String>,
        [f32; 3],
        f32,
        Option<f32>,
        Option<i32>,
    )> = serde_json::from_str(endpoint).unwrap();
    let mut keys = std::collections::BTreeMap::new();
    for (key, id, team, position, radius, health, resource) in rows {
        let team = if team.as_deref() == Some("Player(0)") {
            Team::Player(0)
        } else {
            Team::Player(1)
        };
        let mut actor = app.world_mut().spawn((
            MatchScopedEntity,
            Transform::from_translation(Vec3::from_array(position)),
            Selectable { radius },
            team,
        ));
        if id == "wall" {
            actor.insert(TerrainWall);
        } else if id == "resource" {
            actor.insert(ResourceNode {
                kind: ResourceKind::Ore,
                amount: resource.unwrap(),
            });
        } else {
            let def = registry::entity(&id).unwrap();
            actor.insert((Health::new(health.unwrap()), MovementDomain::Terrain));
            if matches!(def.role, registry::EntityRole::Unit) {
                actor.insert(Unit {
                    id: def.id,
                    speed: def.speed,
                    can_crush: def.can_crush,
                    can_be_crushed: def.can_be_crushed,
                });
                if def.domain == registry::MoveDomain::Air {
                    actor.insert(MovementDomain::Air);
                }
            } else {
                actor.insert(Structure { id: def.id });
            }
        }
        keys.insert(key, actor.id());
    }
    (app, keys)
}

#[test]
fn measured_resource_pursuit_tank_progresses_without_entering_any_static_body() {
    let (mut app, keys) = measured_app(ENDPOINT);
    let tank = keys[&408021892504];
    let target = keys[&4294967074];
    let start = app.world().get::<Transform>(tank).unwrap().translation;
    let goal = app.world().get::<Transform>(target).unwrap().translation;
    app.world_mut().entity_mut(tank).insert((
        MoveOrder { target: goal },
        PlannedPath {
            goal,
            waypoints: vec![Vec3::new(-19.75, 0.0, -8.25), Vec3::new(-21.25, 0.0, -8.75)],
            next: 0,
        },
    ));
    let obstacles: Vec<_> = app
        .world_mut()
        .query_filtered::<(&Transform, &Selectable, Option<&Structure>), (
            Without<Unit>,
            Or<(With<ResourceNode>, With<Structure>, With<TerrainWall>)>,
        )>()
        .iter(app.world())
        .map(|(t, s, st)| MovementObstacleSnapshot {
            position: t.translation,
            radius: s.radius,
            is_structure: st.is_some(),
        })
        .collect();
    let mut previous = start;
    for tick in 0..90 {
        app.update();
        let p = app.world().get::<Transform>(tank).unwrap().translation;
        assert!(
            static_movement_segment_clear(previous, p, 0.9, &obstacles),
            "illegal body entry at tick{tick}: {previous:?}->{p:?}"
        );
        assert!(
            !app.world()
                .resource::<TerrainHeightField>()
                .step_blocked(previous, p)
        );
        assert!(
            xz_distance(previous, p) <= 2.75 / 30.0 + 0.001,
            "movement teleported"
        );
        previous = p;
    }
    println!(
        "measured pursuit90ticks start{start:?} end{previous:?} displacement{} targetdistance{}->{}",
        xz_distance(start, previous),
        xz_distance(start, goal),
        xz_distance(previous, goal)
    );
    assert!(
        xz_distance(start, previous) > 1.0,
        "actual case0101 Tank remains stalled: {start:?}->{previous:?}"
    );
    assert!(
        xz_distance(previous, goal) < xz_distance(start, goal),
        "pursuit did not close actual target distance"
    );
}

#[test]
fn physical_reroute_cannot_escape_an_existing_resource_body_overlap() {
    let obstacles = [MovementObstacleSnapshot {
        position: Vec3::ZERO,
        radius: 1.0,
        is_structure: false,
    }];
    assert!(
        physical_movement_path(
            Vec3::X * 0.7,
            Vec3::X * 4.0,
            0.6,
            &obstacles,
            &TerrainHeightField::default(),
            MapBounds::from_size((10.0, 10.0))
        )
        .is_none()
    );
}

#[test]
fn physical_reroute_cannot_cross_a_solid_wall_or_leave_map_bounds() {
    let obstacles: Vec<_> = (-8..=8)
        .map(|z| MovementObstacleSnapshot {
            position: Vec3::new(0.0, 0.0, z as f32),
            radius: 0.8,
            is_structure: false,
        })
        .collect();
    assert!(
        physical_movement_path(
            Vec3::new(-4.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            0.9,
            &obstacles,
            &TerrainHeightField::default(),
            MapBounds::from_size((10.0, 10.0))
        )
        .is_none()
    );
}

#[test]
fn physical_reroute_cannot_cross_a_real_cliff_without_a_ramp() {
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
    assert!(
        physical_movement_path(
            Vec3::new(-4.0, 0.05, 0.0),
            Vec3::new(4.0, 1.4, 0.0),
            0.9,
            &[],
            &terrain,
            MapBounds::from_map(&map)
        )
        .is_none()
    );
}
