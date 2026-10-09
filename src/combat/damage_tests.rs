use super::*;

fn app(faction: SkirmishFaction) -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings
            .player_controllers
            .fill(SkirmishPlayerController::Human);
        settings.player_factions[1] = faction;
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    app
}

fn worker(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Unit {
                id: "Worker",
                speed: 2.5,
                can_crush: false,
                can_be_crushed: true,
            },
            Team::Player(1),
            Transform::from_xyz(1.0, 0.05, 0.0),
            Selectable { radius: 0.6 },
            MovementDomain::Terrain,
            Health::new(6.0),
        ))
        .id()
}

fn armed_mine(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Unit {
                id: "LandMine",
                speed: 0.0,
                can_crush: false,
                can_be_crushed: false,
            },
            Team::Player(0),
            Transform::default(),
            Selectable { radius: 0.2 },
            MovementDomain::Terrain,
            Health::new(1.0),
            Mine {
                damage: 4.0,
                trigger_radius: 0.9,
                blast_radius: 1.5,
                arming_remaining: 0.0,
                source: None,
            },
        ))
        .id()
}

fn assert_hp(app: &App, target: Entity, expected: f32) {
    assert!((app.world().get::<Health>(target).unwrap().current - expected).abs() < 1e-5);
}

#[test]
fn mine_damage_starts_demon_out_of_combat_delay() {
    let mut app = app(SkirmishFaction::Demon);
    let target = worker(&mut app);
    let mine = armed_mine(&mut app);
    app.update();
    assert!(
        app.world().get::<Mine>(mine).is_none(),
        "real mine must trigger"
    );
    assert_hp(&app, target, 2.0);
    assert!(app.world().get::<RecentDamage>(target).is_some());
    for _ in 0..30 {
        app.update();
    }
    assert_hp(&app, target, 2.0);
    for _ in 0..150 {
        app.update();
    }
    assert!(
        app.world().get::<Health>(target).unwrap().current > 2.0,
        "normal regeneration must resume after the actual damage window expires"
    );
}

#[test]
fn mine_damage_suspends_chaos_shield_recharge() {
    let mut app = app(SkirmishFaction::Chaos);
    let target = worker(&mut app);
    // Let the real Added<Unit> initializer attach its authored shield first.
    app.update();
    app.world_mut()
        .get_mut::<FactionShield>(target)
        .unwrap()
        .current = 0.0;
    armed_mine(&mut app);
    app.update();
    assert_hp(&app, target, 2.0);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().get::<FactionShield>(target).unwrap().current,
        0.0
    );
    for _ in 0..150 {
        app.update();
    }
    assert!(app.world().get::<FactionShield>(target).unwrap().current > 0.0);
}

#[test]
fn orbital_damage_suspends_alliance_field_repair() {
    let mut app = app(SkirmishFaction::Alliance);
    let target = app
        .world_mut()
        .spawn((
            Structure { id: "PowerReactor" },
            Team::Player(1),
            Transform::default(),
            Selectable { radius: 1.45 },
            MovementDomain::Terrain,
            Health::new(12.0),
        ))
        .id();
    app.world_mut().spawn((
        PendingOrbitalStrike {
            remaining: 0.0,
            radius: 2.0,
            damage: 4.0,
            impact_scale: 1.0,
            team: Team::Player(0),
        },
        Transform::default(),
    ));
    app.update();
    assert_hp(&app, target, 8.0);
    assert!(app.world().get::<RecentDamage>(target).is_some());
    for _ in 0..30 {
        app.update();
    }
    assert_hp(&app, target, 8.0);
    for _ in 0..150 {
        app.update();
    }
    assert!(app.world().get::<Health>(target).unwrap().current > 8.0);
}
