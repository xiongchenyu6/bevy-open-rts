//! Paid active construction/production and naturally advancing weather only.
use super::*;
use bevy_open_rts::{
    capture_disk_lifecycle_state as disk, capture_lifecycle_setup_button,
    capture_live_lifecycle_state as live, capture_save_restore_ready,
};
use serde_json::{Value, json};

fn write(dir: &Path, name: &str, value: &Value) -> Result<(), String> {
    std::fs::write(dir.join(name), serde_json::to_vec_pretty(value).unwrap())
        .map_err(|e| e.to_string())
}
fn n(v: &Value) -> f32 {
    v.as_f64().unwrap() as f32
}
fn position(v: &Value) -> Vec3 {
    Vec3::new(n(&v[0]), n(&v[1]), n(&v[2]))
}
fn foundation(s: &Value) -> Option<&Value> {
    s["structures"]
        .as_array()?
        .iter()
        .find(|s| s["team"] == 0 && !s["construction"].is_null())
}
fn job(s: &Value) -> Option<&Value> {
    s["build_jobs"]
        .as_array()?
        .iter()
        .find(|j| j["team"] == 0 && j["id"] == "LightRifleInfantry")
}
fn start() -> Result<App, String> {
    let mut app = build_capture_app(WIDTH, HEIGHT);
    wait_for_startup(&mut app)?;
    for action in [
        "play",
        "player",
        "human-player",
        "opponent",
        "beginner",
        "start",
    ] {
        let screen = capture_lifecycle_setup_button(&mut app, action)
            .ok_or_else(|| format!("normal setup {action} unavailable"))?;
        capture_click_screen(&mut app, screen, MouseButton::Left);
        for _ in 0..8 {
            app.update();
        }
    }
    if !wait_until(&mut app, 120, |a| live(a).is_ok()) {
        return Err("normal Start Match failed".into());
    }
    capture_dismiss_match_briefing(&mut app);
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    Ok(app)
}
fn select_structure(app: &mut App, at: Vec3) -> Result<(), String> {
    save_effects::focus(app, at);
    for _ in 0..12 {
        app.update();
    }
    let screen = capture_world_to_screen(app, at).ok_or("producer offscreen")?;
    bevy_open_rts::capture_world_click_safety(app, screen, Vec2::new(WIDTH as f32, HEIGHT as f32))?;
    capture_click_screen(app, screen, MouseButton::Left);
    Ok(())
}

pub(super) fn create(dir: &Path) -> Result<(), String> {
    if save_effects::sandbox_save()?.exists() {
        return Err("existing sandbox save refused".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start()?;
    let handle = capture_handle(&app);
    let result = (|| {
        let initial = live(&mut app)?;
        if initial["settings"]["player_controllers"] != json!([1, 2])
            || initial["settings"]["visible_team"] != 0
        {
            return Err(
                "actual normal setup is not Human0/Beginner1 with player0 visibility".into(),
            );
        }
        if initial["economies"][0]["ore"] != 32 || initial["economies"][0]["crystal"] != 16 {
            return Err("normal starting32/16 required".into());
        }
        build_campaign_structure_via_real_input(&mut app, "PowerReactor", &mut |_| {})?;
        let barracks = build_campaign_structure_via_real_input(&mut app, "Barracks", &mut |_| {})?;
        if !wait_until(&mut app, 7200, |a| {
            live(a).is_ok_and(|s| {
                n(&s["weather"]["blend"]) < 0.15
                    && s["weather"]["previous"] != s["weather"]["current"]
            })
        }) {
            return Err("natural first weather fade not observed".into());
        }
        let mut observer = |_: &mut App| {};
        place_campaign_structure_with_offsets(
            &mut app,
            "PowerReactor",
            [
                Vec3::new(7.0, 0.0, 0.0),
                Vec3::new(7.0, 0.0, 3.0),
                Vec3::new(7.0, 0.0, -3.0),
                Vec3::new(9.0, 0.0, 3.0),
                Vec3::new(9.0, 0.0, -3.0),
            ],
            &mut observer,
        )?;
        select_structure(&mut app, barracks)?;
        let key = capture_enabled_train_hotkey_for(&mut app, "LightRifleInfantry")
            .ok_or("paid infantry command unavailable")?;
        tap_key(&mut app, key);
        save_effects::menu(&mut app, false)?;
        let expected = disk()?;
        write(dir, "save_state.json", &expected)?;
        let f = foundation(&expected).ok_or("no active paid foundation saved")?;
        let j = job(&expected).ok_or("no active paid infantry job saved")?;
        if n(&expected["weather"]["blend"]) >= 1.0
            || expected["weather"]["previous"] == expected["weather"]["current"]
            || n(&f["construction"][0]) <= 0.0
            || n(&j["timer"]) <= 0.0
        {
            return Err("saved state lacks simultaneous natural fade and active paid work".into());
        }
        let construct_target = f["key"].clone();
        if !expected["units"].as_array().unwrap().iter().any(|u| {
            u["team"] == 0
                && u["continuity"]["active"].as_array().is_some_and(|orders| {
                    orders
                        .iter()
                        .any(|order| order["Construct"] == construct_target)
                })
        }) {
            return Err("saved foundation has no real Worker construct reference".into());
        }
        shoot(&mut app, &handle, dir.join("01_saved.png"));
        write(
            dir,
            "observations.json",
            &json!({"process_id":std::process::id(),"initial":initial,"expected":expected}),
        )
    })();
    if result.is_err() {
        write(dir, "failure.json", &live(&mut app)?)?;
        shoot(&mut app, &handle, dir.join("failure.png"));
    }
    result
}

pub(super) fn resume(evidence: &Path, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(save_effects::sandbox_save()?).map_err(|e| e.to_string())?;
    let source: Value = serde_json::from_slice(
        &std::fs::read(evidence.join("observations.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if source["process_id"].as_u64() == Some(std::process::id() as u64) {
        return Err("fresh process required".into());
    }
    let mut app = start_match_app()?;
    capture_dismiss_match_briefing(&mut app);
    let handle = capture_handle(&app);
    save_effects::menu(&mut app, true)?;
    if !wait_until(&mut app, 120, |a| capture_save_restore_ready(a)) {
        return Err("normal mouse Load failed".into());
    }
    let restored = live(&mut app)?;
    let expected = &source["expected"];
    write(
        dir,
        "restore_fields.json",
        &json!({"expected":expected,"restored":restored}),
    )?;
    let result = (|| {
        let elapsed = n(&restored["clock_sec"]) - n(&expected["clock_sec"]);
        let w = &expected["weather"];
        let rw = &restored["weather"];
        for field in ["previous", "current", "step"] {
            if w[field] != rw[field] {
                return Err(format!("weather {field} reset"));
            }
        }
        if (n(&w["phase_remaining"]) - n(&rw["phase_remaining"]) - elapsed).abs() > 0.1
            || (n(&rw["blend"]) - (n(&w["blend"]) + elapsed / 9.0).min(1.0)).abs() > 0.02
        {
            return Err("weather fade/phase timing mismatch".into());
        }
        let old = foundation(expected).ok_or("saved foundation missing")?;
        let new = restored["structures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| {
                s["team"] == 0
                    && s["id"] == old["id"]
                    && position(&s["position"]) == position(&old["position"])
            })
            .ok_or("foundation not restored")?;
        if new["construction_cost"] != old["construction_cost"]
            || new["construction"][1] != old["construction"][1]
            || (n(&new["construction"][0]) - n(&old["construction"][0])).abs() > elapsed + 0.1
        {
            return Err("paid foundation cost/progress mismatch".into());
        }
        if !restored["units"].as_array().unwrap().iter().any(|u| {
            u["team"] == 0
                && u["continuity"]["active"].as_array().is_some_and(|orders| {
                    orders.iter().any(|order| order["Construct"] == new["key"])
                })
        }) {
            return Err("Worker construction target not remapped".into());
        }
        let oldj = job(expected).ok_or("saved job absent")?;
        let newj = job(&restored).ok_or("loaded job absent")?;
        let producer = restored["structures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == newj["producer"])
            .ok_or("job producer unresolved")?;
        let old_producer = expected["structures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|structure| structure["key"] == oldj["producer"])
            .ok_or("saved job producer missing")?;
        if position(&producer["position"]) != position(&old_producer["position"])
            || newj["train"] != oldj["train"]
            || position(&newj["origin"]) != position(&oldj["origin"])
            || producer["id"] != "Barracks"
            || newj["cost"] != oldj["cost"]
            || newj["producer_id"] != oldj["producer_id"]
            || (n(&oldj["timer"]) - n(&newj["timer"]) - elapsed).abs() > 0.1
            || expected["economies"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .any(|(team, economy)| restored["economies"][team] != *economy)
        {
            return Err("paid job/economy continuity mismatch".into());
        }
        shoot(&mut app, &handle, dir.join("01_restored.png"));
        let at = position(&old["position"]);
        let count = |s: &Value| {
            s["units"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|u| u["team"] == 0 && u["id"] == "LightRifleInfantry")
                .count()
        };
        let before = count(&restored);
        if !wait_until(&mut app, 1800, |a| {
            live(a).is_ok_and(|s| {
                s["build_jobs"].as_array().unwrap().is_empty()
                    && count(&s) == before + 1
                    && n(&s["weather"]["blend"]) == 1.0
                    && s["structures"].as_array().unwrap().iter().any(|f| {
                        f["team"] == 0
                            && position(&f["position"]) == at
                            && f["construction"].is_null()
                    })
            })
        }) {
            return Err("restored paid work/weather did not complete normally".into());
        }
        let after = live(&mut app)?;
        if after["weather"]["step"] != rw["step"]
            || n(&after["weather"]["phase_remaining"]) >= n(&rw["phase_remaining"])
        {
            return Err("weather did not advance normally".into());
        }
        if bytes != std::fs::read(save_effects::sandbox_save()?).map_err(|e| e.to_string())? {
            return Err("load modified save bytes".into());
        }
        shoot(&mut app, &handle, dir.join("02_completed.png"));
        write(
            dir,
            "observations.json",
            &json!({"process_id":std::process::id(),"expected":expected,"restored":restored,"after":after,"save_bytes_unchanged":true,"scope":"Paid active construction/production and natural midfade weather only"}),
        )
    })();
    if result.is_err() {
        write(dir, "failure.json", &live(&mut app)?)?;
        shoot(&mut app, &handle, dir.join("failure.png"));
    }
    result
}
