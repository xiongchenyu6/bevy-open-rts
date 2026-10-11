//! Prepared diagnostic/RED fixture only. Not registered or compiled yet.
//! Insert as a test child of orders::contact_planning when compiler is released.
use super::*;

fn normal_case_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
        setup.map_path = SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == "big_arena")
            .unwrap()
            .godot_path;
        setup.starting_resources = StartingResources::new(32, 16);
        setup.startup_loadout = StartupLoadoutMode::GodotSkirmish;
        setup.victory_condition = VictoryCondition::Annihilation;
        setup.visible_player = VisiblePlayer::all_players(Team::Player(0));
        setup.active_teams = vec![false; MAX_SKIRMISH_LOBBY_SLOTS];
        setup.player_controllers = vec![SkirmishPlayerController::None; MAX_SKIRMISH_LOBBY_SLOTS];
        setup.player_spawn_slots = default_player_spawn_slots();
        for (player, slot, difficulty) in
            [(0, 6, AiDifficulty::Easy), (1, 1, AiDifficulty::Beginner)]
        {
            setup.active_teams[player] = true;
            setup.player_controllers[player] = SkirmishPlayerController::Ai(difficulty);
            setup.player_factions[player] = SkirmishFaction::Chaos;
            setup.player_spawn_slots[player] = slot;
            setup
                .ai_difficulties
                .set_difficulty(Team::Player(player), difficulty);
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<AppScreen>>().get(),
        AppScreen::InMatch
    );
    app
}

#[test]
fn recorded_big_arena_2641_constructor_contact_route_is_reachable() {
    // Actual2641 normal140s circle snapshot SHA e34bae879b41e0e7f034dc60630a29e88c6311dc2f046c4147cffa99792a5277
    let input: serde_json::Value = serde_json::from_str(r#"{"sample":{"construction_approach_decisions":[{"static_obstacles":[{"position":[-10.520679473876953,0.0,-37.778358459472656],"radius":1.5},{"position":[-12.992353439331055,0.0,35.42076873779297],"radius":1.5},{"position":[-15.878107070922852,0.0,-33.055294036865234],"radius":1.4500000476837158},{"position":[-8.021577835083008,0.0,40.5491943359375],"radius":1.4500000476837158},{"position":[-18.67932891845703,0.0,-43.38563919067383],"radius":1.399999976158142},{"position":[-11.656465530395508,0.0,43.71763229370117],"radius":1.399999976158142},{"position":[-12.214849472045898,0.0,-46.42206573486328],"radius":1.5499999523162842},{"position":[-14.396018981933594,0.0,48.45000076293945],"radius":1.5499999523162842},{"position":[-23.998214721679688,0.0,-39.82072067260742],"radius":1.600000023841858},{"position":[-14.754180908203125,0.0,31.00335693359375],"radius":1.4500000476837158},{"position":[-6.025001525878906,0.0,-39.329627990722656],"radius":1.4500000476837158},{"position":[-10.398225784301758,0.0,38.044578552246094],"radius":1.600000023841858},{"position":[-13.264057159423828,0.0,-35.3110237121582],"radius":1.600000023841858},{"position":[-19.765718460083008,0.0,41.51258850097656],"radius":1.600000023841858},{"position":[-17.751911163330078,0.0,48.54999923706055],"radius":1.4500000476837158},{"position":[-12.0,0.0,6.0],"radius":1.649999976158142},{"position":[12.0,0.0,-6.0],"radius":1.649999976158142},{"position":[0.0,0.0,-16.0],"radius":1.4500000476837158},{"position":[16.0,0.0,0.0],"radius":1.4500000476837158},{"position":[0.0,0.0,16.0],"radius":1.4500000476837158},{"position":[-16.0,0.0,0.0],"radius":1.4500000476837158},{"position":[-12.0,0.0,-6.0],"radius":1.7999999523162842},{"position":[12.0,0.0,6.0],"radius":1.7999999523162842},{"position":[6.0,0.0,-12.0],"radius":1.5499999523162842},{"position":[-6.0,0.0,12.0],"radius":1.5499999523162842},{"position":[-15.0,0.0,40.0],"radius":2.0},{"position":[-15.0,0.0,-40.0],"radius":2.0},{"position":[-18.995868682861328,0.0,32.04745101928711],"radius":1.5},{"position":[-11.324620246887207,0.0,32.105255126953125],"radius":1.4500000476837158},{"position":[-6.0,0.0,-12.0],"radius":1.149999976158142},{"position":[12.0,0.0,-12.0],"radius":1.149999976158142},{"position":[6.0,0.0,12.0],"radius":1.149999976158142},{"position":[-12.0,0.0,12.0],"radius":1.149999976158142},{"position":[-36.184600830078125,0.0,-26.431900024414062],"radius":0.6000000238418579},{"position":[-37.655799865722656,0.0,-28.613300323486328],"radius":0.6000000238418579},{"position":[-40.48155212402344,0.0,-27.790599822998047],"radius":0.6000000238418579},{"position":[-46.39944839477539,0.0,-26.628799438476562],"radius":0.6000000238418579},{"position":[-22.505599975585938,0.0,-46.538658142089844],"radius":0.6000000238418579},{"position":[-24.609100341796875,0.0,-44.95811080932617],"radius":0.6000000238418579},{"position":[-23.643400192260742,0.0,-42.17802047729492],"radius":0.6000000238418579},{"position":[-35.39099884033203,0.0,24.718399047851562],"radius":0.6000000238418579},{"position":[23.35260009765625,0.0,-37.2947998046875],"radius":0.6000000238418579},{"position":[25.277297973632812,0.0,-39.08879852294922],"radius":0.6000000238418579},{"position":[24.023101806640625,0.0,-41.75127029418945],"radius":0.6000000238418579},{"position":[21.950698852539062,0.0,-47.41489028930664],"radius":0.6000000238418579},{"position":[44.88690185546875,0.0,-24.64579963684082],"radius":0.6000000238418579},{"position":[43.367897033691406,0.0,-26.794099807739258],"radius":0.6000000238418579},{"position":[40.560997009277344,0.0,-25.90920066833496],"radius":0.6000000238418579},{"position":[34.670196533203125,0.0,-24.616899490356445],"radius":0.6000000238418579},{"position":[34.742103576660156,0.0,23.747703552246094],"radius":0.6000000238418579},{"position":[36.058998107910156,0.0,26.025596618652344],"radius":0.6000000238418579},{"position":[38.934898376464844,0.0,25.400299072265625],"radius":0.6000000238418579},{"position":[44.91889953613281,0.0,24.650497436523438],"radius":0.6000000238418579},{"position":[21.285598754882812,0.0,44.484703063964844],"radius":0.6000000238418579},{"position":[23.29180145263672,0.0,42.78230285644531],"radius":0.6000000238418579},{"position":[22.163002014160156,0.0,40.064300537109375],"radius":0.6000000238418579},{"position":[20.356796264648438,0.0,34.310302734375],"radius":0.6000000238418579},{"position":[-20.858600616455078,0.0,35.051300048828125],"radius":0.6000000238418579},{"position":[-22.97010040283203,0.0,36.62110137939453],"radius":0.6000000238418579},{"position":[-22.01849937438965,0.0,39.406097412109375],"radius":0.6000000238418579},{"position":[-20.586200714111328,0.0,45.26439666748047],"radius":0.6000000238418579},{"position":[-45.607521057128906,0.0,24.78900146484375],"radius":0.6000000238418579},{"position":[-44.0797004699707,0.0,26.93109893798828],"radius":0.6000000238418579},{"position":[-41.27648162841797,0.0,26.03479766845703],"radius":0.6000000238418579},{"position":[-7.296714782714844,0.0,31.906827926635742],"radius":1.5}],"occupied":[{"position":[-21.151941299438477,0.05000000074505806,-35.27616882324219],"radius":0.6000000238418579},{"position":[-27.133108139038086,0.05000000074505806,-34.84282302856445],"radius":0.6000000238418579},{"position":[-27.94801139831543,0.05000000074505806,-32.8852424621582],"radius":0.6000000238418579},{"position":[-19.187519073486328,0.05000000074505806,-46.13444519042969],"radius":0.6000000238418579},{"position":[-19.199050903320312,0.05000000074505806,44.25886535644531],"radius":0.6000000238418579},{"position":[-18.940305709838867,0.05000000074505806,45.64404296875],"radius":0.6000000238418579},{"position":[-21.39569664001465,0.05000000074505806,32.07613754272461],"radius":0.550000011920929},{"position":[-7.878796577453613,0.05000000074505806,-0.8090735673904419],"radius":0.5},{"position":[-7.981892108917236,0.05000000074505806,0.28608438372612],"radius":0.550000011920929}]}],"constructors":[{"entity":12884901113,"move_target":null,"path_goal":null,"position":[-19.18723487854004,0.05000000074505806,35.14212417602539],"radius":0.6000000238418579,"remaining_waypoints":null,"target":1816771165049,"target_structure":"AircraftFactory","target_under_construction":{"remaining":1.0,"total":1.0}}],"pending_foundations":[{"entity":1816771165049,"health":1.0,"position":[-7.296714782714844,0.0,31.906827926635742],"remaining":1.0,"structure":"AircraftFactory","team":0,"total":1.0}]},"witness":{"physical_waypoint_witness":[[-19.18723487854004,35.14212417602539],[-19.875,36.125],[-20.125,36.375],[-20.375,36.375],[-20.625,36.625],[-20.875,36.625],[-21.125,36.875],[-21.375,37.125],[-21.625,37.375],[-21.875,37.625],[-22.125,37.875],[-22.375,37.875],[-22.625,38.125],[-22.875,38.125],[-23.125,38.125],[-23.375,38.125],[-23.625,37.875],[-23.875,37.875],[-24.125,37.625],[-24.375,37.375],[-24.625,37.125],[-24.875,36.875],[-25.125,36.625],[-25.375,36.375],[-25.625,36.125],[-25.875,35.875],[-26.125,35.625],[-26.375,35.375],[-26.375,35.125],[-26.125,34.875],[-25.875,34.625],[-25.625,34.375],[-25.375,34.125],[-25.125,33.875],[-24.875,33.625],[-24.625,33.375],[-24.375,33.125],[-24.125,32.875],[-23.875,32.625],[-23.625,32.375],[-23.375,32.125],[-23.125,31.875],[-22.875,31.625],[-22.625,31.375],[-22.375,31.125],[-22.125,30.875],[-21.875,30.625],[-21.625,30.375],[-21.375,30.125],[-21.125,29.875],[-20.875,29.625],[-20.625,29.375],[-20.375,29.125],[-20.125,28.875],[-19.875,28.625],[-19.625,28.375],[-19.375,28.125],[-19.125,27.875],[-18.875,27.625],[-18.625,27.375],[-18.375,27.125],[-18.125,26.875],[-17.875,26.625],[-17.625,26.375],[-17.375,26.125],[-17.125,25.875],[-16.875,25.625],[-16.625,25.375],[-16.375,25.125],[-16.125,24.875],[-15.875,24.625],[-15.625,24.625],[-15.375,24.875],[-15.125,25.125],[-14.875,25.375],[-14.625,25.625],[-14.375,25.875],[-14.125,26.125],[-13.875,26.375],[-13.625,26.625],[-13.375,26.875],[-13.125,27.125],[-12.875,27.375],[-12.625,27.625],[-12.375,27.875],[-12.125,28.125],[-11.875,28.375],[-11.625,28.625],[-11.375,28.875],[-11.125,29.125],[-10.875,29.375],[-10.625,29.625],[-10.375,29.875],[-10.125,30.125],[-9.875,30.375],[-9.625,30.625],[-9.413325817150895,30.775475758253346]]}}"#).unwrap();
    let row = &input["sample"];
    let decision = &row["construction_approach_decisions"][0];
    let point = |v: &serde_json::Value| {
        Vec3::new(
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
        )
    };
    let circles = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|o| (point(&o["position"]), o["radius"].as_f64().unwrap() as f32))
            .collect::<Vec<_>>()
    };
    let source_position = point(&row["constructors"][0]["position"]);
    let source_radius = row["constructors"][0]["radius"].as_f64().unwrap() as f32;
    let target_position = point(&row["pending_foundations"][0]["position"]);
    let target_radius = 1.5;
    let obstacle_data = circles(&decision["static_obstacles"]);
    let obstacles = obstacle_data.as_slice();
    let occupied_data = circles(&decision["occupied"]);
    let occupied = occupied_data.as_slice();
    // No body/padding overlap exception is needed anywhere on the witness.
    let buildings: &[(Vec3, f32)] = &[];
    assert!(contact_position_is_clear(
        source_position,
        source_radius,
        &obstacles,
        &occupied
    ));
    let app = normal_case_app();
    let terrain = app.world().resource::<TerrainHeightField>();
    let bounds = *app.world().resource::<MapBounds>();
    assert_eq!((bounds.half_width, bounds.half_depth), (50.0, 50.0));
    assert!(terrain.is_flat(), "Use actual authored BigArena terrain");
    let mut grid = NavGrid::default();
    grid.rebuild_with_terrain(bounds, &obstacles, terrain);
    let grid = &grid;
    let witness = input["witness"]["physical_waypoint_witness"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            Vec3::new(
                p[0].as_f64().unwrap() as f32,
                source_position.y,
                p[1].as_f64().unwrap() as f32,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(witness[0], source_position);
    for leg in witness.windows(2) {
        assert!(
            leg.iter()
                .all(|p| p.x.abs() <= bounds.half_width && p.z.abs() <= bounds.half_depth)
        );
        assert!(grid.terrain_line_clear(leg[0], leg[1]));
        assert!(
            contact_escape_segment_is_clear(
                leg[0],
                leg[1],
                source_radius,
                &obstacles,
                &buildings,
                &occupied,
                false
            ),
            "Witness cannot tunnel or use an overlap exemption: {leg:?}"
        );
    }
    assert!(move_order_targets_contact(
        Some(&MoveOrder {
            target: *witness.last().unwrap()
        }),
        target_position,
        source_radius,
        target_radius
    ));
    let actual = reachable_unit_contact_approach(
        source_position,
        source_radius,
        target_position,
        target_radius,
        grid,
        &obstacles,
        &buildings,
        &occupied,
    );
    assert!(
        actual.is_some(),
        "Actual terrain/physical witness exists; shared contact planner must find a continuous safe route"
    );
    let actual = actual.unwrap();
    let mut anchor = source_position;
    assert!(
        actual.waypoints.len() > 2,
        "Keep real prefix and detour waypoints"
    );
    for point in actual.waypoints {
        assert!(grid.terrain_line_clear(anchor, point));
        assert!(contact_escape_segment_is_clear(
            anchor,
            point,
            source_radius,
            obstacles,
            buildings,
            occupied,
            false
        ));
        anchor = point;
    }
    let mut blocked = NavGrid::default();
    blocked.rebuild_with_terrain(bounds, obstacles, terrain);
    blocked.terrain_blocked.fill(true);
    blocked.blocked.fill(true);
    assert!(
        reachable_unit_contact_approach(
            source_position,
            source_radius,
            target_position,
            target_radius,
            &blocked,
            obstacles,
            buildings,
            occupied
        )
        .is_none(),
        "Actual terrain wall must retain safe no-route behavior"
    );
}

#[test]
fn normal_paid_big_arena_factory_constructor_recovers_with_continuous_movement() {
    let mut app = normal_case_app();
    let mut paid = false;
    let mut builder = None;
    let mut site_entity = None;
    let mut completed = false;
    for tick in 0..6060 {
        assert!(
            app.world().resource::<MatchState>().is_running(),
            "Actual normal match must remain running, tick {tick}"
        );
        let previous = builder.and_then(|e| app.world().get::<Transform>(e).map(|t| t.translation));
        app.update();
        let mut sites = app
            .world_mut()
            .query::<(Entity, &Team, &Structure, Option<&UnderConstruction>)>();
        if let Some((entity, _, _, construction)) = sites
            .iter(app.world())
            .find(|(_, team, s, _)| **team == Team::Player(0) && s.id == "AircraftFactory")
        {
            site_entity = Some(entity);
            if let Some(site) = construction {
                let def = registry::entity("AircraftFactory").unwrap();
                assert_eq!(
                    (site.cost.ore, site.cost.crystal),
                    (def.cost.ore, def.cost.crystal)
                );
                paid = true;
            } else if paid {
                completed = true;
            }
        }
        if let Some(target) = site_entity {
            let mut constructors = app
                .world_mut()
                .query::<(Entity, &Team, &Unit, &ConstructOrder)>();
            if let Some((entity, _, _, _)) =
                constructors
                    .iter(app.world())
                    .find(|(_, team, unit, order)| {
                        **team == Team::Player(0) && unit.id == "Worker" && order.target == target
                    })
            {
                builder = Some(entity);
            }
        }
        if let (Some(entity), Some(previous)) = (builder, previous) {
            let current = app.world().get::<Transform>(entity).unwrap().translation;
            let unit = app.world().get::<Unit>(entity).unwrap();
            let radius = app.world().get::<Selectable>(entity).unwrap().radius;
            assert!(
                xz_distance(previous, current)
                    <= (unit.speed + UNIT_SEPARATION_MAX_SPEED) / 30.0 + 0.005,
                "No teleport beyond actual movement and separation speed caps"
            );
            let mut statics = app.world_mut().query_filtered::<(
                &Transform,
                &Selectable,
                Option<&Health>,
                Option<&Structure>,
            ), (
                Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
                Without<Unit>,
            )>();
            let obstacles = statics
                .iter(app.world())
                .filter(|(_, _, hp, _)| !hp.is_some_and(|hp| hp.current <= 0.0))
                .map(
                    |(t, s, _, structure)| crate::combat::MovementObstacleSnapshot {
                        position: t.translation,
                        radius: s.radius,
                        is_structure: structure.is_some(),
                    },
                )
                .collect::<Vec<_>>();
            assert!(
                crate::combat::static_movement_segment_clear(previous, current, radius, &obstacles),
                "Every actual constructor step preserves physical obstacle avoidance"
            );
            assert!(
                !app.world()
                    .resource::<TerrainHeightField>()
                    .step_blocked(previous, current)
            );
        }
        if completed {
            break;
        }
        if app.world().resource::<MatchState>().start_time_sec >= 200.0 {
            break;
        }
    }
    eprintln!(
        "2641 normal paidFactory recovered={completed}, paid={paid}, clock{}",
        app.world().resource::<MatchState>().start_time_sec
    );
    assert!(paid, "Actual normal economy must pay for the Factory");
    assert!(
        completed,
        "Same naturally assigned constructor must finish the paid Factory"
    );
}
