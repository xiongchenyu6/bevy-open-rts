//! Ordinary group hotkeys and cooldown-governed public support controls.
use super::*;

pub(super) fn recall_army(app: &mut App) -> Result<bool, String> {
    let units = capture_campaign_units(app);
    let requested: HashSet<_> = units
        .iter()
        .filter(|u| u.combat && (u.groups.contains(&DEFENSE) || u.groups.contains(&ASSAULT)))
        .map(|u| u.entity)
        .collect();
    if requested.is_empty() {
        return Ok(false);
    }
    let first = if !grouped(&units, DEFENSE).is_empty() {
        DEFENSE
    } else {
        ASSAULT
    };
    capture_key(app, KeyCode::ShiftLeft, false);
    tap_key(app, GROUP_KEYS[first]);
    let second = 1 - first;
    if !grouped(&units, second).is_empty() {
        capture_key(app, KeyCode::ShiftLeft, true);
        tap_key(app, GROUP_KEYS[second]);
        capture_key(app, KeyCode::ShiftLeft, false);
    }
    running(app)?;
    let now = capture_campaign_units(app);
    let expected: HashSet<_> = now
        .iter()
        .filter(|u| requested.contains(&u.entity))
        .map(|u| u.entity)
        .collect();
    let actual: HashSet<_> = now
        .iter()
        .filter(|u| u.selected)
        .map(|u| u.entity)
        .collect();
    if actual != expected {
        return Err(format!(
            "ordinary group union strict selection mismatch expected={expected:?} actual={actual:?}"
        ));
    }
    Ok(!actual.is_empty())
}

fn cast(
    app: &mut App,
    dir: &Path,
    index: usize,
    key: KeyCode,
    required: &str,
    target: Vec3,
    reason: &str,
) -> Result<bool, String> {
    running(app)?;
    let state = capture_campaign_state(app);
    if state.low_power
        || !state
            .structures
            .iter()
            .any(|s| s.id == required && s.completed && s.health > 0.0)
    {
        return Ok(false);
    }
    let before = bevy_open_rts::capture_live_effect_state(app)?;
    let remaining = before["cooldowns"][0][index]
        .as_f64()
        .ok_or("support cooldown missing")?;
    if remaining > 0.01 {
        return Ok(false);
    }
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    let ground = target.with_y(0.0);
    capture_focus_camera_on(app, ground);
    advance(app, 12);
    running(app)?;
    tap_key(app, key);
    let screen = capture_world_to_screen(app, ground).ok_or("support terrain target offscreen")?;
    bevy_open_rts::capture_world_click_safety(app, screen, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    capture_click_screen(app, screen, MouseButton::Left);
    running(app)?;
    let after = bevy_open_rts::capture_live_effect_state(app)?;
    let remaining_after = after["cooldowns"][0][index]
        .as_f64()
        .ok_or("support cooldown after click missing")?;
    let activated = remaining_after > remaining;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("support-actions.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(file,"{}",serde_json::json!({"clock":after["clock"],"key":format!("{key:?}"),"reason":reason,"target":ground.to_array(),"cooldown_before":remaining,"cooldown_after":remaining_after,"activated":activated,"scope":"ordinary keyboard and terrain click; no direct effect/fog/health/resource writes"})).map_err(|e|e.to_string())?;
    println!(
        "[capture] ordinary support {key:?} {reason} activated={activated} cooldown={remaining_after:.2}"
    );
    Ok(activated)
}

pub(super) fn maintain(app: &mut App, dir: &Path) -> Result<(), String> {
    let units = capture_campaign_units(app);
    let contacts = visible_enemy_contacts(app)?;
    let state = capture_campaign_state(app);
    let Some(base) = state.headquarters.as_ref().map(|s| s.position) else {
        return Ok(());
    };
    // This is the existing public eastern scouting sector, not a hidden HQ read.
    if !contacts.iter().any(|c| c.id == "CommandCenter") {
        cast(
            app,
            dir,
            0,
            KeyCode::F1,
            "RadarUplink",
            Vec3::new(-base.x, 0.0, base.z),
            "public-map scouting sector",
        )?;
    }
    let contacts = visible_enemy_contacts(app)?;
    let threat = contacts
        .iter()
        .filter(|c| {
            matches!(
                c.id.as_str(),
                "HeavyBombardmentAirship"
                    | "SiegeAirship"
                    | "RailArtilleryWalker"
                    | "LongbowMissileCrawler"
                    | "RailgunTank"
            )
        })
        .min_by(|a, b| {
            let distance = |c: &VisibleEnemyContact| {
                units
                    .iter()
                    .filter(|u| u.combat)
                    .map(|u| u.position.distance_squared(c.position))
                    .fold(f32::INFINITY, f32::min)
            };
            distance(a).total_cmp(&distance(b))
        });
    if let Some(threat) = threat {
        if let Some(ally) = units.iter().filter(|u| u.combat).min_by(|a, b| {
            a.position
                .distance_squared(threat.position)
                .total_cmp(&b.position.distance_squared(threat.position))
        }) {
            cast(
                app,
                dir,
                4,
                KeyCode::F5,
                "TechLab",
                ally.position,
                "protect own combat near visible threat",
            )?;
            cast(
                app,
                dir,
                3,
                KeyCode::F4,
                "TechLab",
                ally.position,
                "move own combat near visible threat",
            )?;
        }
        cast(
            app,
            dir,
            2,
            KeyCode::F3,
            "RoboticsBay",
            threat.position,
            "disable actual visible heavy combat",
        )?;
        cast(
            app,
            dir,
            1,
            KeyCode::F2,
            "TechLab",
            threat.position,
            "strike actual visible heavy combat",
        )?;
    }
    let effects = bevy_open_rts::capture_live_effect_state(app)?;
    if let Some(hurt) = effects["units"]
        .as_array()
        .ok_or("own effect units missing")?
        .iter()
        .filter(|u| {
            u["id"].as_str() != Some("Worker")
                && u["health"]["current"]
                    .as_f64()
                    .zip(u["health"]["max"].as_f64())
                    .is_some_and(|(a, b)| a > 0.0 && a < b * 0.8)
        })
        .min_by(|a, b| {
            a["health"]["current"]
                .as_f64()
                .unwrap()
                .total_cmp(&b["health"]["current"].as_f64().unwrap())
        })
    {
        let p = hurt["position"]
            .as_array()
            .ok_or("own hurt position missing")?;
        cast(
            app,
            dir,
            5,
            KeyCode::F6,
            "RoboticsBay",
            Vec3::new(
                p[0].as_f64().unwrap() as f32,
                p[1].as_f64().unwrap() as f32,
                p[2].as_f64().unwrap() as f32,
            ),
            "repair actual own wounded combat",
        )?;
    }
    Ok(())
}
