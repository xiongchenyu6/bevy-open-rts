//! Opt-in actual guest ECS observations, separate from received network DTOs.
use super::super::*;
use super::OnlineObserver;

pub(in crate::online) fn record(
    mut observer: ResMut<OnlineObserver>,
    session: Res<OnlineSession>,
    replication: Res<OnlineMatchReplication>,
    entities: Query<(Entity, OnlineSnapshotSource<'static>)>,
    interpolation: Query<&NetworkInterpolation>,
    network_entities: Query<(Entity, &NetworkEntityId)>,
    economies: Res<Economies>,
    build_queue: Res<BuildQueue>,
    cooldowns: Res<SupportCooldowns>,
    match_state: Res<MatchState>,
) {
    let tick = replication.last_applied_tick;
    if !observer.enabled
        || session.is_host
        || session.phase != OnlinePhase::InMatch
        || tick == 0
        || observer.applied_worlds.contains_key(&tick)
    {
        return;
    }
    let mut actual = Vec::new();
    let mut classified = std::collections::HashSet::new();
    for (
        entity,
        (
            id,
            transform,
            team,
            unit,
            structure,
            resource,
            crate_item,
            health,
            faction,
            cargo,
            construction,
            veterancy,
        ),
    ) in &entities
    {
        let kind = if let Some(unit) = unit {
            OnlineEntityKind::Unit {
                id: unit.id.to_string(),
            }
        } else if let Some(structure) = structure {
            OnlineEntityKind::Structure {
                id: structure.id.to_string(),
            }
        } else if let Some(resource) = resource {
            OnlineEntityKind::Resource {
                kind: OnlineResourceKind::from_game(resource.kind),
                amount: resource.amount,
            }
        } else if let Some(crate_item) = crate_item {
            OnlineEntityKind::SupplyCrate {
                effect: OnlineSupplyCrateEffect::from_game(crate_item.effect),
            }
        } else {
            continue;
        };
        classified.insert(entity);
        let target = interpolation.get(entity).ok();
        let render = serde_json::json!({"translation":transform.translation.to_array(),"rotation":transform.rotation.to_array(),"scale":transform.scale.to_array()});
        let replicated = target.map(|target|serde_json::json!({"translation":target.translation.to_array(),"rotation":target.rotation.to_array(),"scale":target.scale.to_array()})).unwrap_or_else(||render.clone());
        actual.push(serde_json::json!({"id":id.0,"kind":kind,"team":OnlineEntityTeam::from_game(*team),
            "health":health.map(|h|[h.current,h.max]),"visual_faction":faction.map(|f|OnlineFaction::from_game(f.0)),
            "cargo":cargo.map(|c|[c.capacity,c.ore,c.crystal]),"construction":construction.map(|c|[c.remaining,c.total]),
            "veterancy":veterancy.map(|v|(v.rank,v.experience_points)),
            "render_transform":render,"replicated_transform":replicated,"interpolation_active":target.is_some()}));
    }
    actual.sort_by_key(|entity| entity["id"].as_u64());
    let ids = network_entities
        .iter()
        .map(|(entity, id)| (entity, id.0))
        .collect::<HashMap<_, _>>();
    let mut incomplete = network_entities
        .iter()
        .filter(|(entity, _)| !classified.contains(entity))
        .map(|(_, id)| id.0)
        .collect::<Vec<_>>();
    incomplete.sort_unstable();
    let actual_banks = economies
        .players
        .iter()
        .map(|e| OnlineEconomySnapshot {
            ore: e.ore,
            crystal: e.crystal,
            power_used: e.power_used,
            power_capacity: e.power_capacity,
            power_sabotage_remaining: e.power_sabotage_remaining,
            production_veterancy_ranks: e.production_veterancy_ranks.to_vec(),
        })
        .collect::<Vec<_>>();
    let jobs=build_queue.0.iter().map(|job|serde_json::json!({"team":OnlineEntityTeam::from_game(job.team),"action":OnlineBuildActionSnapshot::from_game(job.action),"producer_entity":ids.get(&job.producer_entity),"producer_id":job.producer_id,"timer":job.timer,"origin":job.origin.to_array(),"cost":[job.cost.ore,job.cost.crystal]})).collect::<Vec<_>>();
    let row = serde_json::json!({"applied_tick":tick,"semantics":"actual_guest_ecs_after_Update_deferred_commands",
        "entities":actual,"incomplete_network_entities":incomplete,"economies":actual_banks,"build_queue":jobs,
        "support_cooldowns":cooldowns.remaining,"support_initial_charge_started":cooldowns.initial_charge_started,
        "match_state":{"start_time_sec":match_state.start_time_sec,"remaining_teams":match_state.remaining_teams,"remaining_anchors":match_state.remaining_anchors,"active_anchor_teams":match_state.active_anchor_teams,"finished":!match_state.is_running()},
        "transform_semantics":"render_transform is actual interpolated Transform; replicated_transform reads actual NetworkInterpolation target when present, otherwise Transform"});
    observer.applied_worlds.insert(tick, row);
    observer.dirty = true;
    while observer.applied_worlds.len() > 40 {
        observer.applied_worlds.pop_first();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn fixture() -> App {
        let mut app = build_game_app(GameAppMode::Headless);
        for _ in 0..3 {
            app.update();
        }
        let mut session = app.world_mut().resource_mut::<OnlineSession>();
        session.phase = OnlinePhase::InMatch;
        session.is_host = false;
        drop(session);
        app.world_mut().resource_mut::<OnlineObserver>().enabled = true;
        app
    }
    fn snapshot(tick: u64, power: bool) -> OnlineWorldSnapshot {
        let mut entities = vec![OnlineEntitySnapshot {
            id: 501,
            kind: OnlineEntityKind::Unit {
                id: "Worker".into(),
            },
            team: OnlineEntityTeam::Player(0),
            translation: [2.0, 0.05, 2.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            health: Some([6.0, 6.0]),
            visual_faction: Some(OnlineFaction::Alliance),
            cargo: Some([6, 0, 0]),
            construction: None,
            veterancy: None,
        }];
        if power {
            entities.push(OnlineEntitySnapshot {
                id: 502,
                kind: OnlineEntityKind::Structure {
                    id: "PowerReactor".into(),
                },
                team: OnlineEntityTeam::Player(0),
                translation: [8.0, 0.0, 2.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
                health: Some([12.0, 12.0]),
                visual_faction: Some(OnlineFaction::Alliance),
                cargo: None,
                construction: None,
                veterancy: None,
            });
        }
        OnlineWorldSnapshot {
            protocol: RTS_ONLINE_PROTOCOL,
            tick,
            entities,
            economies: vec![OnlineEconomySnapshot {
                ore: 29,
                crystal: 15,
                power_used: 0,
                power_capacity: 26,
                power_sabotage_remaining: 0.0,
                production_veterancy_ranks: vec![0; 3],
            }],
            build_queue: vec![],
            support_cooldowns: vec![[0.0; SupportPowerKind::ALL.len()]],
            support_initial_charge_started: vec![[false; SupportPowerKind::ALL.len()]],
            match_state: OnlineMatchStateSnapshot {
                start_time_sec: tick as f32,
                remaining_teams: 2,
                remaining_anchors: 2,
                active_anchor_teams: vec![true; 2],
                finished: false,
            },
            transient_events: vec![],
        }
    }
    fn apply(app: &mut App, snapshot: OnlineWorldSnapshot) {
        app.world_mut()
            .resource_mut::<OnlineObserver>()
            .record(&snapshot);
        app.world_mut()
            .resource_mut::<OnlineMatchReplication>()
            .pending_snapshot = Some(snapshot);
        app.world_mut()
            .run_system_once(apply_pending_online_snapshot)
            .unwrap();
    }
    fn entity(app: &mut App, id: u64) -> Entity {
        app.world_mut()
            .query::<(Entity, &NetworkEntityId)>()
            .iter(app.world())
            .find(|(_, n)| n.0 == id)
            .unwrap()
            .0
    }
    #[test]
    fn actual_world_observer_sees_deferred_snapshot_spawns_and_despawns() {
        let mut app = fixture();
        apply(&mut app, snapshot(1, true));
        let power = entity(&mut app, 502);
        app.world_mut().run_system_once(record).unwrap();
        let observer = app.world().resource::<OnlineObserver>();
        let row = observer
            .applied_worlds
            .get(&1)
            .expect("actual ECS row after deferred spawn");
        assert_eq!(row["entities"].as_array().unwrap().len(), 2);
        assert_eq!(row["economies"][0]["ore"], 29);
        apply(&mut app, snapshot(2, false));
        assert!(
            app.world().get_entity(power).is_err(),
            "actual deferred despawn required"
        );
        app.world_mut().run_system_once(record).unwrap();
        let row = &app.world().resource::<OnlineObserver>().applied_worlds[&2];
        assert_eq!(row["entities"].as_array().unwrap().len(), 1);
        assert_eq!(row["entities"][0]["id"], 501);
    }
    #[test]
    fn actual_world_observer_detects_ecs_health_divergence_from_unchanged_received_dto() {
        let mut app = fixture();
        apply(&mut app, snapshot(1, false));
        let worker = entity(&mut app, 501);
        app.world_mut().get_mut::<Health>(worker).unwrap().current = 2.0;
        app.world_mut().run_system_once(record).unwrap();
        let observer = app.world().resource::<OnlineObserver>();
        assert_eq!(observer.snapshots[&1].entities[0].health, Some([6.0, 6.0]));
        let row = observer
            .applied_worlds
            .get(&1)
            .expect("must query actual actor, not received DTO");
        assert_eq!(row["entities"][0]["health"], serde_json::json!([2.0, 6.0]));
        assert_eq!(
            app.world().get::<Health>(worker).unwrap().current,
            2.0,
            "observer cannot repair divergent actor"
        );
    }
    #[test]
    fn actual_world_observer_is_disabled_without_writes_and_event_bounded_when_enabled() {
        let mut app = fixture();
        apply(&mut app, snapshot(1, false));
        app.world_mut().resource_mut::<OnlineObserver>().enabled = false;
        let worker = entity(&mut app, 501);
        let before = *app.world().get::<Transform>(worker).unwrap();
        app.world_mut().run_system_once(record).unwrap();
        assert!(
            app.world()
                .resource::<OnlineObserver>()
                .applied_worlds
                .is_empty()
        );
        assert_eq!(*app.world().get::<Transform>(worker).unwrap(), before);
        assert!(
            app.world()
                .resource::<OnlineCommandOutbox>()
                .pending
                .is_empty()
        );
        assert!(app.world().resource::<OnlineSession>().room.is_none());
        app.world_mut().resource_mut::<OnlineObserver>().enabled = true;
        for tick in 1..=50 {
            app.world_mut()
                .resource_mut::<OnlineMatchReplication>()
                .last_applied_tick = tick;
            app.world_mut().run_system_once(record).unwrap();
        }
        let rows = &app.world().resource::<OnlineObserver>().applied_worlds;
        assert_eq!(rows.len(), 40);
        assert_eq!(rows.first_key_value().unwrap().0, &11);
        let old = rows.clone();
        app.world_mut().run_system_once(record).unwrap();
        assert_eq!(app.world().resource::<OnlineObserver>().applied_worlds, old);
    }
    #[test]
    fn registered_last_observer_queries_snapshot_entities_after_update_deferred_commands() {
        let mut app = fixture();
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::InMatch
        );
        let packet = snapshot(1, true);
        app.world_mut()
            .resource_mut::<OnlineObserver>()
            .record(&packet);
        app.world_mut()
            .resource_mut::<OnlineMatchReplication>()
            .pending_snapshot = Some(packet);
        app.update();
        let row = app
            .world()
            .resource::<OnlineObserver>()
            .applied_worlds
            .get(&1)
            .expect("registered Last must see Update deferred spawns");
        assert_eq!(row["entities"].as_array().unwrap().len(), 2);
        assert_eq!(row["entities"][1]["id"], 502);
        assert_eq!(row["entities"][1]["construction"], serde_json::Value::Null);
        assert_eq!(
            row["entities"][1]["health"],
            serde_json::json!([12.0, 12.0])
        );
    }
    #[test]
    fn actual_world_observer_separates_interpolated_render_pose_from_component_target() {
        let mut app = fixture();
        apply(&mut app, snapshot(1, false));
        let mut packet = snapshot(2, false);
        packet.entities[0].translation[0] = 3.0;
        apply(&mut app, packet);
        app.world_mut().run_system_once(record).unwrap();
        let row = &app.world().resource::<OnlineObserver>().applied_worlds[&2];
        assert_eq!(
            row["entities"][0]["render_transform"]["translation"][0],
            2.0
        );
        assert_eq!(
            row["entities"][0]["replicated_transform"]["translation"][0],
            3.0
        );
        assert_eq!(row["entities"][0]["interpolation_active"], true);
    }
    #[test]
    fn registered_new_match_clears_received_and_actual_world_history_before_tick_one() {
        let mut app = fixture();
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..8 {
            app.update();
        }
        for tick in [1, 5] {
            let packet = snapshot(tick, true);
            app.world_mut()
                .resource_mut::<OnlineObserver>()
                .record(&packet);
            app.world_mut()
                .resource_mut::<OnlineMatchReplication>()
                .pending_snapshot = Some(packet);
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<OnlineObserver>()
                .applied_worlds
                .len(),
            2
        );
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::MainMenu);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        app.update();
        let mut packet = snapshot(1, false);
        packet.entities[0].id = 777;
        packet.entities[0].health = Some([4.0, 6.0]);
        app.world_mut()
            .resource_mut::<OnlineObserver>()
            .record(&packet);
        app.world_mut()
            .resource_mut::<OnlineMatchReplication>()
            .pending_snapshot = Some(packet);
        app.update();
        let worker = entity(&mut app, 777);
        assert_eq!(
            app.world().get::<Health>(worker).unwrap().current,
            4.0,
            "actual new match actor must apply"
        );
        let observer = app.world().resource::<OnlineObserver>();
        let row = &observer.applied_worlds[&1];
        assert_eq!(
            row["entities"][0]["id"], 777,
            "tick1 cannot reuse previous match actor"
        );
        assert_eq!(row["entities"][0]["health"], serde_json::json!([4.0, 6.0]));
        assert_eq!(observer.applied_worlds.len(), 1);
        assert_eq!(
            observer.snapshots.len(),
            1,
            "received DTO history must reset too"
        );
    }
}
