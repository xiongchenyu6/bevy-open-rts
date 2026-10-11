//! Genuine Hold/separation contract tests; fixture geometry is not paid GUI or exact M2 causality proof.
use super::*;
use bevy::ecs::system::RunSystemOnce;

fn full_schedule() -> App {
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
            .find(|m| m.id == "crossfire")
            .unwrap()
            .godot_path;
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    let own_units: Vec<_> = app
        .world_mut()
        .query_filtered::<(Entity, &Team), With<Unit>>()
        .iter(app.world())
        .filter_map(|(e, t)| (*t == Team::Player(0)).then_some(e))
        .collect();
    for e in own_units {
        app.world_mut().despawn(e);
    }
    assert_eq!(app.world().resource::<MapBounds>().half_width, 32.0);
    assert_eq!(
        app.world().resource::<MatchState>().phase,
        MatchPhase::Running
    );
    app
}

fn unit(app: &mut App, id: &'static str, position: Vec3) -> Entity {
    let def = registry::entity(id).unwrap();
    assert!(def.speed > 0.0 && def.domain == registry::MoveDomain::Terrain);
    let weapon = def.weapon.expect("Registry-correct armed unit required");
    app.world_mut()
        .spawn((
            MatchScopedEntity,
            Transform::from_translation(position),
            Unit {
                id: def.id,
                speed: def.speed,
                can_crush: def.can_crush,
                can_be_crushed: def.can_be_crushed,
            },
            Team::Player(0),
            MovementDomain::Terrain,
            Selectable { radius: def.radius },
            Health::new(def.health),
            VisionRadius(def.sight_range),
            VisibilityState { visible: true },
            HoldPosition { enabled: false },
            Weapon::new(
                weapon.range,
                weapon.damage,
                weapon.cooldown,
                weapon.splash_radius,
                weapon.splash_damage_multiplier,
                weapon.structure_damage_multiplier,
                weapon.can_attack_air,
                weapon.can_attack_ground,
            ),
        ))
        .id()
}

#[test]
fn held_registry_aa_keeps_position_under_incoming_actual_move_in_full_schedule() {
    let mut app = full_schedule();
    let start = Vec3::new(-19.957565, 0.05, -19.264338);
    let aa = unit(&mut app, "AntiAirWalker", start);
    // Real normal selection shortcut, then real Hold keyboard/order system.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::KeyA);
    }
    app.world_mut().run_system_once(selection_hotkeys).unwrap();
    assert!(app.world().get::<Selected>(aa).is_some());
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::ControlLeft);
        keys.release(KeyCode::AltLeft);
        keys.release(KeyCode::KeyA);
        keys.clear();
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyH);
    app.world_mut()
        .run_system_once(command_queue_controls)
        .unwrap();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyH);
        keys.clear();
    }
    assert!(app.world().get::<HoldPosition>(aa).unwrap().enabled);
    assert!(app.world().get::<MoveOrder>(aa).is_none());
    let incoming = unit(
        &mut app,
        "HeavyMachinegunTrooper",
        Vec3::new(-22.738352, 0.05, -19.219507),
    );
    let goal = Vec3::new(-19.497992, 0.0, -19.99953);
    // Real order application helper; no actor transform writes after fixture setup.
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            issue_unit_order(&mut commands, incoming, UnitQueuedOrder::Move(goal));
        })
        .unwrap();
    assert!(!app.world().get::<HoldPosition>(incoming).unwrap().enabled);
    assert!(app.world().get::<MoveOrder>(incoming).is_some());
    let incoming_start = app.world().get::<Transform>(incoming).unwrap().translation;
    let clock_start = app.world().resource::<MatchState>().start_time_sec;
    let mut trace = Vec::new();
    let mut maximum = 0.0_f32;
    for frame in 0..180 {
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        assert!(app.world().get::<HoldPosition>(aa).unwrap().enabled);
        assert!(!app.world().get::<HoldPosition>(incoming).unwrap().enabled);
        let position = app.world().get::<Transform>(aa).unwrap().translation;
        maximum = maximum.max(xz_distance(position, start));
        trace.push(serde_json::json!({"frame":frame,"clock":app.world().resource::<MatchState>().start_time_sec,"held_pose":position.to_array(),
            "held":app.world().get::<HoldPosition>(aa).unwrap().enabled,
            "held_has_Move":app.world().get::<MoveOrder>(aa).is_some(),
            "incoming_pose":app.world().get::<Transform>(incoming).unwrap().translation.to_array(),
            "incoming_has_Move":app.world().get::<MoveOrder>(incoming).is_some()}));
    }
    if let Ok(path) = std::env::var("M2_HOLD_FIXTURE_TRACE") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
            "scope":"CPU full scheduled two-actor Hold/traffic fixture, NOT exact 08/player proof",
            "registry_aa_radius":registry::entity("AntiAirWalker").unwrap().radius,
            "maximum_held_xz_displacement":maximum,"frames":trace}))
            .unwrap(),
        )
        .unwrap();
    }
    assert!(
        xz_distance(
            app.world().get::<Transform>(incoming).unwrap().translation,
            incoming_start
        ) > 0.5,
        "Incoming real Move never progressed"
    );
    assert!(
        app.world().resource::<MatchState>().start_time_sec - clock_start > 5.0,
        "Full match clock did not advance"
    );
    assert!(
        maximum < 1e-4,
        "Actual held AA displaced {maximum}m despite no Move/chase order"
    );
}

#[test]
fn observed_short_move_records_first_removal_and_postarrival_traffic_in_full_schedule() {
    let mut app = full_schedule();
    let start = Vec3::new(-19.957565, 0.05, -19.264338);
    let goal = Vec3::new(-20.504892, 0.0, -18.998405);
    let aa = unit(&mut app, "AntiAirWalker", start);
    let hmg = unit(
        &mut app,
        "HeavyMachinegunTrooper",
        Vec3::new(-22.738352, 0.05, -19.219507),
    );
    let peer = unit(
        &mut app,
        "AntiAirWalker",
        Vec3::new(-19.553562, 0.05, -20.72915),
    );
    let rally = Vec3::new(-19.497992, 0.0, -19.99953);
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            issue_unit_order(&mut commands, aa, UnitQueuedOrder::Move(goal));
            issue_unit_order(&mut commands, hmg, UnitQueuedOrder::Move(rally));
        })
        .unwrap();
    let before_separation = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let records = before_separation.clone();
    app.add_systems(
        Update,
        (move |q: Query<(&Transform, Option<&MoveOrder>, Option<&PlannedPath>)>| {
            let (t, order, path) = q.get(aa).unwrap();
            records.lock().unwrap().push(serde_json::json!({
            "pose":t.translation.to_array(),"Move":order.map(|o|o.target.to_array()),
            "PlannedPath":path.map(|p|serde_json::json!({"goal":p.goal.to_array(),"next":p.next,
                "next_waypoint":p.waypoints.get(p.next).map(|v|v.to_array())}))}));
        })
        .after(move_units)
        .before(separate_units)
        .in_set(SimulationPhase::Combat)
        .run_if(match_in_progress),
    );
    let mut trace = Vec::new();
    let mut first_removal = None;
    for frame in 0..180 {
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        let had_move = app.world().get::<MoveOrder>(aa).is_some();
        app.update();
        let position = app.world().get::<Transform>(aa).unwrap().translation;
        let pre = before_separation.lock().unwrap().last().unwrap().clone();
        let row = serde_json::json!({"frame":frame,"clock":app.world().resource::<MatchState>().start_time_sec,
            "before_separation":pre,"after_pose":position.to_array(),
            "after_Move":app.world().get::<MoveOrder>(aa).map(|o|o.target.to_array()),
            "after_goal_distance":xz_distance(position,goal),
            "incoming_pose":app.world().get::<Transform>(hmg).unwrap().translation.to_array(),
            "idle_peer_pose":app.world().get::<Transform>(peer).unwrap().translation.to_array()});
        if had_move && app.world().get::<MoveOrder>(aa).is_none() && first_removal.is_none() {
            first_removal = Some(row.clone());
        }
        trace.push(row);
    }
    if let Ok(path) = std::env::var("M2_UNHELD_FIXTURE_TRACE") {
        std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({
            "scope":"CPU scheduled observed short Move plus two OWN traffic actors; NOT exact08 scene or paid GUI proof",
            "start":start.to_array(),"requested_goal":goal.to_array(),"first_Move_removal":first_removal,
            "frames":trace})).unwrap()).unwrap();
    }
    assert!(
        first_removal.is_some(),
        "Short normal Move never completed in observed three-actor mechanism fixture"
    );
}

#[test]
fn held_clear_goal_aa_keeps_position_under_incoming_actual_move_in_full_schedule() {
    let mut app = full_schedule();
    let start = Vec3::new(-19.957565, 0.05, -19.264338);
    let aa = unit(&mut app, "AntiAirWalker", start);
    // Real normal selection shortcut, then real Hold keyboard/order system.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::KeyA);
    }
    app.world_mut().run_system_once(selection_hotkeys).unwrap();
    assert!(app.world().get::<Selected>(aa).is_some());
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::ControlLeft);
        keys.release(KeyCode::AltLeft);
        keys.release(KeyCode::KeyA);
        keys.clear();
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyH);
    app.world_mut()
        .run_system_once(command_queue_controls)
        .unwrap();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyH);
        keys.clear();
    }
    assert!(app.world().get::<HoldPosition>(aa).unwrap().enabled);
    assert!(app.world().get::<MoveOrder>(aa).is_none());
    let incoming = unit(
        &mut app,
        "HeavyMachinegunTrooper",
        Vec3::new(-22.738352, 0.05, -19.219507),
    );
    let original_goal = Vec3::new(-19.497992, 0.0, -19.99953);
    let origin = app.world().get::<Transform>(incoming).unwrap().translation;
    let ray = Vec3::new(original_goal.x - origin.x, 0.0, original_goal.z - origin.z).normalize();
    let goal = original_goal + ray * 3.0;
    let mover_radius = registry::entity("HeavyMachinegunTrooper").unwrap().radius;
    let mut clearance_checks = Vec::new();
    for (entity, t, radius, health, domain, unit, structure, resource, wall) in app
        .world_mut()
        .query::<(
            Entity,
            &Transform,
            &Selectable,
            Option<&Health>,
            Option<&MovementDomain>,
            Option<&Unit>,
            Option<&Structure>,
            Option<&ResourceNode>,
            Option<&TerrainWall>,
        )>()
        .iter(app.world())
    {
        if entity == incoming || health.is_some_and(|h| h.current <= 0.0) {
            continue;
        }
        if unit.is_some() && domain != Some(&MovementDomain::Terrain) {
            continue;
        }
        if unit.is_none() && structure.is_none() && resource.is_none() && wall.is_none() {
            continue;
        }
        let clearance = radius.radius
            + mover_radius
            + if unit.is_some() {
                UNIT_SEPARATION_GAP_M
            } else {
                MOVEMENT_OBSTACLE_CLEARANCE_M
            };
        let distance = xz_distance(goal, t.translation);
        clearance_checks.push(serde_json::json!({"radius":radius.radius,"distance":distance,"required":clearance,"terrain_unit":unit.is_some()}));
        assert!(
            distance > clearance,
            "Valid02endpoint overlaps healthy Terrain/static footprint: {distance} <= {clearance}"
        );
    }
    assert!(
        !app.world()
            .resource::<TerrainHeightField>()
            .step_blocked(goal, goal)
    );
    // Real order application helper; no actor transform writes after fixture setup.
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            issue_unit_order(&mut commands, incoming, UnitQueuedOrder::Move(goal));
        })
        .unwrap();
    assert!(!app.world().get::<HoldPosition>(incoming).unwrap().enabled);
    assert!(app.world().get::<MoveOrder>(incoming).is_some());
    let incoming_start = app.world().get::<Transform>(incoming).unwrap().translation;
    let clock_start = app.world().resource::<MatchState>().start_time_sec;
    let mut trace = Vec::new();
    let mut maximum = 0.0_f32;
    for frame in 0..180 {
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        assert!(app.world().get::<HoldPosition>(aa).unwrap().enabled);
        assert!(!app.world().get::<HoldPosition>(incoming).unwrap().enabled);
        let position = app.world().get::<Transform>(aa).unwrap().translation;
        maximum = maximum.max(xz_distance(position, start));
        trace.push(serde_json::json!({"frame":frame,"clock":app.world().resource::<MatchState>().start_time_sec,"held_pose":position.to_array(),
            "held":app.world().get::<HoldPosition>(aa).unwrap().enabled,
            "held_has_Move":app.world().get::<MoveOrder>(aa).is_some(),
            "incoming_pose":app.world().get::<Transform>(incoming).unwrap().translation.to_array(),
            "incoming_has_Move":app.world().get::<MoveOrder>(incoming).is_some()}));
    }
    if let Ok(path) = std::env::var("M2_HOLD_CLEAR_GOAL_TRACE") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
            "scope":"CPU full scheduled valid clear-goal02 Hold/traffic fixture, NOT exact08/player proof",
            "valid_goal":goal.to_array(),"clearance_checks":clearance_checks,
            "registry_aa_radius":registry::entity("AntiAirWalker").unwrap().radius,
            "maximum_held_xz_displacement":maximum,"frames":trace}))
            .unwrap(),
        )
        .unwrap();
    }
    assert!(
        xz_distance(
            app.world().get::<Transform>(incoming).unwrap().translation,
            incoming_start
        ) > 3.0,
        "Valid02incoming real Move failed original3m progress"
    );
    assert!(
        app.world().resource::<MatchState>().start_time_sec - clock_start > 5.0,
        "Full match clock did not advance"
    );
    assert!(
        maximum < 1e-4,
        "Actual held AA displaced {maximum}m despite no Move/chase order"
    );
}

#[test]
fn scheduled_separation_keeps_unheld_mobility_and_ignores_enemy_pairs() {
    let mut app = full_schedule();
    let a = unit(&mut app, "AntiAirWalker", Vec3::new(0.0, 0.05, 0.0));
    let _b = unit(
        &mut app,
        "HeavyMachinegunTrooper",
        Vec3::new(0.5, 0.05, 0.0),
    );
    let start = app.world().get::<Transform>(a).unwrap().translation;
    app.update();
    assert!(
        xz_distance(app.world().get::<Transform>(a).unwrap().translation, start) > 0.001,
        "Unheld healthy terrain units must retain normal separation mobility"
    );
    let mut enemy_app = full_schedule();
    let a = unit(&mut enemy_app, "AntiAirWalker", Vec3::new(0.0, 0.05, 0.0));
    let b = unit(&mut enemy_app, "AntiAirWalker", Vec3::new(0.5, 0.05, 0.0));
    enemy_app.world_mut().entity_mut(b).insert(Team::Player(1)); // Fixture initial team setup only.
    assert!(
        enemy_app
            .world()
            .resource::<TeamRelations>()
            .are_enemies(Team::Player(0), Team::Player(1))
    );
    let start_a = enemy_app.world().get::<Transform>(a).unwrap().translation;
    let start_b = enemy_app.world().get::<Transform>(b).unwrap().translation;
    enemy_app.update();
    assert_eq!(
        enemy_app.world().get::<Transform>(a).unwrap().translation,
        start_a
    );
    assert_eq!(
        enemy_app.world().get::<Transform>(b).unwrap().translation,
        start_b
    );
}

#[test]
fn genuinely_deployed_zero_speed_unit_stays_immovable_under_separation() {
    let mut app = full_schedule();
    let deployed = unit(&mut app, "LongbowMissileCrawler", Vec3::new(0.0, 0.05, 0.0));
    app.world_mut()
        .entity_mut(deployed)
        .insert(DeployModeToggleRequest);
    app.world_mut()
        .run_system_once(update_deploy_mode_requests)
        .unwrap();
    assert_eq!(app.world().get::<Unit>(deployed).unwrap().speed, 0.0);
    assert!(app.world().get::<DeployedSiegeMode>(deployed).is_some());
    let incoming = unit(
        &mut app,
        "HeavyMachinegunTrooper",
        Vec3::new(0.5, 0.05, 0.0),
    );
    let start = app.world().get::<Transform>(deployed).unwrap().translation;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Transform>(deployed).unwrap().translation,
        start
    );
    assert!(
        xz_distance(
            app.world().get::<Transform>(incoming).unwrap().translation,
            start
        ) > 0.5
    );
}
