//! World positions are already elevated when collected; load must not add height twice.
use super::*;

fn natural_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
        setup.player_controllers[0] = SkirmishPlayerController::Human;
        setup.player_controllers[1] = SkirmishPlayerController::Human;
        assert_eq!(setup.starting_resources, StartingResources::new(32, 16));
        assert_eq!(setup.startup_loadout, StartupLoadoutMode::GodotSkirmish);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    app
}
fn poses(world: &mut World, id: &str) -> Vec<(Vec3, f32, Vec3)> {
    let mut result = world
        .query::<(&Unit, &Team, &Transform)>()
        .iter(world)
        .filter(|(unit, team, _)| unit.id == id && **team == Team::Player(0))
        .map(|(_, _, transform)| (transform.translation, yaw_of(transform), transform.scale))
        .collect::<Vec<_>>();
    result.sort_by(|a, b| {
        a.0.x
            .total_cmp(&b.0.x)
            .then_with(|| a.0.z.total_cmp(&b.0.z))
    });
    result
}
fn assert_poses(world: &mut World, id: &str, expected: &[(Vec3, f32, Vec3)]) {
    let actual = poses(world, id);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(
            actual.0, expected.0,
            "{id}: saved world translation must survive without another authored-height offset"
        );
        assert!(
            (actual.1 - expected.1).abs() < 1e-6,
            "{id}: existing saved yaw must survive"
        );
        assert_eq!(
            actual.2, expected.2,
            "{id}: normal spawn scale must remain unchanged"
        );
    }
}
fn roundtrips(id: &str) {
    let mut app = natural_app();
    // Exercise the existing yaw field on naturally spawned actors; no unit/resource grants.
    for (unit, team, mut transform) in app
        .world_mut()
        .query::<(&Unit, &Team, &mut Transform)>()
        .iter_mut(app.world_mut())
    {
        if unit.id == id && *team == Team::Player(0) {
            transform.rotation = Quat::from_rotation_y(0.63);
        }
    }
    let expected = poses(app.world_mut(), id);
    assert!(!expected.is_empty());
    let def = registry::entity(id).unwrap();
    assert!(expected.iter().all(|p| p.0.y == def.height));
    let original = collect_save_game(app.world_mut()).unwrap();
    assert!(
        original
            .units
            .iter()
            .any(|u| u.id == id && u.team == 0 && u.position[1] == def.height)
    );
    // First apply, then a new collect/save/load cycle: any erroneous offset would accumulate.
    let mut next = original.clone();
    for _ in 0..3 {
        app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(next);
        apply_loaded_save(app.world_mut());
        assert_poses(app.world_mut(), id, &expected);
        next = collect_save_game(app.world_mut()).unwrap();
    }
    // A fresh App, with the real RestartingMatch/OnEnter restore path and serialized save.
    let bytes = ron::to_string(&original).unwrap();
    let saved: SaveGame = ron::from_str(&bytes).unwrap();
    let mut fresh = natural_app();
    *fresh.world_mut().resource_mut::<MatchSetupSettings>() =
        match_settings_from_save(&saved.settings).unwrap();
    fresh.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(saved);
    fresh
        .world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    for _ in 0..20 {
        fresh.update();
    }
    assert_poses(fresh.world_mut(), id, &expected);
}
#[test]
fn airborne_world_translation_survives_first_repeated_and_fresh_app_loads() {
    roundtrips("Drone");
}
#[test]
fn ground_world_translation_survives_first_repeated_and_fresh_app_loads() {
    roundtrips("Worker");
}
