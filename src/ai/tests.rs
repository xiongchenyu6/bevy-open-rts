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
