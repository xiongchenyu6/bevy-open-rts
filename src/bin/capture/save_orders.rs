//! Real selection/right-click/Shift queue, menu Save and fresh-process menu Load.
use super::*;
use bevy_open_rts::{
    capture_campaign_units, capture_disk_order_state, capture_live_order_state,
    capture_save_restore_ready,
};
use serde_json::Value;

fn sandbox_save() -> Result<PathBuf, String> {
    let root = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("save-orders requires isolated absolute XDG_DATA_HOME")?;
    Ok(root.join("bevy-open-rts/quicksave.ron"))
}

fn menu(app: &mut App, load: bool) -> Result<(), String> {
    tap_key(app, KeyCode::Escape);
    for _ in 0..8 {
        app.update();
    }
    let point =
        bevy_open_rts::capture_match_menu_button_center(app, if load { "load" } else { "save" })
            .ok_or("match menu save/load button unavailable")?;
    capture_click_screen(app, point, MouseButton::Left);
    for _ in 0..4 {
        app.update();
    }
    Ok(())
}

fn harvest(unit: &Value) -> Option<u64> {
    unit["active"]
        .as_array()?
        .iter()
        .find_map(|order| order.get("Harvest")?.get("target")?.as_u64())
}
fn resource(state: &Value, key: u64) -> Result<&Value, String> {
    state["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node[0].as_u64() == Some(key))
        .ok_or("harvest target is not a live resource".into())
}
fn point(value: &Value) -> Vec3 {
    let p = value.as_array().unwrap();
    Vec3::new(
        p[0].as_f64().unwrap() as f32,
        p[1].as_f64().unwrap() as f32,
        p[2].as_f64().unwrap() as f32,
    )
}
fn ground_click(app: &mut App, position: Vec3) -> Result<(), String> {
    // The command card occupies the right side of the match. Center each real
    // destination before projecting it so a queued click cannot hit that UI.
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, position.with_y(0.0));
    for _ in 0..2 {
        app.update();
    }
    let screen = capture_world_to_screen(app, position).ok_or("ground command offscreen")?;
    bevy_open_rts::capture_world_click_safety(app, screen, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    println!(
        "[capture] ground click {position:?} screen={screen:?} shift={}",
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ShiftLeft)
    );
    capture_click_screen(app, screen, MouseButton::Right);
    Ok(())
}

pub(super) fn create(dir: &Path) -> Result<(), String> {
    let save = sandbox_save()?;
    if save.exists() {
        return Err("save-orders refuses an existing sandbox quicksave".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    let worker = capture_player_worker_position(&mut app).ok_or("player Worker unavailable")?;
    capture_focus_camera_on(&mut app, worker);
    for _ in 0..12 {
        app.update();
    }
    let worker =
        capture_player_onscreen_worker_position(&mut app).ok_or("click-safe Worker unavailable")?;
    let mineral = capture_nearest_visible_resource_click_position_to(&mut app, worker)
        .ok_or("visible mineral unavailable")?;
    capture_focus_camera_on(&mut app, worker.lerp(mineral.with_y(worker.y), 0.5));
    for _ in 0..12 {
        app.update();
    }
    let worker = capture_campaign_units(&mut app)
        .into_iter()
        .filter(|unit| unit.id == "Worker")
        .min_by(|a, b| {
            a.position
                .distance_squared(worker)
                .total_cmp(&b.position.distance_squared(worker))
        })
        .ok_or("Worker disappeared")?;
    let screen =
        capture_world_to_screen(&mut app, worker.click_position).ok_or("worker offscreen")?;
    capture_click_screen(&mut app, screen, MouseButton::Left);
    let selected = capture_campaign_units(&mut app)
        .into_iter()
        .find(|unit| unit.selected && unit.id == "Worker")
        .ok_or("worker click did not select Worker")?;
    let mineral = capture_nearest_visible_resource_click_position_to(&mut app, worker.position)
        .ok_or("visible mineral unavailable")?;
    ground_click(&mut app, mineral)?;
    let other = capture_campaign_units(&mut app)
        .into_iter()
        .find(|unit| unit.id == "Worker" && unit.entity != selected.entity)
        .ok_or("second Worker unavailable")?;
    let targets = [
        (other.position + Vec3::new(5.0, 0.0, 4.0)).with_y(0.0),
        (other.position + Vec3::new(8.0, 0.0, 4.0)).with_y(0.0),
        (other.position + Vec3::new(8.0, 0.0, 7.0)).with_y(0.0),
    ];
    capture_focus_camera_on(&mut app, other.position.lerp(targets[2], 0.5));
    for _ in 0..12 {
        app.update();
    }
    let other = capture_campaign_units(&mut app)
        .into_iter()
        .find(|unit| unit.entity == other.entity)
        .ok_or("second Worker disappeared")?;
    let screen =
        capture_world_to_screen(&mut app, other.click_position).ok_or("second Worker offscreen")?;
    capture_click_screen(&mut app, screen, MouseButton::Left);
    if !capture_entity_is_selected(&mut app, other.entity) {
        return Err("second Worker was not selected by click".into());
    }
    ground_click(&mut app, targets[0])?;
    capture_key(&mut app, KeyCode::ShiftLeft, true);
    app.update();
    ground_click(&mut app, targets[1])?;
    ground_click(&mut app, targets[2])?;
    capture_key(&mut app, KeyCode::ShiftLeft, false);
    app.update();
    menu(&mut app, false)?;
    let expected = capture_disk_order_state()?;
    let units = expected["units"].as_array().unwrap();
    let gather = units
        .iter()
        .find(|unit| harvest(unit).is_some())
        .ok_or("right-click did not save HarvestOrder")?;
    resource(&expected, harvest(gather).unwrap())?;
    let Some(walker) = units
        .iter()
        .find(|unit| unit["queued"].as_array().is_some_and(|q| q.len() == 2))
    else {
        shoot(&mut app, &handle, dir.join("failure_queued_orders.png"));
        return Err("Shift clicks did not save two queued moves".into());
    };
    if walker["queued"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&targets[1..])
        .any(|(order, target)| {
            order
                .get("Move")
                .is_none_or(|value| point(value).distance(*target) > 0.05)
        })
    {
        return Err("saved queued movement does not match real clicks".into());
    }
    std::fs::write(dir.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"process_id":std::process::id(),"expected":expected,"final_target":targets[2].to_array()})).unwrap()).map_err(|e|e.to_string())?;
    let workers = capture_campaign_units(&mut app)
        .into_iter()
        .filter(|unit| unit.id == "Worker")
        .collect::<Vec<_>>();
    let focus = workers.iter().map(|unit| unit.position).sum::<Vec3>() / workers.len() as f32;
    capture_set_cursor(&mut app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(&mut app, focus);
    for _ in 0..2 {
        app.update();
    }
    shoot(&mut app, &handle, dir.join("00_saved_orders.png"));
    println!("[capture] real right-click harvest and Shift-queued moves saved through mouse menu");
    Ok(())
}

pub(super) fn resume(evidence: &Path, dir: &Path) -> Result<(), String> {
    let save = sandbox_save()?;
    let bytes = std::fs::read(&save).map_err(|e| e.to_string())?;
    let evidence: Value = serde_json::from_slice(
        &std::fs::read(evidence.join("observations.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if evidence["process_id"].as_u64() == Some(std::process::id() as u64) {
        return Err("resume must run in a fresh process".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    menu(&mut app, true)?;
    if !wait_until(&mut app, 120, |app| capture_save_restore_ready(app)) {
        return Err("fresh-process menu load did not finish".into());
    }
    let restored = capture_live_order_state(&mut app)?;
    let expected = &evidence["expected"];
    let expected_units = expected["units"].as_array().unwrap();
    let units = restored["units"].as_array().unwrap();
    let old_gather = expected_units
        .iter()
        .find(|u| harvest(u).is_some())
        .unwrap();
    let gather = units
        .iter()
        .find(|u| harvest(u).is_some())
        .ok_or("loaded HarvestOrder missing")?;
    let old_target = harvest(old_gather).unwrap();
    let target = harvest(gather).unwrap();
    let old_node = resource(expected, old_target)?;
    let node = resource(&restored, target)?;
    if old_target == target || old_node[1] != node[1] || old_node[2] != node[2] {
        return Err("harvest resource reference was not remapped to equivalent live node".into());
    }
    let old_walker = expected_units
        .iter()
        .find(|u| u["queued"].as_array().is_some_and(|q| q.len() == 2))
        .unwrap();
    let walker = units
        .iter()
        .find(|u| u["queued"] == old_walker["queued"])
        .ok_or("loaded queued targets missing/reordered")?;
    let walker_key = walker["key"].as_u64().unwrap();
    let final_target = point(&evidence["final_target"]);
    let ore = restored["ore"].as_i64().unwrap();
    let crystal = restored["crystal"].as_i64().unwrap();
    if !wait_until(&mut app, 3000, |app| {
        let Ok(s) = capture_live_order_state(app) else {
            return false;
        };
        let delivered =
            s["ore"].as_i64().unwrap() > ore || s["crystal"].as_i64().unwrap() > crystal;
        let finished = s["units"].as_array().unwrap().iter().any(|u| {
            u["key"].as_u64() == Some(walker_key)
                && u["queued"].as_array().unwrap().is_empty()
                && u["active"].as_array().unwrap().is_empty()
                && point(&u["position"]).distance(final_target) < 0.6
        });
        delivered && finished
    }) {
        return Err(
            "loaded orders did not resume normal harvesting and finish queued walking".into(),
        );
    }
    let after = capture_live_order_state(&mut app)?;
    if resource(&after, target)?[3].as_i64().unwrap() >= node[3].as_i64().unwrap() {
        return Err("loaded harvest did not consume its remapped resource".into());
    }
    if std::fs::read(save).map_err(|e| e.to_string())? != bytes {
        return Err("resume modified saved bytes".into());
    }
    std::fs::write(dir.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"process_id":std::process::id(),"expected":expected,"restored":restored,"after":after,"target_reference_remapped":true,"save_bytes_unchanged":true})).unwrap()).map_err(|e|e.to_string())?;
    capture_focus_camera_on(&mut app, final_target);
    for _ in 0..12 {
        app.update();
    }
    shoot(&mut app, &handle, dir.join("01_resumed_orders.png"));
    println!(
        "[capture] fresh-process remapped harvest target, delivered resources and consumed queued movement"
    );
    Ok(())
}
