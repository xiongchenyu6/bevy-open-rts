//! Normal-input cursor metadata proof; offscreen screenshots omit the OS pointer.
use super::*;
use bevy_open_rts::{
    capture_campaign_units, capture_cursor_command_button, capture_cursor_state,
    capture_live_effect_state,
};

fn observe(
    app: &mut App,
    output: &Path,
    observations: &mut Vec<serde_json::Value>,
    phase: &str,
    expected: usize,
    pointer: Vec2,
) -> Result<(), String> {
    capture_set_cursor(app, pointer);
    for _ in 0..8 {
        app.update();
    }
    let state = capture_cursor_state(app, observations.is_empty())?;
    observations.push(serde_json::json!({"phase":phase,"state":state}));
    std::fs::write(output.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"scope":"Normal-input cursor asset/state metadata; native offscreen images omit OS pointer","observations":observations,"all4_native_pointer_visual_verified":false})).unwrap()).map_err(|e|e.to_string())?;
    let last = &observations.last().unwrap()["state"];
    if last["applied_index"] != expected
        || last["desired_index"] != expected
        || last["custom_index"] != expected
        || last["custom_image_matches_loaded"] != true
        || last["custom_layout_matches_loaded"] != true
    {
        return Err(format!("actual cursor state mismatch in {phase}: {last}"));
    }
    let handle = capture_handle(app);
    shoot(
        app,
        &handle,
        output.join(format!("{phase}-offscreen-no-pointer.png")),
    );
    Ok(())
}

fn click_command(app: &mut App, action: &str) -> Result<(), String> {
    let button = capture_cursor_command_button(app, action)
        .ok_or_else(|| format!("enabled {action} command button missing"))?;
    capture_click_screen(app, button, MouseButton::Left);
    for _ in 0..4 {
        app.update();
    }
    Ok(())
}

fn select_infantry(app: &mut App, output: &Path, target: Entity) -> Result<(), String> {
    let mut attempts = Vec::new();
    for attempt in 0..3 {
        let before = capture_campaign_units(app)
            .into_iter()
            .find(|u| u.entity == target)
            .ok_or("trained infantry no longer alive")?;
        capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
        capture_focus_camera_on(app, before.position);
        for _ in 0..12 {
            app.update();
        }
        let current = capture_campaign_units(app)
            .into_iter()
            .find(|u| u.entity == target)
            .ok_or("infantry died during camera settle")?;
        let at = if attempt == 0 {
            current.position
        } else {
            current.click_position
        };
        let point =
            capture_world_to_screen(app, at).ok_or("current infantry projection unavailable")?;
        if point.x < 24.0
            || point.x > WIDTH as f32 - 24.0
            || point.y < 90.0
            || point.y > HEIGHT as f32 - 180.0
        {
            return Err(format!("infantry outside safe click field: {point:?}"));
        }
        capture_key(app, KeyCode::ShiftLeft, false);
        capture_click_screen(app, point, MouseButton::Left);
        let selected = capture_entity_is_selected(app, target);
        attempts.push(serde_json::json!({"attempt":attempt,"target":target.to_bits(),"before_focus_position":before.position.to_array(),"before_focus_click_position":before.click_position.to_array(),"fresh_position":current.position.to_array(),"fresh_click_position":current.click_position.to_array(),"screen":point.to_array(),"selected_expected":selected,"cursor":capture_cursor_state(app,false)?,"units":capture_campaign_units(app).iter().map(|u|serde_json::json!({"key":u.entity.to_bits(),"id":u.id,"health":u.health,"position":u.position.to_array(),"selected":u.selected})).collect::<Vec<_>>()}));
        std::fs::write(
            output.join("selection-diagnostics.json"),
            serde_json::to_vec_pretty(&attempts).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        if selected {
            return Ok(());
        }
    }
    let handle = capture_handle(app);
    shoot(app, &handle, output.join("failure-infantry-selection.png"));
    Err("normal infantry selection failed after fresh grounded/visual clicks".into())
}

pub(super) fn run(output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err("cursor proof output already exists".into());
    }
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let mut app = build_capture_app(WIDTH, HEIGHT);
    wait_for_startup(&mut app)?;
    for action in ["play", "opponent", "beginner", "start"] {
        let point = bevy_open_rts::capture_effect_setup_button(&mut app, action)
            .ok_or_else(|| format!("normal setup button {action} missing"))?;
        capture_click_screen(&mut app, point, MouseButton::Left);
        for _ in 0..8 {
            app.update();
        }
    }
    if !wait_until(&mut app, 180, |a| capture_live_effect_state(a).is_ok()) {
        return Err("normal offline match did not start".into());
    }
    let start = capture_live_effect_state(&mut app)?;
    if start["ore"] != 32 || start["crystal"] != 16 {
        return Err("normal32/16 starting budget required".into());
    }
    let mut observations = Vec::new();
    let hud = Vec2::new(600.0, HEIGHT as f32 - 60.0);
    let ground = Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.45);
    observe(
        &mut app,
        output,
        &mut observations,
        "00_default_hud",
        0,
        hud,
    )?;
    campaign_bot::select_builder(&mut app)?;
    click_command(&mut app, "patrol")?;
    observe(
        &mut app,
        output,
        &mut observations,
        "01_move_patrol",
        1,
        ground,
    )?;
    observe(
        &mut app,
        output,
        &mut observations,
        "02_move_over_hud_default",
        0,
        hud,
    )?;
    capture_click_screen(&mut app, ground, MouseButton::Right);
    for _ in 0..4 {
        app.update();
    }
    for id in ["PowerReactor", "Barracks"] {
        build_campaign_structure_via_real_input(&mut app, id, &mut |_| {})?;
    }
    campaign_bot::queue_to_quota(&mut app, "Barracks", "LightRifleInfantry", 1)?;
    if !wait_until(&mut app, 1200, |a| {
        capture_campaign_units(a)
            .iter()
            .any(|u| u.id == "LightRifleInfantry")
    }) {
        return Err("normal paid infantry training did not finish".into());
    }
    let infantry = capture_campaign_units(&mut app)
        .into_iter()
        .find(|u| u.id == "LightRifleInfantry")
        .ok_or("trained infantry missing")?;
    select_infantry(&mut app, output, infantry.entity)?;
    click_command(&mut app, "attack")?;
    observe(
        &mut app,
        output,
        &mut observations,
        "03_attack_move",
        2,
        ground,
    )?;
    capture_click_screen(&mut app, ground, MouseButton::Right);
    for _ in 0..4 {
        app.update();
    }
    campaign_bot::select_builder(&mut app)?;
    click_command(&mut app, "build_power")?;
    observe(
        &mut app,
        output,
        &mut observations,
        "04_build_ghost",
        3,
        ground,
    )?;
    observe(
        &mut app,
        output,
        &mut observations,
        "05_build_over_hud_default",
        0,
        hud,
    )?;
    capture_click_screen(&mut app, ground, MouseButton::Right);
    for _ in 0..4 {
        app.update();
    }
    observe(
        &mut app,
        output,
        &mut observations,
        "06_cancel_default",
        0,
        hud,
    )?;
    let end = capture_live_effect_state(&mut app)?;
    std::fs::write(output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"start":start,"end":end,"all4_native_cursor_metadata_verified":true,"native_os_pointer_visual_verified":false,"browser_pointer_alignment_verified":false,"scope":"Normal budget/paid builds/training/selection +normal command clicks; no state/resource/actor injection"})).unwrap()).map_err(|e|e.to_string())?;
    Ok(())
}
