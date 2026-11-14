//! Real match-schedule regressions for the presentation-only combat feel
//! layer: it must react to effect entities, clean up when a match resolves,
//! and never write simulation state.
use super::*;

fn running_match() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
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
