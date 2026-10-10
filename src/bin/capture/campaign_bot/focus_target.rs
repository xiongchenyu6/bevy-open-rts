//! Verify a normal right-click attack against an actually visible enemy.
use super::*;

pub(super) fn attack(app: &mut App, dir: &Path, target: u64) -> Result<bool, String> {
    running(app)?;
    let Some(contact) = visible_enemy_contacts(app)?
        .into_iter()
        .find(|c| c.key == target)
    else {
        return Ok(false);
    };
    if bevy_open_rts::capture_has_pending_command_mode(app) {
        tap_key(app, KeyCode::Escape);
    }
    running(app)?;
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, contact.position);
    advance(app, 12);
    running(app)?;
    let Some(current) = visible_enemy_contacts(app)?
        .into_iter()
        .find(|c| c.key == target)
    else {
        return Ok(false);
    };
    let point =
        capture_world_to_screen(app, current.position).ok_or("visible focus target offscreen")?;
    bevy_open_rts::capture_world_click_safety(app, point, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    capture_click_screen(app, point, MouseButton::Right);
    running(app)?;
    let orders = bevy_open_rts::capture_live_order_state(app)?;
    let selected: HashSet<_> = capture_campaign_units(app)
        .iter()
        .filter(|u| u.selected)
        .map(|u| u.entity.to_bits())
        .collect();
    let assigned: Vec<_> = orders["units"]
        .as_array()
        .ok_or("focus attack orders missing")?
        .iter()
        .filter(|u| u["key"].as_u64().is_some_and(|key| selected.contains(&key)))
        .filter(|u| {
            u["active"].as_array().is_some_and(|active| {
                active
                    .iter()
                    .any(|order| order["Attack"].as_u64() == Some(target))
            })
        })
        .filter_map(|u| u["key"].as_u64())
        .collect();
    let target_still_visible = visible_enemy_contacts(app)?.iter().any(|c| c.key == target);
    let mut trace = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("focus-attacks.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(trace, "{}", serde_json::json!({"clock":orders["clock"],"target":target,"id":current.id,"selected":selected,"assigned_attack_actors":assigned,"target_still_visible":target_still_visible,"scope":"Visible enemy observation, normal mode cancellation and normal right click; no direct order/resource/actor writes"})).map_err(|e|e.to_string())?;
    if assigned.is_empty() && target_still_visible {
        return Err(format!(
            "normal right click did not focus visible enemy {target}"
        ));
    }
    println!(
        "[capture] Twin normal focused attack target={target} actors={}",
        assigned.len()
    );
    Ok(true)
}
