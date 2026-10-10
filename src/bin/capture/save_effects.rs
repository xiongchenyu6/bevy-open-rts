//! Real paid technology, F1/F5 clicks, mouse Save and fresh-process mouse Load.
use super::*;
use bevy_open_rts::{
    capture_campaign_units, capture_disk_effect_state, capture_effect_setup_button,
    capture_live_effect_state, capture_save_restore_ready,
};
use serde_json::Value;

pub(super) fn sandbox_save() -> Result<PathBuf, String> {
    let root = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or("save-effects requires isolated absolute XDG_DATA_HOME")?;
    Ok(root.join("bevy-open-rts/quicksave.ron"))
}
pub(super) fn menu(app: &mut App, load: bool) -> Result<(), String> {
    tap_key(app, KeyCode::Escape);
    for _ in 0..8 {
        app.update();
    }
    let screen =
        bevy_open_rts::capture_match_menu_button_center(app, if load { "load" } else { "save" })
            .ok_or("save/load menu target unavailable")?;
    capture_click_screen(app, screen, MouseButton::Left);
    for _ in 0..4 {
        app.update();
    }
    Ok(())
}
fn setup_click(app: &mut App, action: &str) -> Result<(), String> {
    let screen = capture_effect_setup_button(app, action)
        .ok_or_else(|| format!("setup action {action} unavailable"))?;
    capture_click_screen(app, screen, MouseButton::Left);
    for _ in 0..8 {
        app.update();
    }
    Ok(())
}
pub(super) fn start() -> Result<App, String> {
    let mut app = build_capture_app(WIDTH, HEIGHT);
    wait_for_startup(&mut app)?;
    for action in ["play", "faction", "chaos", "opponent", "beginner", "start"] {
        setup_click(&mut app, action)?;
    }
    if !wait_until(&mut app, 120, |a| capture_live_effect_state(a).is_ok()) {
        return Err("mouse Start Match did not start offline match".into());
    }
    capture_dismiss_match_briefing(&mut app);
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    Ok(app)
}
fn point(value: &Value) -> Vec3 {
    let p = value.as_array().unwrap();
    Vec3::new(
        p[0].as_f64().unwrap() as f32,
        p[1].as_f64().unwrap() as f32,
        p[2].as_f64().unwrap() as f32,
    )
}
fn shield(unit: &Value) -> Option<(f64, f64)> {
    let s = unit["effects"]["shield"].as_array()?;
    Some((s[0].as_f64()?, s[1].as_f64()?))
}
fn reveal(state: &Value) -> Option<&Value> {
    state["support_entities"]
        .as_array()?
        .iter()
        .find_map(|e| e.get("Reveal"))
}
fn number(value: &Value) -> f64 {
    value.as_f64().unwrap()
}
pub(super) fn focus(app: &mut App, at: Vec3) {
    capture_set_cursor(app, Vec2::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5));
    capture_focus_camera_on(app, at.with_y(0.0));
    for _ in 0..2 {
        app.update();
    }
}
fn fire(app: &mut App, key: KeyCode, at: Vec3) -> Result<(), String> {
    focus(app, at);
    tap_key(app, key);
    let screen = capture_world_to_screen(app, at).ok_or("support target offscreen")?;
    bevy_open_rts::capture_world_click_safety(app, screen, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    capture_click_screen(app, screen, MouseButton::Left);
    Ok(())
}
fn write(dir: &Path, value: Value) -> Result<(), String> {
    std::fs::write(
        dir.join("observations.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .map_err(|e| e.to_string())
}

pub(super) fn create(dir: &Path) -> Result<(), String> {
    let save = sandbox_save()?;
    if save.exists() {
        return Err("existing sandbox quicksave refused".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start()?;
    let handle = capture_handle(&app);
    let initial = capture_live_effect_state(&mut app)?;
    if initial["ore"] != 32 || initial["crystal"] != 16 {
        return Err(format!("normal starting budget expected: {initial}"));
    }
    for id in ["PowerReactor", "RadarUplink", "RoboticsBay", "TechLab"] {
        build_campaign_structure_via_real_input(&mut app, id, &mut |_| {})?;
    }
    let technology = capture_live_effect_state(&mut app)?;
    if technology["ore"] != 13 || technology["crystal"] != 0 {
        return Err("paid technology did not debit19ore/16crystal from normal budget".into());
    }
    campaign_bot::select_builder(&mut app)?;
    let worker = capture_campaign_units(&mut app)
        .into_iter()
        .find(|u| u.id == "Worker" && u.selected)
        .ok_or("actual Worker click selection missing")?;
    let target_key = worker.entity.to_bits();
    fire(&mut app, KeyCode::F1, worker.position)?;
    fire(&mut app, KeyCode::F5, worker.click_position)?;
    let active = capture_live_effect_state(&mut app)?;
    if reveal(&active).is_none()
        || !active["units"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["key"].as_u64() == Some(target_key) && shield(u).is_some())
    {
        shoot(&mut app, &handle, dir.join("failure_support_fire.png"));
        return Err("F1/F5 clicks did not create reveal and Worker shield".into());
    }
    menu(&mut app, false)?;
    let expected = capture_disk_effect_state()?;
    let worker = expected["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["key"].as_u64() == Some(target_key))
        .ok_or("saved shield Worker missing")?;
    if shield(worker).is_none_or(|(remaining, _)| remaining <= 0.0) || reveal(&expected).is_none() {
        return Err("support effects expired before mouse Save".into());
    }
    write(
        dir,
        serde_json::json!({"process_id":std::process::id(),"target_key":target_key,"initial":initial,"paid_technology":technology,"active_before_save":active,"expected":expected}),
    )?;
    focus(&mut app, point(&worker["position"]));
    shoot(&mut app, &handle, dir.join("00_saved_effects.png"));
    println!(
        "[capture] actual Chaos/Beginner setup, paid tech19ore16crystal, F1 reveal/F5 Worker shield saved through mouse menu"
    );
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
        return Err("fresh process required".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    menu(&mut app, true)?;
    if !wait_until(&mut app, 120, |app| capture_save_restore_ready(app)) {
        return Err("fresh-process mouse Load did not finish".into());
    }
    let restored = capture_live_effect_state(&mut app)?;
    let expected = &evidence["expected"];
    let old = expected["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["key"] == evidence["target_key"])
        .ok_or("saved target missing")?;
    let worker = restored["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| {
            u["id"] == old["id"]
                && point(&u["position"]).distance(point(&old["position"])) < 0.15
                && shield(u).is_some()
        })
        .ok_or("shield not restored onto equivalent live Worker")?;
    let old_shield = shield(old).unwrap();
    let new_shield = shield(worker).unwrap();
    let elapsed = number(&restored["clock"]) - number(&expected["clock"]);
    let old_reveal = reveal(expected).ok_or("saved reveal missing")?;
    let new_reveal = reveal(&restored).ok_or("loaded reveal missing")?;
    std::fs::write(
            dir.join("restore_fields.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "process_id":std::process::id(),"expected":expected,"restored":restored,
                "old_target_key":old["key"],"new_target_key":worker["key"],
                "elapsed":elapsed,"old_shield":old_shield,"new_shield":new_shield,
                "old_reveal":old_reveal,"new_reveal":new_reveal,
                "shield_timer_delta":old_shield.0-new_shield.0-elapsed,
                "reveal_timer_delta":number(&old_reveal["remaining"])-number(&new_reveal["remaining"])-elapsed
            })).unwrap(),
        ).map_err(|e|e.to_string())?;
    if old["key"] == worker["key"]
        || old_shield.1 != new_shield.1
        || (old_shield.0 - new_shield.0 - elapsed).abs() > 0.1
        || point(&old_reveal["position"]) != point(&new_reveal["position"])
        || (number(&old_reveal["remaining"]) - number(&new_reveal["remaining"]) - elapsed).abs()
            > 0.1
    {
        shoot(&mut app, &handle, dir.join("failure_restore_fields.png"));
        return Err("effect target remap/remaining timer/scale/reveal position mismatch".into());
    }
    for index in [0, 4] {
        let a = number(&expected["cooldowns"][0][index]);
        let b = number(&restored["cooldowns"][0][index]);
        if a <= 0.0 || (a - b - elapsed).abs() > 0.1 {
            return Err("support cooldown did not restore real elapsed time".into());
        }
    }
    focus(&mut app, point(&worker["position"]));
    shoot(&mut app, &handle, dir.join("01_restored_effects.png"));
    let key = worker["key"].as_u64().unwrap();
    if !wait_until(&mut app, 360, |a| {
        capture_live_effect_state(a).is_ok_and(|s| {
            reveal(&s).is_none()
                && s["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|u| u["key"].as_u64() == Some(key) && shield(u).is_none())
        })
    }) {
        return Err("loaded reveal/shield did not expire through ordinary simulation".into());
    }
    let expired = capture_live_effect_state(&mut app)?;
    if number(&expired["cooldowns"][0][0]) >= number(&restored["cooldowns"][0][0])
        || number(&expired["cooldowns"][0][4]) >= number(&restored["cooldowns"][0][4])
    {
        return Err("loaded cooldown did not continue counting down".into());
    }
    if std::fs::read(&save).map_err(|e| e.to_string())? != bytes {
        return Err("resume changed saved bytes".into());
    }
    write(
        dir,
        serde_json::json!({"process_id":std::process::id(),"expected":expected,"restored":restored,"expired":expired,"old_target_key":old["key"],"new_target_key":key,"elapsed_after_load":elapsed,"save_bytes_unchanged":true,"scope":"Radar reveal and Shield Overdrive only; other effects and shield combat mitigation unverified"}),
    )?;
    shoot(&mut app, &handle, dir.join("02_expired_effects.png"));
    println!(
        "[capture] fresh-process Worker shield target remapped; reveal/buff timers and cooldowns preserved, normal expiry verified, save bytes unchanged"
    );
    Ok(())
}
