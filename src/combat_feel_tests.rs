//! Real match-schedule regressions for the presentation-only combat feel
//! layer: it must react to effect entities, clean up when a match resolves,
//! and never write simulation state.
use super::*;

fn running_match() -> App {
    running_match_with(|_| {})
}

/// `configure` runs before the first frame, so option changes made there are
/// the app's starting state and are never written to the player's real
/// preferences file by the persistence system.
fn running_match_with(configure: impl FnOnce(&mut App)) -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    configure(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    app.world_mut().resource_mut::<TutorialState>().active = false;
    assert!(app.world().resource::<MatchState>().is_running());
    app
}

/// Everything the simulation and the online snapshot read off an actor.
fn simulation_fingerprint(app: &mut App) -> Vec<(Entity, [u32; 3], [u32; 2])> {
    let mut actors: Vec<_> = app
        .world_mut()
        .query_filtered::<(Entity, &Transform, &Health), Or<(With<Unit>, With<Structure>)>>()
        .iter(app.world())
        .map(|(entity, transform, health)| {
            (
                entity,
                transform.translation.to_array().map(f32::to_bits),
                [health.current.to_bits(), health.max.to_bits()],
            )
        })
        .collect();
    actors.sort_by_key(|(entity, ..)| *entity);
    actors
}

fn spawn_weapon_pulse(commands: &mut Commands, from: Vec3, kind: ImpactBurstKind) {
    commands.spawn((
        ShotPulse {
            from: from + Vec3::Y * 0.6,
            to: from + Vec3::new(3.0, 0.6, 0.0),
            ttl: 0.13,
            team: Team::Player(1),
        },
        ShotWeaponClass { kind, total: 0.13 },
        MatchScopedEntity,
    ));
}

fn spawn_replicated_unit_death(commands: &mut Commands, position: Vec3) {
    commands.spawn((
        Transform::from_translation(position),
        UnitDeathVfx {
            remaining: UNIT_DEATH_VFX_LIFETIME_SECONDS,
            radius: 0.6,
            team: Team::Player(1),
            replica: true,
        },
        MatchScopedEntity,
    ));
}

fn count<F: bevy::ecs::query::QueryFilter>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), F>()
        .iter(app.world())
        .count()
}

fn spawn_strike_at_camera_focus(app: &mut App) {
    let focus = app.world().resource::<RtsCamera>().focus;
    let world = app.world_mut();
    let mut commands = world.commands();
    spawn_strike_impact(&mut commands, focus, 4.8, Team::Player(0));
    world.flush();
}

#[test]
fn strike_near_the_camera_shakes_it_and_queues_secondary_explosions() {
    let mut app = running_match();
    assert_eq!(app.world().resource::<CameraShake>().trauma, 0.0);
    spawn_strike_at_camera_focus(&mut app);
    app.update();
    let shake = *app.world().resource::<CameraShake>();
    assert!(
        shake.trauma > 0.5,
        "a strike on the camera focus must shake hard, got {}",
        shake.trauma
    );
    let secondaries = app
        .world_mut()
        .query::<&SecondaryExplosion>()
        .iter(app.world())
        .count();
    assert_eq!(secondaries, 3);
}

#[test]
fn distant_strike_does_not_shake_the_camera() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    let far = focus + Vec3::new(CAMERA_SHAKE_FAR_DISTANCE + 5.0, 0.0, 0.0);
    {
        let world = app.world_mut();
        let mut commands = world.commands();
        spawn_strike_impact(&mut commands, far, 4.8, Team::Player(0));
        world.flush();
    }
    // Reveal the whole map so only distance, not fog, can mute the strike.
    app.world_mut().resource_mut::<VisiblePlayer>().visibility = PlayerVisibilityMode::AllPlayers;
    app.update();
    assert_eq!(app.world().resource::<CameraShake>().trauma, 0.0);
}

#[test]
fn rifle_fire_never_shakes_the_camera() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    {
        let world = app.world_mut();
        let mut commands = world.commands();
        for _ in 0..40 {
            spawn_impact_burst(
                &mut commands,
                focus,
                0.5,
                6.0,
                false,
                Team::Player(0),
                ImpactBurstKind::Ballistic,
            );
        }
        world.flush();
    }
    app.update();
    assert_eq!(app.world().resource::<CameraShake>().trauma, 0.0);
}

/// Twin-run check: two identical matches advance the same ticks; one of them
/// is hammered with strike and building-destruction feel events on the camera
/// focus. Every actor must end bit-identical in both.
#[test]
fn combat_feel_never_writes_simulation_state() {
    let mut quiet = running_match();
    let mut loud = running_match();
    assert_eq!(
        simulation_fingerprint(&mut quiet),
        simulation_fingerprint(&mut loud),
        "twin matches must start identical"
    );
    let focus_before = loud.world().resource::<RtsCamera>().focus;
    let mut shook = false;
    for tick in 0..90 {
        if tick % 15 == 0 {
            spawn_strike_at_camera_focus(&mut loud);
            let world = loud.world_mut();
            let mut commands = world.commands();
            spawn_structure_destruction_vfx(&mut commands, focus_before, 1.4, Team::Player(1));
            // What an online client receives: weapon tracers and unit deaths.
            for kind in [
                ImpactBurstKind::Ballistic,
                ImpactBurstKind::Siege,
                ImpactBurstKind::Electric,
            ] {
                spawn_weapon_pulse(&mut commands, focus_before, kind);
            }
            spawn_replicated_unit_death(&mut commands, focus_before);
            world.flush();
        }
        quiet.update();
        loud.update();
        shook |= loud.world().resource::<CameraShake>().trauma > 0.0;
    }
    assert!(shook, "the loud match never produced any feel");
    let quiet_state = simulation_fingerprint(&mut quiet);
    assert!(
        !quiet_state.is_empty(),
        "normal match must have real actors"
    );
    assert_eq!(
        simulation_fingerprint(&mut loud),
        quiet_state,
        "feel effects moved or damaged a real actor"
    );
    assert_eq!(
        loud.world().resource::<RtsCamera>().focus,
        quiet.world().resource::<RtsCamera>().focus,
        "camera shake leaked into the camera focus the game reads"
    );
}

#[test]
fn damaged_unit_is_flashed_without_the_flash_touching_its_health() {
    let mut app = running_match();
    let (unit, health) = app
        .world_mut()
        .query_filtered::<(Entity, &Health, &VisibilityState), With<Unit>>()
        .iter(app.world())
        .find(|(_, _, visibility)| visibility.visible)
        .map(|(entity, health, _)| (entity, *health))
        .expect("normal match must have a visible unit");
    app.update();
    app.update();
    assert!(
        app.world().get::<HitFlashTracker>(unit).is_some(),
        "units are tracked for hit flashes"
    );
    let damaged = health.current - 1.0;
    app.world_mut().get_mut::<Health>(unit).unwrap().current = damaged;
    app.update();
    let tracker = app.world().get::<HitFlashTracker>(unit).unwrap();
    assert!(tracker.cooldown > 0.0, "a health drop must trigger a flash");
    // The simulation may regenerate the unit; the flash itself only reads.
    let current = app.world().get::<Health>(unit).unwrap().current;
    assert_eq!(tracker.last_health, current);
    assert!(
        current >= damaged && current < health.current,
        "the hit flash must not change health: {current}"
    );
}

#[test]
fn resolved_match_clears_in_flight_feel_effects_but_keeps_the_battlefield() {
    for phase in [
        MatchPhase::HumanVictory,
        MatchPhase::HumanDefeat,
        MatchPhase::MatchFinished,
    ] {
        let mut app = running_match();
        let ring = app
            .world_mut()
            .spawn((
                Transform::default(),
                ShockwaveRing {
                    age: 0.0,
                    ttl: 5.0,
                    start_radius: 1.0,
                    end_radius: 3.0,
                    material: Handle::default(),
                    peak_alpha: 0.7,
                },
            ))
            .id();
        let secondary = app
            .world_mut()
            .spawn((
                Transform::default(),
                SecondaryExplosion {
                    delay: 5.0,
                    scale: 1.0,
                },
            ))
            .id();
        let debris = app
            .world_mut()
            .spawn((
                Transform::default(),
                DebrisChunk {
                    velocity: Vec3::Y,
                    spin_axis: Vec3::Y,
                    spin_rate: 1.0,
                    age: 0.0,
                    ttl: 5.0,
                    material: Handle::default(),
                },
            ))
            .id();
        let wreck = app
            .world_mut()
            .spawn(CombatWreckage { remaining: 30.0 })
            .id();
        app.world_mut().resource_mut::<CameraShake>().kick(0.8);

        app.world_mut().resource_mut::<TacticalPause>().0 = true;
        app.update();
        for entity in [ring, secondary, debris] {
            assert!(
                app.world().get_entity(entity).is_ok(),
                "a running tactical pause must retain feel effects"
            );
        }

        app.world_mut().resource_mut::<MatchState>().phase = phase;
        app.update();
        for entity in [ring, secondary, debris] {
            assert!(
                app.world().get_entity(entity).is_err(),
                "resolved {phase:?} retained a frozen feel effect"
            );
        }
        assert_eq!(app.world().resource::<CameraShake>().trauma, 0.0);
        assert!(
            app.world().get::<CombatWreckage>(wreck).is_some(),
            "result cleanup removed wreckage"
        );
    }
}

#[test]
fn weapon_tracer_alone_stages_a_muzzle_flash() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    app.update();
    let before = count::<With<CombatFlash>>(&mut app);
    {
        let world = app.world_mut();
        let mut commands = world.commands();
        spawn_weapon_pulse(&mut commands, focus, ImpactBurstKind::Heavy);
        world.flush();
    }
    app.update();
    assert!(
        count::<With<CombatFlash>>(&mut app) > before,
        "a weapon-class tracer must produce its muzzle flash without the firing code"
    );
}

#[test]
fn replicated_unit_death_stages_a_wreck_once_and_then_expires() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    app.update();
    let wrecks = count::<With<CombatWreckage>>(&mut app);
    {
        let world = app.world_mut();
        let mut commands = world.commands();
        spawn_replicated_unit_death(&mut commands, focus);
        world.flush();
    }
    app.update();
    app.update();
    assert_eq!(count::<With<CombatWreckage>>(&mut app), wrecks + 1);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        count::<With<UnitDeathVfx>>(&mut app),
        0,
        "the death marker must expire"
    );
    assert_eq!(
        count::<With<CombatWreckage>>(&mut app),
        wrecks + 1,
        "one death must stage exactly one wreck"
    );
}

#[test]
fn locally_simulated_unit_death_is_announced_but_not_staged_twice() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    app.update();
    let wrecks = count::<With<CombatWreckage>>(&mut app);
    {
        let world = app.world_mut();
        world.resource_scope(|world, asset_server: Mut<AssetServer>| {
            let mut commands = world.commands();
            spawn_destruction_effects(
                &mut commands,
                &asset_server,
                focus,
                0.6,
                false,
                Team::Player(1),
                false,
            );
        });
        world.flush();
    }
    assert_eq!(count::<With<UnitDeathVfx>>(&mut app), 1);
    app.update();
    app.update();
    assert_eq!(count::<With<CombatWreckage>>(&mut app), wrecks + 1);
}

#[test]
fn damaged_structure_is_flashed_without_the_flash_touching_its_health() {
    let mut app = running_match();
    app.update();
    app.update();
    let (structure, health) = app
        .world_mut()
        .query_filtered::<(Entity, &Health, &HitFlashTracker), With<Structure>>()
        .iter(app.world())
        .map(|(entity, health, _)| (entity, *health))
        .next()
        .expect("structures are tracked for hit flashes");
    let damaged = health.current * 0.5;
    app.world_mut()
        .get_mut::<Health>(structure)
        .unwrap()
        .current = damaged;
    app.update();
    let tracker = app.world().get::<HitFlashTracker>(structure).unwrap();
    assert!(
        tracker.cooldown > HIT_FLASH_INTERVAL_SECONDS,
        "a damaged building must flash on its own slower cadence"
    );
    let current = app.world().get::<Health>(structure).unwrap().current;
    assert!(
        current >= damaged && current < health.current,
        "the hit flash must not change health: {current}"
    );
}

#[test]
fn screen_shake_option_off_keeps_the_camera_still() {
    let mut app = running_match_with(|app| {
        app.world_mut()
            .resource_mut::<MenuOptionsState>()
            .screen_shake = false;
    });
    assert!(!app.world().resource::<MenuOptionsState>().screen_shake);
    spawn_strike_at_camera_focus(&mut app);
    for _ in 0..3 {
        app.update();
        let shake = *app.world().resource::<CameraShake>();
        assert_eq!(shake.trauma, 0.0);
        assert_eq!(shake.offset(), Vec2::ZERO);
        assert_eq!(shake.zoom_punch_offset(), 0.0);
    }
}

#[test]
fn units_wire_live_motion_state_react_to_damage_and_emp_without_drifting_root() {
    let mut app = running_match();
    app.update();
    app.update();
    // Pick a human team unit and hold it stationary so any root translation
    // change could only come from the motion system (which must never write it).
    let (unit, health) = app
        .world_mut()
        .query_filtered::<(Entity, &Team, &Health), With<Unit>>()
        .iter(app.world())
        .find(|(_, team, _)| **team == Team::Player(0))
        .map(|(e, _, h)| (e, *h))
        .expect("normal match has a human unit");
    {
        let mut em = app.world_mut().entity_mut(unit);
        em.remove::<MoveOrder>();
        em.remove::<HarvestOrder>();
        em.insert(HoldPosition { enabled: true });
    }
    app.update();
    let initial_pos = app.world().get::<Transform>(unit).unwrap().translation;
    assert!(
        app.world().get::<MotionState>(unit).is_some(),
        "every unit must receive a live MotionState"
    );

    // Damage the unit -> triggers directional flinch + 62.5Hz impact micro-tremor.
    app.world_mut().get_mut::<Health>(unit).unwrap().current = (health.current * 0.7).max(1.0);
    app.update();
    let motion = *app.world().get::<MotionState>(unit).unwrap();
    assert!(
        motion.tremor_remaining > 0.0,
        "health drop must engage 62.5Hz impact micro-tremor"
    );
    assert_eq!(
        motion.active.spec().category,
        MotionCategory::HitReaction,
        "damage must switch unit into hit reaction motion, got {:?}",
        motion.active
    );
    assert_eq!(
        app.world().get::<Transform>(unit).unwrap().translation,
        initial_pos,
        "motion engine must never mutate unit root Transform.translation"
    );

    // Apply EMP -> transitions into EMP stun wobble / tremor.
    app.world_mut()
        .entity_mut(unit)
        .insert(EmpDisabled { remaining: 2.0 });
    app.update();
    let emp_motion = *app.world().get::<MotionState>(unit).unwrap();
    assert!(
        matches!(
            emp_motion.active,
            MotionId::EmpStunWobble | MotionId::EmpDisruptedTremor
        ),
        "EMP must drive EMP wobble/tremor motion, got {:?}",
        emp_motion.active
    );
    assert_eq!(
        app.world().get::<Transform>(unit).unwrap().translation,
        initial_pos,
        "EMP motion must never mutate unit root Transform.translation"
    );
}

#[test]
fn strike_and_shots_spawn_additive_juice_and_zoom_punch_then_clear_on_match_end() {
    let mut app = running_match();
    let focus = app.world().resource::<RtsCamera>().focus;
    app.update();
    {
        let world = app.world_mut();
        let mut commands = world.commands();
        spawn_strike_impact(&mut commands, focus, 4.8, Team::Player(0));
        spawn_weapon_pulse(&mut commands, focus, ImpactBurstKind::Heavy);
        spawn_replicated_unit_death(&mut commands, focus);
        world.flush();
    }
    app.update();
    assert!(
        app.world().resource::<CameraShake>().zoom_punch_offset() > 0.0,
        "strike at focus must trigger camera impact zoom punch"
    );
    assert!(
        count::<With<ContactHitCore>>(&mut app) > 0,
        "impacts must spawn 85ms additive contact hit cores"
    );
    assert!(
        count::<With<ImpactRazorSpark>>(&mut app) > 0,
        "impacts must spawn directional additive razor sparks"
    );
    assert!(
        count::<With<ImpactCrossGlint>>(&mut app) > 0,
        "strikes/kills must spawn additive cross-glints"
    );
    assert!(
        count::<With<WeaponArcSmear>>(&mut app) > 0,
        "weapon tracers must spawn tapered additive arc smears"
    );
    assert!(
        count::<With<ShockwaveRing>>(&mut app) > 0,
        "strikes/kills must spawn additive finisher shock-rings"
    );

    // Resolving the match must clear all in-flight juice cleanly.
    app.world_mut().resource_mut::<MatchState>().phase = MatchPhase::HumanVictory;
    app.update();
    assert_eq!(count::<With<ContactHitCore>>(&mut app), 0);
    assert_eq!(count::<With<ImpactRazorSpark>>(&mut app), 0);
    assert_eq!(count::<With<ImpactCrossGlint>>(&mut app), 0);
    assert_eq!(count::<With<WeaponArcSmear>>(&mut app), 0);
    assert_eq!(count::<With<TreadDustPuff>>(&mut app), 0);
    assert_eq!(
        app.world().resource::<CameraShake>().zoom_punch_offset(),
        0.0
    );
}
