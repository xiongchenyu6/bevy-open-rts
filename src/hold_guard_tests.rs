use super::*;

fn live_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(0.1),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings
            .player_controllers
            .fill(SkirmishPlayerController::Human);
        settings.player_factions.fill(SkirmishFaction::Alliance);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    app
}

fn guard(app: &mut App, id: &'static str) -> Entity {
    let def = registry::entity(id).unwrap();
    let w = def.weapon.unwrap();
    app.world_mut()
        .spawn((
            Unit {
                id,
                speed: def.speed,
                can_crush: def.can_crush,
                can_be_crushed: def.can_be_crushed,
            },
            Team::Player(0),
            Transform::from_xyz(0.0, def.height, 0.0),
            Selectable { radius: def.radius },
            Health::new(def.health),
            MovementDomain::Terrain,
            VisionRadius(def.sight_range),
            VisibilityState { visible: true },
            Visibility::Visible,
            HoldPosition { enabled: true },
            Weapon::new(
                w.range,
                w.damage,
                w.cooldown,
                w.splash_radius,
                w.splash_damage_multiplier,
                w.structure_damage_multiplier,
                w.can_attack_air,
                w.can_attack_ground,
            ),
        ))
        .id()
}
fn target(app: &mut App, air: bool, x: f32) -> Entity {
    let id = if air { "Drone" } else { "Worker" };
    let def = registry::entity(id).unwrap();
    app.world_mut()
        .spawn((
            Unit {
                id,
                speed: def.speed,
                can_crush: def.can_crush,
                can_be_crushed: def.can_be_crushed,
            },
            Team::Player(1),
            Transform::from_xyz(x, if air { 1.5 } else { def.height }, 0.0),
            Selectable { radius: def.radius },
            Health::new(def.health),
            if air {
                MovementDomain::Air
            } else {
                MovementDomain::Terrain
            },
            VisionRadius(def.sight_range),
            VisibilityState { visible: true },
            Visibility::Visible,
        ))
        .id()
}
fn assert_stationary(app: &App, entity: Entity, start: Vec3) {
    assert!(
        app.world().get::<MoveOrder>(entity).is_none(),
        "Hold must never chase"
    );
    assert!(
        app.world().get::<PlannedPath>(entity).is_none(),
        "Hold must not plan pursuit"
    );
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().translation,
        start
    );
}
#[test]
fn held_rocket_auto_fires_at_air_in_range_without_moving() {
    let mut app = live_app();
    let shooter = guard(&mut app, "RocketInfantry");
    let enemy = target(&mut app, true, 3.0);
    let hp = app.world().get::<Health>(enemy).unwrap().current;
    let start = app.world().get::<Transform>(shooter).unwrap().translation;
    for _ in 0..6 {
        app.update();
        assert_stationary(&app, shooter, start);
    }
    assert!(
        app.world().get::<Health>(enemy).unwrap().current < hp,
        "held rocket must auto-fire at a compatible nearby aircraft"
    );
}
#[test]
fn held_tank_auto_fires_at_ground_in_range_without_moving() {
    let mut app = live_app();
    let shooter = guard(&mut app, "Tank");
    let enemy = target(&mut app, false, 3.0);
    let hp = app.world().get::<Health>(enemy).unwrap().current;
    let start = app.world().get::<Transform>(shooter).unwrap().translation;
    for _ in 0..3 {
        app.update();
        assert_stationary(&app, shooter, start);
    }
    assert!(
        app.world().get::<Health>(enemy).unwrap().current < hp,
        "held tank must auto-fire at nearby ground target"
    );
}
#[test]
fn held_rocket_does_not_acquire_or_chase_air_outside_weapon_range() {
    let mut app = live_app();
    let shooter = guard(&mut app, "RocketInfantry");
    let range = app.world().get::<Weapon>(shooter).unwrap().range;
    let enemy = target(&mut app, true, range + 1.0);
    let hp = app.world().get::<Health>(enemy).unwrap().current;
    let start = app.world().get::<Transform>(shooter).unwrap().translation;
    for _ in 0..10 {
        app.update();
        assert_stationary(&app, shooter, start);
    }
    assert_eq!(app.world().get::<Health>(enemy).unwrap().current, hp);
    assert!(app.world().get::<AttackOrder>(shooter).is_none());
}

#[test]
fn held_ordered_target_can_leave_range_without_guard_pursuit() {
    let mut app = live_app();
    let shooter = guard(&mut app, "RocketInfantry");
    let enemy = target(&mut app, true, 3.0);
    let hp = app.world().get::<Health>(enemy).unwrap().current;
    app.world_mut()
        .entity_mut(shooter)
        .insert(AttackOrder { target: enemy });
    let start = app.world().get::<Transform>(shooter).unwrap().translation;
    app.update();
    assert!(
        app.world().get::<Health>(enemy).unwrap().current < hp,
        "existing explicit attack must still fire in range"
    );
    let range = app.world().get::<Weapon>(shooter).unwrap().range;
    app.world_mut().entity_mut(enemy).insert(MoveOrder {
        target: Vec3::new(range + 3.0, 1.5, 0.0),
    });
    for _ in 0..30 {
        app.update();
        assert_stationary(&app, shooter, start);
    }
    assert!(
        app.world().get::<Transform>(enemy).unwrap().translation.x > range,
        "target genuinely moved beyond firing range"
    );
}
