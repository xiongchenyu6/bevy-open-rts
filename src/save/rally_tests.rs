//! Real naturally spawned target references must survive save/apply and key remapping.
use super::*;

fn app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
    setup.player_controllers[0] = SkirmishPlayerController::Human;
    setup.player_controllers[1] = SkirmishPlayerController::Human;
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    app
}

fn roundtrip(resource: bool) {
    let mut original = app();
    let world = original.world_mut();
    let producer = world
        .query::<(Entity, &Structure, &Team)>()
        .iter(world)
        .find(|(_, s, t)| s.id == "CommandCenter" && **t == Team::Player(0))
        .unwrap()
        .0;
    let target = if resource {
        world
            .query::<(Entity, &ResourceNode, &VisibilityState)>()
            .iter(world)
            .find(|(_, _, v)| v.visible)
            .expect("Normal visible resource required")
            .0
    } else {
        world
            .query::<(Entity, &Unit, &Team)>()
            .iter(world)
            .find(|(_, u, t)| u.id == "Worker" && **t == Team::Player(0))
            .unwrap()
            .0
    };
    let position = world.get::<Transform>(target).unwrap().translation;
    // Existing normal-command application helper; no fabricated target or producer.
    let bounds = *world.resource::<MapBounds>();
    assert!(apply_rally_point_command_in_bounds(
        &mut world.get_mut::<RallyPoint>(producer).unwrap(),
        position,
        Some((target, position)),
        RallyMode::Move,
        bounds
    ));
    let mut saved = collect_save_game(world).unwrap();
    for _ in 0..3 {
        let bytes = ron::to_string(&saved).unwrap();
        let decoded: SaveGame = ron::from_str(&bytes).unwrap();
        let mut fresh = app();
        fresh.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(decoded);
        apply_loaded_save(fresh.world_mut());
        let world = fresh.world_mut();
        let (_, _, _, rally) = world
            .query::<(Entity, &Structure, &Team, &RallyPoint)>()
            .iter(world)
            .find(|(_, s, t, _)| s.id == "CommandCenter" && **t == Team::Player(0))
            .unwrap();
        let actual = rally.target_unit.expect(
            "Saved rally entity target was lost; ground fallback changes Follow/Harvest behavior",
        );
        assert_eq!(rally.target, Some(position));
        assert_eq!(
            world.get::<Transform>(actual).unwrap().translation,
            position
        );
        if resource {
            assert!(world.get::<ResourceNode>(actual).is_some());
        } else {
            assert_eq!(world.get::<Unit>(actual).unwrap().id, "Worker");
        }
        saved = collect_save_game(world).unwrap();
        let producer = world
            .query::<(Entity, &Structure, &Team)>()
            .iter(world)
            .find(|(_, s, t)| s.id == "CommandCenter" && **t == Team::Player(0))
            .unwrap()
            .0;
        let origin = world.get::<Transform>(producer).unwrap().translation;
        let old_units = world
            .query::<(Entity, &Unit)>()
            .iter(world)
            .map(|(e, _)| e)
            .collect::<std::collections::HashSet<_>>();
        let def = registry::entity("Worker").unwrap();
        let bank = world.resource::<Economies>().get(Team::Player(0)).ore;
        world.resource_scope(|world, mut queue: Mut<BuildQueue>| {
            let mut economies = world.resource_mut::<Economies>();
            assert!(matches!(
                enqueue_build_jobs_for_producers(
                    Team::Player(0),
                    SkirmishFaction::Alliance,
                    BuildAction::Train("Worker"),
                    def,
                    &[(producer, "CommandCenter", origin)],
                    false,
                    &mut economies,
                    &mut queue
                ),
                EnqueueBuildActionResult::Enqueued
            ));
        });
        assert_eq!(
            world.resource::<Economies>().get(Team::Player(0)).ore,
            bank - def.cost.ore,
            "Worker must be paid from real starting bank"
        );
        for _ in 0..180 {
            fresh.update();
            let world = fresh.world_mut();
            if let Some(entity) = world
                .query::<(Entity, &Unit, &Team)>()
                .iter(world)
                .find(|(e, u, t)| {
                    u.id == "Worker" && **t == Team::Player(0) && !old_units.contains(e)
                })
                .map(|x| x.0)
            {
                if resource {
                    assert_eq!(
                        world
                            .get::<HarvestOrder>(entity)
                            .expect("Restored resource rally must issue Harvest, not ground Move")
                            .resource,
                        Some(actual)
                    );
                } else {
                    assert_eq!(
                        world
                            .get::<FollowOrder>(entity)
                            .expect("Restored owned-unit rally must issue Follow, not ground Move")
                            .target,
                        actual
                    );
                }
                break;
            }
        }
        let world = fresh.world_mut();
        assert_eq!(
            world
                .query::<(Entity, &Unit, &Team)>()
                .iter(world)
                .filter(|(e, u, t)| u.id == "Worker"
                    && **t == Team::Player(0)
                    && !old_units.contains(e))
                .count(),
            1,
            "One real paid Worker must complete"
        );
    }
}
#[test]
fn own_worker_rally_entity_survives_repeated_fresh_app_remapping() {
    roundtrip(false);
}
#[test]
fn resource_rally_entity_survives_repeated_fresh_app_remapping() {
    roundtrip(true);
}

#[test]
fn dangling_rally_target_is_rejected_instead_of_ground_fallback() {
    let mut app = app();
    let mut saved = collect_save_game(app.world_mut()).unwrap();
    let structure = saved
        .structures
        .iter_mut()
        .find(|s| s.id == "CommandCenter" && s.team == 0)
        .unwrap();
    structure.rally = Some(SavedRally {
        target: structure.position,
        mode: 0,
        target_entity: Some(u64::MAX),
    });
    assert_eq!(
        validate_save(&saved).unwrap_err(),
        "Invalid saved rally target"
    );
}
