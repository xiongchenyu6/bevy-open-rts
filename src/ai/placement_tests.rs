//! Real paid AI births and spatial failure/cap contracts; no match grants.
use super::*;
use bevy::ecs::system::{RunSystemOnce, SystemState};

fn normal_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings.map_path = "res://source/match/maps/PlainAndSimple.tscn";
        settings.starting_resources = StartingResources::new(32, 16);
        settings.startup_loadout = StartupLoadoutMode::GodotSkirmish;
        settings.victory_condition = VictoryCondition::Annihilation;
        settings.visible_player = VisiblePlayer::all_players(Team::Player(0));
        for (player, difficulty) in [(0, AiDifficulty::Normal), (1, AiDifficulty::Easy)] {
            settings.player_controllers[player] = SkirmishPlayerController::Ai(difficulty);
            settings.player_factions[player] = SkirmishFaction::Alliance;
            settings.player_spawn_slots[player] = player;
            settings
                .ai_difficulties
                .set_difficulty(Team::Player(player), difficulty);
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    app
}

#[test]
fn exact_0012_paid_ai_armed_births_clear_resource_bodies() {
    let mut app = normal_app();
    let mut seen = {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<Unit>>();
        query.iter(world).collect::<std::collections::HashSet<_>>()
    };
    let mut births = 0;
    for _ in 0..18000 {
        app.update();
        let world = app.world_mut();
        let clock = world.resource::<MatchState>().start_time_sec;
        let mut nodes = world.query_filtered::<(&Transform, &Selectable), With<ResourceNode>>();
        let nodes = nodes
            .iter(world)
            .map(|(p, r)| (p.translation, r.radius))
            .collect::<Vec<_>>();
        let mut units = world.query::<(
            Entity,
            &Unit,
            &Transform,
            &Selectable,
            &MovementDomain,
            Option<&Weapon>,
        )>();
        for (entity, unit, position, radius, domain, weapon) in units.iter(world) {
            if !seen.insert(entity) || *domain != MovementDomain::Terrain || weapon.is_none() {
                continue;
            }
            births += 1;
            for (node, node_radius) in &nodes {
                assert!(
                    xz_distance(position.translation, *node) + 1e-4 >= radius.radius + node_radius,
                    "paid AI armed birth inside resource: clock={clock} id={} entity={entity:?} position={:?} node={node:?} distance={} physical={}",
                    unit.id,
                    position.translation,
                    xz_distance(position.translation, *node),
                    radius.radius + node_radius
                );
            }
        }
        if !matches!(world.resource::<MatchState>().phase, MatchPhase::Running) {
            break;
        }
    }
    assert!(
        births > 0,
        "normal match must actually produce paid armed units"
    );
}

#[test]
fn blocked_ai_training_keeps_bank_and_does_not_spawn() {
    let mut app = normal_app();
    // A spatial-only fixture closes every legal spawn in the normal map. No
    // resources, unit stats or game-time budgets are injected.
    app.world_mut().spawn((
        TerrainWall,
        Transform::default(),
        Selectable { radius: 100.0 },
    ));
    let before = app
        .world()
        .resource::<Economies>()
        .get(Team::Player(1))
        .clone();
    let before_id = app.world().resource::<NextSpawnId>().0;
    let result = app
        .world_mut()
        .run_system_once(
            |mut commands: Commands,
             assets: Res<AssetServer>,
             mut economies: ResMut<Economies>,
             mut next_id: ResMut<NextSpawnId>,
             structures: Query<StructurePrereqItem<'_>>,
             bounds: Res<MapBounds>,
             occupiers: Query<
                (Entity, &Transform, &Selectable, Option<&Health>),
                Or<(
                    With<Unit>,
                    With<Structure>,
                    With<ResourceNode>,
                    With<TerrainWall>,
                )>,
            >| {
                let mut occupied = production_occupied_spawn_points(&occupiers);
                try_spawn_ai_trained_unit(
                    &mut commands,
                    &assets,
                    &mut economies,
                    &mut next_id,
                    Team::Player(1),
                    SkirmishFaction::Alliance,
                    "Worker",
                    &structures,
                    *bounds,
                    Team::Player(0),
                    &mut occupied,
                )
            },
        )
        .unwrap();
    assert!(
        !result,
        "training must not succeed without a physical spawn position"
    );
    let after = app.world().resource::<Economies>().get(Team::Player(1));
    assert_eq!((after.ore, after.crystal), (before.ore, before.crystal));
    assert_eq!(app.world().resource::<NextSpawnId>().0, before_id);
}

#[test]
fn pending_defenses_use_the_existing_normal_profile_cap() {
    let mut app = App::new();
    let profile =
        faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Normal);
    let limit = ai_structure_profile_limit("AntiGroundTurret", &profile);
    assert!(limit > 0);
    for index in 0..limit {
        app.world_mut().spawn((
            Structure {
                id: "AntiGroundTurret",
            },
            Team::Player(0),
            Transform::from_xyz(index as f32 * 3.0, 0.0, 0.0),
            UnderConstruction {
                remaining: 1.0,
                total: 1.0,
                cost: registry::entity("AntiGroundTurret").unwrap().cost,
                free_worker_origin: None,
            },
        ));
    }
    let mut state = SystemState::<Query<StructurePrereqItem<'_>>>::new(app.world_mut());
    let query = state.get(app.world()).unwrap();
    assert!(
        !ai_structure_under_profile_limit(Team::Player(0), "AntiGroundTurret", &query, &profile),
        "already purchased foundations must occupy the unchanged defense cap"
    );
}

#[test]
fn same_frame_ai_spawns_reserve_the_only_slot_and_charge_only_success() {
    let mut app = normal_app();
    let before = app
        .world()
        .resource::<Economies>()
        .get(Team::Player(1))
        .clone();
    let before_id = app.world().resource::<NextSpawnId>().0;
    let outcomes = app
        .world_mut()
        .run_system_once(
            |mut commands: Commands,
             assets: Res<AssetServer>,
             mut economies: ResMut<Economies>,
             mut next_id: ResMut<NextSpawnId>,
             structures: Query<StructurePrereqItem<'_>>,
             bounds: Res<MapBounds>,
             occupiers: Query<
                (Entity, &Transform, &Selectable, Option<&Health>),
                Or<(
                    With<Unit>,
                    With<Structure>,
                    With<ResourceNode>,
                    With<TerrainWall>,
                )>,
            >| {
                let def = registry::entity("Worker").unwrap();
                let (_, origin) = ai_production_origin_for_faction(
                    Team::Player(1),
                    SkirmishFaction::Alliance,
                    "Worker",
                    &structures,
                )
                .unwrap();
                let mut occupied = production_occupied_spawn_points(&occupiers);
                let seed = next_id.0;
                let slot = find_production_spawn_position(
                    origin,
                    "CommandCenter",
                    def.radius,
                    seed,
                    &occupied,
                    *bounds,
                )
                .unwrap();
                // Four large physical circles leave a small valid pocket. Two
                // unreserved attempts can reach it, but two Worker bodies cannot fit.
                for direction in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
                    occupied.push((slot + direction * 100.0, 100.0 - def.radius - 0.05 - 0.16));
                }
                let alternative = find_production_spawn_position(
                    origin,
                    "CommandCenter",
                    def.radius,
                    seed + 1,
                    &occupied,
                    *bounds,
                )
                .expect("second unreserved attempt reaches the same pocket");
                assert!(xz_distance(slot, alternative) < def.radius * 2.0);
                let first = try_spawn_ai_trained_unit(
                    &mut commands,
                    &assets,
                    &mut economies,
                    &mut next_id,
                    Team::Player(1),
                    SkirmishFaction::Alliance,
                    "Worker",
                    &structures,
                    *bounds,
                    Team::Player(0),
                    &mut occupied,
                );
                let second = try_spawn_ai_trained_unit(
                    &mut commands,
                    &assets,
                    &mut economies,
                    &mut next_id,
                    Team::Player(1),
                    SkirmishFaction::Alliance,
                    "Worker",
                    &structures,
                    *bounds,
                    Team::Player(0),
                    &mut occupied,
                );
                (first, second)
            },
        )
        .unwrap();
    assert_eq!(outcomes, (true, false));
    let cost = faction_unit_cost(
        Some(SkirmishFaction::Alliance),
        registry::entity("Worker").unwrap().cost,
    );
    let after = app.world().resource::<Economies>().get(Team::Player(1));
    assert_eq!(
        (after.ore, after.crystal),
        (before.ore - cost.ore, before.crystal - cost.crystal)
    );
    assert_eq!(app.world().resource::<NextSpawnId>().0, before_id + 1);
}

#[test]
fn ai_structure_picks_reject_all_real_blocked_sites() {
    let mut app = normal_app();
    let mut state =
        SystemState::<Query<(Entity, &Team, &Transform), With<Health>>>::new(app.world_mut());
    let targets = state.get(app.world()).unwrap();
    let origin = targets
        .iter()
        .find(|(_, team, position)| {
            **team == Team::Player(0) && position.translation == Vec3::new(-15.0, 0.0, -18.0)
        })
        .unwrap()
        .2
        .translation;
    let blocked = [(Vec3::ZERO, 100.0)];
    assert!(
        ai_structure_build_position(
            Team::Player(0),
            origin,
            "AntiGroundTurret",
            AiStructureBuildKind::Defense,
            19,
            &targets,
            *app.world().resource::<MapBounds>(),
            app.world().resource::<TerrainHeightField>(),
            &blocked
        )
        .is_none()
    );
}

#[test]
fn six_ai_economy_picks_escape_a_blocked_sector_without_extra_attempts() {
    let mut app = normal_app();
    let mut state =
        SystemState::<Query<(Entity, &Team, &Transform), With<Health>>>::new(app.world_mut());
    let targets = state.get(app.world()).unwrap();
    let origin = Vec3::new(-15.0, 0.0, -18.0);
    let seed = 19;
    let bounds = *app.world().resource::<MapBounds>();
    let first = ai_structure_build_position_unchecked(
        Team::Player(0),
        origin,
        "Refinery",
        AiStructureBuildKind::Economy,
        seed,
        &targets,
        bounds,
    );
    let occupied = [(first, 0.6), (origin, 2.0)];
    let radius = registry::entity("Refinery").unwrap().radius;
    // The original stride aliases the golden-angle factor to only 3.36°.
    // One normal resource-sized footprint blocks all six former attempts.
    for attempt in 0..6 {
        let candidate = ai_structure_build_position_unchecked(
            Team::Player(0),
            origin,
            "Refinery",
            AiStructureBuildKind::Economy,
            seed + attempt * 101,
            &targets,
            bounds,
        );
        assert!(!is_spawn_position_free(
            candidate, radius, &occupied, bounds
        ));
    }
    let site = ai_structure_build_position(
        Team::Player(0),
        origin,
        "Refinery",
        AiStructureBuildKind::Economy,
        seed,
        &targets,
        bounds,
        app.world().resource::<TerrainHeightField>(),
        &occupied,
    )
    .expect("six bounded picks must cover other legal sectors of the existing ring");
    assert!(is_spawn_position_free(site, radius, &occupied, bounds));
    assert!(terrain_site_is_buildable(
        app.world().resource::<TerrainHeightField>(),
        site,
        radius
    ));
}

#[test]
fn recorded_41_second_radar_request_finds_the_legal_sixth_uniform_pick() {
    let mut app = App::new();
    let mut state =
        SystemState::<Query<(Entity, &Team, &Transform), With<Health>>>::new(app.world_mut());
    let targets = state.get(app.world()).unwrap();
    let map = SKIRMISH_MAPS.iter().find(|m| m.id == "big_arena").unwrap();
    let origin = Vec3::new(40.0, 0.0, 20.0);
    let bounds = MapBounds {
        half_width: 50.0,
        half_depth: 50.0,
    };
    // Exact live41.033337s snapshot blockers from diagnostic09, plus all
    // authored static resources, HQ and completed Factory. No world injection.
    let mut occupied = vec![
        (Vec3::new(34.742104, 0.0, 23.747704), 0.6),
        (Vec3::new(35.110890, 0.0, 22.142815), 0.6),
        (Vec3::new(35.750237, 0.0, 17.365707), 1.45),
        (Vec3::new(36.168830, 0.0, 22.808586), 0.6),
        (Vec3::new(36.408100, 0.0, 24.035471), 0.6),
        (Vec3::new(42.007645, 0.0, 15.420769), 1.5),
        (Vec3::new(43.813328, 0.0, 23.396313), 0.6),
        (Vec3::new(44.918900, 0.0, 24.650497), 0.6),
        (origin, 2.0),
        (Vec3::new(29.411005, 0.0, 23.565561), 1.5),
    ];
    occupied.extend(
        map.resources
            .iter()
            .map(|r| (map_local_to_world(map, r.position), 0.6)),
    );
    let radius = registry::entity("RadarUplink").unwrap().radius;
    for attempt in 0..6 {
        let old = ai_structure_build_position_unchecked(
            Team::Player(1),
            origin,
            "RadarUplink",
            AiStructureBuildKind::Economy,
            28 + attempt,
            &targets,
            bounds,
        );
        assert!(!is_spawn_position_free(old, radius, &occupied, bounds));
    }
    let phase = (28 + 19) as f32 * 1.618_034;
    for attempt in 0..5 {
        let angle = phase + attempt as f32 * std::f32::consts::TAU / 6.0;
        let point = origin + Vec3::new(angle.cos(), 0.0, angle.sin()) * 5.0;
        assert!(!is_spawn_position_free(point, radius, &occupied, bounds));
    }
    let site = ai_structure_build_position(
        Team::Player(1),
        origin,
        "RadarUplink",
        AiStructureBuildKind::Economy,
        28,
        &targets,
        bounds,
        &TerrainHeightField::default(),
        &occupied,
    )
    .expect("actual six-pick budget contains a legal sixth uniform candidate");
    assert!(xz_distance(site, origin) <= 2.0 + radius + BASE_CONSTRUCTION_RADIUS_M);
    assert!(is_spawn_position_free(site, radius, &occupied, bounds));
}

#[test]
fn recorded_packed_52_second_city_allows_bounded_influence_preserving_expansion() {
    let mut app = App::new();
    let mut state =
        SystemState::<Query<(Entity, &Team, &Transform), With<Health>>>::new(app.world_mut());
    let targets = state.get(app.world()).unwrap();
    let origin = Vec3::new(40.0, 0.0, 20.0);
    let bounds = MapBounds {
        half_width: 50.0,
        half_depth: 50.0,
    };
    let occupied = [
        (Vec3::new(-12.0, 0.0, 6.0), 1.65),
        (Vec3::new(12.0, 0.0, -6.0), 1.65),
        (Vec3::new(-42.0, 0.0, -22.0), 0.6),
        (Vec3::new(-43.0, 0.0, -17.0), 0.6),
        (Vec3::new(-37.0, 0.0, -17.0), 0.6),
        (Vec3::new(42.007645, 0.0, 15.420769), 1.5),
        (Vec3::new(37.431328, 0.0, 15.710255), 1.45),
        (Vec3::new(44.60953, 0.0, 18.062933), 1.4),
        (Vec3::new(-40.0, 0.0, -20.0), 2.0),
        (Vec3::new(40.0, 0.0, 20.0), 2.0),
        (Vec3::new(29.411005, 0.0, 23.565561), 1.5),
        (Vec3::new(-12.0, 0.0, -6.0), 1.8),
        (Vec3::new(12.0, 0.0, 6.0), 1.8),
        (Vec3::new(0.0, 0.0, -16.0), 1.45),
        (Vec3::new(16.0, 0.0, 0.0), 1.45),
        (Vec3::new(0.0, 0.0, 16.0), 1.45),
        (Vec3::new(-16.0, 0.0, 0.0), 1.45),
        (Vec3::new(6.0, 0.0, -12.0), 1.55),
        (Vec3::new(-6.0, 0.0, 12.0), 1.55),
        (Vec3::new(-6.0, 0.0, -12.0), 1.15),
        (Vec3::new(12.0, 0.0, -12.0), 1.15),
        (Vec3::new(6.0, 0.0, 12.0), 1.15),
        (Vec3::new(-12.0, 0.0, 12.0), 1.15),
        (Vec3::new(-36.1846, 0.0, -26.4319), 0.6),
        (Vec3::new(-37.6558, 0.0, -28.6133), 0.6),
        (Vec3::new(-40.481552, 0.0, -27.7906), 0.6),
        (Vec3::new(-46.39945, 0.0, -26.6288), 0.6),
        (Vec3::new(-22.5056, 0.0, -46.53866), 0.6),
        (Vec3::new(-24.6091, 0.0, -44.95811), 0.6),
        (Vec3::new(-23.6434, 0.0, -42.17802), 0.6),
        (Vec3::new(-22.1813, 0.0, -36.3271), 0.6),
        (Vec3::new(23.3526, 0.0, -37.2948), 0.6),
        (Vec3::new(25.277298, 0.0, -39.0888), 0.6),
        (Vec3::new(24.023102, 0.0, -41.75127), 0.6),
        (Vec3::new(21.950699, 0.0, -47.41489), 0.6),
        (Vec3::new(44.8869, 0.0, -24.6458), 0.6),
        (Vec3::new(43.367897, 0.0, -26.7941), 0.6),
        (Vec3::new(40.560997, 0.0, -25.9092), 0.6),
        (Vec3::new(34.670197, 0.0, -24.6169), 0.6),
        (Vec3::new(34.742104, 0.0, 23.747704), 0.6),
        (Vec3::new(36.059, 0.0, 26.025597), 0.6),
        (Vec3::new(38.9349, 0.0, 25.4003), 0.6),
        (Vec3::new(44.9189, 0.0, 24.650497), 0.6),
        (Vec3::new(21.285599, 0.0, 44.484703), 0.6),
        (Vec3::new(23.291801, 0.0, 42.782303), 0.6),
        (Vec3::new(22.163002, 0.0, 40.0643), 0.6),
        (Vec3::new(20.356796, 0.0, 34.310303), 0.6),
        (Vec3::new(-20.8586, 0.0, 35.0513), 0.6),
        (Vec3::new(-22.9701, 0.0, 36.6211), 0.6),
        (Vec3::new(-22.0185, 0.0, 39.406097), 0.6),
        (Vec3::new(-20.5862, 0.0, 45.264397), 0.6),
        (Vec3::new(-45.60752, 0.0, 24.789001), 0.6),
        (Vec3::new(-44.0797, 0.0, 26.931099), 0.6),
        (Vec3::new(-41.27648, 0.0, 26.034798), 0.6),
        (Vec3::new(-35.391, 0.0, 24.7184), 0.6),
        (Vec3::new(44.646336, 0.0, 22.979004), 0.6),
        (Vec3::new(36.42043, 0.0, 24.105253), 0.6),
        (Vec3::new(35.181858, 0.0, 22.142555), 0.6),
        (Vec3::new(36.194126, 0.0, 22.87591), 0.6),
        (Vec3::new(-0.10635927, 0.0, -7.8894815), 0.5),
        (Vec3::new(29.058962, 0.0, 21.292664), 0.45),
        (Vec3::new(31.707064, 0.0, 22.86691), 0.55),
        (Vec3::new(27.059278, 0.0, 24.044495), 0.55),
        (Vec3::new(31.333212, 0.0, 25.116697), 0.62),
    ];
    let radius = registry::entity("RoboticsBay").unwrap().radius;
    let phase = (32 + 19) as f32 * 1.618_034;
    for attempt in 0..6 {
        let angle = phase + attempt as f32 * std::f32::consts::TAU / 6.0;
        let inner = origin + Vec3::new(angle.cos(), 0.0, angle.sin()) * 5.0;
        assert!(!is_spawn_position_free(inner, radius, &occupied, bounds));
    }
    let site = ai_structure_build_position(
        Team::Player(1),
        origin,
        "RoboticsBay",
        AiStructureBuildKind::Economy,
        32,
        &targets,
        bounds,
        &TerrainHeightField::default(),
        &occupied,
    )
    .expect("packed inner city has a real legal expanded base site within six attempts");
    assert!(is_spawn_position_free(site, radius, &occupied, bounds));
    assert!(xz_distance(site, origin) > 5.0);
    assert!(xz_distance(site, origin) <= 2.0 + radius + BASE_CONSTRUCTION_RADIUS_M);
}
