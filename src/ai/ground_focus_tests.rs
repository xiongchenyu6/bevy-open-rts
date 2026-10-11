use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn normal_easy_ground_wave_members_concentrate_fire_against_repaired_city() {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
        setup.map_path = SKIRMISH_MAPS
            .iter()
            .find(|m| m.id == "big_arena")
            .unwrap()
            .godot_path;
        setup.starting_resources = StartingResources::new(32, 16);
        setup.startup_loadout = StartupLoadoutMode::GodotSkirmish;
        setup.victory_condition = VictoryCondition::Annihilation;
        setup.visible_player = VisiblePlayer::all_players(Team::Player(0));
        setup.active_teams = vec![false; MAX_SKIRMISH_LOBBY_SLOTS];
        setup.player_controllers = vec![SkirmishPlayerController::None; MAX_SKIRMISH_LOBBY_SLOTS];
        setup.player_factions = vec![SkirmishFaction::Alliance; MAX_SKIRMISH_LOBBY_SLOTS];
        setup.player_spawn_slots = default_player_spawn_slots();
        setup.team_relations = TeamRelations::default();
        for (player, slot, difficulty) in
            [(0, 6, AiDifficulty::Easy), (1, 2, AiDifficulty::Beginner)]
        {
            setup.active_teams[player] = true;
            setup.player_controllers[player] = SkirmishPlayerController::Ai(difficulty);
            setup.player_factions[player] = SkirmishFaction::Chaos;
            setup.player_spawn_slots[player] = slot;
            setup
                .ai_difficulties
                .set_difficulty(Team::Player(player), difficulty);
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..6060 {
        app.update();
        let state = app.world().resource::<MatchState>();
        if state.start_time_sec >= 200.0 || (!state.is_running() && state.start_time_sec > 0.0) {
            break;
        }
    }
    let state = app.world().resource::<MatchState>();
    if !state.is_running() {
        assert!(
            state.active_anchor_teams[0],
            "Easy must survive normal match"
        );
        return;
    }
    let mut query = app.world_mut().query::<(
        Entity,
        &Team,
        &Unit,
        &Weapon,
        &AttackOrder,
        Option<&AiAttackWaveMember>,
    )>();
    let rows = query
        .iter(app.world())
        .filter(|(_, team, _, weapon, _, _)| **team == Team::Player(0) && weapon.can_attack_ground)
        .map(|(entity, _, unit, _, attack, member)| {
            (
                entity.to_bits(),
                unit.id,
                attack.target.to_bits(),
                member.is_some(),
            )
        })
        .collect::<Vec<_>>();
    eprintln!(
        "normal2668 at{} wave relationship{rows:?}",
        app.world().resource::<MatchState>().start_time_sec
    );
    assert_eq!(rows.len(), 3, "normal paid Easy army must exist");
    assert!(
        rows.iter().all(|r| r.3),
        "actual attackers must be assigned wave members"
    );
    let targets = rows
        .iter()
        .map(|r| r.2)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        targets.len(),
        1,
        "ground wave keeps split fire into individually repaired structures"
    );
    // The same registered director must leave explicit orders and player control
    // intact even when the other members have an unambiguous shared target.
    let actor = Entity::from_bits(rows[0].0);
    let focused = Entity::from_bits(rows[0].2);
    let mut targets = app
        .world_mut()
        .query::<(Entity, &Team, &Structure, &Health)>();
    let alternate = targets
        .iter(app.world())
        .find(|(e, team, _, hp)| *e != focused && **team == Team::Player(1) && hp.current > 0.0)
        .unwrap()
        .0;
    for protection in [
        "selected",
        "queue",
        "construct",
        "repair",
        "follow",
        "move",
        "human",
    ] {
        app.world_mut()
            .entity_mut(actor)
            .insert(AttackOrder { target: alternate });
        let unrelated = Vec3::new(0.0, 0.0, 0.0);
        match protection {
            "selected" => {
                app.world_mut().entity_mut(actor).insert(Selected);
            }
            "queue" => {
                app.world_mut().entity_mut(actor).insert(OrderQueue {
                    orders: std::collections::VecDeque::from([UnitQueuedOrder::Move(unrelated)]),
                });
            }
            "construct" => {
                app.world_mut()
                    .entity_mut(actor)
                    .insert(ConstructOrder { target: alternate });
            }
            "repair" => {
                app.world_mut()
                    .entity_mut(actor)
                    .insert(RepairOrder { target: alternate });
            }
            "follow" => {
                app.world_mut().entity_mut(actor).insert(FollowOrder {
                    target: alternate,
                    allow_enemy: true,
                    offset: Vec3::ZERO,
                });
            }
            "move" => {
                app.world_mut()
                    .entity_mut(actor)
                    .insert(MoveOrder { target: unrelated });
            }
            "human" => {
                app.world_mut()
                    .resource_mut::<MatchSetupSettings>()
                    .player_controllers[0] = SkirmishPlayerController::Human;
            }
            _ => unreachable!(),
        }
        app.world_mut().resource_mut::<AiDirector>().attack_timer[0] = 0.0;
        app.world_mut().run_system_once(ai_director).unwrap();
        assert_eq!(
            app.world().get::<AttackOrder>(actor).unwrap().target,
            alternate,
            "ground focus must preserve {protection} control"
        );
        app.world_mut().entity_mut(actor).remove::<(
            Selected,
            OrderQueue,
            ConstructOrder,
            RepairOrder,
            FollowOrder,
            MoveOrder,
        )>();
        app.world_mut()
            .resource_mut::<MatchSetupSettings>()
            .player_controllers[0] = SkirmishPlayerController::Ai(AiDifficulty::Easy);
        app.world_mut()
            .entity_mut(actor)
            .insert(AttackOrder { target: focused });
    }
}
