//! Normal harvesting, scouting, proxy construction and mine-source persistence.
use super::*;
use bevy_open_rts::{
    capture_campaign_resources, capture_campaign_state, capture_campaign_units,
    capture_disk_mine_state, capture_live_mine_state, capture_save_restore_ready,
    capture_world_click_safety,
};
use serde_json::Value;

fn point(v: &Value) -> Vec3 {
    let p = v.as_array().unwrap();
    Vec3::new(
        p[0].as_f64().unwrap() as f32,
        p[1].as_f64().unwrap() as f32,
        p[2].as_f64().unwrap() as f32,
    )
}
fn distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
}
fn actor(state: &Value, key: u64) -> Option<&Value> {
    state["units"]
        .as_array()?
        .iter()
        .find(|u| u["key"].as_u64() == Some(key))
}
fn mine(u: &Value) -> Option<&Vec<Value>> {
    u["mine"].as_array()
}
fn select(app: &mut App, key: u64) -> Result<(), String> {
    let unit = capture_campaign_units(app)
        .into_iter()
        .find(|u| u.entity.to_bits() == key)
        .ok_or("friendly actor unavailable")?;
    save_effects::focus(app, unit.position);
    let current = capture_campaign_units(app)
        .into_iter()
        .find(|u| u.entity.to_bits() == key)
        .ok_or("actor disappeared")?;
    let screen = capture_world_to_screen(app, current.click_position).ok_or("actor offscreen")?;
    capture_click_screen(app, screen, MouseButton::Left);
    if !capture_entity_is_selected(app, current.entity) {
        return Err("real actor click did not select intended actor".into());
    }
    Ok(())
}
fn click(app: &mut App, at: Vec3, button: MouseButton) -> Result<(), String> {
    save_effects::focus(app, at);
    let screen = capture_world_to_screen(app, at).ok_or("world target offscreen")?;
    capture_world_click_safety(app, screen, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    capture_click_screen(app, screen, button);
    Ok(())
}
fn move_to(app: &mut App, at: Vec3) -> Result<(), String> {
    capture_key(app, KeyCode::AltLeft, true);
    app.update();
    click(app, at.with_y(0.0), MouseButton::Right)?;
    capture_key(app, KeyCode::AltLeft, false);
    app.update();
    Ok(())
}
fn write(dir: &Path, name: &str, value: Value) -> Result<(), String> {
    std::fs::write(dir.join(name), serde_json::to_vec_pretty(&value).unwrap())
        .map_err(|e| e.to_string())
}
fn enemy_base(state: &Value) -> Option<&Value> {
    state["structures"]
        .as_array()?
        .iter()
        .find(|s| s["team"] == 1 && s["id"] == "CommandCenter" && s["visible"] == true)
}
#[path = "save_mines/restore.rs"]
mod restore;
#[path = "save_mines/scenario.rs"]
mod scenario;

pub(super) fn create(dir: &Path) -> Result<(), String> {
    if save_effects::sandbox_save()?.exists() {
        return Err("existing sandbox quicksave refused".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = save_effects::start()?;
    let handle = capture_handle(&app);
    match scenario::prepare(&mut app, &handle, dir) {
        Ok(evidence) => {
            write(dir, "observations.json", evidence)?;
            println!(
                "[capture] normal harvest/scouting/proxy factory and paid mine-layer deployed corridor mine; mouse Save completed"
            );
            Ok(())
        }
        Err(error) => {
            if let Ok(state) = capture_live_mine_state(&mut app) {
                write(dir, "failure.json", state)?;
            }
            shoot(&mut app, &handle, dir.join("failure.png"));
            Err(error)
        }
    }
}

pub(super) fn resume(evidence: &Path, dir: &Path) -> Result<(), String> {
    restore::resume(evidence, dir)
}
