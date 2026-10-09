use super::*;
use bevy_open_rts::{capture_save_observation, capture_save_restore_ready};

fn chord(app: &mut App, key: KeyCode) {
    capture_key(app, KeyCode::ControlLeft, true);
    app.update();
    tap_key(app, key);
    capture_key(app, KeyCode::ControlLeft, false);
    app.update();
}

fn invoke(
    app: &mut App,
    load: bool,
    menu: bool,
    handle: &Handle<Image>,
    dir: &Path,
) -> Result<(), String> {
    if !menu {
        chord(app, if load { KeyCode::KeyL } else { KeyCode::KeyS });
        return Ok(());
    }
    tap_key(app, KeyCode::Escape);
    for _ in 0..8 {
        app.update();
    }
    let point =
        bevy_open_rts::capture_match_menu_button_center(app, if load { "load" } else { "save" })
            .ok_or("visible match menu has no save/load button")?;
    shoot(
        app,
        handle,
        dir.join(if load {
            "load_menu.png"
        } else {
            "save_menu.png"
        }),
    );
    capture_click_screen(app, point, MouseButton::Left);
    for _ in 0..4 {
        app.update();
    }
    Ok(())
}

fn rejected_load(
    app: &mut App,
    handle: &Handle<Image>,
    path: PathBuf,
    menu: bool,
) -> Result<(), String> {
    let before =
        capture_save_observation(app).ok_or("snapshot before rejected load unavailable")?;
    invoke(app, true, menu, handle, path.parent().unwrap())?;
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
    render_with_input(dir, false, WIDTH, false)
}

pub(super) fn render_menu(dir: &Path, width: u32) -> Result<(), String> {
    render_with_input(dir, true, width, false)
}

pub(super) fn render_mission(dir: &Path) -> Result<(), String> {
    render_with_input(dir, true, WIDTH, true)
}

fn living_units(app: &mut App) -> usize {
    let snapshot = bevy_open_rts::capture_construction_contact_debug(app);
    snapshot["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entity| {
            entity["unit"].is_string() && entity["health"].as_f64().is_some_and(|hp| hp > 0.0)
        })
        .count()
}

fn render_with_input(dir: &Path, menu: bool, width: u32, mission: bool) -> Result<(), String> {
    let sandbox = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("save-load requires an explicit absolute sandbox XDG_DATA_HOME")?;
    let save_file = sandbox.join("bevy-open-rts/quicksave.ron");
    if save_file.exists() {
        return Err("save-load refuses to replace an existing quicksave".into());
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut app = build_capture_app(width, HEIGHT);
    wait_for_startup(&mut app)?;
    if mission {
        if !capture_start_mission(&mut app, 0) {
            return Err("save acceptance mission missing".into());
        }
    } else {
        start_shared_match_scene_with_current_setup(&mut app);
    }
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    tap_key(&mut app, KeyCode::F10);
    rejected_load(&mut app, &handle, dir.join("missing_save.png"), menu)?;
    tap_key(&mut app, KeyCode::F10);
    build_structure_via_real_input(&mut app, "PowerReactor")?;
    let barracks = build_structure_via_real_input(&mut app, "Barracks")?;
    if mission {
        let before_wave = living_units(&mut app);
        if !wait_until(&mut app, 1800, |app| {
            capture_save_observation(app).is_some_and(|state| state.clock >= 46.0)
        }) {
            return Err("mission did not reach its first normal timed wave".into());
        }
        let observed = capture_save_observation(&mut app).unwrap();
        if observed.mission.as_deref() != Some("hold_the_line")
            || observed.fired_triggers.get(1) != Some(&true)
            || living_units(&mut app) < before_wave + 3
        {
            return Err("authored first wave was not observed before saving".into());
        }
    }
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
    let expected_living_units = living_units(&mut app);
    if expected.jobs.len() != 1 {
        return Err("expected one paid queued unit before saving".into());
    }
    invoke(&mut app, false, menu, &handle, dir)?;
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
    invoke(&mut app, true, menu, &handle, dir)?;
    if !wait_until(&mut app, 120, |app| capture_save_restore_ready(app)) {
        return Err("Ctrl+L did not finish the real restart/load flow".into());
    }
    let restored = capture_save_observation(&mut app).ok_or("restored snapshot unavailable")?;
    let restored_living_units = living_units(&mut app);
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
        || restored_living_units != expected_living_units
        || restored.explored_cells < expected.explored_cells
        || (restored.clock - expected.clock).abs() > 0.3
    {
        return Err(format!(
            "save/load continuity mismatch: expected={expected:?} restored={restored:?}"
        ));
    }
    shoot(&mut app, &handle, dir.join("02_restored.png"));
    // Corrupt only our disposable sandbox, then use the ordinary load command.
    // A rejected file must leave the current match and the file intact.
    tap_key(&mut app, KeyCode::F10);
    let valid_bytes = std::fs::read(&save_file).map_err(|error| error.to_string())?;
    let corrupt_bytes = b"invalid quicksave acceptance fixture";
    std::fs::write(&save_file, corrupt_bytes).map_err(|error| error.to_string())?;
    rejected_load(&mut app, &handle, dir.join("corrupt_save.png"), menu)?;
    if std::fs::read(&save_file).map_err(|error| error.to_string())? != corrupt_bytes {
        return Err("failed load modified the corrupt save".into());
    }
    std::fs::write(&save_file, valid_bytes).map_err(|error| error.to_string())?;
    let evidence = serde_json::json!({"input": if menu { "mouse menu" } else { "keyboard" }, "width":width,"expected":expected,"changed":changed,"restored":restored,"living_units_before":expected_living_units,"living_units_restored":restored_living_units});
    std::fs::write(
        dir.join("observations.json"),
        serde_json::to_string_pretty(&evidence).unwrap(),
    )
    .map_err(|error| error.to_string())?;
    if menu {
        tap_key(&mut app, KeyCode::Escape);
        for _ in 0..8 {
            app.update();
        }
        let point = bevy_open_rts::capture_match_menu_button_center(&mut app, "return")
            .ok_or("match menu return button missing")?;
        capture_click_screen(&mut app, point, MouseButton::Left);
        if !wait_until(&mut app, 120, |app| capture_startup_ready(app)) {
            return Err("Main Menu button did not reach the main menu".into());
        }
        shoot(&mut app, &handle, dir.join("03_main_menu.png"));
    }
    println!(
        "[capture] {}: continuity restored; missing/corrupt loads preserve match and show feedback",
        if menu {
            "real mouse Save/Load"
        } else {
            "real Ctrl+S/Ctrl+L"
        }
    );
    Ok(())
}

/// Restore a previously saved match in a newly launched capture process using
/// the same mouse menu as a player. The caller supplies an isolated save sandbox.
pub(super) fn render_resume(evidence_dir: &Path, dir: &Path) -> Result<(), String> {
    let sandbox = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("save-resume requires an explicit absolute sandbox XDG_DATA_HOME")?;
    let save_file = sandbox.join("bevy-open-rts/quicksave.ron");
    let original_bytes = std::fs::read(&save_file).map_err(|error| error.to_string())?;
    let evidence: serde_json::Value = serde_json::from_slice(
        &std::fs::read(evidence_dir.join("observations.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let expected: bevy_open_rts::CaptureSaveObservation =
        serde_json::from_value(evidence["expected"].clone()).map_err(|error| error.to_string())?;
    let expected_living = evidence["living_units_before"]
        .as_u64()
        .ok_or("saved evidence has no living-unit count")? as usize;
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut app = build_capture_app(WIDTH, HEIGHT);
    wait_for_startup(&mut app)?;
    start_shared_match_scene_with_current_setup(&mut app);
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    invoke(&mut app, true, true, &handle, dir)?;
    if !wait_until(&mut app, 120, |app| capture_save_restore_ready(app)) {
        return Err("fresh-process mouse load did not finish".into());
    }
    let actual = capture_save_observation(&mut app).ok_or("restored match unavailable")?;
    let actual_living = living_units(&mut app);
    let same_jobs = actual.jobs.len() == expected.jobs.len()
        && actual
            .jobs
            .iter()
            .zip(&expected.jobs)
            .all(|(actual, saved)| {
                actual.0 == saved.0 && actual.2 == saved.2 && (actual.1 - saved.1).abs() < 0.3
            });
    if (actual.ore, actual.crystal, actual.units, actual.structures)
        != (
            expected.ore,
            expected.crystal,
            expected.units,
            expected.structures,
        )
        || !same_jobs
        || actual.mission != expected.mission
        || actual.fired_triggers != expected.fired_triggers
        || actual_living != expected_living
        || actual.explored_cells < expected.explored_cells
        || (actual.clock - expected.clock).abs() > 0.3
        || bevy_open_rts::capture_match_briefing_visible(&app)
    {
        return Err(format!(
            "fresh-process continuity mismatch: expected={expected:?}, actual={actual:?}"
        ));
    }
    if std::fs::read(&save_file).map_err(|error| error.to_string())? != original_bytes {
        return Err("fresh-process load modified the save file".into());
    }
    shoot(&mut app, &handle, dir.join("00_resumed.png"));
    std::fs::write(
        dir.join("observations.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"input":"fresh-process mouse Load","process_id":std::process::id(),
            "expected":expected,"actual":actual,"living_units_before":expected_living,
            "living_units_restored":actual_living,"save_bytes_unchanged":true}),
        )
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("[capture] fresh-process mouse Load restored campaign, triggers, paid queue and fog");
    Ok(())
}

/// Reject an actual older full save through the player's mouse menu.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn render_reject_old(dir: &Path) -> Result<(), String> {
    let sandbox = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("save-reject requires an absolute isolated XDG_DATA_HOME")?;
    let file = sandbox.join("bevy-open-rts/quicksave.ron");
    let original = std::fs::read(&file).map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    struct Header {
        version: u32,
    }
    let header: Header = ron::from_str(std::str::from_utf8(&original).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if header.version != 2 {
        return Err("save-reject requires the real prior format2 save".into());
    }
    std::fs::create_dir(dir).map_err(|e| e.to_string())?;
    let mut app = build_capture_app(WIDTH, HEIGHT);
    wait_for_startup(&mut app)?;
    start_shared_match_scene_with_current_setup(&mut app);
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    tap_key(&mut app, KeyCode::F10);
    let before = capture_save_observation(&mut app).ok_or("paused match unavailable")?;
    rejected_load(&mut app, &handle, dir.join("unsupported_save.png"), true)?;
    let messages = bevy_open_rts::capture_battle_log_messages(&app);
    if !messages
        .iter()
        .any(|m| m.contains("This save is incompatible with this game version. Start a new match."))
    {
        return Err("old full save omitted clear unsupported-version feedback".into());
    }
    let unchanged = std::fs::read(&file).map_err(|e| e.to_string())? == original;
    if !unchanged {
        return Err("rejected old save changed its bytes".into());
    }
    let after = capture_save_observation(&mut app).ok_or("rejected load discarded match")?;
    std::fs::write(
        dir.join("observations.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "process_id":std::process::id(), "old_save_version":header.version,
            "before":before, "after":after, "messages":messages,
            "save_bytes_unchanged":unchanged, "match_unchanged":true
        }))
        .unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!("[capture] actual old full-schema save rejected without changing match or bytes");
    Ok(())
}
