//! Serializable match records and local entity reference remapping.

use super::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedSettings {
    pub(crate) map_path: String,
    pub(crate) starting_ore: i32,
    pub(crate) starting_crystal: i32,
    pub(crate) active_teams: Vec<bool>,
    /// SkirmishFaction by `index()`.
    pub(crate) player_factions: Vec<usize>,
    pub(crate) player_color_slots: Vec<usize>,
    /// 0 = None, 1 = Human, 2 + AiDifficulty ordinal = Ai.
    pub(crate) player_controllers: Vec<u8>,
    pub(crate) player_spawn_slots: Vec<usize>,
    /// AiDifficulty ordinal per player.
    pub(crate) ai_difficulties: Vec<u8>,
    pub(crate) allied: Vec<Vec<bool>>,
    /// StartupLoadoutMode ordinal.
    pub(crate) startup_loadout: u8,
    /// VictoryCondition ordinal.
    pub(crate) victory_condition: u8,
    pub(crate) visible_team: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedUnit {
    pub(crate) key: u64,
    pub(crate) continuity: SavedUnitContinuity,
    pub(crate) id: String,
    pub(crate) team: usize,
    pub(crate) position: [f32; 3],
    pub(crate) yaw: f32,
    pub(crate) health: f32,
    pub(crate) rank: u8,
    pub(crate) experience: u32,
    pub(crate) cargo: Option<(i32, i32)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedStructure {
    pub(crate) key: u64,
    pub(crate) known: bool,
    pub(crate) effects: SavedEffects,
    pub(crate) weapon_remaining: f32,
    pub(crate) id: String,
    pub(crate) team: usize,
    pub(crate) position: [f32; 3],
    pub(crate) yaw: f32,
    pub(crate) health: f32,
    /// (remaining, total) build seconds when still under construction.
    pub(crate) construction: Option<(f32, f32)>,
    pub(crate) construction_cost: Option<(i32, i32)>,
    pub(crate) free_worker_origin: Option<[f32; 3]>,
    pub(crate) repair_remaining: Option<f32>,
    pub(crate) rally: Option<SavedRally>,
    pub(crate) garrison_count: usize,
}

/// Ground fallback and optional entity target are both part of rally behavior.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedRally {
    pub(crate) target: [f32; 3],
    /// RallyMode ordinal (0 move, 1 attack-move).
    pub(crate) mode: u8,
    pub(crate) target_entity: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedEconomy {
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
    pub(crate) power_sabotage_remaining: f32,
    pub(crate) production_veterancy_ranks: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SaveGame {
    pub(crate) weather: WeatherState,
    pub(crate) version: u32,
    pub(crate) settings: SavedSettings,
    pub(crate) clock_sec: f32,
    pub(crate) enemy_units_destroyed: u32,
    pub(crate) enemy_structures_destroyed: u32,
    pub(crate) units_lost: u32,
    pub(crate) economies: Vec<SavedEconomy>,
    pub(crate) support_remaining: Vec<Vec<f32>>,
    pub(crate) support_charge_started: Vec<Vec<bool>>,
    pub(crate) units: Vec<SavedUnit>,
    pub(crate) structures: Vec<SavedStructure>,
    /// (ResourceKind ordinal, position, remaining amount).
    pub(crate) resources: Vec<(u64, u8, [f32; 3], i32)>,
    pub(crate) mission_id: Option<String>,
    pub(crate) tutorial: Option<TutorialState>,
    pub(crate) mission_triggers: Vec<bool>,
    pub(crate) build_jobs: Vec<SavedBuildJob>,
    pub(crate) explored: Vec<bool>,
    pub(crate) fog_remnants: Vec<([f32; 3], f32)>,
    pub(crate) crates: Vec<SavedCrate>,
    pub(crate) ai: SavedAiDirector,
    pub(crate) support_entities: Vec<SavedSupportEntity>,
    pub(crate) structures_lost: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum SavedOrder {
    Move([f32; 3]),
    Attack(u64),
    Capture(u64),
    Garrison(u64),
    Repair(u64),
    Construct(u64),
    Harvest {
        target: u64,
        state: u8,
    },
    Follow {
        target: u64,
        offset: [f32; 3],
        allow_enemy: bool,
    },
    AttackMove([f32; 3]),
    Patrol {
        origin: [f32; 3],
        destination: [f32; 3],
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct SavedUnitContinuity {
    pub(crate) active: Vec<SavedOrder>,
    pub(crate) effects: SavedEffects,
    pub(crate) deployment: Option<SavedDeployment>,
    pub(crate) ai_wave_member: bool,
    pub(crate) ai_scout: Option<(Option<u64>, f32)>,
    pub(crate) mine_layer: Option<(f32, usize)>,
    pub(crate) queued: Vec<SavedOrder>,
    pub(crate) hold: bool,
    pub(crate) capture_elapsed: f32,
    pub(crate) harvest_remaining: f32,
    pub(crate) harvest_kind: Option<u8>,
    pub(crate) harvest_state: Option<u8>,
    pub(crate) patrol_forward: bool,
    pub(crate) weapon_remaining: f32,
    /// Damage, trigger radius, blast radius, arming time, source key.
    pub(crate) mine: Option<(f32, f32, f32, f32, Option<u64>)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedBuildJob {
    pub(crate) team: usize,
    pub(crate) producer: u64,
    pub(crate) producer_id: String,
    pub(crate) id: String,
    pub(crate) train: bool,
    pub(crate) timer: f32,
    pub(crate) origin: [f32; 3],
    pub(crate) cost: (i32, i32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedCrate {
    pub(crate) key: u64,
    pub(crate) position: [f32; 3],
    pub(crate) effect: u8,
    pub(crate) pickup_radius: f32,
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
    pub(crate) repair_radius: f32,
    pub(crate) repair_amount: f32,
}

pub(crate) fn save_order(order: &UnitQueuedOrder) -> SavedOrder {
    match order {
        UnitQueuedOrder::Move(v) => SavedOrder::Move(v.to_array()),
        UnitQueuedOrder::Attack(e) => SavedOrder::Attack(e.to_bits()),
        UnitQueuedOrder::Capture(e) => SavedOrder::Capture(e.to_bits()),
        UnitQueuedOrder::Garrison(e) => SavedOrder::Garrison(e.to_bits()),
        UnitQueuedOrder::Repair(e) => SavedOrder::Repair(e.to_bits()),
        UnitQueuedOrder::Construct(e) => SavedOrder::Construct(e.to_bits()),
        UnitQueuedOrder::Harvest { target, state } => SavedOrder::Harvest {
            target: target.to_bits(),
            state: *state as u8,
        },
        UnitQueuedOrder::Follow { target, offset } => SavedOrder::Follow {
            target: target.to_bits(),
            offset: offset.to_array(),
            allow_enemy: false,
        },
        UnitQueuedOrder::ForceFollow { target, offset } => SavedOrder::Follow {
            target: target.to_bits(),
            offset: offset.to_array(),
            allow_enemy: true,
        },
        UnitQueuedOrder::AttackMove(v) => SavedOrder::AttackMove(v.to_array()),
        UnitQueuedOrder::Patrol {
            origin,
            destination,
        } => SavedOrder::Patrol {
            origin: origin.to_array(),
            destination: destination.to_array(),
        },
    }
}

pub(crate) fn load_order(
    order: &SavedOrder,
    remap: &std::collections::HashMap<u64, Entity>,
) -> Option<UnitQueuedOrder> {
    Some(match order {
        SavedOrder::Move(v) => UnitQueuedOrder::Move(Vec3::from_array(*v)),
        SavedOrder::Attack(e) => UnitQueuedOrder::Attack(*remap.get(e)?),
        SavedOrder::Capture(e) => UnitQueuedOrder::Capture(*remap.get(e)?),
        SavedOrder::Garrison(e) => UnitQueuedOrder::Garrison(*remap.get(e)?),
        SavedOrder::Repair(e) => UnitQueuedOrder::Repair(*remap.get(e)?),
        SavedOrder::Construct(e) => UnitQueuedOrder::Construct(*remap.get(e)?),
        SavedOrder::Harvest { target, state } => UnitQueuedOrder::Harvest {
            target: *remap.get(target)?,
            state: match state {
                1 => HarvestState::Collecting,
                2 => HarvestState::MovingToDropoff,
                _ => HarvestState::MovingToResource,
            },
        },
        SavedOrder::Follow {
            target,
            offset,
            allow_enemy,
        } => {
            if *allow_enemy {
                UnitQueuedOrder::ForceFollow {
                    target: *remap.get(target)?,
                    offset: Vec3::from_array(*offset),
                }
            } else {
                UnitQueuedOrder::Follow {
                    target: *remap.get(target)?,
                    offset: Vec3::from_array(*offset),
                }
            }
        }
        SavedOrder::AttackMove(v) => UnitQueuedOrder::AttackMove(Vec3::from_array(*v)),
        SavedOrder::Patrol {
            origin,
            destination,
        } => UnitQueuedOrder::Patrol {
            origin: Vec3::from_array(*origin),
            destination: Vec3::from_array(*destination),
        },
    })
}

pub(crate) fn collect_unit_continuity(world: &World, entity: Entity) -> SavedUnitContinuity {
    let mut state = SavedUnitContinuity::default();
    // Multiple order components can coexist (attack targets acquired while
    // attack-moving, for example). Preserve each instead of flattening them.
    if let Some(o) = world.get::<MoveOrder>(entity) {
        state.active.push(SavedOrder::Move(o.target.to_array()));
    }
    if let Some(o) = world.get::<AttackMoveOrder>(entity) {
        state
            .active
            .push(SavedOrder::AttackMove(o.destination.to_array()));
    }
    if let Some(o) = world.get::<AttackOrder>(entity) {
        state.active.push(SavedOrder::Attack(o.target.to_bits()));
    }
    if let Some(o) = world.get::<CaptureOrder>(entity) {
        state.active.push(SavedOrder::Capture(o.target.to_bits()));
        state.capture_elapsed = o.elapsed;
    }
    if let Some(o) = world.get::<GarrisonOrder>(entity) {
        state.active.push(SavedOrder::Garrison(o.target.to_bits()));
    }
    if let Some(o) = world.get::<RepairOrder>(entity) {
        state.active.push(SavedOrder::Repair(o.target.to_bits()));
    }
    if let Some(o) = world.get::<ConstructOrder>(entity) {
        state.active.push(SavedOrder::Construct(o.target.to_bits()));
    }
    if let Some(o) = world.get::<FollowOrder>(entity) {
        state.active.push(SavedOrder::Follow {
            target: o.target.to_bits(),
            offset: o.offset.to_array(),
            allow_enemy: o.allow_enemy,
        });
    }
    if let Some(o) = world.get::<PatrolOrder>(entity) {
        state.active.push(SavedOrder::Patrol {
            origin: o.origin.to_array(),
            destination: o.destination.to_array(),
        });
        state.patrol_forward = o.moving_to_destination;
    }
    if let Some(o) = world.get::<HarvestOrder>(entity) {
        if let Some(target) = o.resource {
            state.active.push(SavedOrder::Harvest {
                target: target.to_bits(),
                state: o.state as u8,
            });
        }
        state.harvest_state = Some(o.state as u8);
        state.harvest_remaining = o.collect_remaining;
        state.harvest_kind = o.last_kind.map(|k| k as u8);
    }
    if let Some(q) = world.get::<OrderQueue>(entity) {
        state.queued = q.orders.iter().map(save_order).collect();
    }
    state.effects = collect_effects(world, entity);
    state.ai_wave_member = world.get::<AiAttackWaveMember>(entity).is_some();
    state.ai_scout = world
        .get::<AiDroneScout>(entity)
        .map(|s| (s.last_target.map(Entity::to_bits), s.cooldown_remaining));
    state.mine_layer = world
        .get::<MineLayer>(entity)
        .map(|l| (l.cooldown, l.deploy_index));
    state.deployment = world.get::<DeployedSiegeMode>(entity).and_then(|d| {
        let unit = world.get::<Unit>(entity)?;
        let weapon = world.get::<Weapon>(entity)?;
        let vision = world.get::<VisionRadius>(entity)?;
        Some(SavedDeployment {
            previous_hold: d.previous_hold_position,
            base: [
                d.base_speed,
                d.base_attack_range,
                d.base_attack_damage,
                d.base_attack_interval,
                d.base_structure_damage_multiplier,
                d.base_sight_range,
            ],
            current: [
                unit.speed,
                weapon.range,
                weapon.damage,
                weapon.cooldown,
                weapon.structure_damage_multiplier,
                vision.0,
            ],
        })
    });
    state.hold = world.get::<HoldPosition>(entity).is_some_and(|o| o.enabled);
    state.weapon_remaining = world.get::<Weapon>(entity).map_or(0.0, |w| w.cooldown_left);
    state.mine = world.get::<Mine>(entity).map(|m| {
        (
            m.damage,
            m.trigger_radius,
            m.blast_radius,
            m.arming_remaining,
            m.source.map(Entity::to_bits),
        )
    });
    state
}

pub(crate) fn restore_unit_continuity(
    world: &mut World,
    entity: Entity,
    state: &SavedUnitContinuity,
    remap: &std::collections::HashMap<u64, Entity>,
) {
    restore_effects(world, entity, &state.effects);
    if state.ai_wave_member {
        world.entity_mut(entity).insert(AiAttackWaveMember);
    }
    if let Some((last, cooldown_remaining)) = state.ai_scout {
        world.entity_mut(entity).insert(AiDroneScout {
            last_target: last.and_then(|key| remap.get(&key).copied()),
            cooldown_remaining,
        });
    }
    if let Some((cooldown, deploy_index)) = state.mine_layer
        && let Some(mut layer) = world.get_mut::<MineLayer>(entity)
    {
        layer.cooldown = cooldown;
        layer.deploy_index = deploy_index;
    }
    if let Some(d) = &state.deployment {
        world.entity_mut(entity).insert(DeployedSiegeMode {
            previous_hold_position: d.previous_hold,
            base_speed: d.base[0],
            base_attack_range: d.base[1],
            base_attack_damage: d.base[2],
            base_attack_interval: d.base[3],
            base_structure_damage_multiplier: d.base[4],
            base_sight_range: d.base[5],
        });
        if let Some(mut unit) = world.get_mut::<Unit>(entity) {
            unit.speed = d.current[0];
        }
        if let Some(mut weapon) = world.get_mut::<Weapon>(entity) {
            weapon.range = d.current[1];
            weapon.damage = d.current[2];
            weapon.cooldown = d.current[3];
            weapon.structure_damage_multiplier = d.current[4];
        }
        if let Some(mut vision) = world.get_mut::<VisionRadius>(entity) {
            vision.0 = d.current[5];
        }
    }
    for saved in &state.active {
        let Some(order) = load_order(saved, remap) else {
            continue;
        };
        let mut target = world.entity_mut(entity);
        match order {
            UnitQueuedOrder::Move(target_position) => {
                target.insert(MoveOrder {
                    target: target_position,
                });
            }
            UnitQueuedOrder::Attack(target_entity) => {
                target.insert(AttackOrder {
                    target: target_entity,
                });
            }
            UnitQueuedOrder::Capture(target_entity) => {
                target.insert(CaptureOrder {
                    target: target_entity,
                    elapsed: state.capture_elapsed,
                });
            }
            UnitQueuedOrder::Garrison(target_entity) => {
                target.insert(GarrisonOrder {
                    target: target_entity,
                });
            }
            UnitQueuedOrder::Repair(target_entity) => {
                target.insert(RepairOrder {
                    target: target_entity,
                });
            }
            UnitQueuedOrder::Construct(target_entity) => {
                target.insert(ConstructOrder {
                    target: target_entity,
                });
            }
            UnitQueuedOrder::Harvest {
                target: target_entity,
                state: harvest_state,
            } => {
                target.insert(HarvestOrder {
                    resource: Some(target_entity),
                    state: harvest_state,
                    collect_remaining: state.harvest_remaining,
                    last_kind: state.harvest_kind.map(|k| {
                        if k == ResourceKind::Crystal as u8 {
                            ResourceKind::Crystal
                        } else {
                            ResourceKind::Ore
                        }
                    }),
                });
            }
            UnitQueuedOrder::Follow {
                target: target_entity,
                offset,
            }
            | UnitQueuedOrder::ForceFollow {
                target: target_entity,
                offset,
            } => {
                target.insert(FollowOrder {
                    target: target_entity,
                    offset,
                    allow_enemy: matches!(
                        saved,
                        SavedOrder::Follow {
                            allow_enemy: true,
                            ..
                        }
                    ),
                });
            }
            UnitQueuedOrder::AttackMove(destination) => {
                target.insert(AttackMoveOrder { destination });
            }
            UnitQueuedOrder::Patrol {
                origin,
                destination,
            } => {
                target.insert(PatrolOrder {
                    origin,
                    destination,
                    moving_to_destination: state.patrol_forward,
                });
            }
        }
    }
    if state.harvest_state.is_some() && world.get::<HarvestOrder>(entity).is_none() {
        world.entity_mut(entity).insert(HarvestOrder {
            resource: None,
            state: match state.harvest_state {
                Some(1) => HarvestState::Collecting,
                Some(2) => HarvestState::MovingToDropoff,
                _ => HarvestState::MovingToResource,
            },
            collect_remaining: state.harvest_remaining,
            last_kind: state.harvest_kind.map(|k| {
                if k == ResourceKind::Crystal as u8 {
                    ResourceKind::Crystal
                } else {
                    ResourceKind::Ore
                }
            }),
        });
    }
    world.entity_mut(entity).insert((
        HoldPosition {
            enabled: state.hold,
        },
        OrderQueue {
            orders: state
                .queued
                .iter()
                .filter_map(|o| load_order(o, remap))
                .collect(),
        },
    ));
    if let Some(mut weapon) = world.get_mut::<Weapon>(entity) {
        weapon.cooldown_left = state.weapon_remaining;
    }
    if let Some((damage, trigger_radius, blast_radius, arming_remaining, source)) = state.mine {
        world.entity_mut(entity).insert(Mine {
            damage,
            trigger_radius,
            blast_radius,
            arming_remaining,
            source: source.and_then(|key| remap.get(&key).copied()),
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct SavedEffects {
    pub(crate) emp: Option<f32>,
    pub(crate) shield: Option<(f32, f32)>,
    pub(crate) chrono: Option<(f32, f32)>,
    pub(crate) projector_remaining: Option<f32>,
    pub(crate) income_remaining: Option<f32>,
}

pub(crate) fn collect_effects(world: &World, entity: Entity) -> SavedEffects {
    SavedEffects {
        income_remaining: world.get::<IncomeSource>(entity).map(|v| v.remaining),
        emp: world.get::<EmpDisabled>(entity).map(|v| v.remaining),
        shield: world
            .get::<SupportShield>(entity)
            .map(|v| (v.remaining, v.damage_scale)),
        chrono: world
            .get::<ChronoRelay>(entity)
            .map(|v| (v.remaining, v.speed_multiplier)),
        projector_remaining: world
            .get::<MobileShieldProjector>(entity)
            .map(|v| v.refresh_remaining),
    }
}

pub(crate) fn restore_effects(world: &mut World, entity: Entity, effects: &SavedEffects) {
    if let Some(remaining) = effects.income_remaining
        && let Some(mut income) = world.get_mut::<IncomeSource>(entity)
    {
        income.remaining = remaining;
    }
    if let Some(remaining) = effects.emp {
        world.entity_mut(entity).insert(EmpDisabled { remaining });
    }
    if let Some((remaining, damage_scale)) = effects.shield {
        world.entity_mut(entity).insert(SupportShield {
            remaining,
            damage_scale,
        });
    }
    if let Some((remaining, speed_multiplier)) = effects.chrono {
        world.entity_mut(entity).insert(ChronoRelay {
            remaining,
            speed_multiplier,
        });
    }
    if let Some(remaining) = effects.projector_remaining
        && let Some(mut projector) = world.get_mut::<MobileShieldProjector>(entity)
    {
        projector.refresh_remaining = remaining;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedDeployment {
    pub(crate) previous_hold: bool,
    pub(crate) base: [f32; 6],
    pub(crate) current: [f32; 6],
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct SavedAiDirector {
    pub(crate) production_timer: Vec<f32>,
    pub(crate) production_cursor: Vec<usize>,
    pub(crate) attack_timer: Vec<f32>,
    pub(crate) opening_attack_grace_applied: Vec<bool>,
    pub(crate) build_timer: Vec<f32>,
    pub(crate) construction_timer: Vec<f32>,
    pub(crate) capture_timer: Vec<f32>,
    pub(crate) saboteur_timer: Vec<f32>,
    pub(crate) support_timer: Vec<f32>,
    pub(crate) repair_timer: Vec<f32>,
}

pub(crate) fn collect_ai_director(d: &AiDirector) -> SavedAiDirector {
    SavedAiDirector {
        production_timer: d.production_timer.clone(),
        production_cursor: d.production_cursor.clone(),
        attack_timer: d.attack_timer.clone(),
        opening_attack_grace_applied: d.opening_attack_grace_applied.clone(),
        build_timer: d.build_timer.clone(),
        construction_timer: d.construction_timer.clone(),
        capture_timer: d.capture_timer.clone(),
        saboteur_timer: d.saboteur_timer.clone(),
        support_timer: d.support_timer.clone(),
        repair_timer: d.repair_timer.clone(),
    }
}

pub(crate) fn restore_ai_director(d: &mut AiDirector, s: &SavedAiDirector) {
    d.production_timer = s.production_timer.clone();
    d.production_cursor = s.production_cursor.clone();
    d.attack_timer = s.attack_timer.clone();
    d.opening_attack_grace_applied = s.opening_attack_grace_applied.clone();
    d.build_timer = s.build_timer.clone();
    d.construction_timer = s.construction_timer.clone();
    d.capture_timer = s.capture_timer.clone();
    d.saboteur_timer = s.saboteur_timer.clone();
    d.support_timer = s.support_timer.clone();
    d.repair_timer = s.repair_timer.clone();
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum SavedSupportEntity {
    Strike {
        position: [f32; 3],
        remaining: f32,
        radius: f32,
        damage: f32,
        impact_scale: f32,
        team: usize,
    },
    Paradrop {
        target: [f32; 3],
        remaining: f32,
        team: usize,
    },
    Reveal {
        position: [f32; 3],
        remaining: f32,
        radius: f32,
        team: usize,
    },
    Warning {
        position: [f32; 3],
        remaining: f32,
        radius: f32,
        color: [f32; 4],
    },
}

pub(crate) fn collect_support_entities(world: &mut World) -> Vec<SavedSupportEntity> {
    let mut entities = Vec::new();
    for (effect, transform) in world
        .query::<(&PendingOrbitalStrike, &Transform)>()
        .iter(world)
    {
        entities.push(SavedSupportEntity::Strike {
            position: transform.translation.to_array(),
            remaining: effect.remaining,
            radius: effect.radius,
            damage: effect.damage,
            impact_scale: effect.impact_scale,
            team: effect.team.economy_index().unwrap_or(usize::MAX),
        });
    }
    for effect in world.query::<&PendingParadrop>().iter(world) {
        entities.push(SavedSupportEntity::Paradrop {
            target: effect.target.to_array(),
            remaining: effect.remaining,
            team: effect.team.economy_index().unwrap_or(usize::MAX),
        });
    }
    for (effect, transform, team) in world
        .query::<(&TemporarySupportReveal, &Transform, &Team)>()
        .iter(world)
    {
        entities.push(SavedSupportEntity::Reveal {
            position: transform.translation.to_array(),
            remaining: effect.remaining,
            radius: effect.radius,
            team: team.economy_index().unwrap_or(usize::MAX),
        });
    }
    for (effect, transform) in world.query::<(&SupportWarning, &Transform)>().iter(world) {
        entities.push(SavedSupportEntity::Warning {
            position: transform.translation.to_array(),
            remaining: effect.remaining,
            radius: effect.radius,
            color: {
                let c = effect.color.to_srgba();
                [c.red, c.green, c.blue, c.alpha]
            },
        });
    }
    entities
}

pub(crate) fn restore_support_entities(commands: &mut Commands, entities: &[SavedSupportEntity]) {
    let team = |i| {
        if i == usize::MAX {
            Team::Neutral
        } else {
            Team::Player(i)
        }
    };
    for entity in entities {
        match entity {
            SavedSupportEntity::Strike {
                position,
                remaining,
                radius,
                damage,
                impact_scale,
                team: i,
            } => {
                commands.spawn((
                    Transform::from_translation(Vec3::from_array(*position)),
                    PendingOrbitalStrike {
                        remaining: *remaining,
                        radius: *radius,
                        damage: *damage,
                        impact_scale: *impact_scale,
                        team: team(*i),
                    },
                    MatchScopedEntity,
                ));
            }
            SavedSupportEntity::Paradrop {
                target,
                remaining,
                team: i,
            } => {
                commands.spawn((
                    Transform::from_translation(Vec3::from_array(*target)),
                    PendingParadrop {
                        remaining: *remaining,
                        team: team(*i),
                        target: Vec3::from_array(*target),
                        unit_paths: SupportPowerKind::Paradrop.definition().unit_paths,
                    },
                    MatchScopedEntity,
                ));
            }
            SavedSupportEntity::Reveal {
                position,
                remaining,
                radius,
                team: i,
            } => {
                commands.spawn((
                    Transform::from_translation(Vec3::from_array(*position)),
                    TemporarySupportReveal {
                        remaining: *remaining,
                        radius: *radius,
                    },
                    team(*i),
                    VisibilityState { visible: true },
                    VisionRadius(*radius),
                    MatchScopedEntity,
                ));
            }
            SavedSupportEntity::Warning {
                position,
                remaining,
                radius,
                color,
            } => {
                commands.spawn((
                    Transform::from_translation(Vec3::from_array(*position)),
                    SupportWarning {
                        remaining: *remaining,
                        radius: *radius,
                        color: Color::srgba(color[0], color[1], color[2], color[3]),
                    },
                    MatchScopedEntity,
                ));
            }
        }
    }
}

pub(crate) fn validate_save(save: &SaveGame) -> Result<(), String> {
    if save.version != SAVE_VERSION {
        return Err(format!(
            "This save uses unsupported format {}",
            save.version
        ));
    }
    if !save.weather.is_valid() {
        return Err("Invalid saved weather".to_string());
    }
    if !save.clock_sec.is_finite() || save.clock_sec < 0.0 {
        return Err("Invalid match clock".to_string());
    }
    let n = save.settings.active_teams.len();
    let settings = &save.settings;
    if skirmish_map_by_path(&settings.map_path).is_none() {
        return Err("Unknown saved map".to_string());
    }
    if n == 0
        || settings.visible_team >= n
        || settings.player_factions.len() != n
        || settings.player_controllers.len() != n
        || settings.player_spawn_slots.len() != n
        || settings.player_color_slots.len() > n
        || settings.ai_difficulties.len() != n
        || settings.allied.len() != n
        || settings.allied.iter().any(|row| row.len() != n)
        || settings
            .player_factions
            .iter()
            .any(|i| *i >= SkirmishFaction::ALL.len())
        || settings.player_controllers.iter().any(|i| *i > 5)
        || settings.ai_difficulties.iter().any(|i| *i > 3)
        || settings.victory_condition as usize >= VictoryCondition::ALL.len()
    {
        return Err("Invalid saved player settings".to_string());
    }
    if let Some(tutorial) = &save.tutorial
        && (tutorial.initial_ore < 0
            || tutorial.last_cargo < 0
            || tutorial.move_origin.iter().any(|v| !v.is_finite()))
    {
        return Err("Invalid saved training progress".to_string());
    }
    if let Some(id) = &save.mission_id {
        let mission = CAMPAIGN_MISSIONS
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| "Unknown saved mission".to_string())?;
        if save.mission_triggers.len() != mission.triggers.len() {
            return Err("Invalid saved mission progress".to_string());
        }
    } else if !save.mission_triggers.is_empty() {
        return Err("Mission state has no mission".to_string());
    }
    if !save.explored.is_empty() && save.explored.len() != FOG_OVERLAY_RES * FOG_OVERLAY_RES {
        return Err("Invalid saved explored terrain".to_string());
    }
    let mut keys = std::collections::HashSet::new();
    for (key, team, id, position, health) in save
        .units
        .iter()
        .map(|u| (u.key, u.team, u.id.as_str(), u.position, u.health))
        .chain(
            save.structures
                .iter()
                .map(|s| (s.key, s.team, s.id.as_str(), s.position, s.health)),
        )
    {
        if !keys.insert(key)
            || (team >= n && team != usize::MAX)
            || registry::entity(id).is_none()
            || position.iter().any(|v| !v.is_finite())
            || !health.is_finite()
            || health <= 0.0
        {
            return Err("Invalid saved battlefield entity".to_string());
        }
    }
    for (key, kind, position, amount) in &save.resources {
        if !keys.insert(*key)
            || *kind > 1
            || *amount <= 0
            || position.iter().any(|v| !v.is_finite())
        {
            return Err("Invalid saved resource".to_string());
        }
    }
    for item in &save.crates {
        if !keys.insert(item.key) || item.effect > 2 || item.position.iter().any(|v| !v.is_finite())
        {
            return Err("Invalid saved supply crate".to_string());
        }
    }
    let ai = &save.ai;
    let timers = [
        &ai.production_timer,
        &ai.attack_timer,
        &ai.build_timer,
        &ai.construction_timer,
        &ai.capture_timer,
        &ai.saboteur_timer,
        &ai.support_timer,
        &ai.repair_timer,
    ];
    let count = ai.production_timer.len();
    if count > n
        || ai.production_cursor.len() != count
        || ai.opening_attack_grace_applied.len() != count
        || timers
            .iter()
            .any(|row| row.len() != count || row.iter().any(|v| !v.is_finite()))
    {
        return Err("Invalid saved AI state".to_string());
    }
    if save.support_remaining.len() != save.support_charge_started.len()
        || save.support_remaining.len() > n
        || save
            .support_remaining
            .iter()
            .any(|row| row.len() != SupportPowerKind::ALL.len())
        || save
            .support_charge_started
            .iter()
            .any(|row| row.len() != SupportPowerKind::ALL.len())
    {
        return Err("Invalid saved support charges".to_string());
    }
    for entity in &save.support_entities {
        let (position, remaining, radius, team) = match entity {
            SavedSupportEntity::Strike {
                position,
                remaining,
                radius,
                damage,
                impact_scale,
                team,
            } => {
                if !damage.is_finite()
                    || *damage < 0.0
                    || !impact_scale.is_finite()
                    || *impact_scale < 0.0
                {
                    return Err("Invalid saved strike".to_string());
                }
                (position, *remaining, *radius, Some(*team))
            }
            SavedSupportEntity::Paradrop {
                target,
                remaining,
                team,
            } => (target, *remaining, 0.0, Some(*team)),
            SavedSupportEntity::Reveal {
                position,
                remaining,
                radius,
                team,
            } => (position, *remaining, *radius, Some(*team)),
            SavedSupportEntity::Warning {
                position,
                remaining,
                radius,
                color,
            } => {
                if color.iter().any(|v| !v.is_finite()) {
                    return Err("Invalid saved warning".to_string());
                }
                (position, *remaining, *radius, None)
            }
        };
        if position.iter().any(|v| !v.is_finite())
            || !remaining.is_finite()
            || !radius.is_finite()
            || radius < 0.0
            || team.is_some_and(|i| i >= n && i != usize::MAX)
        {
            return Err("Invalid saved support effect".to_string());
        }
    }
    for unit in &save.units {
        let c = &unit.continuity;
        validate_effects(&c.effects)?;
        if c.deployment.as_ref().is_some_and(|d| {
            d.base
                .iter()
                .chain(&d.current)
                .any(|v| !v.is_finite() || *v < 0.0)
        }) || c
            .ai_scout
            .is_some_and(|(_, cooldown)| !cooldown.is_finite())
            || c.mine_layer
                .is_some_and(|(cooldown, _)| !cooldown.is_finite())
            || c.mine.is_some_and(|(damage, trigger, blast, arming, _)| {
                [damage, trigger, blast, arming]
                    .iter()
                    .any(|v| !v.is_finite())
            })
        {
            return Err("Invalid saved deployment or mine".to_string());
        }
        if !unit.yaw.is_finite()
            || unit.rank > VETERANCY_MAX_RANK
            || unit
                .cargo
                .is_some_and(|(ore, crystal)| ore < 0 || crystal < 0)
            || !c.capture_elapsed.is_finite()
            || c.capture_elapsed < 0.0
            || !c.harvest_remaining.is_finite()
            || c.harvest_remaining < 0.0
            || !c.weapon_remaining.is_finite()
            || c.harvest_kind.is_some_and(|kind| kind > 1)
            || c.harvest_state.is_some_and(|state| state > 2)
        {
            return Err("Invalid saved unit state".to_string());
        }
        for order in c.active.iter().chain(&c.queued) {
            let position = match order {
                SavedOrder::Move(p) | SavedOrder::AttackMove(p) => Some(p),
                SavedOrder::Follow { offset, .. } => Some(offset),
                SavedOrder::Patrol {
                    origin,
                    destination,
                } => {
                    if destination.iter().any(|v| !v.is_finite()) {
                        return Err("Invalid saved patrol destination".to_string());
                    }
                    Some(origin)
                }
                _ => None,
            };
            if position.is_some_and(|p| p.iter().any(|v| !v.is_finite())) {
                return Err("Invalid saved order position".to_string());
            }
            if matches!(order, SavedOrder::Harvest { state, .. } if *state > 2) {
                return Err("Invalid saved harvest order".to_string());
            }
        }
    }
    for structure in &save.structures {
        validate_effects(&structure.effects)?;
        if !structure.yaw.is_finite()
            || structure.construction.is_some_and(|(remaining, total)| {
                !remaining.is_finite()
                    || !total.is_finite()
                    || remaining < 0.0
                    || total <= 0.0
                    || remaining > total
            })
            || structure
                .rally
                .as_ref()
                .is_some_and(|rally| rally.mode > 1 || rally.target.iter().any(|v| !v.is_finite()))
        {
            return Err("Invalid saved structure state".to_string());
        }
    }
    if save
        .economies
        .iter()
        .any(|e| e.ore < 0 || e.crystal < 0 || !e.power_sabotage_remaining.is_finite())
        || save
            .support_remaining
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("Invalid saved economy or support state".to_string());
    }
    for structure in &save.structures {
        if let Some(target) = structure.rally.as_ref().and_then(|r| r.target_entity) {
            let owned_actor = save
                .units
                .iter()
                .any(|u| u.key == target && u.team == structure.team)
                || save
                    .structures
                    .iter()
                    .any(|s| s.key == target && s.team == structure.team);
            let resource = save.resources.iter().any(|r| r.0 == target && r.3 > 0);
            if !owned_actor && !resource {
                return Err("Invalid saved rally target".to_string());
            }
        }
    }
    for job in &save.build_jobs {
        if job.team >= n
            || !keys.contains(&job.producer)
            || registry::entity(&job.id).is_none()
            || registry::entity(&job.producer_id).is_none()
            || !job.timer.is_finite()
            || job.timer < 0.0
            || job.cost.0 < 0
            || job.cost.1 < 0
        {
            return Err("Invalid saved production queue".to_string());
        }
    }
    Ok(())
}

fn validate_effects(e: &SavedEffects) -> Result<(), String> {
    if e.income_remaining.is_some_and(|v| !v.is_finite())
        || e.emp.is_some_and(|v| !v.is_finite())
        || e.projector_remaining.is_some_and(|v| !v.is_finite())
        || e.shield.is_some_and(|(remaining, scale)| {
            !remaining.is_finite() || !scale.is_finite() || !(0.0..=1.0).contains(&scale)
        })
        || e.chrono.is_some_and(|(remaining, speed)| {
            !remaining.is_finite() || !speed.is_finite() || speed <= 0.0
        })
    {
        return Err("Invalid saved status effect".to_string());
    }
    Ok(())
}
