//! Real-peer human/AI control acceptance. Commands use the normal outbox;
//! every other gameplay access is read-only.
use super::*;

#[derive(Default, Debug)]
pub(super) struct ControlState {
    initialized: bool,
    initial_human_ids: Vec<u64>,
    initial_ai_ids: Vec<u64>,
    origin: Option<Vec3>,
    goals: Vec<Vec3>,
    sent_steps: usize,
    next_command_at: f32,
    queue_seen: bool,
    queue_preserved: bool,
    completed_route: bool,
    ai_factory: bool,
    ai_trained: bool,
    snapshots: BTreeMap<u64, serde_json::Value>,
}

#[derive(SystemParam)]
pub(super) struct ControlQueries<'w, 's> {
    economies: Res<'w, Economies>,
    units: Query<
        'w,
        's,
        (
            &'static NetworkEntityId,
            &'static Team,
            &'static Unit,
            &'static Transform,
            &'static Health,
            Option<&'static Weapon>,
            Option<&'static MoveOrder>,
            Option<&'static OrderQueue>,
            Option<&'static HarvestOrder>,
            Option<&'static ConstructOrder>,
        ),
        With<Unit>,
    >,
    structures: Query<
        'w,
        's,
        (
            &'static Team,
            &'static Structure,
            Option<&'static UnderConstruction>,
        ),
        With<Structure>,
    >,
}

fn goals(bounds: MapBounds, origin: Vec3) -> Vec<Vec3> {
    let first = verification_move_destination(bounds, origin);
    let delta = first - origin;
    let second = bounds.clamp_ground_point(first + Vec3::new(-delta.z, 0.0, delta.x), 1.0);
    vec![first, second, origin]
}

pub(super) fn drive(
    params: &mut OnlineVerificationParams,
    role: OnlineVerificationRole,
) -> Result<(), String> {
    let result = observe(params, role);
    if result.is_err() {
        params.harness.control.queue_preserved = false;
    }
    result
}

fn observe(
    params: &mut OnlineVerificationParams,
    role: OnlineVerificationRole,
) -> Result<(), String> {
    let human = Team::Player(1);
    let ai = Team::Player(2);
    let clock = params.match_state.start_time_sec;
    let harness = &mut *params.harness;
    let state = &mut harness.control;
    if !state.initialized {
        let worker = params
            .control
            .units
            .iter()
            .filter(|(_, team, unit, _, health, ..)| {
                **team == human && unit.id == "Worker" && health.current > 0.0
            })
            .min_by_key(|(id, ..)| id.0);
        let Some((id, _, _, position, ..)) = worker else {
            return Ok(());
        };
        state.origin = Some(position.translation);
        state.goals = goals(*params.map_bounds, position.translation);
        harness.tracked_unit_id = Some(id.0);
        harness.tracked_unit_origin = Some(position.translation);
        state.initial_human_ids = params
            .control
            .units
            .iter()
            .filter(|(_, team, ..)| **team == human)
            .map(|(id, ..)| id.0)
            .collect();
        state.initial_ai_ids = params
            .control
            .units
            .iter()
            .filter(|(_, team, ..)| **team == ai)
            .map(|(id, ..)| id.0)
            .collect();
        state.queue_preserved = false;
        state.initialized = true;
    }
    let worker_id = harness.tracked_unit_id.ok_or("missing tracked Worker")?;
    if role == OnlineVerificationRole::Player {
        let moved = verification_unit_position(&params.units, &params.interpolations, worker_id)
            .is_some_and(|p| {
                p.distance(state.origin.unwrap()) >= ONLINE_VERIFICATION_MOVE_DISTANCE
            });
        if state.sent_steps < state.goals.len()
            && clock >= 30.0
            && (state.sent_steps == 0 || (moved && harness.elapsed >= state.next_command_at))
        {
            let step = state.sent_steps;
            params.outbox.submit(OnlinePlayerCommand::UnitOrders {
                orders: vec![OnlineUnitOrderCommand {
                    unit_id: worker_id,
                    order: OnlineUnitOrderKind::Move {
                        destination: state.goals[step].to_array(),
                    },
                }],
                queue: step != 0,
            });
            state.sent_steps += 1;
            state.next_command_at = harness.elapsed + 1.0;
            harness.command_sent = true;
            harness.stage = OnlineVerificationStage::CommandSent;
        }
        state.completed_route |= state.sent_steps == 3
            && clock >= 46.0
            && verification_unit_position(&params.units, &params.interpolations, worker_id)
                .is_some_and(|p| p.distance(state.origin.unwrap()) < 1.5);
    } else {
        let economy = params.control.economies.get(human);
        if (economy.ore, economy.crystal) != (32, 16) {
            return Err(
                "remote human resources changed without a resource/production command".into(),
            );
        }
        let mut ids = Vec::new();
        for (id, team, _, position, health, _, movement, queue, harvest, construction) in
            &params.control.units
        {
            if *team != human {
                continue;
            }
            if health.current <= 0.0 {
                return Err("remote human unit died during control observation".into());
            }
            ids.push(id.0);
            if harvest.is_some() || construction.is_some() {
                return Err("host assigned harvesting/construction to remote Human Worker".into());
            }
            if id.0 != worker_id
                && (movement.is_some() || queue.is_some_and(|q| !q.orders.is_empty()))
            {
                return Err("untouched remote Human unit acquired an order".into());
            }
            if id.0 != worker_id {
                continue;
            }
            let queued = queue
                .map(|q| q.orders.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            state.queue_seen |= queued.len() == 2;
            let mut active = movement.map(|m| vec![m.target]).unwrap_or_default();
            for order in queued {
                let UnitQueuedOrder::Move(destination) = order else {
                    return Err("remote Human move queue was replaced by another order type".into());
                };
                active.push(*destination);
            }
            if !active.is_empty() {
                let valid_suffix = state.goals.windows(active.len()).any(|expected| {
                    expected
                        .iter()
                        .zip(&active)
                        .all(|(a, b)| a.distance(*b) < 0.1)
                });
                if !valid_suffix {
                    state.queue_preserved = false;
                    return Err("remote Human active/queued destinations changed".into());
                }
            }
            state.queue_preserved |= state.queue_seen;
            state.completed_route |= state.queue_seen
                && clock >= 46.0
                && active.is_empty()
                && position.translation.distance(state.origin.unwrap()) < 1.5;
        }
        ids.sort_unstable();
        state.initial_human_ids.sort_unstable();
        if ids != state.initial_human_ids {
            return Err("remote Human unit roster changed".into());
        }
        state.ai_factory |= params
            .control
            .structures
            .iter()
            .any(|(team, structure, building)| {
                *team == ai && structure.id == "VehicleFactory" && building.is_none()
            });
        state.ai_factory &= params.structures.iter().any(|(team, structure, health)| {
            *team == ai && structure.id == "VehicleFactory" && health.current > 0.0
        });
        state.ai_trained |=
            params
                .control
                .units
                .iter()
                .any(|(id, team, _, _, health, weapon, ..)| {
                    *team == ai
                        && health.current > 0.0
                        && weapon.is_some()
                        && !state.initial_ai_ids.contains(&id.0)
                });
    }
    let snapshot = if role == OnlineVerificationRole::Host {
        params.replication.send_baseline.as_ref()
    } else {
        params.replication.receive_baseline.as_ref()
    };
    if let Some(snapshot) = snapshot {
        state
            .snapshots
            .insert(snapshot.tick, summary(snapshot, worker_id));
        while state.snapshots.len() > 40 {
            state.snapshots.pop_first();
        }
        if role == OnlineVerificationRole::Player {
            state.ai_factory |= snapshot.entities.iter().any(|e| {
                e.team.to_game() == ai
                    && matches!(&e.kind, OnlineEntityKind::Structure{id} if id == "VehicleFactory")
                    && e.construction.is_none()
            });
            state.ai_trained |= snapshot.entities.iter().any(|e| e.team.to_game() == ai
                && !state.initial_ai_ids.contains(&e.id)
                && matches!(&e.kind, OnlineEntityKind::Unit{id} if registry::entity(id).is_some_and(|def| def.weapon.is_some())));
        }
    }
    harness.command_observed = state.completed_route;
    if clock >= 50.0 && state.completed_route && state.ai_factory && state.ai_trained {
        if role == OnlineVerificationRole::Host && !state.queue_seen {
            return Err("both queued Move orders were never observed on authoritative host".into());
        }
        harness.pass("human_control_preserved");
    }
    Ok(())
}

fn summary(snapshot: &OnlineWorldSnapshot, worker: u64) -> serde_json::Value {
    let mut ai = snapshot
        .entities
        .iter()
        .filter(|e| e.team.to_game() == Team::Player(2))
        .map(|e| {
            (
                e.id,
                match &e.kind {
                    OnlineEntityKind::Unit { id } | OnlineEntityKind::Structure { id } => {
                        id.as_str()
                    }
                    _ => "other",
                },
                e.construction.is_none(),
            )
        })
        .collect::<Vec<_>>();
    ai.sort_unstable_by_key(|e| e.0);
    serde_json::json!({"tick":snapshot.tick,"clock":snapshot.match_state.start_time_sec,"worker":snapshot.entities.iter().find(|e| e.id==worker).map(|e|e.translation),
        "human_economy":snapshot.economies.get(1).map(|e|[e.ore,e.crystal]),"ai_entities":ai})
}

pub(super) fn report(state: &ControlState) -> serde_json::Value {
    serde_json::json!({"sent_steps":state.sent_steps,"queue_seen_on_host":state.queue_seen,
        "queue_preserved_on_host":state.queue_preserved,"route_completed":state.completed_route,
        "ai_factory_completed":state.ai_factory,"ai_armed_produced":state.ai_trained,
        "goals":state.goals.iter().map(|p|p.to_array()).collect::<Vec<_>>(),"snapshots":state.snapshots.values().collect::<Vec<_>>()})
}
