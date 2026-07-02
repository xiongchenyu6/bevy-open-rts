//! Headless screenshot / video-frame capture for the **real** Bevy RTS scene.
//!
//! Renders the actual game offscreen (no window) and saves PNG frames via
//! Bevy's `Screenshot::image` + `save_to_disk`. This replaces the previous
//! binary, which was a hand-written 2D rasterizer that drew its own diamond/
//! circle approximation of the scene from a headless simulation snapshot — so
//! its "screenshots" never reflected what the game actually looked like, which
//! defeated the whole point of frame-grounded self-repair.
//!
//! Usage:
//!   capture screenshot [path]            single still (default screenshots/capture/still.png)
//!   capture frames <dir> [count]         numbered frameXXXXX.png sequence (default 450)
//!   capture play <dir>                   real input smoke: select/move/train/build
//!   capture harvest <dir>                real input smoke: Worker right-clicks ore
//!   capture assault <dir> [max-seconds]  real input smoke: select army/attack-move/win
//!   capture match [max-seconds]          headless AI-vs-AI match must resolve
//!   capture ai-duel <a> <b> [max-seconds] AI difficulty duel (beginner|easy|normal|hard)
//!   capture menu [path]                  command menu screenshot
//!   capture menu-wide [path]             command menu screenshot at 2048x1224
//!   capture menu-return [path]           setup -> back -> command menu screenshot
//!   capture factions <dir>               faction base/build smoke screenshots
//!   capture model-harness <dir> [per-page] [page]

use std::env;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use bevy_open_rts::{
    CaptureTarget, build_capture_app, build_model_harness_capture_app, capture_build_options_count,
    capture_enabled_build_hotkey_for, capture_enabled_train_hotkey_for,
    capture_enemy_structure_position, capture_entity_is_selected,
    capture_first_enabled_attack_move_hotkey, capture_first_enabled_build_hotkey,
    capture_first_enabled_train_hotkey, capture_focus_camera_on, capture_key,
    capture_match_phase_label, capture_model_harness_manifest, capture_model_harness_page_count,
    capture_mouse_button, capture_nearest_enemy_anchor_position,
    capture_nearest_visible_resource_click_position_to, capture_nearest_visible_resource_position,
    capture_onscreen_resource_model_center, capture_placement_is_valid,
    capture_player_army_unit_count, capture_player_attack_move_all, capture_player_build_queue_len,
    capture_player_combat_order_count, capture_player_command_center,
    capture_player_completed_structure_count, capture_player_completed_structure_position,
    capture_player_constructing_count, capture_player_harvesting_count,
    capture_player_in_placement_mode, capture_player_onscreen_unit_position,
    capture_player_onscreen_worker_position, capture_player_producer_position,
    capture_player_resources, capture_player_structure_count, capture_player_unit_count,
    capture_player_worker_position, capture_run_ai_duel, capture_run_ai_match_until_resolved,
    capture_selected_player_unit_average_position, capture_selected_player_unit_count,
    capture_selected_player_unit_ids, capture_set_all_factions, capture_set_cursor,
    capture_set_structure_rally, capture_show_credits_menu, capture_show_main_menu,
    capture_show_options_menu, capture_show_skirmish_setup_menu,
    capture_show_skirmish_setup_with_dropdown, capture_spawn_model_harness_page,
    capture_world_to_screen, capture_worst_model_alignment_offset, capture_zoom_camera_closest,
    start_shared_match_scene_with_current_setup,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const WIDE_MENU_WIDTH: u32 = 2048;
const WIDE_MENU_HEIGHT: u32 = 1224;
const MODEL_HARNESS_WIDTH: u32 = 1600;
const MODEL_HARNESS_HEIGHT: u32 = 1000;
const MODEL_HARNESS_DEFAULT_PER_PAGE: usize = 6;
const MODEL_HARNESS_SETTLE_TICKS: usize = 180;
/// Ticks to let assets load and the menu initialize before starting a match.
const WARMUP_TICKS: usize = 90;
/// Ticks to let the match scene populate (bases, units) before first capture.
const MATCH_SETTLE_TICKS: usize = 60;
/// Extra ticks after the final screenshot request so async readback/save lands.
const FLUSH_TICKS: usize = 16;
const TRAIN_COMPLETION_WAIT_TICKS: usize = 360;
const BUILD_COMPLETION_WAIT_TICKS: usize = 900;
const ASSAULT_DEFAULT_MAX_SECONDS: u32 = 240;
const ASSAULT_TARGET_ARMY_UNITS: usize = 12;
const ASSAULT_TRAIN_PRODUCT: &str = "HeavyMachinegunTrooper";
const ASSAULT_RETARGET_TICKS: usize = 450;

fn main() {
    let mut args = env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("screenshot") | None => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/capture/still.png"));
            render_still(&path)
        }
        Some("frames") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/capture"));
            let count = args.next().and_then(|s| s.parse().ok()).unwrap_or(450usize);
            render_frames(&dir, count)
        }
        Some("play") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/play"));
            render_play(&dir)
        }
        Some("menu") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/menu.png"));
            render_menu_at(&path, WIDTH, HEIGHT)
        }
        Some("menu-wide") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/menu-wide.png"));
            render_menu_at(&path, WIDE_MENU_WIDTH, WIDE_MENU_HEIGHT)
        }
        Some("menu-return") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/menu-return.png"));
            render_menu_return(&path)
        }
        Some("menu-options") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/options.png"));
            render_menu_page(&path, capture_show_options_menu)
        }
        Some("menu-credits") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/credits.png"));
            render_menu_page(&path, capture_show_credits_menu)
        }
        Some("menu-setup") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/setup.png"));
            render_menu_page(&path, capture_show_skirmish_setup_menu)
        }
        Some("menu-dropdown") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/menu/dropdown.png"));
            render_menu_page(&path, capture_show_skirmish_setup_with_dropdown)
        }
        Some("harvest") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/harvest"));
            render_harvest(&dir)
        }
        Some("assault") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/assault"));
            let max_seconds = args
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(ASSAULT_DEFAULT_MAX_SECONDS);
            render_assault(&dir, max_seconds)
        }
        Some("match") => {
            let max_seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
            run_match_proof(max_seconds)
        }
        Some("ai-duel") => match (args.next(), args.next()) {
            (Some(a), Some(b)) => {
                let max_seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(600);
                run_ai_duel(&a, &b, max_seconds)
            }
            _ => Err("ai-duel needs <difficultyA> <difficultyB> [max-seconds]".into()),
        },
        Some("verify") => verify_click_alignment(),
        Some("base") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/base/base.png"));
            render_base_selection(&path)
        }
        Some("resources") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/resources/resources.png"));
            render_resources_closeup(&path)
        }
        Some("factions") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/factions"));
            render_factions(&dir)
        }
        Some("model-harness") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/model-harness"));
            let per_page = args
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(MODEL_HARNESS_DEFAULT_PER_PAGE);
            let page = args.next().and_then(|s| s.parse().ok());
            render_model_harness(&dir, per_page, page)
        }
        Some(other) => Err(format!(
            "unknown command '{other}'. Use: capture [screenshot <path> | frames <dir> <count> | menu <path> | menu-wide <path> | menu-return <path> | menu-options <path> | menu-credits <path> | menu-setup <path> | play <dir> | harvest <dir> | assault <dir> <seconds> | match <seconds> | factions <dir> | model-harness <dir> [per-page] [page] | verify]"
        )),
    };
    if let Err(error) = result {
        eprintln!("[capture] error: {error}");
        std::process::exit(1);
    }
}

/// Non-self-referential click harness: confirms each entity's VISIBLE model sits
/// on its logical origin (so gizmos + hit-tests, which use the origin, land on the
/// model), then clicks a resource where it is actually DRAWN and requires the
/// click to select it. This is the check that would have caught the off-origin
/// GLB bug (the old harvest "proof" clicked world_to_viewport(origin) and tested
/// against the same point, so it could never see the model rendered elsewhere).
fn verify_click_alignment() -> Result<(), String> {
    let mut app = start_match_app();
    // Let scenes finish streaming and the recenter settle window elapse.
    for _ in 0..90 {
        app.update();
    }

    let (offset, label) =
        capture_worst_model_alignment_offset(&mut app).ok_or("no models with AABBs loaded")?;
    println!("[verify] worst model-vs-origin offset: {offset:.3}m ({label})");
    const MAX_OFFSET_M: f32 = 0.3;
    if offset > MAX_OFFSET_M {
        return Err(format!(
            "visible model is {offset:.2}m off the entity origin ({label}) — gizmos/clicks miss the model (limit {MAX_OFFSET_M}m)"
        ));
    }

    // Frame a resource so the end-to-end click test has one on-screen.
    if let Some(resource_pos) = capture_nearest_visible_resource_position(&mut app) {
        capture_focus_camera_on(&mut app, resource_pos);
        for _ in 0..20 {
            app.update();
        }
    }
    let (entity, center) = capture_onscreen_resource_model_center(&mut app)
        .ok_or("no on-screen resource model found")?;
    let screen = capture_world_to_screen(&mut app, center).ok_or("resource model offscreen")?;
    capture_set_cursor(&mut app, screen);
    capture_mouse_button(&mut app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(&mut app, MouseButton::Left, false);
    for _ in 0..3 {
        app.update();
    }
    let selected = capture_entity_is_selected(&mut app, entity);
    println!(
        "[verify] clicked visible resource model @ ({:.0},{:.0}): selected={selected}",
        screen.x, screen.y
    );
    if !selected {
        return Err("left-clicking the visible resource model did not select it".into());
    }

    println!("[verify] OK: models aligned to origin; the visible resource is clickable");
    Ok(())
}

fn run_ai_duel(a: &str, b: &str, max_seconds: u32) -> Result<(), String> {
    let (seconds, label, counts) = capture_run_ai_duel(a, b, max_seconds)?;
    // AI-vs-AI ends as MatchFinished (no human perspective), so infer the winner
    // from who is left standing: most structures, units as the tie-break.
    let winner = if label == "Running" {
        "unresolved (time limit)".to_string()
    } else {
        counts
            .iter()
            .max_by_key(|(_, units, structures)| (*structures, *units))
            .map(|(team, _, _)| {
                let name = if *team == 0 { a } else { b };
                format!("player{team} ({name})")
            })
            .unwrap_or_else(|| "unresolved".to_string())
    };
    println!("[capture] ai-duel {a} vs {b}: {label} at ~{seconds}s -> winner: {winner}");
    for (team, units, structures) in &counts {
        println!("[capture]   player{team}: {units} units, {structures} structures alive");
    }
    Ok(())
}

fn run_match_proof(max_seconds: u32) -> Result<(), String> {
    match capture_run_ai_match_until_resolved(max_seconds) {
        Some((seconds, phase)) => {
            println!("[capture] AI-vs-AI resolved at ~{seconds}s: {phase}");
            Ok(())
        }
        None => Err(format!(
            "AI-vs-AI did not resolve within {max_seconds}s; completed-match loop failed"
        )),
    }
}

/// Builds the offscreen render app, warms up assets, and starts a real match.
fn start_match_app() -> App {
    let mut app = build_capture_app(WIDTH, HEIGHT);
    for _ in 0..WARMUP_TICKS {
        app.update();
    }
    start_shared_match_scene_with_current_setup(&mut app);
    for _ in 0..MATCH_SETTLE_TICKS {
        app.update();
    }
    app
}

fn capture_handle(app: &App) -> Handle<Image> {
    app.world().resource::<CaptureTarget>().0.clone()
}

fn shoot(app: &mut App, handle: &Handle<Image>, path: PathBuf) {
    app.world_mut()
        .spawn(Screenshot::image(handle.clone()))
        .observe(save_to_disk(path));
    for _ in 0..FLUSH_TICKS {
        app.update();
    }
}

fn wait_until(app: &mut App, max_ticks: usize, mut done: impl FnMut(&mut App) -> bool) -> bool {
    if done(app) {
        return true;
    }
    for _ in 0..max_ticks {
        app.update();
        if done(app) {
            return true;
        }
    }
    false
}

fn tap_key(app: &mut App, key: KeyCode) {
    capture_key(app, key, true);
    app.update();
    capture_key(app, key, false);
    app.update();
}

fn select_all_player_army(app: &mut App) {
    capture_key(app, KeyCode::ControlLeft, true);
    capture_key(app, KeyCode::AltLeft, true);
    tap_key(app, KeyCode::KeyA);
    capture_key(app, KeyCode::AltLeft, false);
    capture_key(app, KeyCode::ControlLeft, false);
    app.update();
}

/// Selects a producer and presses its train hotkey via real input; returns true
/// if the command reached the build queue.
fn faction_try_train(app: &mut App) -> bool {
    let Some(pos) = capture_player_producer_position(app) else {
        return false;
    };
    capture_focus_camera_on(app, pos);
    for _ in 0..12 {
        app.update();
    }
    let Some(screen) = capture_world_to_screen(app, pos) else {
        return false;
    };
    capture_set_cursor(app, screen);
    capture_mouse_button(app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(app, MouseButton::Left, false);
    app.update();
    for _ in 0..5 {
        app.update();
    }
    let before = capture_player_build_queue_len(app);
    let Some(key) = capture_first_enabled_train_hotkey(app) else {
        return false;
    };
    capture_key(app, key, true);
    app.update();
    capture_key(app, key, false);
    app.update();
    for _ in 0..3 {
        app.update();
    }
    capture_player_build_queue_len(app) > before
}

/// Selects a worker, enters placement via the build hotkey, and places a
/// structure via real input; returns true if a structure was built.
fn faction_try_build(app: &mut App) -> bool {
    let Some(worker_anchor) = capture_player_worker_position(app) else {
        return false;
    };
    capture_focus_camera_on(app, worker_anchor);
    for _ in 0..12 {
        app.update();
    }
    let Some(worker_pos) = capture_player_onscreen_worker_position(app) else {
        return false;
    };
    let Some(screen) = capture_world_to_screen(app, worker_pos) else {
        return false;
    };
    capture_set_cursor(app, screen);
    capture_mouse_button(app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(app, MouseButton::Left, false);
    app.update();
    for _ in 0..5 {
        app.update();
    }
    let before = capture_player_structure_count(app);
    let Some(key) = capture_first_enabled_build_hotkey(app) else {
        return false;
    };
    capture_key(app, key, true);
    app.update();
    capture_key(app, key, false);
    app.update();
    // Scan a radial grid for a spot the game reports as a VALID placement.
    for radius in [3.0_f32, 4.0, 5.0, 6.0, 7.0, 8.0] {
        for step in 0..12 {
            if !capture_player_in_placement_mode(app) {
                return capture_player_structure_count(app) > before;
            }
            let angle = step as f32 * std::f32::consts::TAU / 12.0;
            let candidate = worker_pos + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            let Some(screen) = capture_world_to_screen(app, candidate) else {
                continue;
            };
            capture_set_cursor(app, screen);
            app.update();
            if !capture_placement_is_valid(app) {
                continue;
            }
            capture_mouse_button(app, MouseButton::Left, true);
            app.update();
            capture_mouse_button(app, MouseButton::Left, false);
            app.update();
            for _ in 0..3 {
                app.update();
            }
            if capture_player_structure_count(app) > before {
                return true;
            }
        }
    }
    capture_player_structure_count(app) > before
}

/// Screenshots the front menu (no match started) so menu UI can be visually
/// verified at multiple desktop sizes.
fn render_menu_at(path: &Path, width: u32, height: u32) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = build_capture_app(width, height);
    for _ in 0..120 {
        app.update();
    }
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, path.to_path_buf());
    println!("[capture] wrote menu screenshot to {}", path.display());
    Ok(())
}

fn render_menu_return(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = build_capture_app(WIDE_MENU_WIDTH, WIDE_MENU_HEIGHT);
    for _ in 0..120 {
        app.update();
    }
    capture_show_skirmish_setup_menu(&mut app);
    for _ in 0..60 {
        app.update();
    }
    capture_show_main_menu(&mut app);
    for _ in 0..60 {
        app.update();
    }
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, path.to_path_buf());
    println!(
        "[capture] wrote menu return screenshot to {}",
        path.display()
    );
    Ok(())
}

fn render_menu_page(path: &Path, enter_page: fn(&mut App)) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = build_capture_app(WIDTH, HEIGHT);
    for _ in 0..120 {
        app.update();
    }
    enter_page(&mut app);
    for _ in 0..60 {
        app.update();
    }
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, path.to_path_buf());
    println!("[capture] wrote menu page screenshot to {}", path.display());
    Ok(())
}

/// Verifies MANUAL harvesting: select a worker, right-click an ore node, and
/// confirm the worker takes a harvest order and the player's ore grows.
fn render_harvest(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app();
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, dir.join("00_start.png"));

    // Pick a real Worker, then center the camera on it so the synthetic click
    // lands in the world instead of on HUD/briefing chrome.
    let worker_anchor = capture_player_worker_position(&mut app).ok_or("no player Worker")?;
    capture_focus_camera_on(&mut app, worker_anchor);
    for _ in 0..12 {
        app.update();
    }
    let worker = capture_player_onscreen_worker_position(&mut app)
        .ok_or("no click-safe on-screen player Worker")?;
    let ore = capture_nearest_visible_resource_click_position_to(&mut app, worker)
        .ok_or("no visible resource node near Worker")?;
    let camera_focus = Vec3::new((worker.x + ore.x) * 0.5, 0.0, (worker.z + ore.z) * 0.5);
    capture_focus_camera_on(&mut app, camera_focus);
    for _ in 0..12 {
        app.update();
    }
    let worker = capture_player_onscreen_worker_position(&mut app)
        .ok_or("player Worker moved out of click-safe area after camera focus")?;
    let ore = capture_nearest_visible_resource_click_position_to(&mut app, worker)
        .ok_or("resource moved out of click-safe area after camera focus")?;

    // Select the worker.
    let worker_screen = capture_world_to_screen(&mut app, worker).ok_or("worker offscreen")?;
    capture_set_cursor(&mut app, worker_screen);
    capture_mouse_button(&mut app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(&mut app, MouseButton::Left, false);
    app.update();
    for _ in 0..3 {
        app.update();
    }
    if let Some(now) = capture_player_worker_position(&mut app) {
        eprintln!(
            "[capture][diag] worker drift since click target: {:.3}m (was {:.2},{:.2} now {:.2},{:.2})",
            ((now.x - worker.x).powi(2) + (now.z - worker.z).powi(2)).sqrt(),
            worker.x,
            worker.z,
            now.x,
            now.z
        );
    }
    let selected_ids = capture_selected_player_unit_ids(&mut app);
    println!(
        "[capture] selected {} player unit(s): {}",
        selected_ids.len(),
        selected_ids.join(", ")
    );
    if !selected_ids.iter().any(|id| *id == "Worker") {
        shoot(&mut app, &handle, dir.join("01_selection_failed.png"));
        return Err(format!(
            "manual harvest selected {:?}, expected Worker",
            selected_ids
        ));
    }
    let (ore_before, crystal_before) = capture_player_resources(&mut app);

    // Right-click the ore node to harvest. Center the camera on the ore first: the
    // selected worker's multi-row build card (bottom-right) can genuinely cover the
    // ore's previous screen position, and clicks on real HUD are (correctly)
    // swallowed. Selection survives the camera move.
    capture_focus_camera_on(&mut app, Vec3::new(ore.x, 0.0, ore.z));
    for _ in 0..12 {
        app.update();
    }
    let ore_screen = capture_world_to_screen(&mut app, ore).ok_or("ore offscreen")?;
    capture_set_cursor(&mut app, ore_screen);
    capture_mouse_button(&mut app, MouseButton::Right, true);
    app.update();
    capture_mouse_button(&mut app, MouseButton::Right, false);
    app.update();
    for _ in 0..3 {
        app.update();
    }
    let harvesting = capture_player_harvesting_count(&mut app);
    println!("[capture] after right-click ore: {harvesting} player unit(s) harvesting");
    shoot(&mut app, &handle, dir.join("01_harvest_order.png"));
    if harvesting == 0 {
        return Err("Worker right-clicked ore but no HarvestOrder was issued".into());
    }

    // Let it gather and deposit; watch ore grow.
    for _ in 0..600 {
        app.update();
    }
    let (ore_after, crystal_after) = capture_player_resources(&mut app);
    println!(
        "[capture] harvest resources: ore {ore_before} -> {ore_after}, crystal {crystal_before} -> {crystal_after}"
    );
    shoot(&mut app, &handle, dir.join("02_after_gather.png"));
    if ore_after + crystal_after <= ore_before + crystal_before {
        return Err(format!(
            "Worker harvested but player resources did not grow: ore {ore_before}->{ore_after}, crystal {crystal_before}->{crystal_after}"
        ));
    }
    println!(
        "[capture] wrote manual-harvest verification to {}",
        dir.display()
    );
    Ok(())
}

/// Captures each of the 3 factions' starting base so their distinct units and
/// structures can be visually verified (no missing / fallback models).
fn render_factions(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for index in 0..3 {
        let mut app = build_capture_app(WIDTH, HEIGHT);
        for _ in 0..WARMUP_TICKS {
            app.update();
        }
        let label = capture_set_all_factions(&mut app, index);
        start_shared_match_scene_with_current_setup(&mut app);
        for _ in 0..MATCH_SETTLE_TICKS {
            app.update();
        }
        let handle = capture_handle(&app);
        shoot(&mut app, &handle, dir.join(format!("faction_{index}.png")));

        // Verify the human train + build input path works for THIS faction
        // using the real default-start economy.
        let trained = faction_try_train(&mut app);
        let built = faction_try_build(&mut app);
        shoot(
            &mut app,
            &handle,
            dir.join(format!("faction_{index}_built.png")),
        );
        println!(
            "[capture] faction {index} ({label}): base ok, human train={trained}, build={built}"
        );
        if !trained || !built {
            return Err(format!(
                "faction {index} ({label}) input smoke failed: train={trained}, build={built}"
            ));
        }
    }
    println!("[capture] wrote faction bases to {}", dir.display());
    Ok(())
}

fn render_model_harness(dir: &Path, per_page: usize, page: Option<usize>) -> Result<(), String> {
    let per_page = per_page.max(1);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    write_model_harness_manifest(dir, per_page)?;
    let page_count = capture_model_harness_page_count(per_page);
    if page.is_some_and(|page| page >= page_count) {
        return Err(format!(
            "model harness page {} out of range 0..{}",
            page.unwrap(),
            page_count.saturating_sub(1)
        ));
    }
    let pages: Vec<usize> = page
        .map(|page| vec![page])
        .unwrap_or_else(|| (0..page_count).collect());
    for page in pages {
        let mut app = build_model_harness_capture_app(MODEL_HARNESS_WIDTH, MODEL_HARNESS_HEIGHT);
        let slots = capture_spawn_model_harness_page(&mut app, page, per_page);
        if slots.is_empty() {
            return Err(format!("model harness page {page} has no slots"));
        }
        for _ in 0..MODEL_HARNESS_SETTLE_TICKS {
            app.update();
        }
        let handle = capture_handle(&app);
        let path = dir.join(format!("page_{page:02}.png"));
        shoot(&mut app, &handle, path.clone());
        println!(
            "[capture] model harness page {}/{}: {} slots -> {}",
            page + 1,
            page_count,
            slots.len(),
            path.display()
        );
    }
    println!(
        "[capture] wrote model harness manifest to {}",
        dir.join("manifest.md").display()
    );
    Ok(())
}

fn write_model_harness_manifest(dir: &Path, per_page: usize) -> Result<(), String> {
    let mut lines = vec![
        "# Model Harness Manifest".to_string(),
        String::new(),
        format!("- Per page: {per_page}"),
        format!("- Pages: {}", capture_model_harness_page_count(per_page)),
        String::new(),
        "| Index | Page | Cell | Entity | Label | Role | Parts | Models | Screenshot |".to_string(),
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for slot in capture_model_harness_manifest(per_page) {
        lines.push(format!(
            "| {} | {} | r{} c{} | `{}` | {} | {} | {} | {} | page_{:02}.png |",
            slot.index,
            slot.page,
            slot.row,
            slot.column,
            slot.id,
            slot.label,
            slot.role,
            slot.render_parts,
            slot.model_assets,
            slot.page,
        ));
    }
    std::fs::write(dir.join("manifest.md"), lines.join("\n") + "\n").map_err(|e| e.to_string())
}

/// Drives the human core loop through the REAL mouse-input systems and captures
/// each step, so selection/move can be verified visually (selection ring) and
/// programmatically (Selected count).
fn render_play(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app();
    let handle = capture_handle(&app);

    let unit_anchor = capture_player_onscreen_unit_position(&mut app)
        .or_else(|| capture_player_worker_position(&mut app))
        .ok_or("no player unit found for select/move demo")?;
    capture_focus_camera_on(&mut app, unit_anchor);
    for _ in 0..12 {
        app.update();
    }
    let unit_pos = capture_player_onscreen_unit_position(&mut app)
        .ok_or("no click-safe player unit found for select/move demo")?;
    shoot(&mut app, &handle, dir.join("00_start.png"));

    // SELECT: left-click on the unit's screen position via real input.
    let Some(unit_screen) = capture_world_to_screen(&mut app, unit_pos) else {
        return Err("unit not on screen".into());
    };
    capture_set_cursor(&mut app, unit_screen);
    capture_mouse_button(&mut app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(&mut app, MouseButton::Left, false);
    app.update();
    for _ in 0..3 {
        app.update();
    }
    let selected = capture_selected_player_unit_count(&mut app);
    println!("[capture] after left-click select: {selected} player unit(s) selected");
    shoot(&mut app, &handle, dir.join("01_selected.png"));
    if selected == 0 {
        return Err("left-click select did not select a player unit".into());
    }

    // MOVE: right-click a ground point partway toward the enemy.
    let direction = capture_enemy_structure_position(&mut app)
        .map(|enemy| Vec3::new(enemy.x - unit_pos.x, 0.0, enemy.z - unit_pos.z))
        .and_then(|direction| direction.try_normalize())
        .unwrap_or(Vec3::X);
    let move_target = unit_pos + direction * 3.5;
    capture_focus_camera_on(&mut app, unit_pos.lerp(move_target, 0.5));
    for _ in 0..4 {
        app.update();
    }
    if let Some(move_screen) = capture_world_to_screen(&mut app, move_target) {
        capture_set_cursor(&mut app, move_screen);
        capture_mouse_button(&mut app, MouseButton::Right, true);
        app.update();
        capture_mouse_button(&mut app, MouseButton::Right, false);
        app.update();
    }
    for _ in 0..90 {
        app.update();
    }
    let moved = capture_selected_player_unit_average_position(&mut app)
        .map(|after| {
            let distance = after.distance(unit_pos);
            println!("[capture] move delta: {distance:.2}m");
            distance
        })
        .unwrap_or(0.0);
    shoot(&mut app, &handle, dir.join("02_moved.png"));
    if moved < 0.35 {
        return Err(format!(
            "right-click move did not move selected unit ({moved:.2}m)"
        ));
    }

    // TRAIN: select a production structure, press the train hotkey via real
    // keyboard input, then wait for the unit to actually spawn.
    let mut trained = false;
    if let Some(producer_pos) = capture_player_producer_position(&mut app)
        && let Some(producer_screen) = capture_world_to_screen(&mut app, producer_pos)
    {
        capture_set_cursor(&mut app, producer_screen);
        capture_mouse_button(&mut app, MouseButton::Left, true);
        app.update();
        capture_mouse_button(&mut app, MouseButton::Left, false);
        app.update();
        for _ in 0..5 {
            app.update();
        }
        let queue_before = capture_player_build_queue_len(&mut app);
        let units_before = capture_player_unit_count(&mut app);
        match capture_first_enabled_train_hotkey(&mut app) {
            Some(key) => {
                capture_key(&mut app, key, true);
                app.update();
                capture_key(&mut app, key, false);
                app.update();
                for _ in 0..3 {
                    app.update();
                }
                let queue_after = capture_player_build_queue_len(&mut app);
                println!(
                    "[capture] train hotkey {key:?}: player build queue {queue_before} -> {queue_after}"
                );
                if queue_after > queue_before {
                    trained = wait_until(&mut app, TRAIN_COMPLETION_WAIT_TICKS, |app| {
                        capture_player_unit_count(app) > units_before
                    });
                    let units_after = capture_player_unit_count(&mut app);
                    println!(
                        "[capture] train completed: player units {units_before} -> {units_after}"
                    );
                }
            }
            None => println!("[capture] no enabled train hotkey on the command panel"),
        }
    }
    shoot(&mut app, &handle, dir.join("03_trained.png"));
    if !trained {
        return Err("train hotkey did not produce a completed player unit".into());
    }

    // BUILD: use the real default-start resources, select a worker, enter
    // placement via the build hotkey, and left-click a ground spot.
    let mut built = false;
    if let Some(worker_anchor) = capture_player_worker_position(&mut app) {
        capture_focus_camera_on(&mut app, worker_anchor);
        for _ in 0..12 {
            app.update();
        }
    }
    if let Some(worker_pos) = capture_player_onscreen_worker_position(&mut app)
        && let Some(worker_screen) = capture_world_to_screen(&mut app, worker_pos)
    {
        capture_set_cursor(&mut app, worker_screen);
        capture_mouse_button(&mut app, MouseButton::Left, true);
        app.update();
        capture_mouse_button(&mut app, MouseButton::Left, false);
        app.update();
        for _ in 0..5 {
            app.update();
        }
        let sel = capture_selected_player_unit_count(&mut app);
        let (opts_enabled, opts_total) = capture_build_options_count(&mut app);
        println!(
            "[capture] build menu: {sel} unit(s) selected, build options {opts_enabled} enabled / {opts_total} total"
        );
        let structures_before = capture_player_structure_count(&mut app);
        let completed_before = capture_player_completed_structure_count(&mut app);
        match capture_first_enabled_build_hotkey(&mut app) {
            Some(key) => {
                capture_key(&mut app, key, true);
                app.update();
                capture_key(&mut app, key, false);
                app.update();
                let placement = capture_player_in_placement_mode(&mut app);
                println!("[capture] build hotkey {key:?}: placement mode = {placement}");
                // The base is crowded; scan a grid for a VALID spot (checking the
                // game's own placement feedback) before committing the click.
                let mut placed_at = None;
                'search: for radius in [3.0_f32, 4.0, 5.0, 6.0, 7.0, 8.0] {
                    for step in 0..12 {
                        if !capture_player_in_placement_mode(&mut app) {
                            break 'search;
                        }
                        let angle = step as f32 * std::f32::consts::TAU / 12.0;
                        let candidate =
                            worker_pos + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                        let Some(screen) = capture_world_to_screen(&mut app, candidate) else {
                            continue;
                        };
                        capture_set_cursor(&mut app, screen);
                        app.update();
                        if !capture_placement_is_valid(&mut app) {
                            continue;
                        }
                        capture_mouse_button(&mut app, MouseButton::Left, true);
                        app.update();
                        capture_mouse_button(&mut app, MouseButton::Left, false);
                        app.update();
                        for _ in 0..3 {
                            app.update();
                        }
                        if capture_player_structure_count(&mut app) > structures_before {
                            placed_at = Some(candidate);
                            break 'search;
                        }
                    }
                }
                if let Some(at) = placed_at {
                    println!("[capture] placed structure at ({:.1},{:.1})", at.x, at.z);
                }
                let structures_after = capture_player_structure_count(&mut app);
                let constructing_after = capture_player_constructing_count(&mut app);
                println!(
                    "[capture] build: player structures {structures_before} -> {structures_after}, constructors active {constructing_after}"
                );
                if structures_after > structures_before {
                    built = wait_until(&mut app, BUILD_COMPLETION_WAIT_TICKS, |app| {
                        capture_player_completed_structure_count(app) > completed_before
                    });
                    let completed_after = capture_player_completed_structure_count(&mut app);
                    println!(
                        "[capture] construction completed: player completed structures {completed_before} -> {completed_after}"
                    );
                }
            }
            None => println!("[capture] no enabled build hotkey on the command panel"),
        }
    }
    shoot(&mut app, &handle, dir.join("04_built.png"));
    if !built {
        return Err("build hotkey did not complete a player structure".into());
    }

    println!(
        "[capture] wrote select/move/train/build playthrough to {}",
        dir.display()
    );
    Ok(())
}

fn render_assault(dir: &Path, max_seconds: u32) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app();
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, dir.join("00_start.png"));

    let barracks_pos = build_barracks_via_real_input(&mut app)?;
    shoot(&mut app, &handle, dir.join("01_barracks.png"));
    let army_count = train_barracks_army_via_real_input(
        &mut app,
        barracks_pos,
        ASSAULT_TARGET_ARMY_UNITS,
        ASSAULT_TRAIN_PRODUCT,
    )?;
    shoot(&mut app, &handle, dir.join("02_army_ready.png"));

    let mut assault_orders = 0usize;
    let (selected, attack_key, combat_orders) =
        issue_player_attack_move_to_nearest_anchor(&mut app)?;
    assault_orders += 1;
    println!(
        "[capture] assault order: army {army_count}, selected {selected}, attack key {attack_key:?}, combat orders {combat_orders}"
    );
    shoot(&mut app, &handle, dir.join("03_attack_order.png"));
    if combat_orders == 0 {
        return Err("Attack-Move input did not create any player combat orders".into());
    }

    let max_ticks = max_seconds as usize * 30;
    // Keep the camera on the fight and grab several combat frames so at least one
    // catches active tracers/impacts/health bars (not the empty home base).
    let combat_focus = capture_nearest_enemy_anchor_position(&mut app);
    let combat_shot_ticks = [180usize, 360, 540, 720, 1080];
    let mut combat_shot = 0usize;
    let mut resolved = false;
    let mut since_retarget = 0usize;
    for tick in 0..max_ticks {
        app.update();
        if combat_shot_ticks.contains(&tick) {
            if let Some(focus) = combat_focus {
                capture_focus_camera_on(&mut app, focus);
            }
            combat_shot += 1;
            shoot(
                &mut app,
                &handle,
                dir.join(format!("03b_combat_{combat_shot}.png")),
            );
        }
        if capture_match_phase_label(&mut app) != "Running" {
            resolved = true;
            break;
        }
        since_retarget += 1;
        if since_retarget >= ASSAULT_RETARGET_TICKS {
            let (_, _, orders) = issue_player_attack_move_to_nearest_anchor(&mut app)?;
            assault_orders += 1;
            println!("[capture] assault retarget {assault_orders}: combat orders {orders}");
            since_retarget = 0;
        }
    }
    let phase = capture_match_phase_label(&mut app);
    println!(
        "[capture] assault result after <= {max_seconds}s: {phase}, attack orders issued {assault_orders}"
    );
    shoot(&mut app, &handle, dir.join("04_result.png"));
    if !resolved {
        return Err(format!(
            "player assault did not resolve within {max_seconds}s; phase stayed {phase}"
        ));
    }
    if phase != "HumanVictory" {
        return Err(format!(
            "player assault resolved as {phase}, expected HumanVictory"
        ));
    }

    println!(
        "[capture] wrote player assault verification to {}",
        dir.display()
    );
    Ok(())
}

fn issue_player_attack_move_to_nearest_anchor(
    app: &mut App,
) -> Result<(usize, KeyCode, usize), String> {
    select_all_player_army(app);
    for _ in 0..12 {
        app.update();
    }
    let selected = capture_selected_player_unit_count(app);
    if selected == 0 {
        return Err("Ctrl+Alt+A did not select any player army units".into());
    }

    let Some(attack_key) = capture_first_enabled_attack_move_hotkey(app) else {
        return Err("selected army does not expose an enabled Attack-Move command".into());
    };
    tap_key(app, attack_key);

    let target =
        capture_nearest_enemy_anchor_position(app).ok_or("no living enemy anchor target")?;
    capture_focus_camera_on(app, target);
    for _ in 0..20 {
        app.update();
    }
    let target_screen =
        capture_world_to_screen(app, target).ok_or("enemy anchor target offscreen")?;
    capture_set_cursor(app, target_screen);
    capture_mouse_button(app, MouseButton::Right, true);
    app.update();
    capture_mouse_button(app, MouseButton::Right, false);
    for _ in 0..30 {
        app.update();
    }

    Ok((selected, attack_key, capture_player_combat_order_count(app)))
}

fn build_barracks_via_real_input(app: &mut App) -> Result<Vec3, String> {
    let worker = capture_player_worker_position(app).ok_or("no player Worker to build Barracks")?;
    capture_focus_camera_on(app, worker);
    for _ in 0..12 {
        app.update();
    }
    let worker_screen = capture_world_to_screen(app, worker).ok_or("worker offscreen")?;
    capture_set_cursor(app, worker_screen);
    capture_mouse_button(app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(app, MouseButton::Left, false);
    for _ in 0..12 {
        app.update();
    }

    let Some(build_key) = capture_enabled_build_hotkey_for(app, "Barracks") else {
        return Err("selected Worker does not expose an enabled Barracks build command".into());
    };
    tap_key(app, build_key);
    if !capture_player_in_placement_mode(app) {
        return Err("Barracks build hotkey did not enter placement mode".into());
    }

    let structures_before = capture_player_structure_count(app);
    let placement_offsets = [
        Vec3::new(4.0, 0.0, 2.5),
        Vec3::new(4.5, 0.0, -3.5),
        Vec3::new(-4.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(6.0, 0.0, 0.0),
    ];
    let mut placed_at = None;
    for offset in placement_offsets {
        let candidate = worker + offset;
        capture_focus_camera_on(app, candidate);
        for _ in 0..8 {
            app.update();
        }
        let Some(screen) = capture_world_to_screen(app, candidate) else {
            continue;
        };
        capture_set_cursor(app, screen);
        for _ in 0..6 {
            app.update();
        }
        if !capture_placement_is_valid(app) {
            continue;
        }
        capture_mouse_button(app, MouseButton::Left, true);
        app.update();
        capture_mouse_button(app, MouseButton::Left, false);
        for _ in 0..20 {
            app.update();
        }
        if capture_player_structure_count(app) > structures_before {
            placed_at = Some(candidate);
            break;
        }
    }
    let placed_at = placed_at.ok_or("could not place Barracks at a valid build location")?;
    println!(
        "[capture] assault Barracks placed at ({:.1},{:.1}) via {build_key:?}",
        placed_at.x, placed_at.z
    );

    let completed = wait_until(app, BUILD_COMPLETION_WAIT_TICKS, |app| {
        capture_player_completed_structure_position(app, "Barracks").is_some()
    });
    if !completed {
        return Err("Barracks foundation was placed but did not complete construction".into());
    }
    let barracks = capture_player_completed_structure_position(app, "Barracks")
        .ok_or("completed Barracks position missing after construction wait")?;
    println!(
        "[capture] assault Barracks completed at ({:.1},{:.1})",
        barracks.x, barracks.z
    );
    Ok(barracks)
}

fn train_barracks_army_via_real_input(
    app: &mut App,
    barracks: Vec3,
    target_army_units: usize,
    product_id: &'static str,
) -> Result<usize, String> {
    capture_focus_camera_on(app, barracks);
    for _ in 0..12 {
        app.update();
    }
    let barracks_screen = capture_world_to_screen(app, barracks).ok_or("Barracks offscreen")?;
    capture_set_cursor(app, barracks_screen);
    capture_mouse_button(app, MouseButton::Left, true);
    app.update();
    capture_mouse_button(app, MouseButton::Left, false);
    for _ in 0..12 {
        app.update();
    }

    let initial_army = capture_player_army_unit_count(app);
    let desired_army = target_army_units.max(initial_army);
    let mut train_inputs = 0usize;
    let mut ticks = 0usize;
    while ticks < 3_600 && capture_player_army_unit_count(app) < desired_army {
        if let Some(train_key) = capture_enabled_train_hotkey_for(app, product_id) {
            tap_key(app, train_key);
            train_inputs += 1;
        }
        for _ in 0..15 {
            app.update();
        }
        ticks += 15;
    }
    let final_army = capture_player_army_unit_count(app);
    println!(
        "[capture] assault trained army: {initial_army} -> {final_army}, product {product_id}, train inputs {train_inputs}"
    );
    if final_army < desired_army {
        return Err(format!(
            "Barracks training only reached {final_army}/{desired_army} combat units"
        ));
    }
    Ok(final_army)
}

fn render_still(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = start_match_app();
    let handle = capture_handle(&app);
    app.world_mut()
        .spawn(Screenshot::image(handle))
        .observe(save_to_disk(path.to_path_buf()));
    for _ in 0..FLUSH_TICKS {
        app.update();
    }
    println!("[capture] wrote {}", path.display());
    Ok(())
}

/// Selects the player's command center (via the real click path) and screenshots
/// it framed, so the selection brackets can be eyeballed against the building.
/// Also asserts the brackets' anchor (entity origin) sits on the building's
/// visible center — the bug the user reported was an offset between the two.
fn render_base_selection(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = start_match_app();

    let (entity, origin, center) =
        capture_player_command_center(&mut app).ok_or("no player command center found")?;
    let offset = ((center.x - origin.x).powi(2) + (center.z - origin.z).powi(2)).sqrt();
    println!(
        "[base] command center origin=({:.2},{:.2}) visual_center=({:.2},{:.2}) offset={offset:.3}m",
        origin.x, origin.z, center.x, center.z
    );

    capture_focus_camera_on(&mut app, center);
    for _ in 0..20 {
        app.update();
    }
    if let Some(screen) = capture_world_to_screen(&mut app, center) {
        capture_set_cursor(&mut app, screen);
        capture_mouse_button(&mut app, MouseButton::Left, true);
        app.update();
        capture_mouse_button(&mut app, MouseButton::Left, false);
        for _ in 0..4 {
            app.update();
        }
        println!(
            "[base] clicked base @ ({:.0},{:.0}): selected={}",
            screen.x,
            screen.y,
            capture_entity_is_selected(&mut app, entity)
        );
    }

    // Plant a rally point so the rally-flag visual is exercised in the screenshot.
    capture_set_structure_rally(
        &mut app,
        entity,
        origin + bevy::prelude::Vec3::new(6.0, 0.0, 8.0),
    );
    for _ in 0..4 {
        app.update();
    }

    let handle = capture_handle(&app);
    app.world_mut()
        .spawn(Screenshot::image(handle))
        .observe(save_to_disk(path.to_path_buf()));
    for _ in 0..FLUSH_TICKS {
        app.update();
    }
    if offset > 0.3 {
        return Err(format!(
            "selection brackets are {offset:.2}m off the base model (limit 0.30m)"
        ));
    }
    println!("[capture] wrote {}", path.display());
    Ok(())
}

/// Frames the resource deposits (and a nearby worker) close up so the model +
/// crystal-tint fidelity can be eyeballed against godot's originals.
fn render_resources_closeup(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = start_match_app();
    let focus = capture_nearest_visible_resource_position(&mut app)
        .or_else(|| capture_player_worker_position(&mut app))
        .ok_or("no resource or worker found to frame")?;
    capture_focus_camera_on(&mut app, focus);
    capture_zoom_camera_closest(&mut app);
    for _ in 0..30 {
        app.update();
    }
    let handle = capture_handle(&app);
    app.world_mut()
        .spawn(Screenshot::image(handle))
        .observe(save_to_disk(path.to_path_buf()));
    for _ in 0..FLUSH_TICKS {
        app.update();
    }
    println!("[capture] wrote {}", path.display());
    Ok(())
}

fn render_frames(dir: &Path, count: usize) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut app = start_match_app();
    // Give the camera real action to film: send the player army at the enemy
    // base instead of waiting out the AI's opening grace.
    capture_player_attack_move_all(&mut app);
    let handle = capture_handle(&app);
    for i in 0..count {
        app.update();
        let path = dir.join(format!("frame{i:05}.png"));
        app.world_mut()
            .spawn(Screenshot::image(handle.clone()))
            .observe(save_to_disk(path));
    }
    for _ in 0..FLUSH_TICKS {
        app.update();
    }
    println!("[capture] wrote {count} frames to {}", dir.display());
    Ok(())
}
