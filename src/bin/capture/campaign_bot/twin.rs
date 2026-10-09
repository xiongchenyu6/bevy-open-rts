//! Paid Twin Front opening: unlock rockets early and fight inside static cover.
use super::*;

fn ground_turret(app: &mut App, watch: &mut SetupWatch, north: bool) -> Result<(), String> {
    let offsets = if north {
        [
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(-3.0, 0.0, 6.0),
            Vec3::new(0.0, 0.0, 9.0),
            Vec3::new(-3.0, 0.0, 9.0),
            Vec3::new(3.0, 0.0, 9.0),
        ]
    } else {
        [
            Vec3::new(6.0, 0.0, 3.0),
            Vec3::new(6.0, 0.0, 0.0),
            Vec3::new(9.0, 0.0, 3.0),
            Vec3::new(9.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, -3.0),
        ]
    };
    let name = if north {
        "north-ground-cover"
    } else {
        "east-ground-cover"
    };
    watch.checkpoint(app, &format!("before-{name}"))?;
    build_campaign_structure_with_offsets(app, "AntiGroundTurret", offsets, &mut |app| {
        watch.observe(app)
    })?;
    watch.checkpoint(app, &format!("after-{name}"))
}

fn hold_army(app: &mut App) -> Result<(), String> {
    select_all_player_army(app);
    let needs_hold = capture_campaign_units(app)
        .iter()
        .any(|unit| unit.selected && unit.combat && !unit.holding);
    if needs_hold {
        let key =
            capture_enabled_hold_position_hotkey(app).ok_or("selected army has no Hold command")?;
        tap_key(app, key);
    }
    Ok(())
}

fn refill(app: &mut App, hold: bool) -> Result<(), String> {
    // Five jobs fit in each producer queue. Revisit it as jobs finish instead
    // of assuming the first quota request paid for every planned unit.
    queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
    queue_to_quota(app, "Barracks", "HeavyMachinegunTrooper", 3)?;
    queue_to_quota(app, "Barracks", "RocketInfantry", 3)?;
    queue_to_quota(app, "CommandCenter", "Worker", 4)?;
    ensure_harvesting(app)?;
    if hold {
        hold_army(app)?;
    }
    Ok(())
}

pub(super) fn opening(app: &mut App, watch: &mut SetupWatch) -> Result<(), String> {
    build(app, watch, "PowerReactor")?;
    build(app, watch, "VehicleFactory")?;
    rally_to_safe_staging(app, "VehicleFactory", 4)?;
    queue_to_quota(app, "VehicleFactory", "Tank", 6)?;
    build(app, watch, "RadarUplink")?;
    queue_to_quota(app, "VehicleFactory", "Tank", 9)?;
    queue_to_quota(app, "CommandCenter", "Worker", 4)?;
    build(app, watch, "Barracks")?;
    rally_to_safe_staging(app, "Barracks", 4)?;
    let before_aa = capture_player_completed_structure_count_for(app, "AntiAirTurret");
    watch.checkpoint(app, "before-early-east-AA")?;
    let aa_offsets = [
        Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, -3.0),
        Vec3::new(6.0, 0.0, 3.0),
        Vec3::new(3.0, 0.0, -6.0),
        Vec3::new(-3.0, 0.0, -6.0),
    ];
    place_campaign_structure_with_offsets(app, "AntiAirTurret", aa_offsets, &mut |app| {
        watch.observe(app)
    })?;
    watch.checkpoint(app, "early-east-AA-placed")?;
    refill(app, true)?;
    ground_turret(app, watch, true)?;
    refill(app, true)?;
    let before_power = capture_player_completed_structure_count_for(app, "PowerReactor");
    watch.checkpoint(app, "before-concurrent-AA-power-east-cover")?;
    let power_offsets = [
        Vec3::new(-4.5, 0.0, -3.5),
        Vec3::new(-6.0, 0.0, -5.0),
        Vec3::new(-6.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 5.0),
    ];
    place_campaign_structure_with_offsets(app, "PowerReactor", power_offsets, &mut |app| {
        watch.observe(app)
    })?;
    // Active construction orders exclude those builders from the next real
    // worker selection. East cover starts with a third available worker now,
    // rather than waiting twenty seconds for the rear Power site.
    ground_turret(app, watch, false)?;
    refill(app, true)?;
    for _ in 0..BUILD_COMPLETION_WAIT_TICKS / 15 {
        running(app)?;
        if capture_player_completed_structure_count_for(app, "AntiAirTurret") > before_aa
            && capture_player_completed_structure_count_for(app, "PowerReactor") > before_power
        {
            break;
        }
        watch.advance(app, 15);
    }
    if capture_player_completed_structure_count_for(app, "AntiAirTurret") <= before_aa
        || capture_player_completed_structure_count_for(app, "PowerReactor") <= before_power
    {
        return Err("concurrent paid AA/Power foundations did not complete".into());
    }
    watch.checkpoint(app, "after-concurrent-AA-power-east-cover")?;
    refill(app, true)?;
    // Use the real army shortcut while the opening wave approaches, rather
    // than spending twenty seconds assigning individual roles under fire.
    let base = capture_campaign_state(app)
        .headquarters
        .ok_or("headquarters lost")?
        .position;
    println!("[capture] Twin crystal-field guard staging HQ+6.5,6 and visible ground response16m");
    select_all_player_army(app);
    ground_order(app, base + Vec3::new(6.5, 0.0, 6.0), false)?;
    watch.advance(app, 90);
    hold_army(app)?;
    watch.checkpoint(app, "opening-whole-army-held")?;
    for _ in 0..180 {
        running(app)?;
        refill(app, false)?;
        let state = capture_campaign_state(app);
        let contacts = visible_enemy_contacts(app)?;
        if state.clock >= 110.0
            && !state
                .threats
                .iter()
                .any(|t| contact_is_visible(&contacts, t.id, t.position))
        {
            break;
        }
        let threat = state
            .threats
            .iter()
            .filter(|threat| {
                threat.domain == "ground"
                    && threat.position.distance(base) <= 16.0
                    && contact_is_visible(&contacts, threat.id, threat.position)
            })
            .min_by(|a, b| {
                let priority = |id: &str| match id {
                    "ModularMissileCarrier" => 0,
                    "JammerVehicle" => 1,
                    _ => 2,
                };
                priority(&a.id).cmp(&priority(&b.id)).then_with(|| {
                    a.position
                        .distance_squared(base)
                        .total_cmp(&b.position.distance_squared(base))
                })
            });
        if let Some(threat) = threat {
            // Static Hold lets longer-range artillery fire without retaliation.
            // Engage the live local threat through the normal army shortcut and
            // attack-move command; no individual selection delay under fire.
            select_all_player_army(app);
            ground_order(app, threat.position, true)?;
            println!(
                "[capture] Twin live-ground-response t={:.1}: {}",
                state.clock, threat.id
            );
        } else {
            let units = capture_campaign_units(app);
            if units
                .iter()
                .any(|unit| unit.combat && unit.position.distance(base) > 9.0)
            {
                select_all_player_army(app);
                ground_order(app, base + Vec3::new(6.5, 0.0, 6.0), false)?;
                watch.advance(app, 30);
            } else {
                hold_army(app)?;
            }
        }
        let units = capture_campaign_units(app);
        let held = units
            .iter()
            .filter(|unit| unit.combat && unit.holding)
            .count();
        let moving = units
            .iter()
            .filter(|unit| unit.combat && unit.moving)
            .count();
        println!(
            "[capture] Twin guard window t={:.1}: held={held} moving={moving}",
            capture_campaign_state(app).clock
        );
        watch.advance(app, 15);
    }
    // Split into defense and assault only after the nearby first wave clears.
    refresh_groups(app)?;
    hold_arrived_defenders(app)?;
    maintain_home_defense(app, 10.0, 3.5)?;
    // Replacements remain paid queue targets. Do not wait for a unit quota
    // while another live wave is attacking: the common battle loop keeps
    // harvesting, replenishing and responding to nearby threats.
    refill(app, false)?;
    println!("[capture] Twin defense ready with nonblocking paid replenishment");
    log_state(app, &mut watch.trace, "defense_ready")?;
    shoot(app, &watch.handle, watch.dir.join("01_defense.png"));
    Ok(())
}
