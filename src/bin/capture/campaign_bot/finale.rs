//! The nearby northern base makes the finale a ground-rush opening.
use super::*;

fn roles(app: &mut App) -> Result<(), String> {
    let units = capture_campaign_units(app);
    let mut missing = 3usize.saturating_sub(
        units
            .iter()
            .filter(|unit| unit.id == "Tank" && unit.groups.contains(&DEFENSE))
            .count(),
    );
    let mut defenders = Vec::new();
    let mut attackers = Vec::new();
    for unit in units
        .iter()
        .filter(|unit| unit.combat && unit.groups.is_empty())
    {
        if unit.id == "Tank" && missing > 0 {
            defenders.push(unit.entity);
            missing -= 1;
        } else {
            attackers.push(unit.entity);
        }
    }
    append_group(app, DEFENSE, &defenders)?;
    append_group(app, ASSAULT, &attackers)
}

fn guard_north(app: &mut App) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let Some(base) = state.headquarters.map(|hq| hq.position) else {
        return Err("finale headquarters lost".into());
    };
    let contacts = visible_enemy_contacts(app)?;
    let threats = state.threats.iter().filter(|threat| {
        threat.domain == "ground" && contact_is_visible(&contacts, threat.id, threat.position)
    });
    if let Some(threat) = threats.min_by(|a, b| {
        a.position
            .distance_squared(base)
            .total_cmp(&b.position.distance_squared(base))
    }) {
        if recall(app, DEFENSE)? {
            ground_order(app, threat.position, true)?;
        }
    } else {
        let units = capture_campaign_units(app);
        let away: Vec<_> = units
            .iter()
            .filter(|unit| unit.groups.contains(&DEFENSE) && unit.position.distance(base) > 8.0)
            .map(|unit| unit.entity)
            .collect();
        if !select_units(app, &away)?.is_empty() {
            ground_order(app, base + Vec3::new(0.0, 0.0, 4.5), false)?;
        }
        let home: Vec<_> = capture_campaign_units(app)
            .into_iter()
            .filter(|unit| {
                unit.groups.contains(&DEFENSE)
                    && !unit.holding
                    && unit.position.distance(base) <= 8.0
            })
            .map(|unit| unit.entity)
            .collect();
        if !select_units(app, &home)?.is_empty() {
            let key =
                capture_enabled_hold_position_hotkey(app).ok_or("finale guard hold unavailable")?;
            tap_key(app, key);
        }
    }
    Ok(())
}

pub(super) fn run(app: &mut App, handle: &Handle<Image>, dir: &Path) -> Result<(), String> {
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
        previous_hq: capture_campaign_state(app).headquarters.map(|hq| hq.health),
        error: None,
    };
    let result = (|| {
        build(app, &mut watch, "PowerReactor")?;
        // An EngineerDrone can capture a full-health HQ; ground tanks cannot
        // intercept it. Pay for northern AA before committing the tank queue.
        watch.checkpoint(app, "before-build-north-air-cover")?;
        build_campaign_structure_with_offsets(
            app,
            "AntiAirTurret",
            [
                Vec3::new(0.0, 0.0, 3.5),
                Vec3::new(3.5, 0.0, 3.5),
                Vec3::new(-3.5, 0.0, 3.5),
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::new(3.5, 0.0, 5.0),
            ],
            &mut |app| watch.observe(app),
        )?;
        watch.checkpoint(app, "after-build-north-air-cover")?;
        build(app, &mut watch, "VehicleFactory")?;
        // Reserve the turret's actual cost instead of spending the entire stock
        // on the production queue. Tanks produce while its worker approaches.
        queue_to_quota(app, "VehicleFactory", "Tank", 2)?;
        watch.checkpoint(app, "before-build-north-ground-turret")?;
        let offsets = [
            Vec3::new(0.0, 0.0, 6.5),
            Vec3::new(3.0, 0.0, 6.5),
            Vec3::new(-3.0, 0.0, 6.5),
            Vec3::new(6.0, 0.0, 6.0),
            Vec3::new(-6.0, 0.0, 6.0),
        ];
        let mut observer = |app: &mut App| watch.observe(app);
        build_campaign_structure_with_offsets(app, "AntiGroundTurret", offsets, &mut observer)?;
        watch.checkpoint(app, "after-build-north-ground-turret")?;
        let mut last_attack = -20.0;
        let mut scout_cursor = 0;
        let mut launched = false;
        for round in 0..240 {
            if capture_match_phase_label(app) != "Running" {
                break;
            }
            ensure_harvesting(app)?;
            queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
            roles(app)?;
            guard_north(app)?;
            let state = capture_campaign_state(app);
            let units = capture_campaign_units(app);
            let hq = state
                .headquarters
                .as_ref()
                .ok_or("finale headquarters eliminated")?;
            let contacts = visible_enemy_contacts(app)?;
            let near_ground = state.threats.iter().any(|threat| {
                threat.domain == "ground"
                    && threat.position.distance(hq.position) < 10.0
                    && contact_is_visible(&contacts, threat.id, threat.position)
            });
            let assault_tanks = units
                .iter()
                .filter(|unit| unit.id == "Tank" && unit.groups.contains(&ASSAULT))
                .count();
            if near_ground && hq.health < hq.max_health * 0.75 {
                if recall(app, ASSAULT)? {
                    ground_order(app, hq.position + Vec3::new(0.0, 0.0, 4.5), true)?;
                }
                launched = false;
            } else if assault_tanks >= 3 && !near_ground && state.clock - last_attack >= 15.0 {
                attack(app, &mut scout_cursor, true)?;
                launched = true;
                last_attack = state.clock;
            }
            log_state(
                app,
                &mut watch.trace,
                if launched {
                    "finale-assault"
                } else {
                    "finale-defend"
                },
            )?;
            let power_budget = capture_player_resources(app);
            if round % 12 == 0 && state.low_power && power_budget.0 >= 3 && power_budget.1 >= 1 {
                build(app, &mut watch, "PowerReactor")?;
            }
            watch.advance(app, 90);
        }
        if capture_match_phase_label(app) != "HumanVictory" {
            return Err(format!(
                "finale resolved as {}",
                capture_match_phase_label(app)
            ));
        }
        Ok(())
    })();
    if result.is_err() && capture_match_phase_label(app) != "HumanVictory" {
        std::fs::write(
            dir.join("failure-contact.json"),
            serde_json::to_vec_pretty(&capture_construction_contact_debug(app))
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let _ = log_state(app, &mut watch.trace, "finale-failure");
    }
    result
}
