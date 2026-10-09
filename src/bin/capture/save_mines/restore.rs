use super::*;

fn trigger_between(before: &Value, next: &Value, keys: &[u64], dir: &Path) -> Option<Value> {
    for key in keys {
        let Some(old) = actor(before, *key) else {
            continue;
        };
        if actor(next, *key).is_some() {
            continue;
        }
        let at = point(&old["position"]);
        for enemy in before["units"]
            .as_array()?
            .iter()
            .filter(|u| u["team"] == 1 && u["id"] == "Worker")
        {
            let victim = enemy["key"].as_u64()?;
            let new_hp = actor(next, victim).map_or(0.0, |u| u["health"].as_f64().unwrap());
            let pulse = next["pulses"].as_array()?.iter().find(|p| {
                p["player"] == true
                    && distance(point(&p["from"]), at) < 0.05
                    && distance(point(&p["to"]), point(&enemy["position"])) < 0.25
            });
            if pulse.is_some() {
                let event = serde_json::json!({"loaded_mine_key":key,"mine_before":old,"enemy_before":enemy,
                    "enemy_after":actor(next,victim),"net_hp_drop":enemy["health"].as_f64()? - new_hp,
                    "player_shot_from_exact_mine":pulse});
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("causal_candidates.jsonl"))
                {
                    use std::io::Write;
                    let _ = serde_json::to_writer(&mut file, &event);
                    let _ = file.write_all(b"\n");
                }
            }
            if enemy["health"].as_f64()? - new_hp >= 3.99 && pulse.is_some() {
                return Some(
                    serde_json::json!({"loaded_mine_key":key,"mine_before":old,"enemy_before":enemy,"enemy_after":actor(next,victim),"player_shot_from_exact_mine":pulse,"before":before,"after":next}),
                );
            }
        }
    }
    None
}
fn observed_step(
    app: &mut App,
    previous: &mut Value,
    keys: &[u64],
    trigger: &mut Option<Value>,
    dir: &Path,
) -> Result<(), String> {
    app.update();
    let next = capture_live_mine_state(app)?;
    if trigger.is_none() {
        *trigger = trigger_between(previous, &next, keys, dir);
    }
    *previous = next;
    Ok(())
}
pub(super) fn resume(evidence: &Path, dir: &Path) -> Result<(), String> {
    let save = save_effects::sandbox_save()?;
    let bytes = std::fs::read(&save).map_err(|e| e.to_string())?;
    let evidence: Value = serde_json::from_slice(
        &std::fs::read(evidence.join("observations.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if evidence["process_id"].as_u64() == Some(std::process::id() as u64) {
        return Err("fresh process required".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    let expected = &evidence["expected"];
    let old_layer =
        actor(expected, evidence["layer_key"].as_u64().unwrap()).ok_or("saved layer missing")?;
    let old_mine =
        actor(expected, evidence["mine_key"].as_u64().unwrap()).ok_or("saved mine missing")?;
    tap_key(&mut app, KeyCode::Escape);
    for _ in 0..8 {
        app.update();
    }
    let load_button = bevy_open_rts::capture_match_menu_button_center(&mut app, "load")
        .ok_or("mouse Load button unavailable")?;
    capture_set_cursor(&mut app, load_button);
    capture_mouse_button(&mut app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(&mut app, MouseButton::Left, false);
    // Observe the first restored simulation frame instead of skipping ten
    // frames: authored mines arm in0.25s and may trigger immediately afterward.
    let mut first = None;
    for _ in 0..120 {
        app.update();
        if let Ok(s) = capture_live_mine_state(&mut app) {
            if s["units"].as_array().unwrap().iter().any(|u| {
                u["team"] == 0
                    && mine(u).is_some()
                    && distance(point(&u["position"]), point(&old_mine["position"])) < 0.15
            }) {
                first = Some(s);
                break;
            }
        }
    }
    let restored = first.ok_or("first loaded mine frame unavailable")?;
    let layer = restored["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| {
            u["team"] == 0
                && u["id"] == "DroneMineLayer"
                && distance(point(&u["position"]), point(&old_layer["position"])) < 0.15
        })
        .ok_or("loaded layer missing")?;
    let loaded_mine = restored["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| {
            u["team"] == 0
                && mine(u).is_some()
                && distance(point(&u["position"]), point(&old_mine["position"])) < 0.15
        })
        .ok_or("loaded mine missing")?;
    let elapsed = restored["clock"].as_f64().unwrap() - expected["clock"].as_f64().unwrap();
    let old = mine(old_mine).unwrap();
    let new = mine(loaded_mine).unwrap();
    if old_layer["key"] == layer["key"]
        || old_mine["key"] == loaded_mine["key"]
        || new[4] != layer["key"]
        || old[..3] != new[..3]
        || (old[3].as_f64().unwrap() - new[3].as_f64().unwrap() - elapsed).abs() > 0.1
        || old_layer["layer"][1] != layer["layer"][1]
        || (old_layer["layer"][0].as_f64().unwrap() - layer["layer"][0].as_f64().unwrap() - elapsed)
            .abs()
            > 0.1
    {
        return Err("mine/source remap or arming/layer timer/index mismatch".into());
    }
    let keys = restored["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| mine(u).is_some_and(|m| m[4] == layer["key"]))
        .filter_map(|u| u["key"].as_u64())
        .collect::<Vec<_>>();
    write(dir, "01_restored_state.json", restored.clone())?;
    let mut previous = restored.clone();
    let mut trigger = None;
    capture_set_cursor(&mut app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(&mut app, point(&loaded_mine["position"]));
    observed_step(&mut app, &mut previous, &keys, &mut trigger, dir)?;
    app.world_mut()
        .spawn(Screenshot::image(handle.clone()))
        .observe(save_to_disk(dir.join("01_restored_mines.png")));
    for _ in 0..16 {
        observed_step(&mut app, &mut previous, &keys, &mut trigger, dir)?;
    }
    for _ in 0..1800 {
        if trigger.is_some() {
            break;
        }
        observed_step(&mut app, &mut previous, &keys, &mut trigger, dir)?;
    }
    let Some(trigger) = trigger else {
        write(dir, "failure.json", previous)?;
        shoot(&mut app, &handle, dir.join("failure.png"));
        return Err("restored mine did not trigger with causal4damage/shot evidence".into());
    };
    if !capture_save_restore_ready(&app) {
        return Err("mouse Load did not finish restoring".into());
    }
    let after = capture_live_mine_state(&mut app)?;
    if std::fs::read(&save).map_err(|e| e.to_string())? != bytes {
        return Err("resume modified saved bytes".into());
    }
    write(
        dir,
        "observations.json",
        serde_json::json!({"process_id":std::process::id(),"expected":expected,"restored":restored,"after":after,"trigger":trigger,"elapsed_after_load":elapsed,"old_layer_key":old_layer["key"],"new_layer_key":layer["key"],"mine_source_remapped":true,"save_bytes_unchanged":true}),
    )?;
    shoot(&mut app, &handle, dir.join("02_triggered_mine.png"));
    println!(
        "[capture] fresh-process mine/source remapped; authored arming/layer state retained; restored mine triggered4damage through normal harvesting; save bytes unchanged"
    );
    Ok(())
}
