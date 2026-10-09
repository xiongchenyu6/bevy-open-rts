use super::*;

#[derive(Resource, Default)]
struct TransitionObservation(Option<MatchSnapshot>);

#[derive(Debug, Clone, PartialEq)]
struct MatchSnapshot {
    ai: SavedAiDirector,
    economies: Vec<(i32, i32, f32, Vec<u8>)>,
}

fn observe_after_loaded_save(world: &mut World) {
    let snapshot = MatchSnapshot {
        ai: collect_ai_director(world.resource::<AiDirector>()),
        economies: world
            .resource::<Economies>()
            .players
            .iter()
            .map(|economy| {
                (
                    economy.ore,
                    economy.crystal,
                    economy.power_sabotage_remaining,
                    economy.production_veterancy_ranks.to_vec(),
                )
            })
            .collect(),
    };
    world.resource_mut::<TransitionObservation>().0 = Some(snapshot);
}

fn started_app() -> App {
    let mut app = build_game_app(GameAppMode::Headless);
    app.init_resource::<TransitionObservation>();
    app.add_systems(
        OnEnter(AppScreen::InMatch),
        observe_after_loaded_save.after(apply_loaded_save),
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<AppScreen>>().get(),
        AppScreen::InMatch
    );
    app
}

fn seed_previous_match_director(app: &mut App) {
    for economy in &mut app.world_mut().resource_mut::<Economies>().players {
        economy.power_sabotage_remaining = 23.5;
        economy.production_veterancy_ranks.fill(2);
    }
    let mut director = app.world_mut().resource_mut::<AiDirector>();
    director.ensure_team(Team::Player(1)).unwrap();
    for index in 0..2 {
        director.production_timer[index] = 0.125;
        director.production_cursor[index] = 17;
        director.attack_timer[index] = 0.25;
        director.opening_attack_grace_applied[index] = true;
        director.build_timer[index] = 0.375;
        director.construction_timer[index] = 0.5;
        director.capture_timer[index] = 0.625;
        director.saboteur_timer[index] = 0.75;
        director.support_timer[index] = 0.875;
        director.repair_timer[index] = 1.0;
    }
}

fn restart_through_shared_scene(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::RestartingMatch);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppScreen>>().get(),
        AppScreen::RestartingMatch
    );
    for _ in 0..8 {
        app.update();
        if *app.world().resource::<State<AppScreen>>().get() == AppScreen::InMatch {
            return;
        }
    }
    panic!("shared restart transition did not enter the new match");
}

#[test]
fn fresh_match_transition_resets_previous_ai_director() {
    let mut app = started_app();
    let initial = app
        .world()
        .resource::<TransitionObservation>()
        .0
        .clone()
        .unwrap();
    seed_previous_match_director(&mut app);
    assert_ne!(
        collect_ai_director(app.world().resource::<AiDirector>()),
        initial.ai
    );
    restart_through_shared_scene(&mut app);
    assert_eq!(
        app.world()
            .resource::<TransitionObservation>()
            .0
            .as_ref()
            .unwrap(),
        &initial,
        "a new match must regain its initial AI/economy state without prior sabotage or veterancy"
    );
}

#[test]
fn loaded_match_transition_restores_all_saved_ai_director_state() {
    let mut app = started_app();
    seed_previous_match_director(&mut app);
    let save = collect_save_game(app.world_mut()).expect("active match can be saved");
    validate_save(&save).expect("fixture must be a valid actual match save");
    let expected = MatchSnapshot {
        ai: save.ai.clone(),
        economies: save
            .economies
            .iter()
            .map(|economy| {
                (
                    economy.ore,
                    economy.crystal,
                    economy.power_sabotage_remaining,
                    economy.production_veterancy_ranks.clone(),
                )
            })
            .collect(),
    };
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(save);
    restart_through_shared_scene(&mut app);
    assert_eq!(
        app.world()
            .resource::<TransitionObservation>()
            .0
            .as_ref()
            .unwrap(),
        &expected,
        "loading must restore every saved timer, production cursor and opening-grace flag"
    );
}

fn enqueue_actual_paid_worker(app: &mut App) -> registry::Cost {
    let world = app.world_mut();
    let (entity, position) = world
        .query::<(Entity, &Structure, &Team, &Transform)>()
        .iter(world)
        .find(|(_, structure, team, _)| {
            structure.id == "CommandCenter" && **team == Team::Player(0)
        })
        .map(|(entity, _, _, transform)| (entity, transform.translation))
        .unwrap();
    let def = registry::entity("Worker").unwrap();
    let faction = world
        .resource::<PlayerFactions>()
        .slot_faction(Team::Player(0));
    let before = {
        let economy = world.resource::<Economies>().get(Team::Player(0));
        (economy.ore, economy.crystal)
    };
    world.resource_scope(|world, mut queue: Mut<BuildQueue>| {
        let result = enqueue_build_jobs_for_producers(
            Team::Player(0),
            faction,
            BuildAction::Train("Worker"),
            def,
            &[(entity, "CommandCenter", position)],
            false,
            &mut world.resource_mut::<Economies>(),
            &mut queue,
        );
        assert!(matches!(result, EnqueueBuildActionResult::Enqueued));
    });
    let cost = world.resource::<BuildQueue>().0.last().unwrap().cost;
    let economy = world.resource::<Economies>().get(Team::Player(0));
    assert_eq!(
        (economy.ore, economy.crystal),
        (before.0 - cost.ore, before.1 - cost.crystal)
    );
    cost
}

#[test]
fn fresh_match_transition_discards_previous_paid_queue_without_refunding() {
    let mut app = started_app();
    let baseline = {
        let economy = app.world().resource::<Economies>().get(Team::Player(0));
        (economy.ore, economy.crystal)
    };
    assert_eq!(baseline, (32, 16));
    enqueue_actual_paid_worker(&mut app);
    app.world_mut().resource_mut::<LatestBattleEvent>().focus = Some(Vec3::new(99.0, 0.0, 99.0));
    restart_through_shared_scene(&mut app);
    assert!(app.world().resource::<BuildQueue>().0.is_empty());
    let economy = app.world().resource::<Economies>().get(Team::Player(0));
    assert_eq!(
        (economy.ore, economy.crystal),
        baseline,
        "previous match production must not refund into a new match"
    );
    assert!(app.world().resource::<LatestBattleEvent>().focus.is_none());
}

#[test]
fn loaded_match_transition_restores_paid_queue_and_remaps_producer() {
    let mut app = started_app();
    let cost = enqueue_actual_paid_worker(&mut app);
    let old_producer = app.world().resource::<BuildQueue>().0[0].producer_entity;
    let save = collect_save_game(app.world_mut()).unwrap();
    validate_save(&save).unwrap();
    let balance = (save.economies[0].ore, save.economies[0].crystal);
    let remaining = save.build_jobs[0].timer;
    app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(save);
    restart_through_shared_scene(&mut app);
    let queue = app.world().resource::<BuildQueue>();
    assert_eq!(queue.0.len(), 1);
    let job = queue.0[0];
    assert_ne!(job.producer_entity, old_producer);
    assert_eq!(
        app.world()
            .get::<Structure>(job.producer_entity)
            .unwrap()
            .id,
        "CommandCenter"
    );
    assert_eq!(job.cost.ore, cost.ore);
    assert_eq!(job.cost.crystal, cost.crystal);
    assert_eq!(job.timer, remaining);
    let economy = app.world().resource::<Economies>().get(Team::Player(0));
    assert_eq!(
        (economy.ore, economy.crystal),
        balance,
        "restored job was already paid and must not charge or refund again"
    );
}

#[test]
fn fresh_match_transition_clears_previous_battle_event_focus() {
    let mut app = started_app();
    app.world_mut().resource_mut::<LatestBattleEvent>().focus = Some(Vec3::new(99.0, 0.0, 99.0));
    restart_through_shared_scene(&mut app);
    assert!(
        app.world().resource::<LatestBattleEvent>().focus.is_none(),
        "Space must not focus a battle event from the previous map"
    );
}
