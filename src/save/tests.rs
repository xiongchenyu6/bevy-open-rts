use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn invalid_save_files_report_actionable_errors_without_modifying_files() {
    let directory = std::env::temp_dir().join(format!("rts-save-errors-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let missing = directory.join("missing.ron");
    assert_eq!(
        read_save_file(&missing).unwrap_err(),
        t(
            "没有找到存档。请先保存对局。",
            "No saved game found. Save a match first."
        )
    );
    assert!(!missing.exists());
    let corrupt = directory.join("corrupt.ron");
    let bytes = b"not a valid save";
    std::fs::write(&corrupt, bytes).unwrap();
    assert_eq!(
        read_save_file(&corrupt).unwrap_err(),
        t(
            "存档已损坏，无法载入。当前对局未受影响。",
            "The saved game is damaged and cannot be loaded. Your current match is unchanged."
        )
    );
    assert_eq!(std::fs::read(&corrupt).unwrap(), bytes);
    let incompatible = directory.join("incompatible.ron");
    let mut save = sample_save();
    save.version = 1;
    std::fs::write(&incompatible, ron::to_string(&save).unwrap()).unwrap();
    assert_eq!(
        read_save_file(&incompatible).unwrap_err(),
        t(
            "此存档与当前版本不兼容。请开始新对局。",
            "This save is incompatible with this game version. Start a new match."
        )
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn mission_orders_and_production_survive_load_without_replaying_triggers() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    let mission = &CAMPAIGN_MISSIONS[0];
    *app.world_mut().resource_mut::<MatchSetupSettings>() =
        match_settings_for_mission(mission).unwrap();
    app.world_mut().resource_mut::<ActiveMission>().0 = Some(0);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..40 {
        app.update();
    }
    let world = app.world_mut();
    world.resource_mut::<MatchState>().start_time_sec = 121.0;
    world.resource_mut::<MissionTriggerState>().fired =
        mission.triggers.iter().map(|t| t.at_sec <= 121.0).collect();
    world.resource_mut::<Economies>().players[0].ore = 777;
    let worker = world
        .query::<(Entity, &Unit, &Team)>()
        .iter(world)
        .find(|(_, unit, team)| unit.id == "Worker" && **team == Team::Player(0))
        .unwrap()
        .0;
    let resource = world
        .query_filtered::<Entity, With<ResourceNode>>()
        .iter(world)
        .next()
        .unwrap();
    world.entity_mut(worker).insert((
        HarvestOrder {
            resource: Some(resource),
            state: HarvestState::Collecting,
            collect_remaining: 0.75,
            last_kind: Some(ResourceKind::Ore),
        },
        OrderQueue {
            orders: [UnitQueuedOrder::Move(Vec3::new(3.0, 0.0, 4.0))].into(),
        },
    ));
    let producer = world
        .query::<(Entity, &Structure, &Team)>()
        .iter(world)
        .find(|(_, structure, team)| structure.id == "CommandCenter" && **team == Team::Player(0))
        .unwrap()
        .0;
    world.resource_mut::<BuildQueue>().0.push(BuildJob {
        team: Team::Player(0),
        action: BuildAction::Train("Worker"),
        producer_entity: producer,
        producer_id: "CommandCenter",
        timer: 7.0,
        origin: Vec3::ZERO,
        cost: registry::Cost { ore: 4, crystal: 0 },
    });
    let save = collect_save_game(world).unwrap();
    validate_save(&save).unwrap();
    // Restore synchronously to inspect exact state before simulation advances.
    world.resource_mut::<PendingLoadedSave>().0 = Some(save.clone());
    apply_loaded_save(world);
    let after = collect_save_game(world).unwrap();
    assert_eq!(after.mission_id, save.mission_id);
    assert_eq!(after.mission_triggers, save.mission_triggers);
    assert_eq!(after.economies[0].ore, 777);
    assert_eq!(after.build_jobs.len(), 1);
    assert_eq!(after.build_jobs[0].timer, 7.0);
    let saved_worker = after
        .units
        .iter()
        .find(|u| !u.continuity.queued.is_empty())
        .unwrap();
    assert_eq!(saved_worker.continuity.harvest_remaining, 0.75);
    assert_eq!(
        saved_worker.continuity.harvest_state,
        Some(HarvestState::Collecting as u8)
    );
    assert!(matches!(
        saved_worker.continuity.active[0],
        SavedOrder::Harvest { .. }
    ));
    assert!(
        world
            .get::<Structure>(world.resource::<BuildQueue>().0[0].producer_entity)
            .is_some()
    );
}

#[test]
fn invalid_save_schema_and_player_state_are_rejected() {
    let mut save = sample_save();
    validate_save(&save).unwrap();
    save.version = 1;
    assert!(validate_save(&save).is_err());
    save.version = SAVE_VERSION;
    save.settings.visible_team = 99;
    assert!(validate_save(&save).is_err());
    save.settings.visible_team = 0;
    save.clock_sec = f32::NAN;
    assert!(validate_save(&save).is_err());
}

#[test]
fn targeted_orders_remap_entities_instead_of_reusing_runtime_ids() {
    let mut world = World::new();
    let old = world.spawn_empty().id();
    let new = world.spawn_empty().id();
    let map = [(old.to_bits(), new)].into_iter().collect();
    let order = save_order(&UnitQueuedOrder::Attack(old));
    assert!(matches!(load_order(&order, &map), Some(UnitQueuedOrder::Attack(e)) if e == new));
    assert!(load_order(&order, &std::collections::HashMap::new()).is_none());
}

fn sample_save() -> SaveGame {
    SaveGame {
        weather: WeatherState::default(),
        version: SAVE_VERSION,
        ai: SavedAiDirector::default(),
        support_entities: vec![],
        structures_lost: 0,
        mission_id: None,
        tutorial: None,
        mission_triggers: vec![],
        build_jobs: vec![],
        explored: vec![],
        fog_remnants: vec![],
        crates: vec![],
        settings: SavedSettings {
            map_path: "res://source/match/maps/PlainAndSimple.tscn".to_string(),
            starting_ore: 32,
            starting_crystal: 16,
            active_teams: vec![true, true],
            player_factions: vec![0, 1],
            player_color_slots: vec![0, 1],
            player_controllers: vec![1, 2 + AiDifficulty::Normal as u8],
            player_spawn_slots: vec![0, 1],
            ai_difficulties: vec![2, 2],
            allied: vec![vec![true, false], vec![false, true]],
            startup_loadout: StartupLoadoutMode::GodotSkirmish as u8,
            victory_condition: 0,
            visible_team: 0,
        },
        clock_sec: 123.5,
        enemy_units_destroyed: 4,
        enemy_structures_destroyed: 1,
        units_lost: 2,
        economies: vec![SavedEconomy {
            ore: 100,
            crystal: 50,
            power_sabotage_remaining: 0.0,
            production_veterancy_ranks: vec![0, 1],
        }],
        support_remaining: vec![vec![30.0; SupportPowerKind::ALL.len()]],
        support_charge_started: vec![vec![true; SupportPowerKind::ALL.len()]],
        units: vec![SavedUnit {
            key: 1,
            continuity: SavedUnitContinuity::default(),
            id: "Worker".to_string(),
            team: 0,
            position: [1.0, 0.0, 2.0],
            yaw: 0.5,
            health: 4.0,
            rank: 1,
            experience: 10,
            cargo: Some((3, 0)),
        }],
        structures: vec![SavedStructure {
            key: 2,
            known: true,
            effects: SavedEffects::default(),
            weapon_remaining: 0.0,
            id: "CommandCenter".to_string(),
            team: 0,
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
            health: 20.0,
            construction: None,
            construction_cost: None,
            free_worker_origin: None,
            repair_remaining: None,
            rally: Some(SavedRally {
                target: [5.0, 0.0, 5.0],
                mode: 1,
                target_entity: None,
            }),
            garrison_count: 0,
        }],
        resources: vec![(3, 0, [8.0, 0.0, 8.0], 40)],
    }
}

#[test]
fn quicksave_roundtrip_restores_world_through_restart_flow() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..40 {
        app.update();
    }
    // Distinctive state the roundtrip must carry over.
    app.world_mut().resource_mut::<Economies>().players[0].ore = 777;
    let save = collect_save_game(app.world_mut()).expect("running match must save");
    assert!(!save.units.is_empty(), "start units must be captured");
    assert!(!save.structures.is_empty(), "bases must be captured");
    assert!(
        !save.resources.is_empty(),
        "resource nodes must be captured"
    );
    assert_eq!(save.economies[0].ore, 777);

    // Load: the same path quickload_hotkey takes.
    *app.world_mut().resource_mut::<MatchSetupSettings>() =
        match_settings_from_save(&save.settings).expect("map resolves");
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(save.clone());
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    for _ in 0..40 {
        app.update();
    }

    assert!(
        !app.world().resource::<MatchBriefingState>().visible,
        "loading must not replay the opening briefing"
    );
    let after = collect_save_game(app.world_mut()).expect("match running after load");
    assert_eq!(
        after.units.len(),
        save.units.len(),
        "unit count survives the load"
    );
    assert_eq!(
        after.structures.len(),
        save.structures.len(),
        "structure count survives the load"
    );
    assert_eq!(after.economies[0].ore, 777, "economy survives the load");
    assert!(
        (after.clock_sec - save.clock_sec).abs() < 5.0,
        "match clock resumes from the save"
    );
}

#[test]
fn replay_jump_picks_neighbouring_keyframes() {
    let clocks = [0.0, 30.0, 60.0];
    // Mid-match: back goes to the last frame safely before now.
    assert_eq!(replay_jump_target(&clocks, 45.0, true), Some(1));
    assert_eq!(replay_jump_target(&clocks, 45.0, false), Some(2));
    // Right after a keyframe, back skips to the PREVIOUS one (epsilon).
    assert_eq!(replay_jump_target(&clocks, 30.5, true), Some(0));
    // Edges.
    assert_eq!(replay_jump_target(&clocks, 0.5, true), None);
    assert_eq!(replay_jump_target(&clocks, 61.0, false), None);
    assert_eq!(replay_jump_target(&[], 10.0, true), None);
}

#[test]
fn replay_records_keyframe_zero_and_jumps_back_to_it() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..60 {
        app.update();
    }
    let frames = app.world().resource::<ReplayTimeline>().frames.len();
    assert_eq!(frames, 1, "keyframe 0 recorded at match start");
    let frame = app.world().resource::<ReplayTimeline>().frames[0].clone();
    assert!(frame.clock_sec < 1.0);

    // Jump back through the same pipeline the PageUp hotkey uses.
    *app.world_mut().resource_mut::<MatchSetupSettings>() =
        match_settings_from_save(&frame.settings).expect("map resolves");
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(frame.clone());
    app.world_mut()
        .resource_mut::<ReplayTimeline>()
        .next_record_at = f32::MAX;
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    for _ in 0..40 {
        app.update();
    }
    let state = app.world().resource::<MatchState>();
    assert_eq!(state.phase, MatchPhase::Running);
    assert!(
        state.start_time_sec < frame.clock_sec + 5.0,
        "clock rewound to the keyframe"
    );
    assert!(
        !app.world().resource::<ReplayTimeline>().frames.is_empty(),
        "timeline survives the jump for forward navigation"
    );
}

#[test]
fn save_game_roundtrips_through_ron() {
    let save = sample_save();
    let text = ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default()).unwrap();
    let back: SaveGame = ron::from_str(&text).unwrap();
    assert_eq!(back, save);
}

#[test]
fn saved_settings_rebuild_match_setup() {
    let save = sample_save();
    let settings = match_settings_from_save(&save.settings).expect("known map must resolve");
    assert_eq!(settings.starting_resources.ore, 32);
    assert_eq!(
        settings.player_controllers[1],
        SkirmishPlayerController::Ai(AiDifficulty::Normal)
    );
    assert_eq!(settings.visible_player.team, Team::Player(0));
    assert!(settings.team_relations.allied[0][0]);
}

#[test]
fn delayed_support_and_status_effects_resume_with_saved_timers() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..40 {
        app.update();
    }
    let world = app.world_mut();
    let worker = world
        .query::<(Entity, &Unit, &Team)>()
        .iter(world)
        .find(|(_, u, t)| u.id == "Worker" && **t == Team::Player(0))
        .unwrap()
        .0;
    world.entity_mut(worker).insert((
        EmpDisabled { remaining: 3.25 },
        SupportShield {
            remaining: 5.5,
            damage_scale: 0.5,
        },
        ChronoRelay {
            remaining: 7.75,
            speed_multiplier: 1.3,
        },
    ));
    world.spawn((
        Transform::from_xyz(1.0, 0.03, 2.0),
        PendingOrbitalStrike {
            remaining: 1.5,
            radius: 3.0,
            damage: 12.0,
            impact_scale: 1.2,
            team: Team::Player(0),
        },
        MatchScopedEntity,
    ));
    world.spawn((
        PendingParadrop {
            remaining: 2.0,
            team: Team::Player(0),
            target: Vec3::new(2.0, 0.0, 3.0),
            unit_paths: SupportPowerKind::Paradrop.definition().unit_paths,
        },
        MatchScopedEntity,
    ));
    world
        .resource_mut::<AiDirector>()
        .ensure_team(Team::Player(0));
    world.resource_mut::<AiDirector>().attack_timer[0] = 17.25;
    world.resource_mut::<SupportCooldowns>().set(
        Team::Player(0),
        SupportPowerKind::StrategicMissile,
        43.5,
    );
    let save = collect_save_game(world).unwrap();
    validate_save(&save).unwrap();
    world.resource_mut::<PendingLoadedSave>().0 = Some(save.clone());
    apply_loaded_save(world);
    let restored = collect_save_game(world).unwrap();
    assert_eq!(restored.ai, save.ai);
    assert_eq!(restored.support_remaining, save.support_remaining);
    assert_eq!(restored.support_charge_started, save.support_charge_started);
    assert_eq!(restored.support_entities, save.support_entities);
    let unit = restored
        .units
        .iter()
        .find(|u| u.continuity.effects.emp.is_some())
        .unwrap();
    assert_eq!(unit.continuity.effects.emp, Some(3.25));
    assert_eq!(unit.continuity.effects.shield, Some((5.5, 0.5)));
    assert_eq!(unit.continuity.effects.chrono, Some((7.75, 1.3)));
}

#[test]
fn siege_deployment_preserves_mobility_and_base_stats_for_undeploying() {
    let mut world = World::new();
    let mut weapon = Weapon::new(5.0, 7.0, 1.0, 0.0, 1.0, 1.0, true, false);
    weapon.range = 8.0;
    let unit = Unit {
        id: "Tank",
        speed: 2.0,
        can_crush: true,
        can_be_crushed: false,
    };
    let old = world
        .spawn((
            unit,
            weapon,
            VisionRadius(9.0),
            HoldPosition { enabled: true },
            DeployedSiegeMode {
                previous_hold_position: false,
                base_speed: 2.0,
                base_attack_range: 5.0,
                base_attack_damage: 7.0,
                base_attack_interval: 1.0,
                base_structure_damage_multiplier: 1.0,
                base_sight_range: 6.0,
            },
        ))
        .id();
    world.get_mut::<Unit>(old).unwrap().speed = 0.0;
    let saved = collect_unit_continuity(&world, old);
    let another_unit = world.get::<Unit>(old).unwrap().clone();
    let new = world
        .spawn((
            another_unit,
            weapon,
            VisionRadius(1.0),
            HoldPosition { enabled: false },
        ))
        .id();
    restore_unit_continuity(&mut world, new, &saved, &std::collections::HashMap::new());
    assert_eq!(world.get::<Unit>(new).unwrap().speed, 0.0);
    assert_eq!(world.get::<Weapon>(new).unwrap().range, 8.0);
    assert_eq!(world.get::<VisionRadius>(new).unwrap().0, 9.0);
    let deployment = world.get::<DeployedSiegeMode>(new).unwrap();
    assert_eq!(deployment.base_speed, 2.0);
    assert_eq!(deployment.base_attack_range, 5.0);
    assert!(!deployment.previous_hold_position);
}

#[test]
fn tutorial_stage_and_observation_baselines_survive_a_serialized_load() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..40 {
        app.update();
    }
    let tutorial = TutorialState {
        active: true,
        stage: TutorialStage::Move,
        initial_ore: 64,
        initial_units: 2,
        initial_kills: 3,
        last_cargo: 4,
        move_origin: [1.0, 0.0, 2.0],
    };
    *app.world_mut().resource_mut::<TutorialState>() = tutorial.clone();
    let save = collect_save_game(app.world_mut()).unwrap();
    let encoded = ron::to_string(&save).unwrap();
    let restored: SaveGame = ron::from_str(&encoded).unwrap();
    validate_save(&restored).unwrap();
    *app.world_mut().resource_mut::<TutorialState>() = TutorialState::default();
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(restored);
    apply_loaded_save(app.world_mut());
    assert_eq!(*app.world().resource::<TutorialState>(), tutorial);
    assert!(app.world().contains_resource::<LoadedMatchMarker>());
}

#[test]
fn saves_accept_virtual_spawn_slots_and_the_runtime_player_count() {
    let mut save = sample_save();
    save.settings.player_spawn_slots = vec![12, 35];
    validate_save(&save).unwrap();
    let n = 17;
    save.settings.active_teams = vec![true; n];
    save.settings.player_factions = vec![0; n];
    save.settings.player_controllers = vec![1; n];
    save.settings.player_spawn_slots = (0..n).collect();
    save.settings.player_color_slots = (0..n).collect();
    save.settings.ai_difficulties = vec![2; n];
    save.settings.allied = (0..n).map(|i| (0..n).map(|j| i == j).collect()).collect();
    validate_save(&save).unwrap();
    save.tutorial = Some(TutorialState {
        move_origin: [f32::NAN, 0.0, 0.0],
        ..default()
    });
    assert!(validate_save(&save).is_err());
}

#[test]
fn sparse_runtime_settings_save_the_effective_defaults_without_changing_colors() {
    let mut app = build_game_app(GameAppMode::Headless);
    let mut settings = MatchSetupSettings::default();
    settings.ai_difficulties.players.clear();
    settings.team_relations.allied.clear();
    settings.player_factions.clear();
    settings.player_controllers.clear();
    settings.player_spawn_slots.clear();
    settings.player_color_slots.clear();
    let count = settings.active_teams.len();
    *app.world_mut().resource_mut::<MatchSetupSettings>() = settings;
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..40 {
        app.update();
    }
    let save = collect_save_game(app.world_mut()).unwrap();
    validate_save(&save).unwrap();
    assert_eq!(
        save.settings.ai_difficulties,
        vec![AiDifficulty::Normal as u8; count]
    );
    assert_eq!(
        save.settings.player_spawn_slots,
        (0..count).collect::<Vec<_>>()
    );
    assert!(save.settings.player_color_slots.is_empty());
    for index in 0..count {
        assert_eq!(
            save.settings.player_factions[index],
            SkirmishFaction::from_team(Team::Player(index)).index()
        );
        assert!(save.settings.allied[index][index]);
    }
    let restored = match_settings_from_save(&save.settings).unwrap();
    assert!(restored.player_color_slots.is_empty());
    assert_eq!(
        restored.ai_difficulties.difficulty(Team::Player(0)),
        AiDifficulty::Normal
    );
}

fn weather_transition_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.25),
    ));
    let settings = app.world_mut().resource_mut::<MatchSetupSettings>();
    settings.into_inner().player_controllers = vec![SkirmishPlayerController::Human; 2];
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..600 {
        app.update();
        let weather = app.world().resource::<WeatherState>();
        if weather.current != WeatherKind::Clear && weather.blend > 0.2 && weather.blend < 0.8 {
            return app;
        }
    }
    panic!("actual weather cycle never reached a mid-fade transition");
}

#[test]
fn weather_midfade_save_restores_exactly_and_advances_normally() {
    let mut app = weather_transition_app();
    let before = app.world().resource::<WeatherState>().clone();
    assert_eq!(before.previous, WeatherKind::Clear);
    assert_eq!(before.step, 1);
    let save = collect_save_game(app.world_mut()).unwrap();
    let decoded: SaveGame = ron::from_str(&ron::to_string(&save).unwrap()).unwrap();
    assert_eq!(decoded.weather, before);
    validate_save(&decoded).unwrap();
    // A normal new match resets ambient state before the pending load applies.
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(decoded);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(*app.world().resource::<WeatherState>(), before);
    assert!(app.world().resource::<PendingLoadedSave>().0.is_none());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.25),
    ));
    app.update();
    let after = app.world().resource::<WeatherState>();
    assert_eq!(after.current, before.current);
    assert_eq!(after.previous, before.previous);
    assert_eq!(after.step, before.step);
    assert!((after.phase_remaining - (before.phase_remaining - 0.25)).abs() < 0.0001);
    assert!((after.blend - (before.blend + 0.25 / 9.0)).abs() < 0.0001);
}

#[test]
fn weather_real_restart_resets_previous_match_transition() {
    let mut app = weather_transition_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<WeatherState>(),
        WeatherState::default()
    );
}

#[test]
fn weather_save_validation_rejects_nonfinite_and_out_of_range_fields() {
    let mut save = sample_save();
    for value in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        save.weather.blend = value;
        assert!(validate_save(&save).is_err(), "invalid blend {value}");
    }
    save.weather.blend = 0.5;
    for value in [f32::NAN, f32::INFINITY, -0.01, 170.01] {
        save.weather.phase_remaining = value;
        assert!(validate_save(&save).is_err(), "invalid phase {value}");
    }
    save.weather.phase_remaining = 170.0;
    validate_save(&save).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn weather_schema_rejects_old_header_without_modifying_file() {
    let path =
        std::env::temp_dir().join(format!("rts-old-weather-schema-{}.ron", std::process::id()));
    let bytes = b"(version:2)";
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        read_save_file(&path).unwrap_err(),
        t(
            "此存档与当前版本不兼容。请开始新对局。",
            "This save is incompatible with this game version. Start a new match."
        )
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}
