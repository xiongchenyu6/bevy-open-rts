use super::*;
use bevy_open_rts::{capture_save_observation, capture_save_restore_ready};

fn chord(app: &mut App, key: KeyCode) {
    capture_key(app, KeyCode::ControlLeft, true);
    app.update();
    tap_key(app, key);
    capture_key(app, KeyCode::ControlLeft, false);
    app.update();
}

fn rejected_load(app: &mut App, handle: &Handle<Image>, path: PathBuf) -> Result<(), String> {
    let before =
        capture_save_observation(app).ok_or("snapshot before rejected load unavailable")?;
    chord(app, KeyCode::KeyL);
    let after = capture_save_observation(app).ok_or("failed load discarded the active match")?;
    if serde_json::to_value(&before).unwrap() != serde_json::to_value(&after).unwrap()
        || !bevy_open_rts::capture_battle_log_messages(app)
            .iter()
            .any(|message| message.starts_with("Load failed:"))
    {
        return Err("invalid load changed the paused match or omitted player feedback".into());
    }
    shoot(app, handle, path);
    Ok(())
}

pub(super) fn render(dir: &Path) -> Result<(), String> {
    let sandbox = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("save-load requires an explicit absolute sandbox XDG_DATA_HOME")?;
    let save_file = sandbox.join("bevy-open-rts/quicksave.ron");
    if save_file.exists() {
        return Err("save-load refuses to replace an existing quicksave".into());
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    tap_key(&mut app, KeyCode::F10);
    rejected_load(&mut app, &handle, dir.join("missing_save.png"))?;
    tap_key(&mut app, KeyCode::F10);
    build_structure_via_real_input(&mut app, "PowerReactor")?;
    let barracks = build_structure_via_real_input(&mut app, "Barracks")?;
    capture_focus_camera_on(&mut app, barracks);
    for _ in 0..12 {
        app.update();
    }
    let point = capture_world_to_screen(&mut app, barracks).ok_or("producer outside view")?;
    capture_click_screen(&mut app, point, MouseButton::Left);
    tap_key(&mut app, KeyCode::F10);
    let train = capture_enabled_train_hotkey_for(&mut app, "LightRifleInfantry")
        .ok_or("infantry training command unavailable")?;
    tap_key(&mut app, train);
    let expected = capture_save_observation(&mut app).ok_or("running snapshot unavailable")?;
    if expected.jobs.len() != 1 {
        return Err("expected one paid queued unit before saving".into());
    }
    chord(&mut app, KeyCode::KeyS);
    if !save_file.is_file() {
        return Err("Ctrl+S did not write the sandbox quicksave".into());
    }
    shoot(&mut app, &handle, dir.join("00_saved.png"));
    tap_key(&mut app, train);
    let changed = capture_save_observation(&mut app).ok_or("changed snapshot unavailable")?;
    if changed.jobs.len() != 2 || changed.ore >= expected.ore {
        return Err("second normal train command did not change paid production".into());
    }
    shoot(&mut app, &handle, dir.join("01_changed.png"));
    chord(&mut app, KeyCode::KeyL);
    if !wait_until(&mut app, 120, |app| capture_save_restore_ready(app)) {
        return Err("Ctrl+L did not finish the real restart/load flow".into());
    }
    let restored = capture_save_observation(&mut app).ok_or("restored snapshot unavailable")?;
    if bevy_open_rts::capture_match_briefing_visible(&app) {
        return Err("loading replayed the opening briefing".into());
    }
    let same_jobs = restored.jobs.len() == expected.jobs.len()
        && restored
            .jobs
            .iter()
            .zip(&expected.jobs)
            .all(|(actual, saved)| {
                actual.0 == saved.0 && actual.2 == saved.2 && (actual.1 - saved.1).abs() < 0.3
            });
    if (
        restored.ore,
        restored.crystal,
        restored.units,
        restored.structures,
    ) != (
        expected.ore,
        expected.crystal,
        expected.units,
        expected.structures,
    ) || !same_jobs
        || restored.mission != expected.mission
        || restored.fired_triggers != expected.fired_triggers
        || restored.explored_cells < expected.explored_cells
        || (restored.clock - expected.clock).abs() > 0.3
    {
        return Err(format!(
            "save/load continuity mismatch: expected={expected:?} restored={restored:?}"
        ));
    }
    shoot(&mut app, &handle, dir.join("02_restored.png"));
    // Corrupt only our disposable sandbox, then use the ordinary load chord.
    // A rejected file must leave the current match and the file intact.
    tap_key(&mut app, KeyCode::F10);
    let valid_bytes = std::fs::read(&save_file).map_err(|error| error.to_string())?;
    let corrupt_bytes = b"invalid quicksave acceptance fixture";
    std::fs::write(&save_file, corrupt_bytes).map_err(|error| error.to_string())?;
    rejected_load(&mut app, &handle, dir.join("corrupt_save.png"))?;
    if std::fs::read(&save_file).map_err(|error| error.to_string())? != corrupt_bytes {
        return Err("failed load modified the corrupt save".into());
    }
    std::fs::write(&save_file, valid_bytes).map_err(|error| error.to_string())?;
    let evidence = serde_json::json!({"expected":expected,"changed":changed,"restored":restored});
    std::fs::write(
        dir.join("observations.json"),
        serde_json::to_string_pretty(&evidence).unwrap(),
    )
    .map_err(|error| error.to_string())?;
    println!(
        "[capture] real Ctrl+S/Ctrl+L: continuity restored; missing/corrupt loads preserve match and show feedback"
    );
    Ok(())
}
