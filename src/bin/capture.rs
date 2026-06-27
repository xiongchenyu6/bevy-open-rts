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
//!   capture match [max-seconds]          headless AI-vs-AI match must resolve
//!   capture menu [path]                  lobby/setup screenshot
//!   capture factions <dir>               faction base/build smoke screenshots

use std::env;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use bevy_open_rts::{
    CaptureTarget, build_capture_app, capture_build_options_count,
    capture_enemy_structure_position, capture_first_enabled_build_hotkey,
    capture_first_enabled_train_hotkey, capture_focus_camera_on, capture_key, capture_mouse_button,
    capture_nearest_visible_resource_click_position_to, capture_placement_is_valid,
    capture_player_attack_move_all, capture_player_build_queue_len,
    capture_player_completed_structure_count, capture_player_constructing_count,
    capture_player_harvesting_count, capture_player_in_placement_mode,
    capture_player_onscreen_unit_position, capture_player_onscreen_worker_position,
    capture_player_producer_position, capture_player_resources, capture_player_structure_count,
    capture_player_unit_count, capture_player_worker_position, capture_run_ai_match_until_resolved,
    capture_selected_player_unit_average_position, capture_selected_player_unit_count,
    capture_selected_player_unit_ids, capture_set_all_factions, capture_set_cursor,
    capture_world_to_screen, start_shared_match_scene_with_current_setup,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
/// Ticks to let assets load and the menu initialize before starting a match.
const WARMUP_TICKS: usize = 90;
/// Ticks to let the match scene populate (bases, units) before first capture.
const MATCH_SETTLE_TICKS: usize = 60;
/// Extra ticks after the final screenshot request so async readback/save lands.
const FLUSH_TICKS: usize = 16;
const TRAIN_COMPLETION_WAIT_TICKS: usize = 360;
const BUILD_COMPLETION_WAIT_TICKS: usize = 900;

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
            render_menu(&path)
        }
        Some("harvest") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/harvest"));
            render_harvest(&dir)
        }
        Some("match") => {
            let max_seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
            run_match_proof(max_seconds)
        }
        Some("factions") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/factions"));
            render_factions(&dir)
        }
        Some(other) => Err(format!(
            "unknown command '{other}'. Use: capture [screenshot <path> | frames <dir> <count> | play <dir> | harvest <dir> | match <seconds> | factions <dir>]"
        )),
    };
    if let Err(error) = result {
        eprintln!("[capture] error: {error}");
        std::process::exit(1);
    }
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

/// Screenshots the lobby / setup menu (no match started) so menu UI can be
/// visually verified.
fn render_menu(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut app = build_capture_app(WIDTH, HEIGHT);
    for _ in 0..120 {
        app.update();
    }
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, path.to_path_buf());
    println!("[capture] wrote menu screenshot to {}", path.display());
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

    // Right-click the ore node to harvest.
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
