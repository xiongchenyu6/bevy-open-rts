//! Boot Camp QA uses normal scouting and visible contacts, never hidden enemy positions.
use super::*;

pub(super) fn finish_attack_lesson(app: &mut App, dir: &Path) -> Result<bool, String> {
    use std::io::Write;
    let mut trace = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dir.join("attack-scout.jsonl"))
        .map_err(|e| e.to_string())?;
    let start = bevy_open_rts::capture_campaign_state(app);
    let base = start
        .headquarters
        .ok_or("training headquarters missing")?
        .position;
    // Public map quadrants, derived only from the player's own starting location.
    let route = [
        Vec3::new(-base.x, 0.0, base.z),
        Vec3::new(-base.x, 0.0, -base.z),
        Vec3::new(base.x, 0.0, -base.z),
        Vec3::ZERO,
    ];
    let deadline = start.clock + 300.0;
    let mut leg = 0;
    let mut next_observation = start.clock;
    loop {
        if capture_tutorial_stage(app) == 7 {
            return Ok(true);
        }
        let phase = capture_match_phase_label(app);
        if phase == "HumanVictory" {
            // Natural match-end and tutorial observation can run on adjacent frames.
            // Allow UI updates to settle, without input or extending the attack budget.
            for _ in 0..4 {
                if bevy_open_rts::capture_campaign_state(app).clock >= deadline {
                    break;
                }
                app.update();
            }
            return Ok(
                capture_tutorial_stage(app) == 7 && !bevy_open_rts::capture_tutorial_active(app)
            );
        }
        if phase != "Running" {
            return Ok(false);
        }
        let clock = bevy_open_rts::capture_campaign_state(app).clock;
        if clock >= deadline {
            return Ok(false);
        }
        if clock >= next_observation && deadline - clock >= 2.0 {
            let view = bevy_open_rts::capture_live_mine_state(app)?;
            let units = bevy_open_rts::capture_campaign_units(app);
            let army: Vec<_> = units
                .iter()
                .filter(|u| u.id == "LightRifleInfantry" && u.health > 0.0)
                .collect();
            if army.len() != 4 {
                return Err(format!(
                    "training assault expected four living paid infantry, found {}",
                    army.len()
                ));
            }
            let centre = army.iter().map(|u| u.position).sum::<Vec3>() / army.len() as f32;
            let visible_hq = view["structures"]
                .as_array()
                .ok_or("visible structure metadata missing")?
                .iter()
                .find(|e| {
                    e["team"].as_u64() == Some(1)
                        && e["id"].as_str() == Some("CommandCenter")
                        && e["visible"].as_bool() == Some(true)
                        && e["health"].as_f64().is_some_and(|h| h > 0.0)
                });
            let target = if let Some(hq) = visible_hq {
                let p = hq["position"]
                    .as_array()
                    .ok_or("visible headquarters position missing")?;
                Vec3::new(
                    p[0].as_f64().ok_or("x missing")? as f32,
                    0.0,
                    p[2].as_f64().ok_or("z missing")? as f32,
                )
            } else {
                if centre.distance(route[leg % route.len()]) < 4.0 {
                    leg += 1;
                }
                route[leg % route.len()]
            };
            select_all_player_army(app);
            let selected = bevy_open_rts::capture_campaign_units(app);
            if selected
                .iter()
                .filter(|u| u.selected && u.id == "LightRifleInfantry")
                .count()
                != 4
                || selected
                    .iter()
                    .filter(|u| u.selected && u.id == "Drone")
                    .count()
                    != 1
            {
                return Err(
                    "normal army selection did not select four infantry and the starting Drone"
                        .into(),
                );
            }
            let key = capture_first_enabled_attack_move_hotkey(app)
                .ok_or("training Attack-Move unavailable")?;
            tap_key(app, key);
            capture_focus_camera_on(app, target);
            for _ in 0..12 {
                app.update();
            }
            let point = capture_world_to_screen(app, target)
                .ok_or("normal training scout target outside viewport")?;
            capture_click_screen(app, point, MouseButton::Right);
            writeln!(trace,"{}",serde_json::json!({"clock":clock,"deadline":deadline,"leg":leg,"target":target.to_array(),"visible_hq":visible_hq.is_some(),"selected_infantry":4,"selected_drone":1,"actual_orders":capture_player_combat_order_count(app),"world":view})).map_err(|e|e.to_string())?;
            trace.flush().map_err(|e| e.to_string())?;
            next_observation = clock + 3.0;
        }
        app.update();
    }
}
