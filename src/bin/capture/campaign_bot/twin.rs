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

/// Leave ground squads on their current response while existing rockets focus a visible capturer.
pub(super) fn defend_visible_engineer(
    app: &mut App,
    watch: &mut SetupWatch,
) -> Result<bool, String> {
    let state = capture_campaign_state(app);
    let Some(base) = state.headquarters.map(|hq| hq.position) else {
        return Ok(false);
    };
    let Some(engineer) = visible_enemy_contacts(app)?
        .into_iter()
        .filter(|contact| contact.id == "EngineerDrone" && contact.position.distance(base) <= 16.0)
        .min_by(|a, b| {
            a.position
                .distance_squared(base)
                .total_cmp(&b.position.distance_squared(base))
        })
    else {
        return Ok(false);
    };
    let rockets: Vec<_> = capture_campaign_units(app)
        .iter()
        .filter(|unit| unit.id == "RocketInfantry" && unit.health > 0.0)
        .map(|unit| unit.entity)
        .collect();
    if rockets.is_empty() {
        return Ok(false);
    }
    select_units(app, &rockets)?;
    // The existing helper re-reads visibility after the normal selection/camera input.
    let focused = focus_target::attack(app, &watch.dir, engineer.key)?;
    if focused {
        println!(
            "[capture] M5 visible Engineer normal Rocket priority target={}",
            engineer.key
        );
        log_state(app, &mut watch.trace, "visible-engineer-rocket-defense")?;
    }
    Ok(focused)
}

/// Recover only the first lost infantry producer through ordinary paid placement.
pub(super) fn recover_first_barracks(
    app: &mut App,
    watch: &mut SetupWatch,
    attempted: &mut bool,
) -> Result<(), String> {
    if defend_visible_engineer(app, watch)? {
        return Ok(());
    }
    if *attempted {
        return queue_to_quota(app, "Barracks", "HeavyMachinegunTrooper", 3);
    }
    let state = capture_campaign_state(app);
    if state
        .structures
        .iter()
        .any(|structure| structure.id == "Barracks")
    {
        return Ok(());
    }
    // Current authored Alliance Barracks cost; the actual placement UI charges it.
    let bank = capture_player_resources(app);
    if bank.0 < 3 || bank.1 < 1 {
        return Ok(());
    }
    *attempted = true;
    watch.checkpoint(app, "before-first-paid-rear-Barracks-recovery")?;
    let result = place_campaign_structure_with_offsets(
        app,
        "Barracks",
        [
            Vec3::new(-3.0, 0.0, -7.0),
            Vec3::new(-8.0, 0.0, -5.0),
            Vec3::new(-8.0, 0.0, 0.0),
            Vec3::new(-5.0, 0.0, -8.0),
            Vec3::new(0.0, 0.0, -8.0),
        ],
        &mut |app| watch.observe(app),
    );
    if let Err(error) = result {
        watch.checkpoint(app, "first-paid-rear-Barracks-recovery-failed")?;
        return Err(error);
    }
    println!("[capture] M5 first lost Barracks normal paid rear recovery; original 3HMG quota");
    watch.checkpoint(app, "first-paid-rear-Barracks-recovery-placed")?;
    queue_to_quota(app, "Barracks", "HeavyMachinegunTrooper", 3)
}

pub(super) fn opening(
    app: &mut App,
    watch: &mut SetupWatch,
    recovery_attempted: &mut bool,
) -> Result<(), String> {
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
    // Keep the distinct producer rallies. Consolidating every unit at one
    // point put all fifteen ground actors inside a single enemy EMP.
    watch.advance(app, 90);
    hold_army(app)?;
    watch.checkpoint(app, "opening-whole-army-held")?;
    for _ in 0..180 {
        running(app)?;
        if defend_visible_engineer(app, watch)? {
            watch.advance(app, 15);
            continue;
        }
        recover_first_barracks(app, watch, recovery_attempted)?;
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
            if let Some(contact) = contacts.iter().find(|contact| {
                contact.id == threat.id
                    && contact.position.distance_squared(threat.position) < 0.0001
            }) {
                focus_target::attack(app, &watch.dir, contact.key)?;
            }
            println!(
                "[capture] Twin live-ground-response t={:.1}: {}",
                state.clock, threat.id
            );
        } else {
            let units = capture_campaign_units(app);
            if units
                .iter()
                .any(|unit| unit.combat && unit.position.distance(base) > 18.0)
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
