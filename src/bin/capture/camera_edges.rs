//! Normal camera input QA; no world actor/resource creation or options writes.
use super::*;
use bevy_open_rts::{
    capture_camera_edge_observation, capture_camera_menu_button, capture_camera_menu_observation,
    capture_camera_minimap_point, capture_camera_mouse_wheel,
};

fn observe(app: &mut App, dir: &Path, name: &str) -> Result<serde_json::Value, String> {
    let handle = capture_handle(app);
    shoot(app, &handle, dir.join(format!("{name}.png")));
    let observation = capture_camera_edge_observation(app);
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&observation).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(observation)
}
fn focus(observation: &serde_json::Value) -> Vec3 {
    let a = observation["focus"].as_array().unwrap();
    Vec3::new(
        a[0].as_f64().unwrap() as f32,
        a[1].as_f64().unwrap() as f32,
        a[2].as_f64().unwrap() as f32,
    )
}
fn settle(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.update();
    }
}
fn click_menu(app: &mut App, dir: &Path, action: &str) -> Result<(), String> {
    use std::io::Write;
    let before = capture_camera_menu_observation(app);
    let mut trace = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("menu-input-trace.jsonl"))
        .map_err(|e| e.to_string())?;
    let Some(point) = capture_camera_menu_button(app, action) else {
        writeln!(
            trace,
            "{}",
            serde_json::json!({"action":action,"missing":true,"before":before})
        )
        .map_err(|e| e.to_string())?;
        return Err(format!(
            "menu button {action} missing; real page {}",
            before["screen"]
        ));
    };
    capture_set_cursor(app, point);
    settle(app, 2);
    let hovered = capture_camera_menu_observation(app);
    capture_mouse_button(app, MouseButton::Left, true);
    app.update();
    let pressed = capture_camera_menu_observation(app);
    capture_mouse_button(app, MouseButton::Left, false);
    app.update();
    settle(app, 8);
    let after = capture_camera_menu_observation(app);
    writeln!(trace,"{}",serde_json::json!({"action":action,"point":point.to_array(),"before":before,"hovered":hovered,"pressed":pressed,"after":after})).map_err(|e|e.to_string())?;
    Ok(())
}

fn require_fog(observation: &serde_json::Value) -> Result<(), String> {
    if observation["fog"]["cells"].as_u64().unwrap_or(0) == 0
        || observation["fog"]["opaque_cells"].as_u64().unwrap_or(0) == 0
    {
        return Err("actual unexplored fog missing".into());
    }
    if observation["outskirts"].as_array().map_or(0, Vec::len) != 2 {
        return Err("render outskirts/rim missing".into());
    }
    Ok(())
}
pub(super) fn run(
    dir: &Path,
    width: u32,
    height: u32,
    requested_tilt: Option<f32>,
) -> Result<(), String> {
    if dir.exists() {
        return Err("refusing existing camera evidence directory".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = build_capture_app(width, height);
    wait_for_startup(&mut app)?;
    if let Some(tilt) = requested_tilt {
        click_menu(&mut app, dir, "options")?;
        let steps = ((tilt - 0.55) / 0.05).round() as i32;
        for _ in 0..steps.unsigned_abs() {
            click_menu(
                &mut app,
                dir,
                if steps > 0 { "tilt-up" } else { "tilt-down" },
            )?;
        }
        click_menu(&mut app, dir, "back")?;
    }
    start_shared_match_scene_with_current_setup(&mut app);
    settle(&mut app, MATCH_SETTLE_TICKS);
    capture_dismiss_match_briefing(&mut app);
    capture_set_cursor(&mut app, Vec2::new(width as f32 * 0.5, height as f32 * 0.5));
    let start = observe(&mut app, dir, "00_initial")?;
    require_fog(&start)?;
    if requested_tilt.is_some_and(|tilt| {
        (start["settings"]["tilt"].as_f64().unwrap_or(0.0) - tilt as f64).abs() > 0.001
    }) {
        return Err("normal options buttons did not reach requested supported tilt".into());
    }
    let before = focus(&start);
    capture_key(&mut app, KeyCode::ArrowUp, true);
    settle(&mut app, 24);
    capture_key(&mut app, KeyCode::ArrowUp, false);
    settle(&mut app, 12);
    let keyboard = observe(&mut app, dir, "01_arrow_pan")?;
    if focus(&keyboard).distance(before) < 0.1 {
        return Err("normal arrow key did not pan actual camera".into());
    }
    capture_set_cursor(&mut app, Vec2::new(1.0, height as f32 * 0.45));
    settle(&mut app, 24);
    capture_set_cursor(&mut app, Vec2::new(width as f32 * 0.5, height as f32 * 0.5));
    settle(&mut app, 12);
    let edge = observe(&mut app, dir, "02_edge_pan")?;
    if focus(&edge).distance(focus(&keyboard)) < 0.1 {
        return Err("normal edge cursor did not pan camera".into());
    }
    build_structure_via_real_input(&mut app, "PowerReactor")?;
    build_structure_via_real_input(&mut app, "RadarUplink")?;
    let radar = observe(&mut app, dir, "03_paid_radar")?;
    if radar["radar_online"].as_bool() != Some(true) {
        return Err(format!("paid radar not online: {}", radar["radar"]));
    }
    for (name, normalized) in [
        ("04_northwest", Vec2::splat(0.02)),
        ("05_northeast", Vec2::new(0.98, 0.02)),
        ("06_southeast", Vec2::splat(0.98)),
        ("07_southwest", Vec2::new(0.02, 0.98)),
    ] {
        let p = capture_camera_minimap_point(&mut app, normalized)
            .ok_or("minimap input coordinates missing")?;
        capture_click_screen(&mut app, p, MouseButton::Left);
        capture_set_cursor(&mut app, Vec2::new(width as f32 * 0.5, height as f32 * 0.5));
        settle(&mut app, 12);
        let o = observe(&mut app, dir, name)?;
        require_fog(&o)?;
        let extent = &o["map_half_extent"];
        let target = Vec3::new(
            (normalized.x * 2.0 - 1.0) * extent[0].as_f64().unwrap() as f32,
            0.0,
            (normalized.y * 2.0 - 1.0) * extent[1].as_f64().unwrap() as f32,
        );
        if focus(&o).distance(target) > 2.0 {
            return Err(format!(
                "normal minimap click focus disagrees: {}",
                o["focus"]
            ));
        }
    }
    capture_camera_mouse_wheel(&mut app, -8.0);
    settle(&mut app, 30);
    let out = observe(&mut app, dir, "08_zoom_out")?;
    capture_camera_mouse_wheel(&mut app, 8.0);
    settle(&mut app, 30);
    let close = observe(&mut app, dir, "09_zoom_in")?;
    if close["distance"].as_f64().unwrap() >= out["distance"].as_f64().unwrap() - 1.0 {
        return Err("normal wheel did not change actual zoom".into());
    }
    capture_key(&mut app, KeyCode::BracketRight, true);
    settle(&mut app, 12);
    capture_key(&mut app, KeyCode::BracketRight, false);
    settle(&mut app, 12);
    let rotated = observe(&mut app, dir, "10_rotated_edge")?;
    require_fog(&rotated)?;
    if rotated["rendered"]["rotation"] == close["rendered"]["rotation"] {
        return Err("normal bracket did not rotate actual camera".into());
    }
    std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"passed":true,"width":width,"height":height,"requested_tilt":requested_tilt,"scope":"actual arrow/edge/minimap/wheel/bracket/options inputs; normal paid radar; real rendered global camera and fog; no actor/resource grants; existing construction focus helper only; camera assertions use ordinary inputs and tilt options buttons"})).unwrap()).map_err(|e|e.to_string())?;
    Ok(())
}
