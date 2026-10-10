//! A campaign macro player: observation is read-only; commands use the same
//! clicks, production keys and control groups as ordinary play.
use super::*;
use bevy_open_rts::{
    CaptureCampaignUnit, capture_campaign_resources, capture_campaign_state,
    capture_campaign_units, capture_construction_contact_debug,
    capture_enabled_hold_position_hotkey,
};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;

#[path = "campaign_bot/finale.rs"]
mod finale;
#[path = "campaign_bot/twin.rs"]
mod twin;
#[path = "campaign_bot/survivors.rs"]
mod survivors;
#[path = "campaign_bot/focus_target.rs"]
mod focus_target;

pub(super) fn run_finale(app: &mut App, handle: &Handle<Image>, dir: &Path) -> Result<(), String> {
    finale::run(app, handle, dir)
}

const DEFENSE: usize = 0;
const ASSAULT: usize = 1;
const GROUP_KEYS: [KeyCode; 2] = [KeyCode::Digit1, KeyCode::Digit2];
const DEFENSE_QUOTAS: [(&str, usize); 3] = [
    ("Tank", 3),
    ("HeavyMachinegunTrooper", 3),
    ("RocketInfantry", 3),
];

struct SetupWatch {
    trace: std::fs::File,
    handle: Handle<Image>,
    dir: PathBuf,
    step: usize,
    sequence: usize,
    first_threat: bool,
    first_damage: bool,
    first_low_power: bool,
    previous_hq: Option<f32>,
    error: Option<String>,
}
impl SetupWatch {
    fn checkpoint(&mut self, app: &mut App, stage: &str) -> Result<(), String> {
        log_state(app, &mut self.trace, stage)?;
        let file = self
            .dir
            .join(format!("setup-{:02}-{stage}.png", self.sequence));
        self.sequence += 1;
        // A render request piggybacks on normal updates; it does not advance
        // the clock or move the camera during a player's placement gesture.
        app.world_mut()
            .spawn(Screenshot::image(self.handle.clone()))
            .observe(save_to_disk(file));
        Ok(())
    }
    fn observe(&mut self, app: &mut App) {
        self.step += 1;
        if self.step % 15 != 0 {
            return;
        }
        let state = capture_campaign_state(app);
        let health = state
            .headquarters
            .as_ref()
            .map(|headquarters| headquarters.health);
        let stage = if !self.first_damage
            && self
                .previous_hq
                .is_some_and(|previous| health.is_none_or(|current| current + 0.1 < previous))
        {
            self.first_damage = true;
            Some("first-hq-damage")
        } else if !self.first_threat
            && visible_enemy_contacts(app).is_ok_and(|c| {
                state
                    .threats
                    .iter()
                    .any(|t| contact_is_visible(&c, t.id, t.position))
            })
        {
            self.first_threat = true;
            Some("first-nearby-threat")
        } else if !self.first_low_power && state.low_power {
            self.first_low_power = true;
            Some("first-low-power")
        } else {
            None
        };
        self.previous_hq = health;
        if let Some(stage) = stage
            && let Err(error) = self.checkpoint(app, stage)
        {
            self.error = Some(error);
        }
    }
    fn advance(&mut self, app: &mut App, ticks: usize) {
        for _ in 0..ticks {
            app.update();
            self.observe(app);
            if capture_match_phase_label(app) != "Running" {
                break;
            }
        }
    }
}

struct VisibleEnemyContact {
    key: u64,
    id: String,
    position: Vec3,
}
fn visible_enemy_contacts(app: &mut App) -> Result<Vec<VisibleEnemyContact>, String> {
    let view = bevy_open_rts::capture_live_mine_state(app)?;
    let mut result = Vec::new();
    for kind in ["units", "structures"] {
        for e in view[kind].as_array().ok_or("actor metadata missing")? {
            if e["visible"].as_bool() != Some(true)
                || e["team"].as_u64() != Some(1)
                || !e["health"].as_f64().is_some_and(|h| h > 0.0)
            {
                continue;
            }
            let p = e["position"].as_array().ok_or("visible position missing")?;
            result.push(VisibleEnemyContact {
                key: e["key"].as_u64().ok_or("visible enemy key missing")?,
                id: e["id"].as_str().ok_or("visible id missing")?.into(),
                position: Vec3::new(
                    p[0].as_f64().ok_or("x missing")? as f32,
                    p[1].as_f64().ok_or("y missing")? as f32,
                    p[2].as_f64().ok_or("z missing")? as f32,
                ),
            });
        }
    }
    Ok(result)
}
fn contact_is_visible(contacts: &[VisibleEnemyContact], id: &str, point: Vec3) -> bool {
    contacts
        .iter()
        .any(|c| c.id == id && c.position.distance_squared(point) < 0.0001)
}

fn running(app: &mut App) -> Result<(), String> {
    let phase = capture_match_phase_label(app);
    if phase == "Running" {
        Ok(())
    } else {
        Err(format!("campaign bot stopped during {phase}"))
    }
}
fn advance(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.update();
        if capture_match_phase_label(app) != "Running" {
            break;
        }
    }
}
fn count(units: &[CaptureCampaignUnit], id: &str) -> usize {
    units.iter().filter(|unit| unit.id == id).count()
}
fn grouped(units: &[CaptureCampaignUnit], slot: usize) -> Vec<Entity> {
    units
        .iter()
        .filter(|unit| unit.groups.contains(&slot))
        .map(|unit| unit.entity)
        .collect()
}

/// Verify the actual click selection, including accidental double-click/type
/// expansion. Never insert Selected or write a control-group resource.
fn clear_selection_with_empty_ground_input(app: &mut App) -> Result<(), String> {
    let base = capture_campaign_state(app)
        .headquarters
        .ok_or("headquarters lost while clearing selection")?
        .position;
    let initial = bevy_open_rts::capture_construction_contact_debug(app);
    let origin_x = initial["grid"]["origin_x"]
        .as_f64()
        .ok_or("map grid origin missing")? as f32;
    let origin_z = initial["grid"]["origin_z"]
        .as_f64()
        .ok_or("map grid origin missing")? as f32;
    for (dx, dz) in [
        (-12.0, -12.0),
        (-12.0, 12.0),
        (12.0, -12.0),
        (12.0, 12.0),
        (-18.0, -8.0),
        (-18.0, 8.0),
        (18.0, -8.0),
        (18.0, 8.0),
        (-8.0, -18.0),
        (8.0, -18.0),
        (-8.0, 18.0),
        (8.0, 18.0),
    ] {
        let point = Vec3::new(
            (base.x + dx).clamp(origin_x + 2.0, -origin_x - 2.0),
            0.0,
            (base.z + dz).clamp(origin_z + 2.0, -origin_z - 2.0),
        );
        capture_key(app, KeyCode::ShiftLeft, false);
        capture_key(app, KeyCode::ControlLeft, false);
        capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
        capture_focus_camera_on(app, point);
        advance(app, 12);
        running(app)?;
        let screen =
            capture_world_to_screen(app, point).ok_or("empty-ground selection point offscreen")?;
        let snapshot = bevy_open_rts::capture_construction_contact_debug(app);
        let positions: Vec<_> = snapshot["entities"]
            .as_array()
            .ok_or("selection entities missing")?
            .iter()
            .filter_map(|entity| {
                let p = entity["position"].as_array()?;
                Some((
                    Vec3::new(
                        p[0].as_f64()? as f32,
                        p[1].as_f64()? as f32,
                        p[2].as_f64()? as f32,
                    ),
                    entity["radius"].as_f64()? as f32,
                ))
            })
            .collect();
        if positions.iter().any(|(position, radius)| {
            position.distance(point) <= radius + 2.0
                || capture_world_to_screen(app, *position)
                    .is_some_and(|other| other.distance(screen) < 140.0)
        }) {
            continue;
        }
        capture_click_screen(app, screen, MouseButton::Left);
        let after = bevy_open_rts::capture_construction_contact_debug(app);
        let selected: Vec<_> = after["entities"]
            .as_array()
            .ok_or("selection entities missing")?
            .iter()
            .filter(|entity| entity["selected"].as_bool() == Some(true))
            .map(|entity| entity["entity"].clone())
            .collect();
        println!(
            "[capture] dead-target ordinary empty-ground click at {point:?}: selected {selected:?}"
        );
        if selected.is_empty() {
            return Ok(());
        }
    }
    Err("could not clear dead-target selection through verified empty-ground clicks".into())
}

fn select_units(app: &mut App, ids: &[Entity]) -> Result<Vec<Entity>, String> {
    running(app)?;
    let initial = capture_campaign_units(app);
    let live: Vec<_> = ids
        .iter()
        .copied()
        .filter(|id| initial.iter().any(|unit| unit.entity == *id))
        .collect();
    if live.is_empty() {
        if !ids.is_empty() {
            println!("[capture] selection targets already dead: requested {ids:?}");
            clear_selection_with_empty_ground_input(app)?;
        }
        return Ok(Vec::new());
    }
    let result = (|| {
        let mut clicked = Vec::new();
        for id in live {
            for attempt in 0..3 {
                let Some(before) = capture_campaign_units(app)
                    .into_iter()
                    .find(|unit| unit.entity == id)
                else {
                    break;
                };
                // Being projected is not enough: a unit can be behind the bottom
                // HUD. Center this particular unit and neutralize edge scrolling
                // while settling, then read its updated ground position.
                capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
                println!(
                    "[capture] selection camera settle: requested {ids:?}, target {id:?}, hp {}",
                    before.health
                );
                capture_focus_camera_on(app, before.position);
                advance(app, 12);
                let Some(unit) = capture_campaign_units(app)
                    .into_iter()
                    .find(|unit| unit.entity == id)
                else {
                    println!(
                        "[capture] selection target {id:?} died during camera settle before click"
                    );
                    break;
                };
                let point = capture_world_to_screen(app, unit.position)
                    .ok_or("campaign unit could not be brought into view")?;
                if point.x < 24.0
                    || point.x > WIDTH as f32 - 24.0
                    || point.y < 90.0
                    || point.y > HEIGHT as f32 - 180.0
                {
                    return Err(format!(
                        "campaign unit is still outside click-safe field after centering: {id:?} at {point:?}"
                    ));
                }
                capture_key(app, KeyCode::ShiftLeft, !clicked.is_empty());
                capture_click_screen(app, point, MouseButton::Left);
                // Per-unit camera settling already exceeds double-click timing.
                if !capture_entity_is_selected(app, id) {
                    let current = capture_campaign_units(app);
                    if !current.iter().any(|unit| unit.entity == id) {
                        println!(
                            "[capture] campaign selection target {id:?} died during ordinary click; continuing with survivors"
                        );
                        break;
                    }
                    if attempt < 2 {
                        println!(
                            "[capture] campaign selection retry {} for living {id:?}; reprojecting current position",
                            attempt + 1
                        );
                        continue;
                    }
                    let selected = current
                        .into_iter()
                        .filter(|unit| unit.selected)
                        .map(|unit| (unit.entity, unit.id, unit.position))
                        .collect::<Vec<_>>();
                    return Err(format!(
                        "campaign foot click did not select {id:?} ({}) at {point:?}; selected {selected:?}",
                        unit.id
                    ));
                }
                clicked.push(id);
                break;
            }
        }
        Ok(clicked)
    })();
    capture_key(app, KeyCode::ShiftLeft, false);
    app.update();
    let clicked = result?;
    let units = capture_campaign_units(app);
    let expected: HashSet<_> = clicked
        .iter()
        .copied()
        .filter(|id| units.iter().any(|unit| unit.entity == *id))
        .collect();
    let selected: HashSet<_> = units
        .iter()
        .filter(|unit| unit.selected)
        .map(|unit| unit.entity)
        .collect();
    if expected.is_empty()
        && ids
            .iter()
            .all(|id| !units.iter().any(|unit| unit.entity == *id))
    {
        println!(
            "[capture] all requested selection targets died: requested {ids:?}, clicked {clicked:?}, remaining selected {selected:?}"
        );
        clear_selection_with_empty_ground_input(app)?;
        return Ok(Vec::new());
    }
    if expected != selected {
        let missing: Vec<_> = expected.difference(&selected).copied().collect();
        let unexpected: Vec<_> = selected.difference(&expected).copied().collect();
        let current: Vec<_> = units
            .iter()
            .filter(|unit| expected.contains(&unit.entity) || selected.contains(&unit.entity))
            .map(|unit| {
                (
                    unit.entity,
                    unit.id,
                    unit.health,
                    unit.position,
                    unit.selected,
                    unit.groups.clone(),
                )
            })
            .collect();
        return Err(format!(
            "campaign selection mismatch: requested {ids:?}, clicked {clicked:?}, expected {expected:?}, actual {selected:?}, missing {missing:?}, unexpected {unexpected:?}; current units {current:?}"
        ));
    }
    Ok(expected.into_iter().collect())
}

pub(super) fn select_builder(app: &mut App) -> Result<Vec3, String> {
    for attempt in 0..3 {
        let base = capture_campaign_state(app)
            .headquarters
            .ok_or("headquarters lost")?
            .position;
        let unit = capture_campaign_units(app)
            .into_iter()
            .filter(|unit| unit.id == "Worker" && !unit.constructing)
            .min_by(|a, b| {
                let score = |unit: &CaptureCampaignUnit| {
                    unit.position.distance_squared(base) + (6.0 - unit.health).max(0.0) * 100.0
                };
                score(a).total_cmp(&score(b))
            })
            .ok_or("no living available campaign builder")?;
        if select_units(app, &[unit.entity])?.is_empty() {
            println!(
                "[capture] builder died during selection; requery living candidate {}",
                attempt + 1
            );
            continue;
        }
        return capture_campaign_units(app)
            .into_iter()
            .find(|current| current.entity == unit.entity && current.selected)
            .map(|current| current.position)
            .ok_or_else(|| "campaign builder click was not verified".to_string());
    }
    Err("campaign builders kept dying during ordinary selection".into())
}

fn append_group(app: &mut App, slot: usize, ids: &[Entity]) -> Result<(), String> {
    let selected = select_units(app, ids)?;
    if selected.is_empty() {
        return Ok(());
    }
    capture_key(app, KeyCode::ControlLeft, true);
    capture_key(app, KeyCode::ShiftLeft, true);
    tap_key(app, GROUP_KEYS[slot]);
    capture_key(app, KeyCode::ShiftLeft, false);
    capture_key(app, KeyCode::ControlLeft, false);
    app.update();
    let units = capture_campaign_units(app);
    for id in selected {
        if units
            .iter()
            .any(|unit| unit.entity == id && !unit.groups.contains(&slot))
        {
            return Err(
                "control-group assignment did not accept the selected campaign unit".into(),
            );
        }
    }
    Ok(())
}
fn recall(app: &mut App, slot: usize) -> Result<bool, String> {
    if grouped(&capture_campaign_units(app), slot).is_empty() {
        return Ok(false);
    }
    tap_key(app, GROUP_KEYS[slot]);
    advance(app, 4);
    let units = capture_campaign_units(app);
    if units
        .iter()
        .any(|unit| unit.selected && !unit.groups.contains(&slot))
    {
        return Err("group recall selected a unit belonging to another role".into());
    }
    Ok(units.iter().any(|unit| unit.selected))
}
fn ground_order(app: &mut App, point: Vec3, attack_move: bool) -> Result<(), String> {
    if attack_move {
        let key = capture_first_enabled_attack_move_hotkey(app)
            .ok_or("selected group has no attack-move command")?;
        tap_key(app, key);
    }
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, point);
    advance(app, 12);
    let screen =
        capture_world_to_screen(app, point).ok_or("campaign order target is outside view")?;
    capture_click_screen(app, screen, MouseButton::Right);
    Ok(())
}

fn select_producer(app: &mut App, id: &str) -> Result<bool, String> {
    let state = capture_campaign_state(app);
    let Some(producer) = state
        .structures
        .iter()
        .find(|structure| structure.id == id && structure.completed)
    else {
        return Ok(false);
    };
    let entity = producer.entity;
    let position = producer.position;
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, position);
    advance(app, 12);
    let point = capture_world_to_screen(app, position).ok_or("campaign producer outside view")?;
    capture_key(app, KeyCode::ShiftLeft, false);
    capture_click_screen(app, point, MouseButton::Left);
    advance(app, 10);
    Ok(capture_entity_is_selected(app, entity))
}
pub(super) fn queue_to_quota(
    app: &mut App,
    producer: &str,
    product: &'static str,
    quota: usize,
) -> Result<(), String> {
    running(app)?;
    let units = capture_campaign_units(app);
    let state = capture_campaign_state(app);
    if count(&units, product) + state.queued.get(product).copied().unwrap_or(0) >= quota {
        return Ok(());
    }
    if !select_producer(app, producer)? {
        return Ok(());
    }
    for _ in 0..5 {
        let units = capture_campaign_units(app);
        let state = capture_campaign_state(app);
        if count(&units, product) + state.queued.get(product).copied().unwrap_or(0) >= quota {
            break;
        }
        let Some(key) = capture_enabled_train_hotkey_for(app, product) else {
            break;
        };
        let before = state.queued.get(product).copied().unwrap_or(0);
        tap_key(app, key);
        let after = capture_campaign_state(app)
            .queued
            .get(product)
            .copied()
            .unwrap_or(0);
        if after <= before {
            break;
        }
    }
    Ok(())
}
fn train_to_count(
    app: &mut App,
    watch: &mut SetupWatch,
    producer: &str,
    product: &'static str,
    quota: usize,
) -> Result<(), String> {
    watch.checkpoint(app, &format!("before-train-{product}-{quota}"))?;
    for _ in 0..240 {
        running(app)?;
        watch.observe(app);
        if count(&capture_campaign_units(app), product) >= quota {
            return Ok(());
        }
        queue_to_quota(app, producer, product, quota)?;
        refresh_groups(app)?;
        hold_arrived_defenders(app)?;
        watch.advance(app, 15);
    }
    Err(format!(
        "type-specific production did not reach {quota} {product} units"
    ))
}

pub(super) fn ensure_harvesting(app: &mut App) -> Result<(), String> {
    let mut workers: Vec<_> = capture_campaign_units(app)
        .into_iter()
        .filter(|unit| unit.id == "Worker")
        .collect();
    workers.sort_by_key(|unit| unit.entity.to_bits());
    let worker_count = workers.len();
    for (index, worker) in workers.into_iter().enumerate() {
        if worker.constructing {
            continue;
        }
        let kind = if worker_count > 1 && index == worker_count - 1 {
            "crystal"
        } else {
            "ore"
        };
        if worker.harvesting && worker.harvest_kind.is_none_or(|current| current == kind) {
            continue;
        }
        let resources = capture_campaign_resources(app);
        let Some(resource) = resources
            .into_iter()
            .filter(|node| node.kind == kind)
            .min_by(|a, b| {
                a.position
                    .distance_squared(worker.position)
                    .total_cmp(&b.position.distance_squared(worker.position))
            })
        else {
            continue;
        };
        if select_units(app, &[worker.entity])?.is_empty() {
            continue;
        }
        ground_order(app, resource.click_position, false)?;
    }
    Ok(())
}
fn build(app: &mut App, watch: &mut SetupWatch, id: &'static str) -> Result<Vec3, String> {
    running(app)?;
    watch.checkpoint(app, &format!("before-build-{id}"))?;
    let result = build_campaign_structure_via_real_input(app, id, &mut |app| watch.observe(app))?;
    if let Some(error) = watch.error.take() {
        return Err(error);
    }
    ensure_harvesting(app)?;
    watch.checkpoint(app, &format!("after-build-{id}"))?;
    Ok(result)
}
fn defense_point(app: &mut App, ordinal: usize) -> Result<Vec3, String> {
    let state = capture_campaign_state(app);
    let base = state
        .headquarters
        .as_ref()
        .ok_or("headquarters lost")?
        .position;
    // Both failed openings approached the north side. Keep the defensive
    // gunline in that lane, outside buildings, instead of holding south of HQ.
    (0..12)
        .map(|step| {
            let slot = (ordinal + step) % 12;
            base + Vec3::new(
                1.5 + (slot % 4) as f32 * 1.8,
                0.0,
                (if capture_active_mission(app) == Some(4) {
                    4.5
                } else {
                    7.5
                }) + (slot / 4) as f32 * 1.8,
            )
        })
        .find(|point| {
            point.x.abs() < state.half_width - 1.5
                && point.z.abs() < state.half_depth - 1.5
                && state
                    .obstacles
                    .iter()
                    .all(|(position, radius)| point.distance(*position) >= radius + 1.2)
        })
        .ok_or_else(|| "no clear north-side defensive position".into())
}
fn refresh_groups(app: &mut App) -> Result<(), String> {
    let units = capture_campaign_units(app);
    let mut quotas: BTreeMap<&str, usize> = DEFENSE_QUOTAS.into_iter().collect();
    if capture_active_mission(app) == Some(1) {
        quotas.insert("AntiAirWalker", 2);
    }
    for unit in units.iter().filter(|unit| unit.groups.contains(&DEFENSE)) {
        if let Some(quota) = quotas.get_mut(unit.id) {
            *quota = quota.saturating_sub(1);
        }
    }
    let mut defenders = Vec::new();
    let mut attackers = Vec::new();
    for unit in units
        .iter()
        .filter(|unit| unit.combat && unit.groups.is_empty())
    {
        if quotas.get(unit.id).copied().unwrap_or(0) > 0 {
            defenders.push(unit.entity);
            *quotas.get_mut(unit.id).unwrap() -= 1;
        } else {
            attackers.push(unit.entity);
        }
    }
    let old_defenders = grouped(&units, DEFENSE).len();
    append_group(app, DEFENSE, &defenders)?;
    for (index, entity) in defenders.into_iter().enumerate() {
        if select_units(app, &[entity])?.is_empty() {
            continue;
        }
        let point = defense_point(app, old_defenders + index)?;
        ground_order(app, point, false)?;
    }
    append_group(app, ASSAULT, &attackers)?;
    Ok(())
}
fn hold_arrived_defenders(app: &mut App) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let Some(base) = state.headquarters.map(|headquarters| headquarters.position) else {
        return Ok(());
    };
    let arrived: Vec<_> = capture_campaign_units(app)
        .into_iter()
        .filter(|unit| {
            unit.groups.contains(&DEFENSE)
                && !unit.holding
                && !unit.moving
                && unit.position.distance(base) < 12.0
        })
        .map(|unit| unit.entity)
        .collect();
    if !select_units(app, &arrived)?.is_empty() {
        let key = capture_enabled_hold_position_hotkey(app)
            .ok_or("defensive units lack hold-position command")?;
        tap_key(app, key);
    }
    Ok(())
}

fn maintain_home_defense(app: &mut App, perimeter: f32, staging_offset: f32) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let Some(base) = state.headquarters.map(|hq| hq.position) else {
        return Ok(());
    };
    let contacts = visible_enemy_contacts(app)?;
    if let Some(threat) = state
        .threats
        .iter()
        .filter(|threat| {
            threat.domain == "ground" && contact_is_visible(&contacts, threat.id, threat.position)
        })
        .min_by(|a, b| {
            a.position
                .distance_squared(base)
                .total_cmp(&b.position.distance_squared(base))
        })
    {
        if recall(app, DEFENSE)? {
            ground_order(app, threat.position, true)?;
        }
        return Ok(());
    }
    let units = capture_campaign_units(app);
    let away: Vec<_> = units
        .iter()
        .filter(|unit| unit.groups.contains(&DEFENSE) && unit.position.distance(base) > perimeter)
        .map(|unit| unit.entity)
        .collect();
    if !select_units(app, &away)?.is_empty() {
        ground_order(app, base + Vec3::new(staging_offset, 0.0, 0.0), false)?;
    }
    // Hold immediately inside the home perimeter, including units pursuing a
    // retreating enemy. Waiting for idle lets guards chase across the map.
    let home: Vec<_> = capture_campaign_units(app)
        .into_iter()
        .filter(|unit| {
            unit.groups.contains(&DEFENSE)
                && !unit.holding
                && unit.position.distance(base) <= perimeter
        })
        .map(|unit| unit.entity)
        .collect();
    if !select_units(app, &home)?.is_empty() {
        let key = capture_enabled_hold_position_hotkey(app).ok_or("defenders lack hold command")?;
        tap_key(app, key);
    }
    Ok(())
}

fn maintain_m1_visible_air_defense(app: &mut App) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let contacts = visible_enemy_contacts(app)?;
    let Some(base) = state.headquarters.map(|headquarters| headquarters.position) else {
        return Ok(());
    };
    let Some(threat) = state
        .threats
        .iter()
        .filter(|threat| {
            threat.domain == "air" && contact_is_visible(&contacts, threat.id, threat.position)
        })
        .min_by(|a, b| {
            a.position
                .distance_squared(base)
                .total_cmp(&b.position.distance_squared(base))
        })
    else {
        return Ok(());
    };
    let guards: Vec<_> = capture_campaign_units(app)
        .iter()
        .filter(|unit| {
            unit.groups.contains(&DEFENSE) && matches!(unit.id, "RocketInfantry" | "AntiAirWalker")
        })
        .map(|unit| unit.entity)
        .collect();
    if select_units(app, &guards)?.is_empty() {
        return Ok(());
    }
    println!(
        "[capture] M1 visible-air guard intercept {} hp={} at {:?}",
        threat.id, threat.health, threat.position
    );
    ground_order(app, threat.position, true)
}

pub(super) fn maintain_survival_defense(app: &mut App) -> Result<(), String> {
    let newcomers: Vec<_> = capture_campaign_units(app)
        .into_iter()
        .filter(|unit| unit.combat && !unit.groups.contains(&DEFENSE))
        .map(|unit| unit.entity)
        .collect();
    append_group(app, DEFENSE, &newcomers)?;
    maintain_home_defense(app, 7.0, 3.5)
}

fn rally_to_safe_staging(
    app: &mut App,
    producer: &str,
    mission_index: usize,
) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let base = state
        .headquarters
        .as_ref()
        .ok_or("headquarters lost")?
        .position;
    let offsets = if mission_index == 4 && producer == "Barracks" {
        // Separate the infantry reserve from the armour rally: one radius-4.8
        // EMP must not immobilise the whole paid opening army.
        [
            (-3.0, 6.0),
            (-6.0, 6.0),
            (-3.0, 9.0),
            (-6.0, 9.0),
            (-3.0, 12.0),
            (-6.0, 12.0),
            (0.0, 12.0),
        ]
    } else if mission_index == 4 {
        [
            (6.0, 5.0),
            (5.0, 6.0),
            (1.0, 4.5),
            (1.0, 7.5),
            (4.5, 7.5),
            (-3.0, 4.5),
            (-3.0, 7.5),
        ]
    } else {
        [
            (6.0, 6.0),
            (-6.0, -6.0),
            (-8.0, 0.0),
            (0.0, -8.0),
            (-8.0, 6.0),
            (6.0, -8.0),
            (8.0, 6.0),
        ]
    };
    let staging = offsets
        .into_iter()
        .map(|(x, z)| base + Vec3::new(x, 0.0, z))
        .find(|point| {
            point.x.abs() < state.half_width - 1.5
                && point.z.abs() < state.half_depth - 1.5
                && state
                    .obstacles
                    .iter()
                    .all(|(position, radius)| point.distance(*position) >= radius + 1.2)
        })
        .ok_or("no clear safe staging point near headquarters")?;
    println!("[capture] campaign {producer} rally staging {staging:?}");
    if !select_producer(app, producer)? {
        return Err(format!("{producer} not selectable for rally"));
    }
    let key = capture_first_enabled_rally_hotkey(app).ok_or("producer rally command missing")?;
    tap_key(app, key);
    ground_order(app, staging, false)
}
pub(super) fn visible_anchor_or_own_map_scout(app: &mut App) -> Result<Vec3, String> {
    let base = capture_campaign_state(app)
        .headquarters
        .ok_or("HQ lost while scouting")?
        .position;
    let contacts = visible_enemy_contacts(app)?;
    if let Some(contact) = contacts.iter().min_by(|a, b| {
        a.position
            .distance_squared(base)
            .total_cmp(&b.position.distance_squared(base))
    }) {
        println!(
            "[capture] visible-only mission-one anchor {} at {:?}",
            contact.id, contact.position
        );
        return Ok(contact.position);
    }
    let target = Vec3::new(-base.x, 0.0, -base.z);
    println!("[capture] visible-only mission-one own-map scouting at {target:?}");
    Ok(target)
}

fn attack(app: &mut App, cursor: &mut usize, headquarters_only: bool) -> Result<(), String> {
    if !recall(app, ASSAULT)? {
        return Ok(());
    }
    let units = capture_campaign_units(app);
    let army: Vec<_> = units
        .iter()
        .filter(|u| u.groups.contains(&ASSAULT))
        .collect();
    if army.is_empty() {
        return Ok(());
    }
    let centre = army.iter().map(|u| u.position).sum::<Vec3>() / army.len() as f32;
    let contacts = visible_enemy_contacts(app)?;
    if let Some(c) = contacts
        .iter()
        .filter(|contact| !headquarters_only || contact.id == "CommandCenter")
        .min_by(|a, b| {
            (a.id != "CommandCenter")
                .cmp(&(b.id != "CommandCenter"))
                .then_with(|| {
                    a.position
                        .distance_squared(centre)
                        .total_cmp(&b.position.distance_squared(centre))
                })
        })
    {
        println!(
            "[capture] visible-only assault anchor {} at {:?}",
            c.id, c.position
        );
        return ground_order(app, c.position, true);
    }
    let base = capture_campaign_state(app)
        .headquarters
        .ok_or("HQ lost while scouting")?
        .position;
    let map = capture_construction_contact_debug(app);
    let x = -map["grid"]["origin_x"]
        .as_f64()
        .ok_or("map width missing")? as f32
        * 0.7;
    let z = -map["grid"]["origin_z"]
        .as_f64()
        .ok_or("map height missing")? as f32
        * 0.7;
    let route = if headquarters_only {
        [
            Vec3::new(-base.x, 0.0, base.z),
            Vec3::new(-base.x, 0.0, -base.z),
            Vec3::new(base.x, 0.0, -base.z),
            Vec3::ZERO,
            Vec3::new(x, 0.0, z),
            Vec3::new(-x, 0.0, z),
        ]
    } else {
        [
            Vec3::new(-base.x, 0.0, -base.z),
            Vec3::new(x, 0.0, z),
            Vec3::new(-x, 0.0, z),
            Vec3::new(-x, 0.0, -z),
            Vec3::new(x, 0.0, -z),
            Vec3::ZERO,
        ]
    };
    if centre.distance(route[*cursor % route.len()]) < 6.0 {
        *cursor += 1;
    }
    let target = route[*cursor % route.len()];
    println!(
        "[capture] visible-only map scouting leg {} at {target:?}",
        *cursor
    );
    ground_order(app, target, true)
}

/// Frame the actual assault body after an order has moved the input camera away.
/// Uses only own living actors and visible contacts; no updates or orders here.
fn frame_assault_screenshot(app: &mut App, dir: &Path) -> Result<(), String> {
    let army: Vec<_> = capture_campaign_units(app)
        .into_iter()
        .filter(|unit| unit.health > 0.0 && unit.combat && unit.groups.contains(&ASSAULT))
        .collect();
    let Some(subject) = army.iter().max_by_key(|candidate| {
        army.iter()
            .filter(|peer| peer.position.distance(candidate.position) <= 6.0)
            .count()
    }) else {
        return Ok(());
    };
    let cluster: Vec<_> = army
        .iter()
        .filter(|unit| unit.position.distance(subject.position) <= 6.0)
        .collect();
    let centre = cluster.iter().map(|unit| unit.position).sum::<Vec3>() / cluster.len() as f32;
    let contacts = visible_enemy_contacts(app)?;
    let contact = contacts
        .iter()
        .filter(|contact| contact.position.distance(centre) <= 8.0)
        .min_by(|a, b| {
            a.position
                .distance_squared(centre)
                .total_cmp(&b.position.distance_squared(centre))
        });
    let focus = contact.map_or(centre, |contact| centre.lerp(contact.position, 0.25));
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, focus);
    std::fs::write(dir.join("assault-framing.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "scope": "ordinary camera framing only; unchanged orders and update budget",
        "clock": capture_campaign_state(app).clock,
        "focus": focus.to_array(), "own_cluster_centre": centre.to_array(),
        "own_cluster": cluster.iter().map(|unit| serde_json::json!({"entity":unit.entity.to_bits(),
            "id":unit.id,"position":unit.position.to_array(),"hp":unit.health})).collect::<Vec<_>>(),
        "nearby_visible_contact": contact.map(|contact| serde_json::json!({"id":contact.id,
            "position":contact.position.to_array(),"visible":true})),
    })).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn log_state(
    app: &mut App,
    trace: &mut std::fs::File,
    stage: &str,
) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let units = capture_campaign_units(app);
    let mut types = BTreeMap::new();
    for unit in &units {
        *types.entry(unit.id).or_insert(0usize) += 1;
    }
    // Join read-only snapshots without advancing the application between reads.
    let live = bevy_open_rts::capture_live_mine_state(app)?;
    let contact = capture_construction_contact_debug(app);
    let live_units: BTreeMap<_, _> = live["units"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|actor| actor["key"].as_u64().map(|key| (key, actor)))
        .collect();
    let contact_entities: BTreeMap<_, _> = contact["entities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|actor| actor["entity"].as_u64().map(|key| (key, actor)))
        .collect();
    let unit_orders: Vec<_> = units.iter()
        .filter(|unit| unit.combat || unit.id == "Worker")
        .map(|unit| {
            let key = unit.entity.to_bits();
            let saved = live_units.get(&key).copied();
            let debug = contact_entities.get(&key).copied();
            serde_json::json!({
                "id": unit.id, "entity": key, "position": unit.position.to_array(),
                "hp": unit.health, "groups": unit.groups, "moving": unit.moving,
                "holding": unit.holding, "constructing": unit.constructing,
                "harvesting": unit.harvesting, "harvest_kind": unit.harvest_kind,
                "move_target": unit.move_target.map(|point| point.to_array()),
                "live_snapshot_present": saved.is_some(), "contact_snapshot_present": debug.is_some(),
                "active_orders": saved.map(|actor| &actor["active"]),
                "path_goal": debug.map(|actor| &actor["path_goal"]),
                "path_next_index": debug.map(|actor| &actor["path_next"]),
                "path_next_position": debug.map(|actor| &actor["path_next_position"]),
                "emp_remaining": debug.map(|actor| &actor["emp_remaining"]),
                "unit_speed": debug.map(|actor| &actor["unit_speed"]),
                "movement_domain": debug.map(|actor| &actor["movement_domain"]),
                "chrono_remaining": debug.map(|actor| &actor["chrono_remaining"]),
                "chrono_speed_multiplier": debug.map(|actor| &actor["chrono_speed_multiplier"]),
            })
        }).collect();
    let visible_enemy_headquarters: Vec<_> = live["structures"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|actor| {
            actor["team"].as_u64() == Some(1)
                && actor["id"].as_str() == Some("CommandCenter")
                && actor["visible"].as_bool() == Some(true)
                && actor["health"].as_f64().is_some_and(|health| health > 0.0)
        })
        .map(|actor| {
            serde_json::json!({"entity": actor["key"], "position": actor["position"],
            "hp": actor["health"], "visible": actor["visible"]})
        })
        .collect();
    let value = serde_json::json!({
        "stage": stage, "clock": state.clock, "phase": capture_match_phase_label(app),
        "resources": capture_player_resources(app), "low_power": state.low_power,
        "headquarters": state.headquarters.as_ref().map(|hq| serde_json::json!({"hp":hq.health,"max":hq.max_health,"position":hq.position.to_array()})),
        "types": types, "queued": state.queued,
        "structures": state.structures.iter().map(|structure| serde_json::json!({"id":structure.id,"position":structure.position.to_array(),"hp":structure.health,"max":structure.max_health,"completed":structure.completed})).collect::<Vec<_>>(),
        "defenders": grouped(&units,DEFENSE).iter().map(|id| id.to_bits()).collect::<Vec<_>>(),
        "attackers": grouped(&units,ASSAULT).iter().map(|id| id.to_bits()).collect::<Vec<_>>(),
        "threats": state.threats.iter().map(|threat| serde_json::json!({"id":threat.id,"domain":threat.domain,"position":threat.position.to_array(),"hp":threat.health})).collect::<Vec<_>>(),
        "unit_orders": unit_orders,
        "observation_clocks": {"campaign": state.clock, "live": live["clock"], "contact": contact["clock"]},
        "visible_enemy_headquarters": visible_enemy_headquarters,
        "movement_gating": contact["movement_gating"],
    });
    writeln!(trace, "{value}").map_err(|error| error.to_string())?;
    trace.flush().map_err(|error| error.to_string())?;
    println!(
        "[capture] campaign bot {stage}: t={:.1} HQ={:?} defenders={} attackers={} types={types:?} queue={:?} threats={}",
        state.clock,
        state.headquarters.as_ref().map(|hq| hq.health),
        grouped(&units, DEFENSE).len(),
        grouped(&units, ASSAULT).len(),
        state.queued,
        state.threats.len()
    );
    Ok(())
}

pub(super) fn run(
    app: &mut App,
    handle: &Handle<Image>,
    dir: &Path,
    index: usize,
) -> Result<(), String> {
    let mut watch = SetupWatch {
        trace: std::fs::File::create(dir.join("telemetry.jsonl"))
            .map_err(|error| error.to_string())?,
        handle: handle.clone(),
        dir: dir.into(),
        step: 0,
        sequence: 0,
        first_threat: false,
        first_damage: false,
        first_low_power: false,
        previous_hq: capture_campaign_state(app)
            .headquarters
            .map(|headquarters| headquarters.health),
        error: None,
    };
    let mut remaining_rounds = 240;
    let result = (|| {
        log_state(app, &mut watch.trace, "opening")?;
        if index == 4 {
            twin::opening(app, &mut watch)?;
        } else {
            build(app, &mut watch, "PowerReactor")?;
            // Factory-first gets real armor onto the battlefield before the Hard
            // opening attack. Production continues while infantry/defenses build.
            build(app, &mut watch, "VehicleFactory")?;
            rally_to_safe_staging(app, "VehicleFactory", index)?;
            queue_to_quota(app, "VehicleFactory", "Tank", 6)?;
            if index == 1 {
                println!("[capture] M1 paid mixed-role early air-ground opening");
                build(app, &mut watch, "RadarUplink")?;
                println!("[capture] M1 prioritize two paid AA Walkers before tank refill");
                queue_to_quota(app, "VehicleFactory", "AntiAirWalker", 2)?;
            }
            build(app, &mut watch, "Barracks")?;
            if index == 1 {
                queue_to_quota(app, "Barracks", "RocketInfantry", 3)?;
            }
            queue_to_quota(app, "Barracks", "HeavyMachinegunTrooper", 3)?;
            if index == 1 {
                queue_to_quota(app, "VehicleFactory", "AntiAirWalker", 2)?;
            }
            build(app, &mut watch, "AntiGroundTurret")?;
            refresh_groups(app)?;
            // Canyon Siege also sends an EngineerDrone straight from the east.
            // Cover its route into HQ instead of placing AA nine metres north,
            // outside the turret's eight-metre weapon range from that approach.
            if matches!(index, 1 | 2) {
                watch.checkpoint(app, "before-build-east-AA")?;
                let offsets = [
                    Vec3::new(6.0, 0.0, 0.0),
                    Vec3::new(6.0, 0.0, -3.0),
                    Vec3::new(9.0, 0.0, 0.0),
                    Vec3::new(9.0, 0.0, -3.0),
                    Vec3::new(4.0, 0.0, -3.0),
                ];
                let mut observer = |app: &mut App| watch.observe(app);
                build_campaign_structure_with_offsets(
                    app,
                    "AntiAirTurret",
                    offsets,
                    &mut observer,
                )?;
                watch.checkpoint(app, "after-build-east-AA")?;
            } else {
                build(app, &mut watch, "AntiAirTurret")?;
            }
            if index == 1 {
                queue_to_quota(app, "VehicleFactory", "AntiAirWalker", 2)?;
            }
            queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
            if index != 1 {
                build(app, &mut watch, "RadarUplink")?;
            }
            rally_to_safe_staging(app, "Barracks", index)?;
            train_to_count(app, &mut watch, "Barracks", "RocketInfantry", 3)?;
            refresh_groups(app)?;
            watch.checkpoint(app, "early-air-defense-ready")?;
            if matches!(index, 1 | 2) || capture_campaign_state(app).low_power {
                build(app, &mut watch, "PowerReactor")?;
            }
            build(app, &mut watch, "AntiGroundTurret")?;
            if index == 1 {
                queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
            } else {
                train_to_count(app, &mut watch, "VehicleFactory", "Tank", 6)?;
            }
            queue_to_quota(app, "CommandCenter", "Worker", 4)?;
            ensure_harvesting(app)?;
            refresh_groups(app)?;
            hold_arrived_defenders(app)?;
            log_state(app, &mut watch.trace, "defense_ready")?;
            shoot(app, handle, dir.join("01_defense.png"));
        }
        let mut launched = false;
        let mut last_attack_clock = -20.0;
        let mut scout_cursor = 0;
        let mut last_hq_health = capture_campaign_state(app)
            .headquarters
            .map(|hq| hq.health)
            .unwrap_or(0.0);
        let mut last_threat_capture = -30.0;
        for round in 0..240 {
            remaining_rounds = 240 - round;
            if capture_match_phase_label(app) != "Running" {
                break;
            }
            ensure_harvesting(app)?;
            refresh_groups(app)?;
            hold_arrived_defenders(app)?;
            if matches!(index, 1 | 4) {
                maintain_home_defense(app, 12.0, 8.0)?;
            }
            if index == 1 {
                queue_to_quota(app, "VehicleFactory", "AntiAirWalker", 2)?;
                queue_to_quota(app, "Barracks", "RocketInfantry", 5)?;
                maintain_m1_visible_air_defense(app)?;
            }
            queue_to_quota(app, "CommandCenter", "Worker", 4)?;
            queue_to_quota(app, "Barracks", "HeavyMachinegunTrooper", 3)?;
            queue_to_quota(app, "Barracks", "RocketInfantry", 5)?;
            queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
            let state = capture_campaign_state(app);
            let units = capture_campaign_units(app);
            let Some(hq) = state.headquarters.as_ref() else {
                return Err("campaign headquarters eliminated".into());
            };
            let contacts = visible_enemy_contacts(app)?;
            let pressure = state
                .threats
                .iter()
                .any(|t| contact_is_visible(&contacts, t.id, t.position))
                || hq.health + 0.1 < last_hq_health;
            if pressure && state.clock - last_threat_capture > 15.0 {
                capture_focus_camera_on(app, hq.position);
                shoot(
                    app,
                    handle,
                    dir.join(format!("defense-threat-{round:03}.png")),
                );
                last_threat_capture = state.clock;
            }
            let attackers: Vec<_> = units
                .iter()
                .filter(|unit| unit.groups.contains(&ASSAULT))
                .collect();
            let tanks = attackers.iter().filter(|unit| unit.id == "Tank").count();
            let defenders_ready = DEFENSE_QUOTAS.iter().all(|(id, quota)| {
                units
                    .iter()
                    .filter(|unit| unit.groups.contains(&DEFENSE) && unit.id == *id)
                    .count()
                    >= *quota
            });
            if pressure && hq.health < hq.max_health * 0.75 {
                if recall(app, ASSAULT)? {
                    ground_order(app, hq.position + Vec3::new(4.5, 0.0, 4.5), true)?;
                }
                launched = false;
            } else if !pressure
                && defenders_ready
                && tanks >= 6
                && (!launched || state.clock - last_attack_clock >= 15.0)
            {
                attack(app, &mut scout_cursor, index == 1)?;
                if !launched {
                    frame_assault_screenshot(app, dir)?;
                    shoot(app, handle, dir.join("02_assault.png"));
                }
                launched = true;
                last_attack_clock = state.clock;
            } else if launched && state.clock - last_attack_clock >= 15.0 && !attackers.is_empty() {
                attack(app, &mut scout_cursor, index == 1)?;
                last_attack_clock = state.clock;
            }
            log_state(
                app,
                &mut watch.trace,
                if launched { "assault" } else { "regroup" },
            )?;
            last_hq_health = hq.health;
            let power_budget = capture_player_resources(app);
            if state.low_power && round % 12 == 0 && power_budget.0 >= 3 && power_budget.1 >= 1 {
                build(app, &mut watch, "PowerReactor")?;
            }
            watch.advance(app, 150);
        }
        log_state(app, &mut watch.trace, "result")?;
        let phase = capture_match_phase_label(app);
        if phase != "HumanVictory" {
            return Err(format!(
                "mission {index} defensive macro resolved as {phase}"
            ));
        }
        Ok(())
    })();
    let result = if result.as_ref().err().is_some_and(|error| {
        error == "headquarters lost"
            || error == "campaign headquarters eliminated"
            || error.starts_with("could not place ")
    }) && matches!(index, 2 | 4)
        && capture_match_phase_label(app) == "Running"
        && capture_campaign_state(app).headquarters.is_none()
    {
        std::fs::write(
            dir.join("pre-survivor-error.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "original_error": result.as_ref().err(),
                "snapshot": capture_construction_contact_debug(app),
                "scope": "Original macro failure retained; only existing army may continue"
            })).map_err(|error| error.to_string())?,
        ).map_err(|error| error.to_string())?;
        survivors::run(app, &mut watch, remaining_rounds)
    } else {
        result
    };
    if result.is_err() {
        let snapshot = capture_construction_contact_debug(app);
        std::fs::write(
            dir.join("failure-contact.json"),
            serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("write contact diagnostic: {error}"))?;
        let _ = log_state(app, &mut watch.trace, "failure");
        if let Some(hq) = capture_campaign_state(app).headquarters {
            capture_focus_camera_on(app, hq.position);
        }
        shoot(app, handle, dir.join("failure-defense.png"));
    }
    result
}
