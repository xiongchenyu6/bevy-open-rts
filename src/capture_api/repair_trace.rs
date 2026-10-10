//! Opt-in read-only JSONL for real skirmish repair/damage diagnosis.
use super::*;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
};

pub(super) struct RepairTrace {
    output: BufWriter<File>,
    from_seconds: f32,
    defender: Team,
    attacker: Team,
    economy_team: Team,
    structure_id: Option<String>,
    cargo: BTreeMap<u64, [i32; 2]>,
    bank: Option<[i32; 2]>,
    target: Option<(u64, f32, Option<f32>)>,
}

impl RepairTrace {
    pub(super) fn open() -> Result<Option<Self>, String> {
        let Ok(path) = std::env::var("OPEN_BEVY_REPAIR_TRACE") else {
            return Ok(None);
        };
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("cannot create repair trace: {error}"))?;
        let from_seconds = std::env::var("OPEN_BEVY_REPAIR_TRACE_FROM_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(200.0);
        let team = |key: &str, fallback: usize| -> Result<Team, String> {
            let value = std::env::var(key).unwrap_or_else(|_| fallback.to_string());
            let index: usize = value.parse().map_err(|_| format!("invalid {key}"))?;
            if index >= MAX_SKIRMISH_LOBBY_SLOTS {
                return Err(format!("{key} outside lobby slots"));
            }
            Ok(Team::Player(index))
        };
        Ok(Some(Self {
            output: BufWriter::new(file),
            from_seconds,
            defender: team("OPEN_BEVY_REPAIR_TRACE_DEFENDER", 1)?,
            attacker: team("OPEN_BEVY_REPAIR_TRACE_ATTACKER", 0)?,
            economy_team: team("OPEN_BEVY_REPAIR_TRACE_ECONOMY_TEAM", 1)?,
            structure_id: std::env::var("OPEN_BEVY_REPAIR_TRACE_STRUCTURE").ok(),
            cargo: default(),
            bank: None,
            target: None,
        }))
    }

    pub(super) fn observe(&mut self, world: &mut World) -> Result<(), String> {
        let clock = world.resource::<MatchState>().start_time_sec;
        let economy = world.resource::<Economies>().get(self.economy_team);
        let bank = [economy.ore, economy.crystal];
        let mut query = world.query::<(Entity, &Team, &Unit, &Health, &ResourceCargo)>();
        let cargo = query
            .iter(world)
            .filter(|(_, team, unit, health, _)| {
                **team == self.economy_team && unit.id == "Worker" && health.current > 0.0
            })
            .map(|(entity, _, _, _, value)| (entity.to_bits(), [value.ore, value.crystal]))
            .collect::<BTreeMap<_, _>>();
        let mut delivered = [0, 0];
        for (entity, value) in &cargo {
            if *value == [0, 0]
                && let Some(previous) = self.cargo.get(entity)
            {
                delivered[0] += previous[0];
                delivered[1] += previous[1];
            }
        }
        let residual_bank_change = self.bank.map(|previous| {
            [
                previous[0] + delivered[0] - bank[0],
                previous[1] + delivered[1] - bank[1],
            ]
        });
        self.cargo = cargo;
        self.bank = Some(bank);
        if clock < self.from_seconds {
            return Ok(());
        }
        let mut attackers_query = world.query::<(
            Entity,
            &Team,
            &Unit,
            &Transform,
            &Weapon,
            &AttackOrder,
            Option<&EmpDisabled>,
        )>();
        let attacks = attackers_query.iter(world).filter(|(_, team, _, _, _, _, _)| **team == self.attacker)
            .map(|(entity, _, unit, position, weapon, order, emp)| (order.target, serde_json::json!({
                "entity":entity.to_bits(),"unit":unit.id,"position":position.translation.to_array(),
                "range":weapon.range,"damage":weapon.damage,"cooldown":weapon.cooldown,
                "cooldown_left":weapon.cooldown_left,"structure_multiplier":weapon.structure_damage_multiplier,
                "can_attack_ground":weapon.can_attack_ground,"emp_seconds":emp.map(|e|e.remaining)
            }))).collect::<Vec<_>>();
        let focus = if let Some(id) = &self.structure_id {
            world
                .query::<(Entity, &Structure, &Team, &Health)>()
                .iter(world)
                .find(|(_, structure, team, hp)| {
                    **team == self.defender && structure.id == id && hp.current > 0.0
                })
                .map(|(entity, ..)| entity)
        } else {
            attacks.iter().map(|(target, _)| *target).find(|target| {
                world.get::<Structure>(*target).is_some()
                    && world.get::<Team>(*target) == Some(&self.defender)
            })
        };
        let target = focus.map(|focus| {
            let structure = world.get::<Structure>(focus).unwrap();
            let health = world.get::<Health>(focus).unwrap();
            let points = world.get::<ManualStructureRepair>(focus).map(|repair| repair.points_remaining);
            let previous = self.target.filter(|(entity, ..)| *entity == focus.to_bits());
            let started = points.is_some() && previous.is_some_and(|(_, _, points)| points.is_none());
            let hp_delta = previous.map(|(_, hp, _)| health.current - hp);
            let points_delta = previous.and_then(|(_, _, old)| old.zip(points)).map(|(a,b)|b-a);
            let cost = registry::entity(structure.id).map(|def| structure_repair_cost(def, health));
            self.target = Some((focus.to_bits(), health.current, points));
            serde_json::json!({"entity":focus.to_bits(),"structure":structure.id,"health":health.current,"max_health":health.max,
                "position":world.get::<Transform>(focus).map(|p|p.translation.to_array()),"hp_delta":hp_delta,
                "repair_points_remaining":points,"repair_points_delta":points_delta,"repair_started":started,
                "manual_repair_hp_per_second":STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND,
                "calculated_repair_cost_for_current_damage_not_transaction":cost.map(|c|[c.ore,c.crystal])})
        });
        let workers = world.query::<(Entity,&Team,&Unit,&Transform,&Health,Option<&ResourceCargo>,Option<&HarvestOrder>,Option<&MoveOrder>)>().iter(world)
            .filter(|(_,team,unit,_,hp,..)|**team==self.economy_team && unit.id=="Worker" && hp.current>0.0)
            .map(|(entity,_,_,p,_,cargo,harvest,movement)|serde_json::json!({"entity":entity.to_bits(),"position":p.translation.to_array(),
                "cargo":cargo.map(|c|[c.ore,c.crystal]),"harvest":harvest.map(|h|serde_json::json!({"resource":h.resource.map(|e|e.to_bits()),"state":format!("{:?}",h.state),"collect_remaining":h.collect_remaining,"last_kind":h.last_kind.map(|k|format!("{k:?}"))})),
                "move_target":movement.map(|m|m.target.to_array())})).collect::<Vec<_>>();
        let units = world.query::<(Entity,&Team,&Unit,&Health,&Transform)>().iter(world).filter(|(_,_,_,hp,_)|hp.current>0.0)
            .map(|(entity,team,unit,hp,p)|serde_json::json!({"entity":entity.to_bits(),"team":team.economy_index(),"unit":unit.id,"hp":hp.current,"position":p.translation.to_array()})).collect::<Vec<_>>();
        let jobs = world.resource::<BuildQueue>().0.iter().map(|job|serde_json::json!({"team":job.team.economy_index(),"product":build_target_product(job.action),"producer":job.producer_id,"remaining":job.timer,"charged_cost":[job.cost.ore,job.cost.crystal]})).collect::<Vec<_>>();
        let resources = world.query::<(Entity,&ResourceNode)>().iter(world).map(|(entity,node)|serde_json::json!({"entity":entity.to_bits(),"kind":format!("{:?}",node.kind),"amount":node.amount})).collect::<Vec<_>>();
        let mut state =
            bevy::ecs::system::SystemState::<Query<StructurePrereqItem<'_>>>::new(world);
        let structures = state
            .get(world)
            .map_err(|e| format!("trace prerequisite query: {e}"))?;
        let economy = world.resource::<Economies>().get(self.economy_team);
        let faction = world
            .resource::<PlayerFactions>()
            .slot_faction(self.economy_team);
        let candidates = ["ScoutRover","RocketTrooperRobot","MirageScoutTank","ModularMissileCarrier","TeslaCrawlerMk2"].into_iter()
            .filter_map(|id|registry::entity(id)).map(|def|serde_json::json!({"id":def.id,"cost":[def.cost.ore,def.cost.crystal],
                "affordable":economy.can_afford(def.cost),"prerequisites_met":requirements_met(def,self.economy_team,&structures),
                "producer_available":ai_production_origin_for_faction(self.economy_team,faction,def.id,&structures).is_some()})).collect::<Vec<_>>();
        let defender_economy = world.resource::<Economies>().get(self.defender);
        let row = serde_json::json!({"clock":clock,"defender_team":self.defender.economy_index(),"defender_bank":[defender_economy.ore,defender_economy.crystal],
            "economy_team":self.economy_team.economy_index(),"economy_bank":bank,"worker_cargo_delivered_estimate":delivered,
            "residual_bank_change_after_cargo_dropoff_estimate":residual_bank_change,"target":target,
            "attackers":attacks.iter().map(|(target,row)|serde_json::json!({"target":target.to_bits(),"actor":row})).collect::<Vec<_>>(),
            "workers":workers,"living_units":units,"paid_jobs":jobs,"resource_amounts":resources,"counter_candidates":candidates});
        serde_json::to_writer(&mut self.output, &row).map_err(|error| error.to_string())?;
        writeln!(self.output).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<(), String> {
        self.output.flush().map_err(|error| error.to_string())
    }
}
