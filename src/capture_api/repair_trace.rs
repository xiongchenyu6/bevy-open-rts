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
        Ok(Some(Self {
            output: BufWriter::new(file),
            from_seconds,
            cargo: default(),
            bank: None,
            target: None,
        }))
    }

    pub(super) fn observe(&mut self, world: &mut World) -> Result<(), String> {
        let clock = world.resource::<MatchState>().start_time_sec;
        let economy = world.resource::<Economies>().get(Team::Player(1));
        let bank = [economy.ore, economy.crystal];
        let mut query = world.query::<(Entity, &Team, &Unit, &Health, &ResourceCargo)>();
        let cargo = query
            .iter(world)
            .filter(|(_, team, unit, health, _)| {
                **team == Team::Player(1) && unit.id == "Worker" && health.current > 0.0
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
        let attacks = attackers_query.iter(world).filter(|(_, team, _, _, _, _, _)| **team == Team::Player(0))
            .map(|(entity, _, unit, position, weapon, order, emp)| (order.target, serde_json::json!({
                "entity":entity.to_bits(),"unit":unit.id,"position":position.translation.to_array(),
                "range":weapon.range,"damage":weapon.damage,"cooldown":weapon.cooldown,
                "cooldown_left":weapon.cooldown_left,"structure_multiplier":weapon.structure_damage_multiplier,
                "can_attack_ground":weapon.can_attack_ground,"emp_seconds":emp.map(|e|e.remaining)
            }))).collect::<Vec<_>>();
        let Some(focus) = attacks.iter().map(|(target, _)| *target).find(|target| {
            world.get::<Structure>(*target).is_some()
                && world.get::<Team>(*target) == Some(&Team::Player(1))
        }) else {
            return Ok(());
        };
        let structure = world
            .get::<Structure>(focus)
            .ok_or("trace target lost structure")?;
        let health = world
            .get::<Health>(focus)
            .ok_or("trace target lost health")?;
        let points = world
            .get::<ManualStructureRepair>(focus)
            .map(|repair| repair.points_remaining);
        let previous = self
            .target
            .filter(|(entity, ..)| *entity == focus.to_bits());
        let repair_started =
            points.is_some() && previous.is_some_and(|(_, _, points)| points.is_none());
        let hp_delta = previous.map(|(_, hp, _)| health.current - hp);
        let repair_points_delta = previous
            .and_then(|(_, _, old)| old.zip(points))
            .map(|(a, b)| b - a);
        let expected_cost_now =
            registry::entity(structure.id).map(|def| structure_repair_cost(def, health));
        let row = serde_json::json!({"clock":clock,"defender_bank":bank,
            "worker_cargo_delivered_estimate":delivered,"residual_bank_change_after_cargo_dropoff_estimate":residual_bank_change,
            "target":{"entity":focus.to_bits(),"structure":structure.id,"health":health.current,"max_health":health.max,
                "position":world.get::<Transform>(focus).map(|p|p.translation.to_array()),
                "hp_delta":hp_delta,"repair_points_remaining":points,"repair_points_delta":repair_points_delta,
                "repair_started":repair_started,"manual_repair_hp_per_second":STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND,
                "calculated_repair_cost_for_current_damage_not_transaction":expected_cost_now.map(|c|[c.ore,c.crystal])},
            "attackers":attacks.iter().filter(|(target, _)|*target == focus).map(|(_, row)|row).collect::<Vec<_>>()});
        self.target = Some((focus.to_bits(), health.current, points));
        serde_json::to_writer(&mut self.output, &row).map_err(|error| error.to_string())?;
        writeln!(self.output).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<(), String> {
        self.output.flush().map_err(|error| error.to_string())
    }
}
