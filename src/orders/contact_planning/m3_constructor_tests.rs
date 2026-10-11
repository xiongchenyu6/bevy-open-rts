//! Actual normal SDK267 M3 paid Power save; no worker substitution or world grants.
use super::*;

fn saved_m3_app() -> (App, SaveGame) {
    let saved: SaveGame =
        ron::from_str(include_str!("fixtures/m3-normal-paid-progress.ron")).unwrap();
    assert_eq!(saved.settings.map_path, "bevy://maps/RingValley");
    assert_eq!(
        (saved.economies[0].ore, saved.economies[0].crystal),
        (61, 31)
    );
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    *app.world_mut().resource_mut::<MatchSetupSettings>() =
        match_settings_from_save(&saved.settings).unwrap();
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(saved.clone());
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<AppScreen>>().get(),
        AppScreen::InMatch
    );
    assert!(app.world().resource::<MatchState>().is_running());
    (app, saved)
}

#[test]
fn actual_paid_m3_worker_finishes_power_without_air_drone_blocking_ground_contact() {
    let (mut app, saved) = saved_m3_app();
    let saved_worker = saved.units.iter().find(|u| u.key == 4294966720).unwrap();
    let source = Vec3::from_array(saved_worker.position);
    let mut sites = app.world_mut().query::<(
        Entity,
        &Team,
        &Structure,
        &Transform,
        &Selectable,
        Option<&UnderConstruction>,
    )>();
    let (target, position, target_radius) = sites
        .iter(app.world())
        .find(|(_, team, s, _, _, _)| **team == Team::Player(0) && s.id == "PowerReactor")
        .map(|(e, _, _, t, s, _)| (e, t.translation, s.radius))
        .unwrap();
    assert_eq!(position, Vec3::new(-18.99508, 0.0, -25.00777));
    let site = app.world().get::<UnderConstruction>(target).unwrap();
    assert_eq!(
        (site.remaining, site.total, site.cost.ore, site.cost.crystal),
        (1.0, 1.0, 3, 1)
    );
    let mut builders = app
        .world_mut()
        .query::<(Entity, &Unit, &Team, &ConstructOrder, &Selectable)>();
    let (builder, radius) = builders
        .iter(app.world())
        .find(|(_, unit, team, order, _)| {
            unit.id == "Worker" && **team == Team::Player(0) && order.target == target
        })
        .map(|(e, _, _, _, s)| (e, s.radius))
        .unwrap();
    let mut static_query = app
        .world_mut()
        .query_filtered::<(&Transform, &Selectable, Option<&Structure>), (
            Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
            Without<Unit>,
        )>();
    let obstacles = static_query
        .iter(app.world())
        .map(|(t, s, _)| (t.translation, s.radius))
        .collect::<Vec<_>>();
    let buildings = static_query
        .iter(app.world())
        .filter_map(|(t, s, structure)| structure.map(|_| (t.translation, s.radius)))
        .collect::<Vec<_>>();
    let mut others = app.world_mut().query::<(
        Entity,
        &Unit,
        &MovementDomain,
        &Transform,
        &Selectable,
        &Health,
    )>();
    let all = others
        .iter(app.world())
        .filter(|(e, _, _, _, _, h)| *e != builder && h.current > 0.0)
        .map(|(_, _, _, t, s, _)| (t.translation, s.radius))
        .collect::<Vec<_>>();
    let ground = others
        .iter(app.world())
        .filter(|(e, _, domain, _, _, h)| {
            *e != builder && **domain == MovementDomain::Terrain && h.current > 0.0
        })
        .map(|(_, _, _, t, s, _)| (t.translation, s.radius))
        .collect::<Vec<_>>();
    let drone = others
        .iter(app.world())
        .find(|(_, unit, domain, t, _, _)| {
            unit.id == "Drone"
                && **domain == MovementDomain::Air
                && t.translation == Vec3::new(-24.0, 1.5, -24.0)
        })
        .unwrap();
    let distance = xz_distance(source, drone.3.translation);
    assert!(
        distance > radius + drone.4.radius
            && distance < radius + drone.4.radius + UNIT_SEPARATION_GAP_M
    );
    let grid = app.world().resource::<NavGrid>();
    let all_route = reachable_unit_contact_approach(
        source,
        radius,
        position,
        target_radius,
        grid,
        &obstacles,
        &buildings,
        &all,
    );
    let ground_route = reachable_unit_contact_approach(
        source,
        radius,
        position,
        target_radius,
        grid,
        &obstacles,
        &buildings,
        &ground,
    );
    eprintln!(
        "M3 exact saved source {source:?}, AirDrone distance {distance}, all-domains route={} actual-ground route={}",
        all_route.is_some(),
        ground_route.is_some()
    );
    assert!(
        ground_route.is_some(),
        "Existing actual ground geometry must admit a safe route"
    );
    let start_clock = app.world().resource::<MatchState>().start_time_sec;
    let mut previous = app.world().get::<Transform>(builder).unwrap().translation;
    let speed = app.world().get::<Unit>(builder).unwrap().speed;
    let mut completed = false;
    for tick in 0..900 {
        assert!(
            app.world().resource::<MatchState>().is_running(),
            "Fixture must remain a real running match, tick {tick}"
        );
        app.update();
        assert!(app.world().get::<Health>(builder).unwrap().current > 0.0);
        let position = app.world().get::<Transform>(builder).unwrap().translation;
        assert!(
            xz_distance(position, previous) <= speed / 30.0 + 0.005,
            "Constructor moves continuously at its ordinary speed"
        );
        previous = position;
        if app.world().get::<UnderConstruction>(target).is_none() {
            completed = true;
            break;
        }
    }
    eprintln!(
        "M3 full schedule final clock{} remaining{:?} worker{:?}",
        app.world().resource::<MatchState>().start_time_sec,
        app.world()
            .get::<UnderConstruction>(target)
            .map(|s| s.remaining),
        app.world().get::<Transform>(builder).unwrap().translation
    );
    assert!(app.world().resource::<MatchState>().start_time_sec - start_clock <= 30.01);
    assert!(
        completed,
        "Same normally paid Worker must finish the Power within unchanged 30s window"
    );
    assert_eq!(
        (
            app.world().resource::<Economies>().players[0].ore,
            app.world().resource::<Economies>().players[0].crystal
        ),
        (61, 31)
    );
}

#[test]
fn actual_ground_body_keeps_its_full_contact_gap_in_the_recorded_m3_geometry() {
    let source = Vec3::new(-25.236988, 0.05, -23.840286);
    let ground_body = (
        Vec3::new(-24.0, 0.05, -24.0),
        registry::entity("Worker").unwrap().radius,
    );
    let radius = registry::entity("Worker").unwrap().radius;
    let outward = source - Vec3::X;
    assert!(distance_point_to_xz_segment(ground_body.0, source, outward) >= radius + ground_body.1);
    assert!(
        !contact_escape_segment_is_clear(source, outward, radius, &[], &[], &[ground_body], false),
        "A genuine ground body cannot be removed from the contact occupancy contract"
    );
}
