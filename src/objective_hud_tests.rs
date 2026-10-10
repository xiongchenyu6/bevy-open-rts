//! Actual match-schedule regressions for mission text and resolved HUD visibility.
use super::*;

fn objective_scene() -> (App, Entity, Entity) {
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
    let existing: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, Or<(With<ObjectiveTrackerRoot>, With<ObjectiveTrackerText>)>>()
        .iter(app.world())
        .collect();
    for entity in existing {
        app.world_mut().despawn(entity);
    }
    app.world_mut().resource_mut::<TutorialState>().active = false;
    let root = app
        .world_mut()
        .spawn((Node::default(), Visibility::Inherited, ObjectiveTrackerRoot))
        .id();
    let text = app
        .world_mut()
        .spawn((Text::new("stale objective"), ObjectiveTrackerText))
        .id();
    (app, root, text)
}

#[test]
fn objective_hud_hides_when_match_resolves() {
    for phase in [MatchPhase::HumanVictory, MatchPhase::HumanDefeat] {
        let (mut app, root, _) = objective_scene();
        app.world_mut().resource_mut::<MatchState>().phase = phase;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Hidden
        );
    }
}

#[test]
fn objective_hud_headquarters_mission_uses_mission_goal() {
    let (mut app, _, text) = objective_scene();
    assert_eq!(
        mission_by_index(1).unwrap().victory,
        MissionVictory::Headquarters
    );
    app.world_mut().resource_mut::<ActiveMission>().0 = Some(1);
    app.world_mut()
        .resource_mut::<MatchSetupSettings>()
        .victory_condition = VictoryCondition::Annihilation;
    app.update();
    let label = &app.world().get::<Text>(text).unwrap().0;
    assert!(
        label.contains(" / "),
        "HQ mission must display headquarters progress: {label}"
    );
    assert!(
        !label.contains(t("和工人", "and workers")),
        "mission objective must not inherit the skirmish worker requirement: {label}"
    );
}
