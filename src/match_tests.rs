//! Headless match-flow integration tests (moved out of lib.rs verbatim).

use super::*;

#[test]
fn rts_cursor_tracks_command_mode_and_hud() {
    let mut window = Window {
        resolution: WindowResolution::new(1280, 720),
        ..default()
    };
    window.set_cursor_position(None);

    let mut command_mode = CommandMode::default();
    let hud_zones = HudHitZones::default();
    assert_eq!(
        desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
        RtsCursorKind::Default
    );

    command_mode.attack_move = true;
    assert_eq!(
        desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
        RtsCursorKind::Attack
    );

    command_mode.attack_move = false;
    command_mode.pending_structure_placement =
        Some(PendingStructurePlacement::new("CommandCenter"));
    assert_eq!(
        desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
        RtsCursorKind::Build
    );

    window.set_cursor_position(Some(Vec2::new(640.0, 10.0)));
    assert_eq!(
        desired_rts_cursor_kind(Some(&command_mode), None, &window, &hud_zones),
        RtsCursorKind::Default
    );
}

#[test]
fn power_readout_shows_used_before_capacity() {
    let mut econ = TeamEconomy::new(0, 0);
    econ.power_used = 26;
    econ.power_capacity = 13;
    assert_eq!(power_readout_text(&econ), "26/13");
    assert!(econ.low_power());
}

#[test]
fn shift_right_click_queues_unit_waypoints_only_when_allowed() {
    let existing = OrderQueue {
        orders: VecDeque::from([UnitQueuedOrder::Move(Vec3::new(1.0, 0.0, 1.0))]),
    };
    assert!(
        should_queue_selected_order(true, true, true, None),
        "shift + active order should append a waypoint"
    );
    assert!(
        should_queue_selected_order(true, true, false, Some(&existing)),
        "shift + existing queued orders should append another waypoint"
    );
    assert!(
        !should_queue_selected_order(true, false, true, Some(&existing)),
        "callers must explicitly allow queueing"
    );
    assert!(
        !should_queue_selected_order(false, true, true, Some(&existing)),
        "plain right-click should replace the current order"
    );
}

#[test]
fn rally_point_tracks_move_and_attack_move_modes() {
    let mut rally = RallyPoint {
        target: None,
        target_unit: None,
        mode: RallyMode::Move,
    };
    let move_target = Vec3::new(3.0, 0.0, 4.0);
    assert!(apply_rally_point_command_in_bounds(
        &mut rally,
        move_target,
        None,
        RallyMode::Move,
        MapBounds::default(),
    ));
    assert_eq!(rally.target, Some(move_target));
    assert_eq!(rally.mode, RallyMode::Move);

    let attack_target = Vec3::new(8.0, 0.0, -2.0);
    assert!(apply_rally_point_command_in_bounds(
        &mut rally,
        attack_target,
        None,
        RallyMode::AttackMove,
        MapBounds::default(),
    ));
    assert_eq!(rally.target, Some(attack_target));
    assert_eq!(rally.mode, RallyMode::AttackMove);
}

#[test]
fn attack_rally_spawns_combat_units_with_attack_move_only() {
    let target = Vec3::new(6.0, 0.0, -3.0);
    let rally = RallyPoint {
        target: Some(target),
        target_unit: None,
        mode: RallyMode::AttackMove,
    };
    let tank = registry::entity("Tank").expect("tank definition should exist");
    assert!(matches!(
        spawned_unit_rally_order(tank, target, Some(rally)),
        UnitQueuedOrder::AttackMove(destination) if destination == target
    ));

    let worker = registry::entity("Worker").expect("worker definition should exist");
    assert!(matches!(
        spawned_unit_rally_order(worker, target, Some(rally)),
        UnitQueuedOrder::Move(destination) if destination == target
    ));
}

// Allies share vision: an enemy unit standing next to an ally's unit (but far
// from the viewing player's own units) must be revealed through the ally, and
// must stay fogged when the same teams are NOT allied. Mirrors godot's
// FogOfWar revealing units `is_allied_with(visible_player)`.
// W/S keyboard pan must match the edge-pan sign convention (pan.y<0 = view up,
// matching cursor at the top edge). Guards against the recurring inversion.
#[test]
fn allied_vision_is_shared_through_allies() {
    fn enemy_visible_with_alliance(allied: bool) -> bool {
        let mut app = App::new();
        app.insert_resource(VisiblePlayer::per_player(Team::Player(0)));
        let mut relations = TeamRelations::default();
        relations.set_allied(Team::Player(0), Team::Player(1), allied);
        app.insert_resource(relations);
        app.add_systems(Update, update_visibility);

        // Viewing player's own unit, far from the action (does not reveal it).
        app.world_mut().spawn((
            Team::Player(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            VisionRadius(8.0),
            VisibilityState { visible: true },
            Visibility::Visible,
        ));
        // Ally unit parked next to the enemy, far from the viewer.
        app.world_mut().spawn((
            Team::Player(1),
            Transform::from_xyz(100.0, 0.0, 0.0),
            VisionRadius(12.0),
            VisibilityState { visible: true },
            Visibility::Visible,
        ));
        // Enemy unit beside the ally — only the ally can see it.
        let enemy = app
            .world_mut()
            .spawn((
                Team::Player(2),
                Transform::from_xyz(101.0, 0.0, 0.0),
                VisionRadius(8.0),
                VisibilityState { visible: false },
                Visibility::Hidden,
            ))
            .id();

        app.update();
        app.world().get::<VisibilityState>(enemy).unwrap().visible
    }

    assert!(
        enemy_visible_with_alliance(true),
        "allied unit should reveal the enemy beside it for the viewing player"
    );
    assert!(
        !enemy_visible_with_alliance(false),
        "without an alliance the enemy beside a neutral team must stay fogged"
    );
}

#[test]
fn edge_pan_is_disabled_by_options_overlays_and_interactive_ui_only() {
    let options = MenuOptionsState::default();
    assert_eq!(
        effective_camera_edge_pan_width(
            &options,
            &MatchMenuState::default(),
            &MatchBriefingState::default(),
            false,
        ),
        CAMERA_EDGE_PAN_WIDTH
    );

    let mut edge_pan_disabled = options;
    edge_pan_disabled.camera_edge_pan = false;
    assert_eq!(
        effective_camera_edge_pan_width(
            &edge_pan_disabled,
            &MatchMenuState::default(),
            &MatchBriefingState::default(),
            false,
        ),
        0.0
    );

    assert_eq!(
        effective_camera_edge_pan_width(
            &options,
            &MatchMenuState { visible: true },
            &MatchBriefingState::default(),
            false,
        ),
        0.0
    );
    assert_eq!(
        effective_camera_edge_pan_width(
            &options,
            &MatchMenuState::default(),
            &MatchBriefingState {
                visible: true,
                ..default()
            },
            false,
        ),
        CAMERA_EDGE_PAN_WIDTH,
        "the opening battle briefing is passive; it must not block edge pan"
    );
    assert_eq!(
        effective_camera_edge_pan_width(
            &options,
            &MatchMenuState::default(),
            &MatchBriefingState::default(),
            true,
        ),
        0.0
    );

    let mut window = Window {
        resolution: WindowResolution::new(1280, 720),
        ..default()
    };
    window.set_cursor_position(Some(Vec2::new(640.0, 10.0)));
    assert!(cursor_is_over_hud(&window, &HudHitZones::default()));
    assert_eq!(
        effective_camera_edge_pan_width(
            &options,
            &MatchMenuState::default(),
            &MatchBriefingState::default(),
            false,
        ),
        CAMERA_EDGE_PAN_WIDTH,
        "passive HUD strips must not block top/bottom edge pan"
    );
}

#[test]
fn support_power_hud_specs_match_godot_strip() {
    let specs = support_power_button_specs();
    assert_eq!(
        specs.iter().map(|spec| spec.kind).collect::<Vec<_>>(),
        vec![
            SupportPowerKind::RadarSweep,
            SupportPowerKind::OrbitalStrike,
            SupportPowerKind::EmpPulse,
            SupportPowerKind::ChronoRelay,
            SupportPowerKind::ShieldOverdrive,
            SupportPowerKind::NaniteRepairSwarm,
            SupportPowerKind::WeatherStorm,
            SupportPowerKind::StrategicMissile,
            SupportPowerKind::Paradrop,
        ]
    );
    assert_eq!(
        specs
            .iter()
            .map(|spec| spec.hotkey_label)
            .collect::<Vec<_>>(),
        vec!["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"]
    );
    for spec in specs {
        let path = std::path::Path::new("assets").join(spec.icon_path);
        assert!(
            path.exists(),
            "support power icon must exist for {:?}: {}",
            spec.kind,
            path.display()
        );
    }
}

#[test]
fn support_power_button_state_matches_lock_cooldown_and_low_power_rules() {
    let ready = support_power_button_state(SupportPowerKind::RadarSweep, true, false, 0.0, false);
    assert!(ready.enabled);
    assert!(ready.unlocked);
    assert!(!ready.active);
    assert_eq!(ready.cooldown_seconds, None);
    assert_eq!(ready.badge_text, "");

    let active = support_power_button_state(SupportPowerKind::RadarSweep, true, false, 0.0, true);
    assert!(active.enabled);
    assert!(active.active);

    let locked = support_power_button_state(SupportPowerKind::RadarSweep, false, false, 0.0, false);
    assert!(!locked.enabled);
    assert!(!locked.unlocked);
    assert_eq!(
        locked.badge_text, "",
        "locked support powers are hidden instead of rendered with a lock badge"
    );

    let cooling =
        support_power_button_state(SupportPowerKind::RadarSweep, true, false, 12.2, false);
    assert!(!cooling.enabled);
    assert_eq!(cooling.cooldown_seconds, Some(13));
    assert_eq!(cooling.badge_text, "13");

    let low_power =
        support_power_button_state(SupportPowerKind::RadarSweep, true, true, 0.0, false);
    assert!(!low_power.enabled);
    assert!(low_power.low_power);
    assert_eq!(low_power.badge_text, "");
}

#[test]
fn support_powers_unlock_only_after_required_structures_are_constructed() {
    let mut world = World::new();
    let team = Team::Player(0);
    let other_team = Team::Player(1);

    {
        let mut query = world.query::<StructurePrereqItem<'_>>();
        let structures = query.query(&world);
        assert_eq!(visible_support_power_count(team, &structures), 0);
        assert!(!support_power_unlocked(
            team,
            SupportPowerKind::RadarSweep,
            &structures
        ));
    }

    world.spawn((Structure { id: "RadarUplink" }, team, Transform::default()));
    {
        let mut query = world.query::<StructurePrereqItem<'_>>();
        let structures = query.query(&world);
        assert!(support_power_unlocked(
            team,
            SupportPowerKind::RadarSweep,
            &structures
        ));
        assert_eq!(visible_support_power_count(team, &structures), 1);
    }

    world.spawn((
        Structure {
            id: "WeatherControlSpire",
        },
        team,
        Transform::default(),
        UnderConstruction {
            remaining: 8.0,
            total: 8.0,
            cost: registry::Cost::default(),
            free_worker_origin: None,
        },
    ));
    {
        let mut query = world.query::<StructurePrereqItem<'_>>();
        let structures = query.query(&world);
        assert_eq!(
            visible_support_power_count(team, &structures),
            1,
            "under-construction tech must not reveal support buttons"
        );
        assert!(!support_power_unlocked(
            team,
            SupportPowerKind::StrategicMissile,
            &structures
        ));
    }

    world.spawn((Structure { id: "TechAirport" }, team, Transform::default()));
    world.spawn((
        Structure {
            id: "WeatherControlSpire",
        },
        team,
        Transform::default(),
    ));
    world.spawn((
        Structure { id: "RoboticsBay" },
        other_team,
        Transform::default(),
    ));
    {
        let mut query = world.query::<StructurePrereqItem<'_>>();
        let structures = query.query(&world);
        assert!(support_power_unlocked(
            team,
            SupportPowerKind::Paradrop,
            &structures
        ));
        assert!(support_power_unlocked(
            team,
            SupportPowerKind::StrategicMissile,
            &structures
        ));
        assert_eq!(
            visible_support_power_count(team, &structures),
            4,
            "radar + airport + weather spire should reveal radar, paradrop, and two superweapons only"
        );
    }
}

#[test]
fn support_power_panel_hit_rect_is_tight() {
    let mut window = Window {
        resolution: WindowResolution::new(1280, 720),
        ..default()
    };
    assert_eq!(
        support_power_panel_width_for_visible_count(0),
        0.0,
        "no unlocked support powers should leave no top-right hit rect"
    );
    assert_eq!(
        support_power_panel_width_for_visible_count(SupportPowerKind::ALL.len()),
        SUPPORT_POWER_PANEL_WIDTH_PX
    );

    let inside = Vec2::new(1278.0 - SUPPORT_POWER_PANEL_RIGHT_PX, 16.0);
    assert!(!support_power_panel_contains_cursor(&window, inside, 0));
    assert!(support_power_panel_contains_cursor(&window, inside, 3));
    window.set_cursor_position(Some(inside));
    let zones = HudHitZones {
        world_rects: hud_world_input_rects(1280.0, 720.0, 3, 0, 0, 0, false),
    };
    assert!(cursor_is_over_hud(&window, &zones));

    let left_of_panel = Vec2::new(
        1280.0
            - SUPPORT_POWER_PANEL_RIGHT_PX
            - support_power_panel_width_for_visible_count(3)
            - 8.0,
        SUPPORT_POWER_PANEL_TOP_PX + 20.0,
    );
    assert!(!support_power_panel_contains_cursor(
        &window,
        left_of_panel,
        3
    ));
    assert!(
        !cursor_blocks_world_order_controls(left_of_panel, &zones),
        "support panel hit rect should not consume the whole top strip"
    );

    let below_panel = Vec2::new(
        1280.0 - SUPPORT_POWER_PANEL_RIGHT_PX - 12.0,
        SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX + 4.0,
    );
    assert!(!support_power_panel_contains_cursor(
        &window,
        below_panel,
        3
    ));
}

#[test]
fn objective_tracker_sits_below_support_power_strip() {
    assert!(
        OBJECTIVE_TRACKER_TOP_PX
            >= SUPPORT_POWER_PANEL_TOP_PX + SUPPORT_POWER_PANEL_HEIGHT_PX + 6.0,
        "the top-center objective HUD must not overlap the Godot-style support power strip"
    );
}

// Every command-panel action must resolve to an icon asset that actually
// exists on disk, so the HUD renders godot-style command icons instead of
// falling back to a blank button. Locks the action->icon mapping.
#[test]
fn command_actions_resolve_to_existing_icon_assets() {
    let standing_orders = [
        BuildAction::SellStructure,
        BuildAction::RepairStructure,
        BuildAction::ToggleDeployMode,
        BuildAction::SetRallyPoint,
        BuildAction::HoldPosition,
        BuildAction::AttackMove,
        BuildAction::Patrol,
        BuildAction::GuardArea,
        BuildAction::StopSelected,
        BuildAction::ScatterSelected,
    ];
    for (idx, action) in standing_orders.into_iter().enumerate() {
        let path = command_action_icon_path(action)
            .unwrap_or_else(|| panic!("no icon for standing order #{idx}"));
        assert!(
            std::path::Path::new("assets").join(path).exists(),
            "missing icon asset {path} for standing order #{idx}"
        );
    }
    // Train/Build pull the produced entity's registry icon (e.g. Worker).
    let worker_icon = command_action_icon_path(BuildAction::Train("Worker"))
        .expect("Worker should have a registry icon");
    assert!(
        std::path::Path::new("assets").join(worker_icon).exists(),
        "missing Worker icon asset {worker_icon}"
    );
    assert_eq!(command_action_icon_path(BuildAction::None), None);
}

#[test]
fn worker_is_the_only_resource_collector_unit() {
    let worker = registry::entity("Worker").expect("Worker must stay in the registry");
    assert!(
        worker.resource_capacity > 0,
        "Worker must carry resources now that separate vehicle collectors are removed"
    );
    for entity in registry::ENTITY_DEFS {
        assert!(
            !entity.is_worker || entity.id == "Worker",
            "{} must not share the worker classification",
            entity.id
        );
        assert!(
            entity.resource_capacity <= 0 || entity.id == "Worker",
            "{} must not share the resource collector role",
            entity.id
        );
    }
    assert!(
        registry::entity("OreHarvester").is_none(),
        "OreHarvester should not exist in the playable registry"
    );
    assert!(
        registry::entity("MobileConstructionVehicle").is_none(),
        "MobileConstructionVehicle should not exist in the playable registry"
    );
    for faction_id in ["alliance", "demon", "chaos"] {
        let faction = registry::faction(faction_id).expect("registered skirmish faction");
        let command_center = faction
            .production
            .iter()
            .find(|production| production.producer == "CommandCenter")
            .expect("CommandCenter production list");
        assert!(
            command_center.products.contains(&"Worker"),
            "{faction_id} CommandCenter must train Worker"
        );
        for production in faction.production {
            assert!(
                !production.products.contains(&"OreHarvester"),
                "{faction_id} {} must not expose OreHarvester",
                production.producer
            );
            assert!(
                !production.products.contains(&"MobileConstructionVehicle"),
                "{faction_id} {} must not expose MobileConstructionVehicle",
                production.producer
            );
        }
    }
}

#[test]
fn critical_units_do_not_share_model_signatures() {
    fn model_signature(id: &str) -> Vec<&'static str> {
        registry::entity(id)
            .unwrap_or_else(|| panic!("{id} must stay in the registry"))
            .render_parts
            .iter()
            .map(|part| part.model)
            .collect()
    }

    let worker = registry::entity("Worker").expect("Worker must stay in the registry");
    let scout = registry::entity("ScoutRover").expect("ScoutRover must stay in the registry");
    assert_eq!(worker.render_parts.len(), 2);
    assert_eq!(scout.render_parts.len(), 1);
    assert!(
        worker
            .render_parts
            .iter()
            .any(|part| part.model == "models/kenney-spacekit/astronautB.glb"),
        "Worker should read as an engineer/infantry unit, not another rover"
    );
    assert!(
        !worker
            .render_parts
            .iter()
            .any(|part| part.model == "models/kenney-spacekit/rover.glb"),
        "Worker must not share ScoutRover's rover mesh"
    );
    assert_eq!(
        scout.render_parts[0].model,
        "models/kenney-spacekit/rover.glb"
    );
    assert_eq!(scout.render_parts[0].translation, [-3.3, 0.0, -2.475]);
    assert_eq!(scout.render_parts[0].scale, [1.65, 1.65, 1.65]);

    for (left, right) in [
        ("Worker", "ScoutRover"),
        ("ScoutRover", "RocketInfantry"),
        ("ScoutRover", "ShieldTrooper"),
        ("RocketInfantry", "ShieldTrooper"),
        ("GrenadierTrooper", "RocketInfantry"),
        ("GrenadierTrooper", "RocketTrooperRobot"),
        ("RocketInfantry", "RocketTrooperRobot"),
    ] {
        assert_ne!(
            model_signature(left),
            model_signature(right),
            "{left} and {right} must have distinct model signatures"
        );
    }
}

#[test]
fn registry_render_part_models_exist_on_disk() {
    for entity in registry::ENTITY_DEFS {
        for part in entity.render_parts {
            assert!(
                std::path::Path::new("assets").join(part.model).exists(),
                "{} references missing model asset {}",
                entity.id,
                part.model
            );
        }
    }
}

#[test]
fn hunyuan_render_parts_have_forward_rotation_and_material_fallback() {
    let mut count = 0;
    for entity in registry::ENTITY_DEFS {
        for part in entity.render_parts {
            if !is_hunyuan_model_path(part.model) {
                continue;
            }
            count += 1;
            assert_eq!(
                part.rotation,
                [0.0, 1.0, 0.0, 0.0],
                "{} must rotate its Hunyuan mesh 180 degrees around Y so the generated front faces the RTS forward axis",
                entity.id
            );
            let material = hunyuan_model_material(entity.id);
            assert!(
                material.metallic > 0.5,
                "{} must use the runtime material fallback for mesh-only Hunyuan GLBs",
                entity.id
            );
        }
    }
    // 2026-07: the 14 Hunyuan stand-ins were retired in favor of the original
    // godot kenney part collages. The per-part guards above still protect any
    // future Hunyuan part; today none should exist in the registry.
    assert_eq!(
        count, 0,
        "unexpected Hunyuan render part in the registry — restore the godot \
         kenney composition or update the guards"
    );
}

#[test]
fn hunyuan_material_system_applies_to_loaded_scene_children() {
    let mut app = App::new();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<HunyuanModelMaterialCache>();
    app.add_systems(Update, apply_hunyuan_model_materials);

    let initial = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.02, 0.02),
            ..default()
        });
    let root = app
        .world_mut()
        .spawn(HunyuanModelPart {
            entity_id: "FlameAssaultBuggy",
        })
        .id();
    let child = app
        .world_mut()
        .spawn((ChildOf(root), MeshMaterial3d(initial.clone())))
        .id();

    app.update();

    let assigned = app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(child)
        .expect("child mesh material")
        .0
        .clone();
    assert_ne!(assigned, initial);
    assert!(app.world().get::<HunyuanModelMaterialized>(root).is_some());
    let material = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&assigned)
        .expect("assigned Hunyuan material");
    let color = material.base_color.to_srgba();
    assert!(
        color.red > color.green && color.red > color.blue,
        "FlameAssaultBuggy fallback should visibly read as a flame vehicle"
    );
}

#[test]
fn model_harness_ids_cover_registry_with_compiler_checked_length() {
    assert_eq!(
        MODEL_HARNESS_ENTITY_IDS.len(),
        registry::ENTITY_DEFS.len(),
        "MODEL_HARNESS_ENTITY_IDS has a const length check against ENTITY_DEFS; this runtime assertion keeps the failure message clear"
    );
    let mut seen = std::collections::BTreeSet::new();
    for id in MODEL_HARNESS_ENTITY_IDS {
        assert!(seen.insert(id), "duplicate model harness id {id}");
        assert!(
            registry::entity(id).is_some(),
            "model harness id {id} must resolve to an EntityDef"
        );
    }
}

#[test]
fn model_harness_roots_are_kind_typed_instances() {
    let mut app = App::new();
    let def = model_harness_entity_def(MODEL_HARNESS_ENTITY_IDS[0]);
    let root = spawn_model_harness_root(app.world_mut(), 0, def, Vec3::ZERO);

    let mut query = app.world_mut().query::<Instance<ModelHarnessRoot>>();
    let roots = query.iter(app.world()).collect::<Vec<_>>();

    assert_eq!(roots, vec![root]);
    let marker = app
        .world()
        .get::<ModelHarnessRoot>(root.entity())
        .expect("typed harness root marker");
    assert_eq!(marker.index, 0);
    assert_eq!(marker.id, def.id);
}

#[test]
fn headless_loading_state_skips_render_asset_preload() {
    let mut app = build_game_app(GameAppMode::Headless);
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        *app.world().resource::<State<AppScreen>>().get(),
        AppScreen::MainMenu
    );
    assert!(
        app.world()
            .resource::<StartupLoadingAssets>()
            .handles
            .is_empty(),
        "headless tests should not preload the full render asset set"
    );
}

#[test]
fn startup_loading_state_tracks_real_asset_handles_when_enabled() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(StartupLoadingPolicy {
        preload_assets: true,
    });

    app.update();

    let retained = app.world().resource::<StartupLoadingAssets>();
    assert!(
        retained.handles.len() > registry::ENTITY_DEFS.len(),
        "iyes_progress startup loading should track UI assets, icons, cursor assets, model-map data, and migrated model scenes"
    );
    let loading = app.world().resource::<AssetsLoading<AppScreen>>();
    assert!(
        !loading.allow_failures,
        "startup loading should fail loudly instead of silently skipping missing critical assets"
    );
    assert!(
        loading.track_dependencies,
        "startup loading should wait for GLB scene dependencies, not only top-level handles"
    );
}

#[test]
fn impact_burst_scales_with_damage_and_structures() {
    let infantry_hit = impact_burst_power(4.0, 0.45, false);
    let heavy_vehicle_hit = impact_burst_power(18.0, 0.8, false);
    let structure_hit = impact_burst_power(18.0, 1.4, true);

    assert!(
        infantry_hit >= 0.45,
        "small hits should still generate a readable impact burst"
    );
    assert!(
        heavy_vehicle_hit > infantry_hit,
        "higher damage should produce a larger impact burst"
    );
    assert!(
        structure_hit > heavy_vehicle_hit,
        "structure impacts should read heavier than same-damage unit impacts"
    );
    assert!(
        impact_burst_lifetime(structure_hit) > impact_burst_lifetime(infantry_hit),
        "larger bursts should stay visible for more frames"
    );
}

#[test]
fn impact_burst_kind_tracks_weapon_family() {
    let rifle = Weapon::new(4.0, 4.0, 0.8, 0.0, 0.0, 1.0, false, true);
    let rocket = Weapon::new(6.0, 10.0, 1.4, 1.0, 0.45, 1.2, true, true);

    assert_eq!(
        impact_burst_kind_for_entity_id("HeavyMachinegunTrooper", &rifle, false),
        ImpactBurstKind::Ballistic
    );
    assert_eq!(
        impact_burst_kind_for_entity_id("RocketInfantry", &rocket, false),
        ImpactBurstKind::Explosive
    );
    assert_eq!(
        impact_burst_kind_for_entity_id("TeslaCrawlerMk2", &rifle, false),
        ImpactBurstKind::Electric
    );
    assert_eq!(
        impact_burst_kind_for_entity_id("FlameAssaultBuggy", &rifle, false),
        ImpactBurstKind::Fire
    );
    assert_eq!(
        impact_burst_kind_for_entity_id("RailCannonBunker", &rifle, true),
        ImpactBurstKind::Heavy
    );
}

#[test]
fn worker_build_menu_orders_opening_before_late_tech() {
    let faction = faction_def(SkirmishFaction::Alliance).expect("alliance faction");
    let ordered = sorted_worker_build_structures(faction);
    let position = |id: &str| {
        ordered
            .iter()
            .position(|candidate| *candidate == id)
            .unwrap_or_else(|| panic!("missing build structure {id}"))
    };

    assert_eq!(ordered.first(), Some(&"PowerReactor"));
    assert!(position("PowerReactor") < position("Refinery"));
    assert!(position("Refinery") < position("Barracks"));
    assert!(position("Barracks") < position("VehicleFactory"));
    assert!(position("VehicleFactory") < position("TechLab"));
    assert!(position("TechLab") < position("WeatherControlSpire"));
}

#[test]
fn worker_build_menu_splits_production_and_defense_tabs() {
    assert_eq!(
        build_structure_tab_for("PowerReactor"),
        BuildStructureTab::Production
    );
    assert_eq!(
        build_structure_tab_for("Refinery"),
        BuildStructureTab::Production
    );
    assert_eq!(
        build_structure_tab_for("Barracks"),
        BuildStructureTab::Production
    );
    assert_eq!(
        build_structure_tab_for("VehicleFactory"),
        BuildStructureTab::Production
    );
    assert_eq!(
        build_structure_tab_for("WeatherControlSpire"),
        BuildStructureTab::Production
    );
    assert_eq!(
        build_structure_tab_for("AntiGroundTurret"),
        BuildStructureTab::Defense
    );
    assert_eq!(
        build_structure_tab_for("AntiAirTurret"),
        BuildStructureTab::Defense
    );
    assert_eq!(
        build_structure_tab_for("TeslaFenceSegment"),
        BuildStructureTab::Defense
    );
    assert_eq!(
        build_structure_tab_for("RailCannonBunker"),
        BuildStructureTab::Defense
    );
}

#[test]
fn worker_cargo_visual_slots_show_loaded_resources() {
    let empty = ResourceCargo {
        capacity: 6,
        ore: 0,
        crystal: 0,
    };
    assert!(harvest_cargo_visual_slots(empty).is_empty());

    let mixed = ResourceCargo {
        capacity: 8,
        ore: 4,
        crystal: 3,
    };
    assert_eq!(
        harvest_cargo_visual_slots(mixed),
        vec![
            ResourceKind::Ore,
            ResourceKind::Ore,
            ResourceKind::Ore,
            ResourceKind::Ore,
            ResourceKind::Crystal,
            ResourceKind::Crystal,
        ],
        "cargo visuals should show both resource kinds and cap to visible slots"
    );
}

#[test]
fn construction_work_pulse_only_lives_while_worker_is_actively_building_same_target() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(Update, update_construction_work_pulses);

    let target = app.world_mut().spawn_empty().id();
    let other_target = app.world_mut().spawn_empty().id();
    let worker = app
        .world_mut()
        .spawn((
            ConstructOrder { target },
            ConstructionWorkPulse {
                target,
                remaining: 0.5,
                total: 0.5,
                seed: 0.0,
            },
        ))
        .id();

    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_secs_f32(0.1));
    }
    app.update();
    assert!(
        app.world().get::<ConstructionWorkPulse>(worker).is_some(),
        "pulse should remain while the worker is still constructing the same target"
    );

    app.world_mut().entity_mut(worker).insert(ConstructOrder {
        target: other_target,
    });
    app.update();
    assert!(
        app.world().get::<ConstructionWorkPulse>(worker).is_none(),
        "pulse must clear when the construction order switches target"
    );
}

#[test]
fn construction_worker_model_scale_animates_and_resets() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(Update, animate_construction_workers);

    let target = app.world_mut().spawn_empty().id();
    let worker = app
        .world_mut()
        .spawn((
            Unit {
                id: "Worker",
                speed: 2.5,
                can_crush: false,
                can_be_crushed: false,
            },
            Transform::from_scale(Vec3::splat(1.0)),
            ConstructionWorkPulse {
                target,
                remaining: 0.5,
                total: 0.5,
                seed: 0.0,
            },
        ))
        .id();

    app.update();
    let animated_scale = app.world().get::<Transform>(worker).unwrap().scale;
    assert_ne!(
        animated_scale,
        Vec3::splat(1.0),
        "active construction should make the worker visibly move"
    );

    app.world_mut()
        .entity_mut(worker)
        .remove::<ConstructionWorkPulse>();
    app.update();
    assert_eq!(
        app.world().get::<Transform>(worker).unwrap().scale,
        Vec3::splat(1.0),
        "worker scale should reset once construction animation stops"
    );
}

#[test]
fn construction_sites_render_as_ghost_shell_then_restore_materials() {
    let mut app = App::new();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Update, apply_construction_ghost_material);

    let original = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    let root = app
        .world_mut()
        .spawn((
            Structure { id: "Barracks" },
            UnderConstruction {
                remaining: 8.0,
                total: 8.0,
                cost: registry::Cost::default(),
                free_worker_origin: None,
            },
        ))
        .id();
    let part = app
        .world_mut()
        .spawn((ChildOf(root), MeshMaterial3d(original.clone())))
        .id();

    app.update();
    let ghosted = app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(part)
        .unwrap()
        .0
        .clone();
    assert_ne!(
        ghosted, original,
        "under-construction parts must swap to the ghost material"
    );

    // A part that streams in later gets ghosted on a following frame too.
    let late_part = app
        .world_mut()
        .spawn((ChildOf(root), MeshMaterial3d(original.clone())))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(late_part)
            .unwrap()
            .0,
        ghosted,
        "late-loading parts join the ghost shell"
    );

    app.world_mut()
        .entity_mut(root)
        .remove::<UnderConstruction>();
    app.update();
    for entity in [part, late_part] {
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .unwrap()
                .0,
            original,
            "finished structures restore their real materials"
        );
    }
}

#[test]
fn kenney_gold_accents_get_repainted_in_the_player_color() {
    let mut app = App::new();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<TeamColorMaterials>();
    app.init_resource::<PlayerColorSlots>();
    app.add_systems(Update, apply_team_color_materials);

    let gold = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            // kenney "metalRed" accent as loaded from the GLBs (linear).
            base_color: Color::linear_rgb(1.0, 0.6285, 0.2028),
            ..default()
        });
    let hull = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::linear_rgb(0.843, 0.87, 0.91),
            ..default()
        });
    let root = app
        .world_mut()
        .spawn((Structure { id: "Barracks" }, Team::Player(0)))
        .id();
    let accent = app
        .world_mut()
        .spawn((ChildOf(root), MeshMaterial3d(gold.clone())))
        .id();
    let body = app
        .world_mut()
        .spawn((ChildOf(root), MeshMaterial3d(hull.clone())))
        .id();

    app.update();
    let painted = app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(accent)
        .unwrap()
        .0
        .clone();
    assert_ne!(painted, gold, "gold accents must swap to the team material");
    assert_eq!(
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(body)
            .unwrap()
            .0,
        hull,
        "non-accent materials stay untouched"
    );
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let team_color = materials.get(&painted).unwrap().base_color;
    let expected = player_color(
        app.world()
            .resource::<PlayerColorSlots>()
            .slot(Team::Player(0))
            .unwrap(),
    );
    assert_eq!(
        team_color, expected,
        "repaint uses the player palette color"
    );
}

#[test]
fn unit_limbs_animate_by_activity_and_return_to_rest() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(Update, animate_unit_limbs);

    let worker = app
        .world_mut()
        .spawn((
            Unit {
                id: "Worker",
                speed: 2.5,
                can_crush: false,
                can_be_crushed: true,
            },
            HarvestOrder {
                resource: None,
                state: HarvestState::Collecting,
                collect_remaining: 1.0,
                last_kind: None,
            },
        ))
        .id();
    let rest = Transform::from_xyz(0.13, 0.47, 0.0);
    let limb = |kind, root| {
        (
            Limb {
                kind,
                root,
                rest_translation: rest.translation,
                rest_rotation: rest.rotation,
                seed: 0.0,
            },
            rest,
        )
    };
    let arm = app.world_mut().spawn(limb(LimbKind::ArmRight, worker)).id();
    let leg = app.world_mut().spawn(limb(LimbKind::LegLeft, worker)).id();

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(400));
    app.update();
    assert_ne!(
        app.world().get::<Transform>(arm).unwrap().rotation,
        rest.rotation,
        "collecting workers must swing their arm"
    );
    assert_eq!(
        app.world().get::<Transform>(leg).unwrap().rotation,
        rest.rotation,
        "legs stay planted while standing at the node"
    );

    // Walking: legs stride and arms swing in the gait.
    app.world_mut().entity_mut(worker).remove::<HarvestOrder>();
    app.world_mut().entity_mut(worker).insert(MoveOrder {
        target: Vec3::new(5.0, 0.0, 0.0),
    });
    app.update();
    assert_ne!(
        app.world().get::<Transform>(leg).unwrap().rotation,
        rest.rotation,
        "moving units must stride"
    );

    // Idle: everything snaps back to the exact rest pose.
    app.world_mut().entity_mut(worker).remove::<MoveOrder>();
    app.update();
    for entity in [arm, leg] {
        let pose = *app.world().get::<Transform>(entity).unwrap();
        assert_eq!(pose.rotation, rest.rotation);
        assert_eq!(pose.translation, rest.translation);
    }
}

#[test]
fn turret_nodes_traverse_toward_the_attack_target() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(Update, animate_turret_nodes);

    let target = app
        .world_mut()
        .spawn(GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 0.0)))
        .id();
    // Tower at the origin facing -Z (identity): the target sits 90° to its left.
    let tower = app
        .world_mut()
        .spawn((
            Structure {
                id: "AntiGroundTurret",
            },
            GlobalTransform::IDENTITY,
            AttackOrder { target },
        ))
        .id();
    let turret = app
        .world_mut()
        .spawn((
            TurretNode {
                root: tower,
                rest_translation: Vec3::new(0.0, 0.35, 0.0),
                rest_rotation: Quat::IDENTITY,
                yaw: 0.0,
            },
            Transform::from_xyz(0.0, 0.35, 0.0),
        ))
        .id();

    // Several frames of traverse: yaw must move toward the -PI/2 bearing.
    for _ in 0..8 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
    }
    let yaw = app.world().get::<TurretNode>(turret).unwrap().yaw;
    assert!(
        (yaw - (-std::f32::consts::FRAC_PI_2)).abs() < 0.05,
        "turret should traverse to face the target, yaw={yaw}"
    );
    assert_ne!(
        app.world().get::<Transform>(turret).unwrap().rotation,
        Quat::IDENTITY,
        "the turret node visibly rotates"
    );

    // Target gone: the turret eases back to center.
    app.world_mut().entity_mut(tower).remove::<AttackOrder>();
    for _ in 0..12 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
    }
    let yaw = app.world().get::<TurretNode>(turret).unwrap().yaw;
    assert!(yaw.abs() < 0.01, "turret returns to center, yaw={yaw}");
}

#[test]
fn chimneys_puff_smoke_that_rises_and_expires() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Update, (emit_chimney_smoke, update_smoke_puffs).chain());

    let structure = app.world_mut().spawn(Structure { id: "PowerReactor" }).id();
    app.world_mut().spawn((
        ChimneyVent {
            root: structure,
            next_emit: 0.1,
        },
        GlobalTransform::from(Transform::from_xyz(3.0, 0.0, 4.0)),
    ));

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(200));
    app.update();
    app.update(); // command flush -> puff visible to queries
    let mut puffs = app.world_mut().query::<(&SmokePuff, &Transform)>();
    let (_, spawn_tf) = puffs.single(app.world()).expect("a puff should spawn");
    let spawn_y = spawn_tf.translation.y;
    assert!(
        spawn_y > 1.0,
        "puff spawns at the chimney mouth, y={spawn_y}"
    );

    // Halfway through life it has risen; past ttl it despawns.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(800));
    app.update();
    let (_, risen_tf) = puffs.single(app.world()).expect("puff still alive");
    assert!(risen_tf.translation.y > spawn_y, "smoke rises");
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(3));
    app.update();
    app.update();
    // The original puff expired; anything alive is from a newer emit cycle.
    for (puff, _) in puffs.iter(app.world()) {
        assert!(puff.age < puff.ttl, "expired puffs must despawn");
    }
}

#[test]
fn weapon_fire_kick_decays_after_the_shot() {
    let mut weapon = Weapon::new(6.0, 2.0, 1.5, 0.0, 1.0, 1.0, true, true);
    weapon.cooldown_left = 1.5; // just fired
    assert!(weapon_fire_kick(&weapon) > 0.9);
    weapon.cooldown_left = 1.2; // 0.3s later, past the kick window
    assert_eq!(weapon_fire_kick(&weapon), 0.0);
    weapon.cooldown_left = 0.0; // ready to fire again
    assert_eq!(weapon_fire_kick(&weapon), 0.0);
}

#[test]
fn structure_emerges_from_ground_with_construction_progress() {
    let mut app = App::new();
    app.add_systems(Update, animate_structure_construction);
    let base_scale = registry::entity("Barracks").unwrap().scale;

    let structure = app
        .world_mut()
        .spawn((
            Structure { id: "Barracks" },
            Transform::from_scale(Vec3::splat(base_scale)),
            UnderConstruction {
                remaining: 8.0,
                total: 8.0,
                cost: registry::Cost::default(),
                free_worker_origin: None,
            },
        ))
        .id();

    app.update();
    let fresh = app.world().get::<Transform>(structure).unwrap().scale;
    assert!(
        fresh.y < base_scale * 0.2,
        "a fresh foundation starts near the ground, got y={}",
        fresh.y
    );
    assert!(
        (fresh.x - base_scale).abs() < 0.001,
        "footprint width stays constant while building"
    );

    app.world_mut()
        .entity_mut(structure)
        .get_mut::<UnderConstruction>()
        .unwrap()
        .remaining = 4.0;
    app.update();
    let halfway = app.world().get::<Transform>(structure).unwrap().scale;
    assert!(
        halfway.y > fresh.y && halfway.y < base_scale,
        "half-built structures sit between foundation and full height"
    );

    app.world_mut()
        .entity_mut(structure)
        .remove::<UnderConstruction>();
    app.update();
    assert_eq!(
        app.world().get::<Transform>(structure).unwrap().scale,
        Vec3::splat(base_scale),
        "finished structures snap back to their registry scale"
    );
}

#[test]
fn right_click_resource_orders_only_workers_to_harvest() {
    let resource = Entity::from_raw_u32(7).unwrap();
    let choices = OrderTargetChoices {
        supply_crate_position: None,
        resource_target: Some(resource),
        resource_dropoff_target: None,
        enemy_target: None,
        repair_target: None,
        construct_target: None,
        garrison_target: None,
        follow_target: None,
    };
    let context = UnitOrderContext {
        force_move: false,
        enemy_target_capturable: false,
        attack_move: false,
        patrol: false,
        origin: Vec3::ZERO,
        point: Vec3::new(4.0, 0.0, 5.0),
        offset: Vec3::ZERO,
    };
    let worker = Unit {
        id: "Worker",
        speed: 2.0,
        can_crush: false,
        can_be_crushed: true,
    };
    let tank = Unit {
        id: "Tank",
        speed: 2.0,
        can_crush: true,
        can_be_crushed: false,
    };

    assert!(matches!(
        desired_order_for_selected_unit(&worker, choices, context),
        Some(UnitQueuedOrder::Harvest {
            target,
            state: HarvestState::MovingToResource
        }) if target == resource
    ));
    assert!(
        !matches!(
            desired_order_for_selected_unit(&tank, choices, context),
            Some(UnitQueuedOrder::Harvest { .. })
        ),
        "combat units should not become resource collectors when right-clicking ore"
    );
}

// An idle defense structure (weapon off cooldown) sweeps over time; one that
// is mid-engagement (cooldown_left > 0) or still under construction stays put.
#[test]
fn idle_defense_tower_scans_when_not_engaging() {
    fn final_yaw(cooldown_left: f32, under_construction: bool) -> f32 {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.add_systems(Update, update_idle_tower_scan);
        let mut entity = app.world_mut().spawn((
            Structure {
                id: "AntiGroundTurret",
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            Weapon {
                range: 10.0,
                damage: 5.0,
                cooldown: 1.0,
                splash_radius: 0.0,
                splash_damage_multiplier: 0.0,
                structure_damage_multiplier: 1.0,
                cooldown_left,
                can_attack_air: false,
                can_attack_ground: true,
            },
        ));
        if under_construction {
            entity.insert(UnderConstruction {
                remaining: 5.0,
                total: 5.0,
                cost: registry::Cost { ore: 0, crystal: 0 },
                free_worker_origin: None,
            });
        }
        let id = entity.id();
        // Advance several frames of simulated time so the scan accumulates.
        for _ in 0..30 {
            let mut time = app.world_mut().resource_mut::<Time<()>>();
            time.advance_by(std::time::Duration::from_secs_f32(0.1));
            app.update();
        }
        app.world()
            .get::<Transform>(id)
            .unwrap()
            .rotation
            .to_euler(EulerRot::YXZ)
            .0
    }

    assert!(
        final_yaw(0.0, false).abs() > 0.01,
        "idle tower should sweep (yaw should change)"
    );
    assert!(
        final_yaw(0.5, false).abs() < 1e-6,
        "engaging tower (on cooldown) should not sweep"
    );
    assert!(
        final_yaw(0.0, true).abs() < 1e-6,
        "tower under construction should not sweep"
    );
}

#[test]
fn active_ai_iteration_uses_all_runtime_players_not_three_slots() {
    let active = ActiveTeams(vec![true, true, true, true, true, true]);

    let teams = active_ai_teams(Some(Team::Player(0)), Some(&active)).collect::<Vec<_>>();

    assert_eq!(
        teams,
        vec![
            Team::Player(1),
            Team::Player(2),
            Team::Player(3),
            Team::Player(4),
            Team::Player(5),
        ]
    );
}

#[test]
fn ai_drone_scouting_moves_idle_ai_drones() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }

    let drone_def = registry::entity("Drone").expect("Drone must stay in the registry");
    let before = Vec3::new(12.0, 0.0, 12.0);
    let drone = app
        .world_mut()
        .spawn((
            Unit {
                id: "Drone",
                speed: drone_def.speed,
                can_crush: drone_def.can_crush,
                can_be_crushed: drone_def.can_be_crushed,
            },
            Team::Player(1),
            Transform::from_translation(before),
            Selectable {
                radius: drone_def.radius,
            },
            Health::new(drone_def.health),
            VisionRadius(unit_vision_radius(drone_def)),
            MovementDomain::from_registry(drone_def.domain),
            VisibilityState { visible: true },
            Visibility::Visible,
            MatchScopedEntity,
        ))
        .id();

    for _ in 0..180 {
        app.update();
    }

    let world = app.world();
    let after = world
        .get::<Transform>(drone)
        .expect("AI Drone should still exist")
        .translation;
    let distance = xz_distance(before, after);
    eprintln!("[diag] AI Drone scout moved {distance:.2}m");
    assert!(
        distance > 0.5,
        "AI Drone did not leave its spawn point for scouting"
    );
    assert!(
        world
            .get::<AiDroneScout>(drone)
            .is_some_and(|scout| scout.last_target.is_some()),
        "AI Drone should remember the current scout target"
    );
}

#[test]
fn ai_defense_profile_limits_match_godot_difficulty_targets() {
    let easy = faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Easy);
    assert!(
        easy.defense_priority.is_empty(),
        "Easy AI should not inherit the Normal defense build queue"
    );
    assert!(
        easy.active_offense_enabled,
        "Easy AI attacks gently (small waves) so it threatens more than Beginner"
    );
    assert!(
        easy.opening_attack_grace >= 120.0,
        "Easy AI must leave a long build-up window before its first wave"
    );
    assert_eq!(
        ai_battlegroup_target_units(&easy),
        3,
        "Easy AI trains one small battlegroup"
    );
    assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &easy), 0);
    assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &easy), 0);

    let normal = faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Normal);
    assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &normal), 1);
    assert_eq!(ai_structure_profile_limit("AntiAirTurret", &normal), 1);
    assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &normal), 2);
    assert_eq!(
        ai_structure_profile_limit("ArcCoilDefenseTower", &normal),
        1
    );
    assert_eq!(
        ai_structure_profile_limit("PrismDefenseObelisk", &normal),
        1
    );
    assert_eq!(ai_structure_profile_limit("RailCannonBunker", &normal), 1);

    let hard = faction_ai_profile_for_difficulty(SkirmishFaction::Alliance, AiDifficulty::Hard);
    assert_eq!(ai_structure_profile_limit("AntiGroundTurret", &hard), 2);
    assert_eq!(ai_structure_profile_limit("AntiAirTurret", &hard), 2);
    assert_eq!(ai_structure_profile_limit("TeslaFenceSegment", &hard), 4);
    assert_eq!(ai_structure_profile_limit("ArcCoilDefenseTower", &hard), 2);
    assert_eq!(ai_structure_profile_limit("PrismDefenseObelisk", &hard), 2);
    assert_eq!(ai_structure_profile_limit("RailCannonBunker", &hard), 2);

    let chaos_normal =
        faction_ai_profile_for_difficulty(SkirmishFaction::Chaos, AiDifficulty::Normal);
    let chaos_hard = faction_ai_profile_for_difficulty(SkirmishFaction::Chaos, AiDifficulty::Hard);
    assert_eq!(
        ai_structure_profile_limit("TeslaFenceSegment", &chaos_normal),
        2
    );
    assert_eq!(
        ai_structure_profile_limit("TeslaFenceSegment", &chaos_hard),
        4
    );
}

#[test]
fn godot_skirmish_startup_uses_worker_only_economy() {
    let mut flavor_units = Vec::new();
    for faction in SkirmishFaction::ALL {
        let startup = faction_startup_for_loadout(faction, StartupLoadoutMode::GodotSkirmish);
        assert_eq!(
            startup.structures,
            &[SpawnSpec {
                id: "CommandCenter",
                offset: (0.0, 0.0),
            }],
            "{} should keep the minimal one-base skirmish opening",
            faction.label()
        );
        assert_eq!(
            startup
                .units
                .iter()
                .filter(|spec| spec.id == "Worker")
                .count(),
            2,
            "{} should still start with two worker economy units",
            faction.label()
        );
        assert!(
            startup
                .units
                .iter()
                .all(|spec| !matches!(spec.id, "OreHarvester" | "MobileConstructionVehicle")),
            "{} must not reintroduce a separate vehicle collector/builder economy start",
            faction.label()
        );
        let flavor = startup
            .units
            .iter()
            .find(|spec| spec.id != "Worker")
            .map(|spec| spec.id)
            .expect("each faction should have a visible faction-specific starter");
        flavor_units.push(flavor);
    }
    flavor_units.sort_unstable();
    flavor_units.dedup();
    assert_eq!(
        flavor_units.len(),
        SkirmishFaction::ALL.len(),
        "each faction should have a distinct starter unit in the default cargo-run opening"
    );
}

#[test]
fn start_camera_focus_prefers_command_center_over_worker() {
    let startup =
        faction_startup_for_loadout(SkirmishFaction::Alliance, StartupLoadoutMode::GodotSkirmish);
    assert_eq!(
        startup_camera_focus_offset(startup),
        Vec3::ZERO,
        "default cargo-run camera should start over the CommandCenter, not the worker offset"
    );
}

#[test]
fn dynamic_team_relations_and_start_status_work_beyond_three_players() {
    let active = vec![true, true, true, true, true, true];
    let team_ids = vec![0, 1, 2, 0, 1, 2];
    let relations = skirmish_team_relations_from_team_ids(&active, &team_ids);

    assert!(relations.are_allied(Team::Player(0), Team::Player(3)));
    assert!(relations.are_allied(Team::Player(1), Team::Player(4)));
    assert!(relations.are_enemies(Team::Player(0), Team::Player(4)));
    assert_eq!(
        skirmish_start_status_for_setup(6, 8, &active, &relations),
        SkirmishStartStatus::Ready
    );
}

#[test]
fn lobby_team_ids_and_capture_teams_are_not_limited_to_three() {
    assert_eq!(
        DEFAULT_LOBBY_TEAM_IDS,
        [0, 1, 2, 3, 4, 5, 6, 7],
        "default lobby rows should not fold 8 players into three teams"
    );
    assert_eq!(
        SKIRMISH_TEAM_OPTION_COUNT as usize,
        MAX_SKIRMISH_LOBBY_SLOTS
    );
}

#[test]
fn runtime_team_ids_from_relations_do_not_clamp_after_lobby_slot_count() {
    let active = vec![true; 12];
    let mut relations = TeamRelations::default();
    relations.ensure_player_count(active.len());

    let team_ids = skirmish_team_ids_from_relations(&active, &relations);

    assert_eq!(team_ids, (0..12).collect::<Vec<_>>());

    let relation_ids = (0..12).collect::<Vec<_>>();
    let relations = skirmish_team_relations_from_team_ids(&active, &relation_ids);
    assert!(relations.are_enemies(Team::Player(0), Team::Player(8)));
    assert!(relations.are_enemies(Team::Player(8), Team::Player(11)));

    let mut allied_ids = (0..12).collect::<Vec<_>>();
    allied_ids[11] = 8;
    let allied_relations = skirmish_team_relations_from_team_ids(&active, &allied_ids);
    assert!(allied_relations.are_allied(Team::Player(8), Team::Player(11)));
}

#[test]
fn runtime_resources_expand_for_late_player_slots() {
    let late_team = Team::Player(11);
    let mut economies = Economies::default();
    assert!(economies.players.is_empty());
    economies.get_mut(late_team).ore = 1234;
    assert_eq!(economies.get(late_team).ore, 1234);

    let mut director = AiDirector::default();
    assert!(director.attack_timer.is_empty());
    let late_index = director
        .ensure_team(late_team)
        .expect("late player slots should be valid runtime teams");
    director.attack_timer[late_index] = 0.25;
    assert_eq!(director.attack_timer[late_index], 0.25);

    let mut support_cooldowns = SupportCooldowns::default();
    assert!(support_cooldowns.remaining.is_empty());
    support_cooldowns.set(late_team, SupportPowerKind::Paradrop, 9.0);
    assert_eq!(
        support_cooldowns.remaining_for(late_team, SupportPowerKind::Paradrop),
        9.0
    );
}

#[test]
fn runtime_team_helpers_do_not_wrap_after_lobby_slot_count() {
    let active = ActiveTeams(vec![true; 16]);
    let teams = active_ai_teams(Some(Team::Player(0)), Some(&active)).collect::<Vec<_>>();
    assert_eq!(teams.len(), 15);
    assert!(teams.contains(&Team::Player(15)));

    assert_eq!(
        default_skirmish_opponent(Team::Player(12), 16),
        Some(Team::Player(0))
    );
    assert_eq!(
        allied_skirmish_ally(Team::Player(12), 16),
        Some(Team::Player(0))
    );
    assert_eq!(
        allied_skirmish_enemy(Team::Player(12), 16),
        Some(Team::Player(1))
    );
    assert_eq!(Team::Neutral.index(), usize::MAX);
    assert!(xz_distance(team_home(Team::Player(0)), team_home(Team::Player(8))) > 1.0);
    assert!(xz_distance(team_home(Team::Player(8)), team_home(Team::Player(16))) > 1.0);
}

#[test]
fn virtual_spawn_positions_exist_after_map_spawn_slots() {
    let map = largest_skirmish_map();
    let bounds = MapBounds::from_map(map);
    let first_extra = team_start_position_for_spawn_slot(map, map.players);
    let second_extra = team_start_position_for_spawn_slot(map, map.players + 1);

    assert!(bounds.contains_ground_point(first_extra));
    assert!(bounds.contains_ground_point(second_extra));
    assert!(xz_distance(first_extra, second_extra) > 1.0);
    assert!(xz_distance(first_extra, team_start_position_for_spawn_slot(map, 0)) > 1.0);
}

// Fast (no-render) diagnostic for the human core loop: what units does the
// player start with, and do they actually move when ordered to attack-move?
#[test]
fn diag_player_units_exist_and_move() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }

    let player = Team::Player(0);
    let snapshot = |app: &mut App| -> Vec<(Entity, &'static str, Vec3)> {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(Entity, &Unit, &Team, &Transform), ()>();
        q.iter(world)
            .filter(|(_, _, team, _)| **team == player)
            .map(|(e, unit, _, tf)| (e, unit.id, tf.translation))
            .collect()
    };

    let before = snapshot(&mut app);
    eprintln!("[diag] player starts with {} units:", before.len());
    for (_, id, pos) in &before {
        let can_attack = registry::entity(id)
            .map(|d| d.weapon.is_some())
            .unwrap_or(false);
        eprintln!("  {id}  weapon={can_attack}  @ ({:.1},{:.1})", pos.x, pos.z);
    }

    assert!(!before.is_empty(), "player should start with units");
}

// Fast (no-render) observation of a full default match (Human P0 vs Easy AI
// P1): does the economy grow, does the AI build an army and attack, does the
// match progress toward a result? Uses a fixed timestep so game-time is real.
#[test]
fn diag_match_economy_and_ai_progress() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    // Minimal start has workers only — you build a Refinery for one. Spectate
    // so the AI drives P0 too: it builds a refinery, the free P0 worker
    // spawns, and (per the auto-harvest fix) that player-team worker
    // auto-harvests so P0 ore grows. Guards the player-team auto-harvest path.
    app.world_mut()
        .insert_resource(VisiblePlayer::spectator_per_player(Team::Player(0)));

    let sample = |app: &mut App| {
        let world = app.world_mut();
        let mut units = world.query::<(&Team, &Unit)>();
        let (mut p0, mut p1) = (0u32, 0u32);
        let mut p0_battle = 0u32;
        for (team, unit) in units.iter(world) {
            match *team {
                Team::Player(0) => {
                    p0 += 1;
                    if ai_battle_unit_id(unit.id) {
                        p0_battle += 1;
                    }
                }
                Team::Player(1) => p1 += 1,
                _ => {}
            }
        }
        let _ = p0_battle;
        let econ = world.resource::<Economies>();
        let (ore0, ore1) = (econ.get(Team::Player(0)).ore, econ.get(Team::Player(1)).ore);
        let phase = world.resource::<MatchState>().phase;
        (p0, p1, ore0, ore1, phase, p0_battle)
    };

    eprintln!("[diag] t(s) | P0_units P1_units | P0_ore P1_ore | phase");
    let mut start_ore = None;
    let mut peak_ore = 0;
    for step in 0..=24 {
        // 5 game-seconds per step at 1/30s per tick = 150 ticks.
        if step > 0 {
            for _ in 0..150 {
                app.update();
            }
        }
        let (p0, p1, o0, o1, phase, p0_battle) = sample(&mut app);
        start_ore.get_or_insert(o0);
        peak_ore = peak_ore.max(o0);
        eprintln!(
            "[diag] {:>4} | P0 {p0:>2} (army {p0_battle:>2}) P1 {p1:>2} | ore {o0:>4} {o1:>4} | {phase:?}",
            step * 5
        );
        if !matches!(phase, MatchPhase::Running) {
            break;
        }
    }
    // Regression guard: a player-team worker (from a built Refinery) must
    // auto-harvest, so P0 ore rises above its start at some point even though
    // the AI also spends it. Guards the auto-harvest fix that previously
    // excluded the player team (which left the human economy dead).
    assert!(
        peak_ore > start_ore.unwrap(),
        "player ore never grew (start {}, peak {peak_ore}): worker not auto-harvesting",
        start_ore.unwrap()
    );
}

// End-to-end loop: with both sides AI (spectator), does a full match actually
// resolve to a victory/defeat (economy -> army -> combat -> result)?
#[test]
fn diag_ai_vs_ai_match_resolves() {
    let resolved = capture_run_ai_match_until_resolved(240);
    match resolved {
        Some((secs, phase)) => eprintln!("[diag] AI-vs-AI resolved at ~{secs}s: {phase}"),
        None => eprintln!("[diag] AI-vs-AI still Running after 240s (possible stalemate)"),
    }
    assert!(
        resolved.is_some(),
        "AI-vs-AI did not resolve within 240s; economy/combat loop is not a completed match"
    );
}

// Each of the 3 factions must be fully playable: from its own base, its
// production chain must build an army. Same-faction AI-vs-AI per faction.
#[test]
fn diag_all_three_factions_playable() {
    for faction in SkirmishFaction::ALL {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        {
            let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
            for slot in settings.player_factions.iter_mut() {
                *slot = faction;
            }
        }
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..20 {
            app.update();
        }
        app.world_mut()
            .insert_resource(VisiblePlayer::spectator_per_player(Team::Player(0)));

        let count_units = |app: &mut App, team: Team| {
            let world = app.world_mut();
            let mut q = world.query_filtered::<&Team, With<Unit>>();
            q.iter(world).filter(|t| **t == team).count()
        };
        let start_units = count_units(&mut app, Team::Player(0));
        let mut peak = start_units;
        let mut resolved = false;
        for _ in 0..72 {
            for _ in 0..150 {
                app.update();
            }
            peak = peak.max(count_units(&mut app, Team::Player(0)));
            if !matches!(
                app.world().resource::<MatchState>().phase,
                MatchPhase::Running
            ) {
                resolved = true;
                break;
            }
        }
        eprintln!(
            "[diag] {:>8}: start {start_units} units, peak {peak}, resolved={resolved}",
            faction.label()
        );
        assert!(
            peak > start_units,
            "{} produced no army (production chain broken)",
            faction.label()
        );
    }
}

// Lobby slots must be closable in 1-2 clicks (RA2/Warcraft style): the
// per-slot controller cycles 关闭 -> 我方 -> 电脑 -> 关闭.
#[test]
fn lobby_slot_closes_in_one_cycle_from_ai() {
    let mut sel = SkirmishMenuSelection::default();
    let slot = 1; // default slot 1 is an AI slot, within every map.
    sel.set_lobby_slot_controller(slot, SkirmishPlayerController::Ai(AiDifficulty::Easy));
    sel.cycle_lobby_slot_controller(slot); // AI -> 关闭
    assert_eq!(
        sel.lobby_controllers[slot],
        SkirmishPlayerController::None,
        "an AI slot must close in a single cycle"
    );
    sel.cycle_lobby_slot_controller(slot); // 关闭 -> 我方
    assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::Human);
    sel.cycle_lobby_slot_controller(slot); // 我方 -> 电脑
    assert!(matches!(
        sel.lobby_controllers[slot],
        SkirmishPlayerController::Ai(_)
    ));
    sel.cycle_lobby_slot_controller(slot); // 电脑 -> 关闭 (closeable again)
    assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::None);
}

// Manual harvesting must work for the human player (who no longer auto-harvests):
// issuing a harvest order to a player Worker should make it gather ore and
// deposit at the CommandCenter, so P0 ore grows.
#[test]
fn manual_harvest_grows_player_ore() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    let player = Team::Player(0);

    // Find a player worker and the nearest resource node.
    let worker = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(Entity, &Team, &Unit), ()>();
        q.iter(world)
            .find(|(_, t, u)| **t == player && can_unit_collect_resources(u))
            .map(|(e, _, _)| e)
    }
    .expect("player should have a collector-capable worker");
    let (node, node_pos) = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Transform, &ResourceNode)>();
        q.iter(world)
            .find(|(_, _, n)| n.amount > 0)
            .map(|(e, tf, _)| (e, tf.translation))
    }
    .expect("map should have a resource node");
    // Make sure the node is visible so it's a legal manual target.
    let _ = node_pos;

    let ore_before = app.world().resource::<Economies>().get(player).ore;
    app.world_mut().entity_mut(worker).insert(HarvestOrder {
        resource: Some(node),
        state: HarvestState::MovingToResource,
        collect_remaining: 0.0,
        last_kind: None,
    });
    for _ in 0..1200 {
        app.update();
    }
    let ore_after = app.world().resource::<Economies>().get(player).ore;
    eprintln!("[diag] manual harvest: P0 ore {ore_before} -> {ore_after}");
    assert!(
        ore_after > ore_before,
        "manual harvest did not grow ore ({ore_before} -> {ore_after})"
    );
}

// Lobby controller dropdown: toggling opens the option list, picking an option
// sets the controller and closes the dropdown (no cycling).
#[test]
fn lobby_controller_dropdown_opens_and_sets() {
    let mut sel = SkirmishMenuSelection::default();
    let slot = 1;
    assert_eq!(sel.controller_dropdown_open, None);
    sel.toggle_controller_dropdown(slot);
    assert_eq!(
        sel.controller_dropdown_open,
        Some(slot),
        "toggle should open"
    );
    sel.set_lobby_slot_controller_choice(slot, SkirmishPlayerController::None);
    assert_eq!(
        sel.lobby_controllers[slot],
        SkirmishPlayerController::None,
        "picking 关闭 should close the slot"
    );
    assert_eq!(
        sel.controller_dropdown_open, None,
        "picking an option should close the dropdown"
    );
    sel.toggle_controller_dropdown(slot);
    sel.set_lobby_slot_controller_choice(slot, SkirmishPlayerController::Human);
    assert_eq!(sel.lobby_controllers[slot], SkirmishPlayerController::Human);
}

#[test]
fn map_and_resource_choices_close_dropdowns() {
    let mut sel = SkirmishMenuSelection::default();
    sel.toggle_map_dropdown();
    assert!(sel.map_dropdown_open);
    sel.set_map_choice(1);
    assert_eq!(sel.map_index, 1);
    assert!(!sel.map_dropdown_open);
    assert!(!sel.resources_dropdown_open);

    sel.toggle_resources_dropdown();
    assert!(sel.resources_dropdown_open);
    sel.set_starting_resource_choice(0);
    assert_eq!(sel.starting_resource_index, 0);
    assert!(!sel.map_dropdown_open);
    assert!(!sel.resources_dropdown_open);
}

#[test]
fn setup_menu_rebuilds_map_and_resource_dropdown_options() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::SkirmishSetup);
    for _ in 0..8 {
        app.update();
    }

    let count_buttons = |app: &mut App, predicate: fn(MainMenuAction) -> bool| -> usize {
        let world = app.world_mut();
        let mut buttons = world.query::<&MainMenuButton>();
        buttons
            .iter(world)
            .filter(|button| predicate(button.action))
            .count()
    };

    assert_eq!(
        count_buttons(&mut app, |action| matches!(
            action,
            MainMenuAction::SelectMap(_)
        )),
        0,
        "closed map dropdown should not render stale map options"
    );
    app.world_mut()
        .resource_mut::<SkirmishMenuSelection>()
        .toggle_map_dropdown();
    app.update();
    assert_eq!(
        count_buttons(&mut app, |action| matches!(
            action,
            MainMenuAction::SelectMap(_)
        )),
        SKIRMISH_MAPS.len() + 1,
        "opening map dropdown should render every map plus random"
    );

    app.world_mut()
        .resource_mut::<SkirmishMenuSelection>()
        .toggle_resources_dropdown();
    app.update();
    assert_eq!(
        count_buttons(&mut app, |action| matches!(
            action,
            MainMenuAction::SelectMap(_)
        )),
        0,
        "opening resources dropdown should close map options"
    );
    assert_eq!(
        count_buttons(&mut app, |action| matches!(
            action,
            MainMenuAction::SelectStartingResources(_)
        )),
        GODOT_STARTING_RESOURCE_OPTIONS.len(),
        "opening resources dropdown should render every resource preset"
    );
}

// Faction dropdown: toggling opens it (and closes the controller dropdown);
// picking a faction sets it and closes the dropdown.
#[test]
fn lobby_faction_dropdown_opens_and_sets() {
    let mut sel = SkirmishMenuSelection::default();
    let slot = 1;
    sel.toggle_controller_dropdown(slot);
    sel.toggle_faction_dropdown(slot);
    assert_eq!(
        sel.faction_dropdown_open,
        Some(slot),
        "faction toggle opens"
    );
    assert_eq!(
        sel.controller_dropdown_open, None,
        "opening faction closes the controller dropdown"
    );
    sel.set_lobby_slot_faction_choice(slot, SkirmishFaction::Chaos);
    assert_eq!(sel.lobby_factions[slot], SkirmishFaction::Chaos);
    assert_eq!(
        sel.faction_dropdown_open, None,
        "picking closes the dropdown"
    );
}

#[test]
fn empty_battle_log_does_not_swallow_world_clicks() {
    // The passive battle-log toasts must never block world clicks — an
    // upper-center point stays free whether or not the log has entries, so the
    // player can always issue orders there.
    let point = Vec2::new(619.0, 210.0);
    let empty = HudHitZones {
        world_rects: hud_world_input_rects(1280.0, 720.0, 0, 0, 0, 0, false),
    };
    assert!(!empty.blocks_world(point));
    let with_log = HudHitZones {
        world_rects: hud_world_input_rects(1280.0, 720.0, 0, 2, 0, 0, false),
    };
    assert!(
        !with_log.blocks_world(point),
        "battle-log toasts must not carve a dead zone out of the playfield"
    );
}

#[test]
fn bottom_world_clicks_pass_outside_actual_hud_panels() {
    // The harvest harness right-clicked ore at (671.5, 584.6); the old
    // full-width bottom band swallowed it. With one command row + selection
    // panel visible, that point is open world and must NOT be blocked.
    let zones = HudHitZones {
        world_rects: hud_world_input_rects(1280.0, 720.0, 0, 0, 4, 0, true),
    };
    let ore_click = Vec2::new(671.5, 584.6);
    assert!(!zones.blocks_world(ore_click));
    // Inside the actual command card (bottom-right) it must block.
    assert!(zones.blocks_world(Vec2::new(1000.0, 700.0)));
    // Inside the minimap (bottom-left) it must block.
    assert!(zones.blocks_world(Vec2::new(80.0, 650.0)));
    // Inside the selection panel (bottom-center) it must block.
    assert!(zones.blocks_world(Vec2::new(400.0, 700.0)));
}

#[test]
fn headquarters_mode_falls_when_command_centers_fall() {
    for (condition, expect_running) in [
        (VictoryCondition::Headquarters, false),
        (VictoryCondition::Annihilation, true),
    ] {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<MatchSetupSettings>()
            .victory_condition = condition;
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..40 {
            app.update();
        }
        // Kill the player's command centers; workers stay alive.
        let doomed: Vec<Entity> = {
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &Structure, &Team)>();
            q.iter(world)
                .filter(|(_, structure, team)| {
                    **team == Team::Player(0) && structure.id == "CommandCenter"
                })
                .map(|(entity, _, _)| entity)
                .collect()
        };
        assert!(!doomed.is_empty());
        for entity in doomed {
            app.world_mut().entity_mut(entity).despawn();
        }
        for _ in 0..10 {
            app.update();
        }
        let running = app.world().resource::<MatchState>().is_running();
        assert_eq!(
            running, expect_running,
            "victory condition {condition:?}: running={running}"
        );
    }
}
