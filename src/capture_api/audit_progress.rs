//! Optional read-only audit progress; no simulation or wall-limit changes.
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

// External QA collector: a Bevy Resource would allocate a gameplay Entity ID.
static SAMPLE_ENABLED: AtomicBool = AtomicBool::new(false);
static CONSTRUCTOR_SAMPLES: Mutex<Vec<serde_json::Value>> = Mutex::new(Vec::new());

pub(crate) fn record_constructor_sample(make_row: impl FnOnce() -> serde_json::Value) {
    if !SAMPLE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    let mut rows = CONSTRUCTOR_SAMPLES
        .lock()
        .expect("constructor diagnostic lock");
    if rows.len() < 32 {
        rows.push(make_row());
    }
}

use std::{
    collections::BTreeMap,
    io::Write,
    time::{Duration, Instant},
};

pub(super) struct AuditProgress {
    file: std::io::BufWriter<std::fs::File>,
    started: Instant,
    last_wall: Instant,
    last_clock: f32,
    max_update_wall: Duration,
    previous_positions: BTreeMap<u64, Vec3>,
    rows: usize,
}

impl AuditProgress {
    pub(super) fn open() -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os("OPEN_BEVY_AUDIT_PROGRESS") else {
            return Ok(None);
        };
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| error.to_string())?;
        SAMPLE_ENABLED.store(false, Ordering::Release);
        CONSTRUCTOR_SAMPLES
            .lock()
            .expect("constructor diagnostic lock")
            .clear();
        let now = Instant::now();
        let mut observer = Self {
            file: std::io::BufWriter::new(file),
            started: now,
            last_wall: now,
            last_clock: -10.0,
            max_update_wall: Duration::ZERO,
            previous_positions: BTreeMap::new(),
            rows: 0,
        };
        observer.write(serde_json::json!({"stage":"before_app_creation","update":0}))?;
        Ok(Some(observer))
    }

    fn write(&mut self, mut row: serde_json::Value) -> Result<(), String> {
        if self.rows >= 4096 {
            return Err("audit progress exceeded 4096-row bound".into());
        }
        row["wall_seconds"] = serde_json::json!(self.started.elapsed().as_secs_f64());
        row["process_user_system_cpu_ticks"] = serde_json::json!(process_cpu_ticks());
        serde_json::to_writer(&mut self.file, &row).map_err(|error| error.to_string())?;
        self.file
            .write_all(b"\n")
            .and_then(|_| self.file.flush())
            .map_err(|error| error.to_string())?;
        self.rows += 1;
        Ok(())
    }

    pub(super) fn before_update(&mut self, world: &mut World, update: u32) -> Result<(), String> {
        let clock = world.resource::<MatchState>().start_time_sec;
        let due = self.last_wall.elapsed().as_secs_f32() >= 2.0 || clock - self.last_clock >= 10.0;
        SAMPLE_ENABLED.store(due, Ordering::Release);
        CONSTRUCTOR_SAMPLES
            .lock()
            .expect("constructor diagnostic lock")
            .clear();
        if due {
            self.write(
                serde_json::json!({"stage":"update_start","update":update,"clock_seconds":clock}),
            )?;
        }
        Ok(())
    }

    pub(super) fn after_update(
        &mut self,
        world: &mut World,
        update: u32,
        elapsed: Duration,
        final_sample: bool,
    ) -> Result<(), String> {
        self.max_update_wall = self.max_update_wall.max(elapsed);
        let state = world.resource::<MatchState>();
        let clock = state.start_time_sec;
        let phase = match_phase_label(state.phase);
        if !final_sample && !SAMPLE_ENABLED.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut types = BTreeMap::<String, BTreeMap<String, usize>>::new();
        let mut counts = BTreeMap::<String, usize>::new();
        let mut stationary = Vec::new();
        let mut positions = BTreeMap::new();
        let mut query = world.query::<(
            Entity,
            &Team,
            &Unit,
            &Health,
            &Transform,
            Option<&MoveOrder>,
            Option<&AttackOrder>,
            Option<&AttackMoveOrder>,
            Option<&PlannedPath>,
        )>();
        for (entity, team, unit, health, transform, movement, attack, attack_move, path) in
            query.iter(world)
        {
            if health.current <= 0.0 {
                continue;
            }
            *types
                .entry(
                    team.economy_index()
                        .map_or_else(|| "neutral".into(), |index| index.to_string()),
                )
                .or_default()
                .entry(unit.id.to_string())
                .or_default() += 1;
            for (label, present) in [
                ("move", movement.is_some()),
                ("attack", attack.is_some()),
                ("attack_move", attack_move.is_some()),
                ("planned_path", path.is_some()),
            ] {
                if present {
                    *counts.entry(label.into()).or_default() += 1;
                }
            }
            let bits = entity.to_bits();
            let position = transform.translation;
            let unchanged = self
                .previous_positions
                .get(&bits)
                .is_some_and(|previous| previous.distance_squared(position) < 0.0025);
            if stationary.len() < 32
                && unchanged
                && (movement.is_some() || attack_move.is_some() || path.is_some())
            {
                stationary.push(serde_json::json!({"entity":bits,"team":team.economy_index(),"unit":unit.id,"position":position.to_array(),
                    "move_target":movement.map(|order|order.target.to_array()),"attack_target":attack.map(|order|order.target.to_bits()),
                    "attack_move_target":attack_move.map(|order|order.destination.to_array()),"path_goal":path.map(|path|path.goal.to_array()),
                    "remaining_waypoints":path.map(|path|path.waypoints.len().saturating_sub(path.next))}));
            }
            positions.insert(bits, position);
        }
        let mut structures = BTreeMap::<String, BTreeMap<String, usize>>::new();
        let mut completed_structures = BTreeMap::<String, BTreeMap<String, usize>>::new();
        let mut pending = Vec::new();
        let mut query = world.query::<(
            Entity,
            &Team,
            &Structure,
            &Health,
            &Transform,
            Option<&UnderConstruction>,
        )>();
        for (entity, team, structure, health, transform, site) in query.iter(world) {
            if health.current <= 0.0 {
                continue;
            }
            let team_key = team
                .economy_index()
                .map_or_else(|| "neutral".into(), |index| index.to_string());
            *structures
                .entry(team_key.clone())
                .or_default()
                .entry(structure.id.into())
                .or_default() += 1;
            if let Some(site) = site {
                if pending.len() < 128 {
                    pending.push(serde_json::json!({"entity":entity.to_bits(),"team":team.economy_index(),"structure":structure.id,
                    "position":transform.translation.to_array(),"health":health.current,"remaining":site.remaining,"total":site.total}));
                }
            } else {
                *completed_structures
                    .entry(team_key)
                    .or_default()
                    .entry(structure.id.into())
                    .or_default() += 1;
            }
        }
        let mut constructors = Vec::new();
        let mut query = world.query::<(
            Entity,
            &ConstructOrder,
            &Transform,
            &Selectable,
            Option<&MoveOrder>,
            Option<&PlannedPath>,
        )>();
        for (entity, order, transform, selectable, movement, path) in query.iter(world).take(32) {
            constructors.push(serde_json::json!({"entity":entity.to_bits(),"target":order.target.to_bits(),"target_structure":world.get::<Structure>(order.target).map(|target|target.id),
                "target_under_construction":world.get::<UnderConstruction>(order.target).map(|site|serde_json::json!({"remaining":site.remaining,"total":site.total})),
                "position":transform.translation.to_array(),"radius":selectable.radius,"move_target":movement.map(|order|order.target.to_array()),
                "path_goal":path.map(|path|path.goal.to_array()),"remaining_waypoints":path.map(|path|path.waypoints.len().saturating_sub(path.next))}));
        }
        let mut harvesters = Vec::new();
        let mut query = world.query::<(Entity, &Team, &HarvestOrder, Option<&ResourceCargo>)>();
        for (entity, team, order, cargo) in query.iter(world).take(128) {
            harvesters.push(serde_json::json!({"entity":entity.to_bits(),"team":team.economy_index(),"state":format!("{:?}",order.state),
                "resource":order.resource.map(|resource|resource.to_bits()),"cargo":cargo.map(|cargo|serde_json::json!({"ore":cargo.ore,"crystal":cargo.crystal}))}));
        }
        let economies = world
            .resource::<Economies>()
            .players
            .iter()
            .enumerate()
            .map(|(team, bank)| {
                serde_json::json!({"team":team,"ore":bank.ore,"crystal":bank.crystal,
            "power_used":bank.power_used,"power_capacity":bank.power_capacity})
            })
            .collect::<Vec<_>>();
        let decisions = std::mem::take(
            &mut *CONSTRUCTOR_SAMPLES
                .lock()
                .expect("constructor diagnostic lock"),
        );
        self.write(serde_json::json!({"stage":if final_sample {"final"} else {"update_complete"},"update":update,"clock_seconds":clock,"phase":phase,
            "living_unit_types":types,"living_structure_types":structures,"completed_structure_types":completed_structures,"pending_foundations":pending,"constructors":constructors,"construction_approach_decisions":decisions,"economies":economies,"harvesters":harvesters,"order_counts":counts,"stationary_ordered_samples":stationary,
            "max_update_wall_seconds":self.max_update_wall.as_secs_f64(),"last_update_wall_seconds":elapsed.as_secs_f64()}))?;
        self.previous_positions = positions;
        self.last_clock = clock;
        self.last_wall = Instant::now();
        Ok(())
    }
}

// Raw Linux process CPU ticks; the driver records `getconf CLK_TCK` separately.
// Update wall duration is not mislabeled as CPU time.
fn process_cpu_ticks() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/stat").ok()?;
    let fields: Vec<_> = text.rsplit_once(')')?.1.split_whitespace().collect();
    Some(fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?)
}

impl Drop for AuditProgress {
    fn drop(&mut self) {
        SAMPLE_ENABLED.store(false, Ordering::Release);
        CONSTRUCTOR_SAMPLES
            .lock()
            .expect("constructor diagnostic lock")
            .clear();
    }
}
