use super::*;

#[test]
fn normal_chaos_builds_paid_technology_and_armed_advanced_army() {
    observe_paid_chaos_progress(None);
    let map = SKIRMISH_MAPS
        .iter()
        .find(|map| map.id == "big_arena")
        .unwrap();
    observe_paid_chaos_progress(Some(map.godot_path));
}

fn observe_paid_chaos_progress(distant_map: Option<&'static str>) {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        if let Some(map_path) = distant_map {
            settings.map_path = map_path;
            settings.player_spawn_slots[1] = 4;
        }
        settings.player_factions[1] = SkirmishFaction::Chaos;
        settings.player_controllers[1] = SkirmishPlayerController::Ai(AiDifficulty::Normal);
        settings
            .ai_difficulties
            .set_difficulty(Team::Player(1), AiDifficulty::Normal);
        assert_eq!(settings.starting_resources, StartingResources::new(32, 16));
        assert_eq!(settings.startup_loadout, StartupLoadoutMode::GodotSkirmish);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    let mut begun = std::collections::BTreeSet::new();
    let mut completed = std::collections::BTreeSet::new();
    let mut advanced = std::collections::BTreeSet::new();
    let mut unarmed_drone_seen = false;
    let mut worker_construction_seen = false;
    let mut worker_contact = std::collections::BTreeSet::new();
    let mut paid = std::collections::BTreeSet::new();
    let technology = ["PowerReactor", "RadarUplink", "RoboticsBay", "TechLab"];
    for _ in 0..20 {
        app.update();
    }
    let initial_units = {
        let world = app.world_mut();
        let mut units = world.query_filtered::<(Entity, &Team), With<Unit>>();
        units
            .iter(world)
            .filter(|(_, team)| **team == Team::Player(1))
            .map(|(entity, _)| entity)
            .collect::<std::collections::HashSet<_>>()
    };
    for tick in 0..5400 {
        let before = {
            let economy = app.world().resource::<Economies>().get(Team::Player(1));
            (economy.ore, economy.crystal)
        };
        app.update();
        let world = app.world_mut();
        let after = {
            let economy = world.resource::<Economies>().get(Team::Player(1));
            (economy.ore, economy.crystal)
        };
        let mut constructors = world.query::<(&Team, &ConstructOrder)>();
        worker_construction_seen |= constructors
            .iter(world)
            .any(|(team, _)| *team == Team::Player(1));
        let mut pulses = world.query::<(&Team, &ConstructionWorkPulse)>();
        for (team, pulse) in pulses.iter(world) {
            if *team == Team::Player(1)
                && let Some(structure) = world.get::<Structure>(pulse.target)
            {
                worker_contact.insert(structure.id);
            }
        }
        let mut structures = world.query::<(&Team, &Structure, Option<&UnderConstruction>)>();
        for (team, structure, construction) in structures.iter(world) {
            if *team != Team::Player(1) {
                continue;
            }
            if let Some(construction) = construction {
                if begun.insert(structure.id) && technology.contains(&structure.id) {
                    let cost = registry::entity(structure.id).unwrap().cost;
                    assert_eq!(construction.cost.ore, cost.ore);
                    assert_eq!(construction.cost.crystal, cost.crystal);
                    assert!(before.0 >= cost.ore && before.1 >= cost.crystal);
                    assert!(
                        after.0 < before.0 || after.1 < before.1,
                        "{} began without a resource debit: {before:?} -> {after:?}",
                        structure.id
                    );
                    paid.insert(structure.id);
                    eprintln!(
                        "[ai-tech-paid] {} cost=({}, {}) resources={before:?}->{after:?}",
                        structure.id, cost.ore, cost.crystal
                    );
                }
            } else {
                completed.insert(structure.id);
            }
        }
        let mut units = world.query::<(Entity, &Team, &Unit, Option<&Weapon>)>();
        for (entity, team, unit, weapon) in units.iter(world) {
            unarmed_drone_seen |= *team == Team::Player(1) && unit.id == "Drone";
            if *team == Team::Player(1)
                && !initial_units.contains(&entity)
                && weapon.is_some()
                && registry::entity(unit.id)
                    .is_some_and(|def| def.requirements.iter().any(|id| technology.contains(id)))
            {
                advanced.insert(unit.id);
            }
        }
        if tick % 900 == 0 {
            eprintln!(
                "[ai-tech] map={distant_map:?} phase={:?} t={} begun={begun:?} completed={completed:?} advanced={advanced:?} economy={:?}",
                world.resource::<MatchState>().phase,
                tick / 30,
                {
                    let e = world.resource::<Economies>().get(Team::Player(1));
                    (e.ore, e.crystal, e.power_used, e.power_capacity)
                }
            );
        }
    }
    assert!(
        worker_construction_seen,
        "technology must use worker construction"
    );
    let phase = app.world().resource::<MatchState>().phase;
    eprintln!(
        "[ai-tech-final] map={distant_map:?} phase={phase:?} completed={completed:?} advanced={advanced:?}"
    );
    for id in technology {
        if id == "TechLab" && distant_map.is_none() && !completed.contains(id) {
            assert!(
                matches!(phase, MatchPhase::HumanDefeat),
                "short map stopped technology progression without AI victory: {phase:?}"
            );
            continue;
        }
        assert!(
            begun.contains(id),
            "{id} never began normal paid construction: {begun:?}"
        );
        assert!(paid.contains(id), "{id} did not debit its normal price");
        assert!(
            worker_contact.contains(id),
            "{id} had no worker contact construction pulse"
        );
        assert!(
            completed.contains(id),
            "{id} never completed: {completed:?}"
        );
    }
    assert!(
        !unarmed_drone_seen,
        "normal army production spammed unarmed scouting drones"
    );
    assert!(
        !advanced.is_empty(),
        "no armed technology-gated unit was trained"
    );
}

#[test]
fn configured_remote_human_keeps_control_while_ai_slot_develops() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings.player_controllers[0] = SkirmishPlayerController::Human;
        settings.player_controllers[1] = SkirmishPlayerController::Human;
        settings.player_controllers[2] = SkirmishPlayerController::Ai(AiDifficulty::Normal);
        settings.active_teams[2] = true;
        settings
            .ai_difficulties
            .set_difficulty(Team::Player(2), AiDifficulty::Normal);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(
        controlled_player_team(Some(app.world().resource::<VisiblePlayer>())),
        Some(Team::Player(0))
    );
    {
        let settings = app.world().resource::<MatchSetupSettings>();
        let active = app.world().resource::<ActiveTeams>();
        assert!(
            active_ai_teams(Some(Team::Player(0)), Some(active))
                .any(|team| team == Team::Player(1)),
            "remote human remains a legitimate enemy target"
        );
        assert_eq!(
            ai_controlled_teams(Some(Team::Player(0)), Some(active), Some(settings))
                .collect::<Vec<_>>(),
            vec![Team::Player(2)]
        );
        assert!(!team_is_ai_controlled(Team::Player(1), None, None));
    }
    let (human_units, ai_units) = {
        let world = app.world_mut();
        let mut units = world.query_filtered::<(Entity, &Team), With<Unit>>();
        (
            units
                .iter(world)
                .filter(|(_, team)| **team == Team::Player(1))
                .map(|(entity, _)| entity)
                .collect::<std::collections::HashSet<_>>(),
            units
                .iter(world)
                .filter(|(_, team)| **team == Team::Player(2))
                .map(|(entity, _)| entity)
                .collect::<std::collections::HashSet<_>>(),
        )
    };
    let initial_human_resources = {
        let economy = app.world().resource::<Economies>().get(Team::Player(1));
        (economy.ore, economy.crystal)
    };
    for _ in 0..1350 {
        app.update();
        let world = app.world_mut();
        let mut builders = world.query::<(&Team, &Transform, &Selectable, &ConstructOrder)>();
        let positions = builders
            .iter(world)
            .filter(|(team, ..)| **team == Team::Player(2))
            .map(|(_, position, radius, _)| (position.translation, radius.radius))
            .collect::<Vec<_>>();
        let mut resources = world.query::<(&ResourceNode, &Transform, &Selectable)>();
        for (position, radius) in positions {
            for (resource, node, node_radius) in resources
                .iter(world)
                .filter(|(resource, ..)| resource.amount > 0)
            {
                let distance = xz_distance(position, node.translation);
                assert!(
                    distance + 1e-4 >= radius + node_radius.radius,
                    "full-schedule constructor entered a resource body: position={position:?} resource={:?} distance={distance} physical={}",
                    node.translation,
                    radius + node_radius.radius
                );
            }
        }
        let economy = world.resource::<Economies>().get(Team::Player(1));
        assert_eq!(
            (economy.ore, economy.crystal),
            initial_human_resources,
            "AI spent or harvested for the remote human"
        );
        let mut ordered_units =
            world.query_filtered::<&Team, (With<Unit>, ActiveUnitOrderFilter)>();
        assert!(
            !ordered_units
                .iter(world)
                .any(|team| *team == Team::Player(1)),
            "AI issued an order to a remote human unit"
        );
    }
    let world = app.world_mut();
    let mut units = world.query::<(Entity, &Team, &Unit, Option<&Weapon>)>();
    let final_human_units = units
        .iter(world)
        .filter(|(_, team, _, _)| **team == Team::Player(1))
        .map(|(entity, _, _, _)| entity)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        final_human_units, human_units,
        "AI trained units for the remote human"
    );
    assert!(
        units
            .iter(world)
            .any(|(entity, team, _unit, weapon)| *team == Team::Player(2)
                && !ai_units.contains(&entity)
                && weapon.is_some()),
        "configured AI slot failed to train an armed unit"
    );
    let mut structures = world.query::<(&Team, &Structure, Option<&UnderConstruction>)>();
    assert!(
        structures
            .iter(world)
            .any(|(team, structure, construction)| *team == Team::Player(2)
                && structure.id == "VehicleFactory"
                && construction.is_none()),
        "configured AI slot failed to build its factory"
    );
    assert_eq!(
        structures
            .iter(world)
            .filter(|(team, _, _)| **team == Team::Player(1))
            .count(),
        1,
        "AI built structures for the remote human"
    );
    eprintln!(
        "[ai-human-control] Human0/Human1/AI2: human1 resources={initial_human_resources:?}, original units retained with no orders; AI2 completed factory and trained armed units"
    );
}

#[test]
fn easy_chaos_ground_force_finishes_big_arena_repair_defense() {
    let config = SkirmishAuditConfig {
        map: "big_arena".to_string(),
        factions: ["chaos".to_string(), "chaos".to_string()],
        difficulties: ["easy".to_string(), "beginner".to_string()],
        spawn_slots: [0, 1],
        max_seconds: 600,
    };
    let result = run_skirmish_audit(&config).unwrap();
    eprintln!("[ai-ground-force] {result}");
    assert_eq!(
        result["starting_resources"],
        serde_json::json!({"ore":32,"crystal":16})
    );
    assert_eq!(result["startup_loadout"], "GodotSkirmish");
    assert_eq!(
        result["timeout"], false,
        "actual paid skirmish remained at repair equilibrium"
    );
    assert_eq!(result["winning_teams"], serde_json::json!([0]));
    assert_eq!(
        result["teams"][0]["unit_types"].get("AntiAirWalker"),
        None,
        "no enemy flying units existed, but AA-only unit consumed the small ground-force cap"
    );
}

#[test]
fn available_aa_only_product_is_reserved_for_an_actual_air_threat() {
    // Isolated production-decision fixture: registry costs, faction products,
    // normal profile, prerequisites and actual threat detection are exercised.
    // The real paid startup/economy/combat acceptance is the test above.
    use bevy::ecs::system::SystemState;
    let mut world = World::new();
    let team = Team::Player(1);
    for id in [
        "CommandCenter",
        "VehicleFactory",
        "RadarUplink",
        "RoboticsBay",
    ] {
        world.spawn((Structure { id }, team, Transform::default()));
    }
    let mut state = SystemState::<Query<StructurePrereqItem<'_>>>::new(&mut world);
    let structures = state.get(&world).unwrap();
    let profile = faction_ai_profile_for_difficulty(SkirmishFaction::Chaos, AiDifficulty::Normal);
    let economy = TeamEconomy::new(32, 16);
    let counts = AiProductionCounts {
        workers: 3,
        battle_units: 0,
    };
    let start = profile
        .production_priority
        .iter()
        .position(|id| *id == "AntiAirWalker")
        .unwrap();
    let mut cursor = start;
    let ground = next_ai_train(
        team,
        SkirmishFaction::Chaos,
        &profile,
        &structures,
        &economy,
        &mut cursor,
        counts,
        false,
    )
    .unwrap();
    assert!(
        registry::entity(ground)
            .unwrap()
            .weapon
            .unwrap()
            .can_attack_ground
    );
    assert_ne!(ground, "AntiAirWalker");
    let need_air = ai_needs_more_anti_air_units(team, [("Drone", Team::Player(0))]);
    assert!(need_air);
    cursor = start;
    assert_eq!(
        next_ai_train(
            team,
            SkirmishFaction::Chaos,
            &profile,
            &structures,
            &economy,
            &mut cursor,
            counts,
            need_air
        ),
        Some("AntiAirWalker")
    );
}

#[test]
fn easy_ground_wave_finishes_alternate_spawn_with_enemy_scout() {
    let config = SkirmishAuditConfig {
        map: "plain_and_simple".into(),
        factions: ["alliance".into(), "alliance".into()],
        difficulties: ["easy".into(), "beginner".into()],
        spawn_slots: [1, 3],
        max_seconds: 600,
    };
    let result = run_skirmish_audit(&config).unwrap();
    if let Ok(path) = std::env::var("OPEN_BEVY_AI_WAVE_TEST_RESULT") {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        use std::io::Write;
        file.write_all(&serde_json::to_vec_pretty(&result).unwrap())
            .unwrap();
    }
    eprintln!("[ai-domain] {result}");
    assert_eq!(
        result["starting_resources"],
        serde_json::json!({"ore":32,"crystal":16})
    );
    assert_eq!(result["startup_loadout"], "GodotSkirmish");
    assert!(
        result["completed"].as_bool().unwrap(),
        "armed Easy wave stalled despite normal ground anchors"
    );
    assert_eq!(result["winning_teams"], serde_json::json!([0]));
}

#[test]
fn ai_wave_target_respects_runtime_ground_and_air_capabilities() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings.player_factions[0] = SkirmishFaction::Alliance;
        settings.player_factions[1] = SkirmishFaction::Alliance;
        settings.player_controllers[1] = SkirmishPlayerController::Ai(AiDifficulty::Beginner);
        settings
            .ai_difficulties
            .set_difficulty(Team::Player(1), AiDifficulty::Beginner);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    let (air_target, origin) = {
        let world = app.world_mut();
        let mut units =
            world.query::<(Entity, &Team, &Transform, &MovementDomain, Option<&Weapon>)>();
        let (target, _, position, _, _) = units
            .iter(world)
            .find(|(_, team, _, domain, _)| {
                **team == Team::Player(1) && matches!(domain, MovementDomain::Air)
            })
            .unwrap();
        (target, position.translation)
    };
    let spec = registry::entity("ScoutRover").unwrap().weapon.unwrap();
    let mut weapon = Weapon::new(
        spec.range,
        spec.damage,
        spec.cooldown,
        spec.splash_radius,
        spec.splash_damage_multiplier,
        spec.structure_damage_multiplier,
        spec.can_attack_air,
        spec.can_attack_ground,
    );
    let relations = app.world().resource::<TeamRelations>().clone();
    let mut state = bevy::ecs::system::SystemState::<
        Query<(Entity, &Team, &Transform, &Health, &MovementDomain)>,
    >::new(app.world_mut());
    let targets = state.get(app.world()).unwrap();
    // Vary only local test values, never the live normal-start world or registry.
    weapon.can_attack_ground = true;
    weapon.can_attack_air = false;
    let ground =
        nearest_ai_wave_enemy(Team::Player(0), origin, Some(&weapon), &relations, &targets)
            .unwrap();
    assert_ne!(ground, air_target);
    let (_, team, _, health, domain) = targets.get(ground).unwrap();
    assert!(health.current > 0.0 && relations.are_enemies(Team::Player(0), *team));
    assert_eq!(*domain, MovementDomain::Terrain);
    weapon.can_attack_air = true;
    assert_eq!(
        nearest_ai_wave_enemy(Team::Player(0), origin, Some(&weapon), &relations, &targets),
        Some(air_target)
    );
    weapon.can_attack_ground = false;
    assert_eq!(
        nearest_ai_wave_enemy(Team::Player(0), origin, Some(&weapon), &relations, &targets),
        Some(air_target)
    );
}

#[test]
fn paid_alternate_refinery_workers_are_born_outside_resource_bodies() {
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
        for (player, difficulty, spawn) in
            [(0, AiDifficulty::Easy, 1), (1, AiDifficulty::Beginner, 3)]
        {
            settings.player_controllers[player] = SkirmishPlayerController::Ai(difficulty);
            settings.player_factions[player] = SkirmishFaction::Alliance;
            settings.player_spawn_slots[player] = spawn;
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
    let mut seen = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &Unit)>();
        query
            .iter(world)
            .filter(|(_, unit)| unit.id == "Worker")
            .map(|(id, _)| id)
            .collect::<std::collections::HashSet<_>>()
    };
    let mut paid_births = 0;
    for _ in 0..18000 {
        app.update();
        let world = app.world_mut();
        let clock = world.resource::<MatchState>().start_time_sec;
        let mut nodes = world.query_filtered::<(&Transform, &Selectable), With<ResourceNode>>();
        let nodes = nodes
            .iter(world)
            .map(|(p, r)| (p.translation, r.radius))
            .collect::<Vec<_>>();
        let mut workers = world.query::<(Entity, &Unit, &Transform, &Selectable)>();
        for (id, unit, position, radius) in
            workers.iter(world).filter(|(_, u, ..)| u.id == "Worker")
        {
            if !seen.insert(id) {
                continue;
            }
            paid_births += 1;
            for (node, node_radius) in &nodes {
                assert!(
                    xz_distance(position.translation, *node) + 1e-4 >= radius.radius + node_radius,
                    "paid Refinery bonus Worker born inside resource: clock={clock} entity={id:?} position={:?} node={node:?} distance={} physical={}",
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
        paid_births > 0,
        "normal AI economy must actually train paid Workers"
    );
}
