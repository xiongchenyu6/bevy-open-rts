use std::{
    env,
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use bevy_open_rts::{
    CaptureEntityKind, CaptureMatchPhase, CaptureMatchSnapshot, CapturePlayableProof,
    CaptureProofFaction, CaptureTeam, CaptureVictoryProof, advance_capture_match,
    advance_capture_match_proof_frame, build_capture_match_app,
    build_capture_match_app_for_faction, capture_match_proof_status, capture_match_snapshot,
    capture_proof_unit_count, run_capture_match_proof_for_faction,
    run_real_default_menu_victory_proof, run_real_menu_ai_pressure_proof_for_faction,
    run_real_menu_allied_victory_proof_for_faction, run_real_menu_build_proof_for_faction,
    run_real_menu_dual_harvest_proof_for_faction, run_real_menu_economy_victory_proof_for_faction,
    run_real_menu_harvest_proof_for_faction, run_real_menu_match_proof_for_faction,
    run_real_menu_playable_proof_for_faction,
    run_real_menu_selected_faction_victory_proof_for_faction,
    run_real_menu_selected_map_victory_proof,
    run_real_menu_three_faction_playable_proof_for_faction,
    run_real_menu_victory_proof_for_faction,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FPS: f32 = 30.0;

#[derive(Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy)]
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

#[derive(Clone, Copy)]
struct CaptureView {
    center: Vec2,
    scale: f32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("[capture] error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => {
            let path = PathBuf::from("screenshots/capture/still.png");
            render_still(&path, 0)?;
            println!("[capture] wrote {}", path.display());
        }
        Some("screenshot") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/capture/still.png"));
            render_still(&path, 0)?;
            println!("[capture] wrote {}", path.display());
        }
        Some("frames") => {
            let directory = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/capture"));
            let count = args
                .next()
                .as_deref()
                .unwrap_or("120")
                .parse::<usize>()
                .map_err(|error| format!("invalid frame count: {error}"))?;
            render_frames(&directory, count)?;
            println!("[capture] wrote {count} frames to {}", directory.display());
        }
        Some("proof-frames") => {
            let directory = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots/capture-proof"));
            let count = args
                .next()
                .as_deref()
                .unwrap_or("900")
                .parse::<usize>()
                .map_err(|error| format!("invalid frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = render_proof_frames(&directory, count, faction)?;
            println!(
                "[capture] proof-frames faction={} label={} product={} phase={:?} frames={} elapsed={}s produced_units={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={} dir={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.product_id,
                proof.phase,
                proof.frames,
                proof.elapsed_seconds,
                proof.produced_units,
                proof.player_units,
                proof.enemy_units_destroyed,
                proof.enemy_structures_destroyed,
                proof.remaining_teams,
                proof.remaining_anchors,
                directory.display()
            );
            if !proof.succeeded() {
                return Err(format!(
                    "proof frames did not reach player victory within {count} frames"
                ));
            }
        }
        Some("match-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("7200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_capture_match_proof_for_faction(faction, max_frames);
            println!(
                "[capture] match-proof faction={} label={} product={} phase={:?} frames={} elapsed={}s produced_units={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.product_id,
                proof.phase,
                proof.frames,
                proof.elapsed_seconds,
                proof.produced_units,
                proof.player_units,
                proof.enemy_units_destroyed,
                proof.enemy_structures_destroyed,
                proof.remaining_teams,
                proof.remaining_anchors
            );
            if !proof.succeeded() {
                return Err(format!(
                    "match proof did not reach player victory within {max_frames} frames"
                ));
            }
        }
        Some("real-match-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("7200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_match_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-match-proof faction={} label={} product={} phase={:?} frames={} elapsed={}s produced_units={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.product_id,
                proof.phase,
                proof.frames,
                proof.elapsed_seconds,
                proof.produced_units,
                proof.player_units,
                proof.enemy_units_destroyed,
                proof.enemy_structures_destroyed,
                proof.remaining_teams,
                proof.remaining_anchors
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu match proof did not reach player victory within {max_frames} frames"
                ));
            }
        }
        Some("real-harvest-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("900")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_harvest_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-harvest-proof faction={} label={} phase={:?} frames={} harvest_ordered={} ore={}->{} resource={}->{} product={} produced_units={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.phase,
                proof.frames,
                proof.harvest_ordered,
                proof.ore_before,
                proof.ore_after,
                proof.resource_before,
                proof.resource_after,
                proof.product_id,
                proof.produced_units
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu harvest proof did not mine and train within {max_frames} frames"
                ));
            }
        }
        Some("real-dual-harvest-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("1800")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_dual_harvest_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-dual-harvest-proof faction={} label={} phase={:?} frames={} ore={}->{} crystal={}->{} harvest_ore={} harvest_crystal={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.phase,
                proof.frames,
                proof.ore_before,
                proof.ore_after,
                proof.crystal_before,
                proof.crystal_after,
                proof.ore_harvest_ordered,
                proof.crystal_harvest_ordered,
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu dual harvest proof did not mine both Ore and Crystal within {max_frames} frames"
                ));
            }
        }
        Some("real-ai-pressure-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("1200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_ai_pressure_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-ai-pressure-proof faction={} label={} phase={:?} frames={} ai_team={:?} ai_units={}=>{} ai_attack_orders={} player_health={:.1}->{:.1}",
                proof.faction.key(),
                proof.faction.label(),
                proof.phase,
                proof.frames,
                proof.ai_team,
                proof.ai_units_before,
                proof.ai_units_peak,
                proof.ai_attack_orders,
                proof.player_health_before,
                proof.player_health_after,
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu AI pressure proof did not produce, attack, and damage the player within {max_frames} frames"
                ));
            }
        }
        Some("real-build-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("900")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_build_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-build-proof faction={} label={} phase={:?} frames={} structure={} placement_started={} placed={} construct_ordered={} constructed={} product={} produced_units={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.phase,
                proof.frames,
                proof.structure_id,
                proof.placement_started,
                proof.placed,
                proof.construct_ordered,
                proof.constructed,
                proof.product_id,
                proof.produced_units
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu build proof did not place, construct, and train within {max_frames} frames"
                ));
            }
        }
        Some("real-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("3600")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_victory_proof_for_faction(faction, max_frames);
            print_victory_proof("real-victory-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real menu victory proof did not train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-default-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("3600")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let proof = run_real_default_menu_victory_proof(max_frames);
            print_victory_proof("real-default-victory-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real default menu victory proof did not train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-selected-faction-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("3600")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof =
                run_real_menu_selected_faction_victory_proof_for_faction(faction, max_frames);
            print_victory_proof("real-selected-faction-victory-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real selected-faction victory proof did not train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-selected-map-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("7200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let map_index = args
                .next()
                .as_deref()
                .unwrap_or("1")
                .parse::<usize>()
                .map_err(|error| format!("invalid map index: {error}"))?;
            let proof = run_real_menu_selected_map_victory_proof(map_index, max_frames);
            print_victory_proof("real-selected-map-victory-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real selected-map victory proof did not train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-allied-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("7200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_allied_victory_proof_for_faction(faction, max_frames);
            print_victory_proof("real-allied-victory-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real allied 2v1 victory proof did not train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-economy-victory-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("3600")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_economy_victory_proof_for_faction(faction, max_frames);
            println!(
                "[capture] real-economy-victory-proof faction={} label={} phase={:?} frames={} ore={}=>{}=>{} crystal={}=>{}=>{} harvest_ore={} harvest_crystal={} product={} target_units={} produced_units={} attack_orders={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={}",
                proof.faction.key(),
                proof.faction.label(),
                proof.phase,
                proof.frames,
                proof.ore_before,
                proof.ore_after_harvest,
                proof.ore_after,
                proof.crystal_before,
                proof.crystal_after_harvest,
                proof.crystal_after,
                proof.ore_harvest_ordered,
                proof.crystal_harvest_ordered,
                proof.product_id,
                proof.target_units,
                proof.produced_units,
                proof.attack_orders,
                proof.player_units,
                proof.enemy_units_destroyed,
                proof.enemy_structures_destroyed,
                proof.remaining_teams,
                proof.remaining_anchors
            );
            if !proof.succeeded() {
                return Err(format!(
                    "real menu economy victory proof did not harvest, train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-playable-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("4200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_playable_proof_for_faction(faction, max_frames);
            print_playable_proof("real-playable-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real menu playable proof did not harvest, build, train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("real-three-faction-playable-proof") => {
            let max_frames = args
                .next()
                .as_deref()
                .unwrap_or("7200")
                .parse::<usize>()
                .map_err(|error| format!("invalid max frame count: {error}"))?;
            let faction = parse_optional_faction(args.next())?;
            let proof = run_real_menu_three_faction_playable_proof_for_faction(faction, max_frames);
            print_playable_proof("real-three-faction-playable-proof", &proof);
            if !proof.succeeded() {
                return Err(format!(
                    "real menu three-faction playable proof did not harvest, build, train, attack, and win within {max_frames} frames"
                ));
            }
        }
        Some("help" | "-h" | "--help") => {
            print_help();
        }
        Some(other) => {
            return Err(format!(
                "unknown command '{other}'. Use: capture [screenshot <path>|frames <dir> <count>]"
            ));
        }
    }
    Ok(())
}

fn print_playable_proof(command: &str, proof: &CapturePlayableProof) {
    println!(
        "[capture] {} faction={} label={} phase={:?} frames={} ore={}=>{}=>{} crystal={}=>{}=>{} harvest_ore={} harvest_crystal={} structure={} placement_started={} placed={} construct_ordered={} constructed={} barracks_product={} barracks_units={} vehicle={} target_units={} produced_units={} attack_orders={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={}",
        command,
        proof.faction.key(),
        proof.faction.label(),
        proof.phase,
        proof.frames,
        proof.ore_before,
        proof.ore_after_harvest,
        proof.ore_after,
        proof.crystal_before,
        proof.crystal_after_harvest,
        proof.crystal_after,
        proof.ore_harvest_ordered,
        proof.crystal_harvest_ordered,
        proof.structure_id,
        proof.placement_started,
        proof.placed,
        proof.construct_ordered,
        proof.constructed,
        proof.barracks_product_id,
        proof.barracks_units,
        proof.vehicle_product_id,
        proof.target_units,
        proof.produced_units,
        proof.attack_orders,
        proof.player_units,
        proof.enemy_units_destroyed,
        proof.enemy_structures_destroyed,
        proof.remaining_teams,
        proof.remaining_anchors
    );
}

fn print_victory_proof(command: &str, proof: &CaptureVictoryProof) {
    println!(
        "[capture] {} faction={} label={} map={} mode={} phase={:?} frames={} product={} target_units={} produced_units={} attack_orders={} player_units={} enemy_kills={} enemy_structures={} remaining_teams={} remaining_anchors={}",
        command,
        proof.faction.key(),
        proof.faction.label(),
        proof.map_id,
        proof.match_mode_id,
        proof.phase,
        proof.frames,
        proof.product_id,
        proof.target_units,
        proof.produced_units,
        proof.attack_orders,
        proof.player_units,
        proof.enemy_units_destroyed,
        proof.enemy_structures_destroyed,
        proof.remaining_teams,
        proof.remaining_anchors
    );
}

fn print_help() {
    println!("Usage:");
    println!("  cargo run --bin capture");
    println!("  cargo run --bin capture -- screenshot screenshots/capture/still.png");
    println!("  cargo run --bin capture -- frames screenshots/result/1 450");
    println!("  cargo run --bin capture -- proof-frames screenshots/result/2 900 human");
    println!("  cargo run --bin capture -- match-proof 7200 human");
    println!("  cargo run --bin capture -- match-proof 7200 demon");
    println!("  cargo run --bin capture -- match-proof 7200 chaos");
    println!("  cargo run --bin capture -- real-match-proof 7200 human");
    println!("  cargo run --bin capture -- real-harvest-proof 900 human");
    println!("  cargo run --bin capture -- real-dual-harvest-proof 1800 human");
    println!("  cargo run --bin capture -- real-ai-pressure-proof 1200 human");
    println!("  cargo run --bin capture -- real-build-proof 900 human");
    println!("  cargo run --bin capture -- real-victory-proof 3600 human");
    println!("  cargo run --bin capture -- real-default-victory-proof 3600");
    println!("  cargo run --bin capture -- real-selected-faction-victory-proof 3600 chaos");
    println!("  cargo run --bin capture -- real-selected-map-victory-proof 7200 1");
    println!("  cargo run --bin capture -- real-allied-victory-proof 7200 demon");
    println!("  cargo run --bin capture -- real-economy-victory-proof 3600 human");
    println!("  cargo run --bin capture -- real-playable-proof 4200 human");
    println!("  cargo run --bin capture -- real-three-faction-playable-proof 7200 human");
}

fn parse_optional_faction(value: Option<String>) -> Result<CaptureProofFaction, String> {
    let Some(value) = value else {
        return Ok(CaptureProofFaction::Human);
    };
    CaptureProofFaction::parse(&value).ok_or_else(|| {
        format!(
            "invalid faction '{value}'. Expected one of: {}",
            CaptureProofFaction::ALL
                .iter()
                .map(|faction| faction.key())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn render_still(path: &Path, frame: usize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let mut app = build_capture_match_app();
    advance_capture_match(&mut app, frame);
    let snapshot = capture_match_snapshot(&mut app);
    let pixels = render_frame(frame, 90, &snapshot, Some(CaptureTeam::Human));
    write_png(path, WIDTH, HEIGHT, &pixels)
}

fn render_frames(directory: &Path, count: usize) -> Result<(), String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let mut app = build_capture_match_app();
    for frame in 0..count {
        if frame > 0 {
            advance_capture_match(&mut app, 1);
        }
        let path = directory.join(format!("frame{frame:05}.png"));
        let snapshot = capture_match_snapshot(&mut app);
        let pixels = render_frame(frame, count.max(1), &snapshot, Some(CaptureTeam::Human));
        write_png(&path, WIDTH, HEIGHT, &pixels)?;
    }
    Ok(())
}

fn render_proof_frames(
    directory: &Path,
    count: usize,
    faction: CaptureProofFaction,
) -> Result<bevy_open_rts::CaptureMatchProof, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let mut app = build_capture_match_app_for_faction(faction);
    let units_before = capture_proof_unit_count(&mut app, faction);
    let mut unit_peak = units_before;
    for frame in 0..count {
        advance_capture_match_proof_frame(&mut app, faction, frame);
        unit_peak = unit_peak.max(capture_proof_unit_count(&mut app, faction));
        let path = directory.join(format!("frame{frame:05}.png"));
        let snapshot = capture_match_snapshot(&mut app);
        let pixels = render_frame(
            frame,
            count.max(1),
            &snapshot,
            Some(capture_team_for_faction(faction)),
        );
        write_png(&path, WIDTH, HEIGHT, &pixels)?;
    }
    Ok(capture_match_proof_status(
        &mut app,
        faction,
        count,
        unit_peak.saturating_sub(units_before),
    ))
}

fn capture_team_for_faction(faction: CaptureProofFaction) -> CaptureTeam {
    match faction {
        CaptureProofFaction::Human => CaptureTeam::Human,
        CaptureProofFaction::Demon => CaptureTeam::Demon,
        CaptureProofFaction::Chaos => CaptureTeam::Chaos,
    }
}

fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder
        .write_header()
        .map_err(|error| format!("could not write PNG header for {}: {error}", path.display()))?;
    png_writer
        .write_image_data(pixels)
        .map_err(|error| format!("could not write PNG data for {}: {error}", path.display()))
}

fn render_frame(
    frame: usize,
    total_frames: usize,
    snapshot: &CaptureMatchSnapshot,
    focus_team: Option<CaptureTeam>,
) -> Vec<u8> {
    let mut pixels = vec![0; (WIDTH * HEIGHT * 4) as usize];
    clear(&mut pixels, Rgba::rgb(20, 31, 35));

    let seconds = frame as f32 / FPS;
    let view = capture_view(snapshot, focus_team);

    draw_ground(&mut pixels, view);
    draw_snapshot(&mut pixels, snapshot, seconds, view, focus_team);
    draw_ui(&mut pixels, frame, total_frames, snapshot);
    pixels
}

fn clear(pixels: &mut [u8], color: Rgba) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = color.r;
        chunk[1] = color.g;
        chunk[2] = color.b;
        chunk[3] = color.a;
    }
}

fn draw_ground(pixels: &mut [u8], view: CaptureView) {
    let horizon = 80;
    fill_rect(
        pixels,
        0,
        horizon,
        WIDTH as i32,
        HEIGHT as i32,
        Rgba::rgb(104, 122, 112),
    );
    fill_rect(pixels, 0, 0, WIDTH as i32, horizon, Rgba::rgb(9, 12, 14));

    let grid_extent = 44;
    for i in -grid_extent..=grid_extent {
        let a = world_to_screen(Vec2::new(i as f32, -grid_extent as f32), view);
        let b = world_to_screen(Vec2::new(i as f32, grid_extent as f32), view);
        draw_line(pixels, a, b, Rgba::rgba(134, 152, 142, 54), 1.0);
        let c = world_to_screen(Vec2::new(-grid_extent as f32, i as f32), view);
        let d = world_to_screen(Vec2::new(grid_extent as f32, i as f32), view);
        draw_line(pixels, c, d, Rgba::rgba(134, 152, 142, 54), 1.0);
    }
}

fn draw_base(pixels: &mut [u8], center: Vec2, color: Rgba, ring: Rgba, view: CaptureView) {
    let screen = world_to_screen(center, view);
    fill_iso_diamond(pixels, screen, 70.0, 34.0, Rgba::rgba(28, 35, 35, 180));
    fill_iso_diamond(pixels, screen, 55.0, 27.0, color);
    fill_iso_diamond(
        pixels,
        Vec2::new(screen.x, screen.y - 18.0),
        31.0,
        17.0,
        Rgba::rgb(231, 236, 237),
    );
    fill_rect(
        pixels,
        (screen.x - 10.0) as i32,
        (screen.y - 45.0) as i32,
        20,
        34,
        Rgba::rgb(178, 190, 194),
    );
    draw_ring(pixels, screen, 66.0, ring);
}

fn draw_snapshot(
    pixels: &mut [u8],
    snapshot: &CaptureMatchSnapshot,
    seconds: f32,
    view: CaptureView,
    focus_team: Option<CaptureTeam>,
) {
    for entity in snapshot
        .entities
        .iter()
        .filter(|entity| entity.kind == CaptureEntityKind::Resource)
    {
        let screen = world_to_screen(Vec2::new(entity.x, entity.z), view);
        let pulse = ((seconds * 3.0 + entity.x * 0.31 + entity.z * 0.17).sin() + 1.0) * 0.5;
        fill_circle(
            pixels,
            screen,
            8.0 + pulse * 3.0,
            Rgba::rgba(83, 196, 221, 210),
        );
        fill_circle(pixels, screen, 4.0, Rgba::rgb(210, 241, 244));
    }

    for entity in snapshot
        .entities
        .iter()
        .filter(|entity| entity.kind == CaptureEntityKind::Structure)
    {
        let (body, ring) = team_colors(entity.team, entity.visible);
        draw_base(pixels, Vec2::new(entity.x, entity.z), body, ring, view);
    }

    for entity in snapshot
        .entities
        .iter()
        .filter(|entity| entity.kind == CaptureEntityKind::Unit)
    {
        let (body, ring) = team_colors(entity.team, entity.visible);
        draw_unit(pixels, Vec2::new(entity.x, entity.z), body, ring, view);
    }

    if let Some(target) = snapshot.entities.iter().find(|entity| {
        focus_team.is_some_and(|focus_team| is_enemy_capture_team(focus_team, entity.team))
            && entity.kind == CaptureEntityKind::Structure
    }) {
        let impact = world_to_screen(Vec2::new(target.x, target.z), view);
        let pulse = ((seconds * 2.0) % 1.0) * 26.0;
        draw_ring(pixels, impact, 22.0 + pulse, Rgba::rgba(255, 88, 64, 150));
    }
}

fn team_colors(team: CaptureTeam, visible: bool) -> (Rgba, Rgba) {
    let alpha = if visible { 230 } else { 95 };
    match team {
        CaptureTeam::Human => (
            Rgba::rgba(216, 226, 229, alpha),
            Rgba::rgba(85, 155, 245, alpha),
        ),
        CaptureTeam::Demon => (
            Rgba::rgba(184, 93, 89, alpha),
            Rgba::rgba(242, 63, 58, alpha),
        ),
        CaptureTeam::Chaos => (
            Rgba::rgba(159, 113, 221, alpha),
            Rgba::rgba(174, 93, 245, alpha),
        ),
        CaptureTeam::Neutral => (
            Rgba::rgba(216, 202, 137, alpha),
            Rgba::rgba(222, 210, 132, alpha),
        ),
    }
}

fn draw_unit(pixels: &mut [u8], world: Vec2, body: Rgba, ring: Rgba, view: CaptureView) {
    let screen = world_to_screen(world, view);
    draw_ring(pixels, screen, 19.0, ring);
    fill_iso_diamond(
        pixels,
        Vec2::new(screen.x + 5.0, screen.y + 10.0),
        23.0,
        9.0,
        Rgba::rgba(20, 28, 28, 120),
    );
    fill_rect(
        pixels,
        (screen.x - 9.0) as i32,
        (screen.y - 9.0) as i32,
        18,
        18,
        body,
    );
    fill_rect(
        pixels,
        (screen.x + 5.0) as i32,
        (screen.y - 3.0) as i32,
        18,
        5,
        Rgba::rgb(42, 48, 49),
    );
}

fn draw_ui(pixels: &mut [u8], frame: usize, total_frames: usize, snapshot: &CaptureMatchSnapshot) {
    fill_rect(pixels, 0, 0, WIDTH as i32, 43, Rgba::rgba(14, 20, 23, 218));
    draw_match_status(pixels, snapshot);
    fill_rect(
        pixels,
        0,
        HEIGHT as i32 - 86,
        WIDTH as i32,
        86,
        Rgba::rgba(17, 27, 30, 220),
    );
    for index in 0..16 {
        fill_rect(
            pixels,
            16 + index * 76,
            HEIGHT as i32 - 70,
            68,
            46,
            Rgba::rgba(58, 73, 74, 235),
        );
    }
    fill_rect(
        pixels,
        WIDTH as i32 - 210,
        HEIGHT as i32 - 210,
        178,
        178,
        Rgba::rgba(9, 20, 21, 230),
    );
    let marker_x =
        WIDTH as i32 - 185 + ((frame as f32 / total_frames.max(1) as f32) * 112.0) as i32;
    fill_rect(
        pixels,
        marker_x,
        HEIGHT as i32 - 112,
        12,
        12,
        Rgba::rgb(101, 168, 245),
    );
}

fn draw_match_status(pixels: &mut [u8], snapshot: &CaptureMatchSnapshot) {
    let phase_color = match snapshot.phase {
        CaptureMatchPhase::Running => Rgba::rgb(95, 196, 142),
        CaptureMatchPhase::HumanVictory => Rgba::rgb(104, 178, 255),
        CaptureMatchPhase::HumanDefeat => Rgba::rgb(230, 82, 74),
        CaptureMatchPhase::MatchFinished => Rgba::rgb(214, 192, 112),
    };
    fill_rect(pixels, 12, 11, 8, 21, phase_color);

    let time_fill = ((snapshot.elapsed_seconds / 180.0).clamp(0.0, 1.0) * 170.0) as i32;
    fill_rect(pixels, 28, 11, 174, 7, Rgba::rgba(62, 78, 82, 230));
    fill_rect(pixels, 30, 13, time_fill, 3, Rgba::rgb(126, 201, 232));

    let anchors_fill = (snapshot.remaining_anchors.min(8) as i32 * 17).max(2);
    fill_rect(pixels, 28, 25, 140, 6, Rgba::rgba(62, 78, 82, 230));
    fill_rect(pixels, 30, 27, anchors_fill, 2, Rgba::rgb(221, 210, 138));

    let teams_fill = (snapshot.remaining_teams.min(3) as i32 * 28).max(2);
    fill_rect(pixels, 176, 25, 86, 6, Rgba::rgba(62, 78, 82, 230));
    fill_rect(pixels, 178, 27, teams_fill, 2, Rgba::rgb(156, 227, 181));

    draw_team_status_row(
        pixels,
        292,
        snapshot.human.units,
        snapshot.human.structures,
        snapshot.human.ore + snapshot.human.crystal,
        Rgba::rgb(85, 155, 245),
    );
    draw_team_status_row(
        pixels,
        508,
        snapshot.demon.units,
        snapshot.demon.structures,
        snapshot.demon.ore + snapshot.demon.crystal,
        Rgba::rgb(242, 63, 58),
    );
    draw_team_status_row(
        pixels,
        724,
        snapshot.chaos.units,
        snapshot.chaos.structures,
        snapshot.chaos.ore + snapshot.chaos.crystal,
        Rgba::rgb(174, 93, 245),
    );

    let destroyed_width =
        ((snapshot.enemy_units_destroyed + snapshot.enemy_structures_destroyed).min(24) as i32 * 5)
            .max(2);
    fill_rect(pixels, 948, 18, 124, 8, Rgba::rgba(62, 78, 82, 230));
    fill_rect(pixels, 950, 20, destroyed_width, 4, Rgba::rgb(244, 147, 90));
}

fn draw_team_status_row(
    pixels: &mut [u8],
    x: i32,
    units: u32,
    structures: u32,
    resources: i32,
    color: Rgba,
) {
    fill_rect(pixels, x, 11, 176, 22, Rgba::rgba(34, 47, 50, 210));
    fill_rect(pixels, x + 7, 16, 8, 8, color);
    let unit_width = (units.min(24) as i32 * 4).max(2);
    let structure_width = (structures.min(12) as i32 * 7).max(2);
    let resource_width = ((resources.max(0).min(360) as f32 / 360.0) * 38.0) as i32;
    fill_rect(pixels, x + 22, 15, 98, 4, Rgba::rgba(74, 89, 92, 230));
    fill_rect(pixels, x + 22, 15, unit_width, 4, color);
    fill_rect(pixels, x + 22, 24, 98, 4, Rgba::rgba(74, 89, 92, 230));
    fill_rect(
        pixels,
        x + 22,
        24,
        structure_width,
        4,
        Rgba::rgb(217, 222, 214),
    );
    fill_rect(pixels, x + 128, 18, 38, 7, Rgba::rgba(74, 89, 92, 230));
    fill_rect(
        pixels,
        x + 128,
        18,
        resource_width,
        7,
        Rgba::rgb(94, 219, 204),
    );
}

fn capture_view(snapshot: &CaptureMatchSnapshot, focus_team: Option<CaptureTeam>) -> CaptureView {
    let Some(focus_team) = focus_team else {
        return CaptureView {
            center: Vec2::new(0.0, 0.0),
            scale: 28.0,
        };
    };
    let focus_points = snapshot
        .entities
        .iter()
        .filter(|entity| {
            entity.team == focus_team
                && matches!(
                    entity.kind,
                    CaptureEntityKind::Unit | CaptureEntityKind::Structure
                )
        })
        .map(entity_position)
        .collect::<Vec<_>>();
    let mut center = average_point(&focus_points).unwrap_or(Vec2::new(0.0, 0.0));
    if let Some(enemy) = nearest_enemy_point(snapshot, focus_team, center) {
        center = center.lerp(enemy, 0.28);
    }
    let mut frame_points = focus_points
        .iter()
        .copied()
        .filter(|point| world_distance(*point, center) <= 20.0)
        .collect::<Vec<_>>();
    if let Some(enemy) = nearest_enemy_point(snapshot, focus_team, center) {
        frame_points.push(enemy);
    }
    if frame_points.is_empty() {
        frame_points.push(center);
    }
    let radius = frame_points
        .iter()
        .map(|point| world_distance(*point, center))
        .fold(6.0_f32, f32::max);
    let scale = (220.0 / radius).clamp(24.0, 42.0);
    CaptureView { center, scale }
}

fn entity_position(entity: &bevy_open_rts::CaptureEntitySnapshot) -> Vec2 {
    Vec2::new(entity.x, entity.z)
}

fn average_point(points: &[Vec2]) -> Option<Vec2> {
    if points.is_empty() {
        return None;
    }
    let mut sum = Vec2::new(0.0, 0.0);
    for point in points {
        sum.x += point.x;
        sum.y += point.y;
    }
    Some(Vec2::new(
        sum.x / points.len() as f32,
        sum.y / points.len() as f32,
    ))
}

fn nearest_enemy_point(
    snapshot: &CaptureMatchSnapshot,
    focus_team: CaptureTeam,
    from: Vec2,
) -> Option<Vec2> {
    snapshot
        .entities
        .iter()
        .filter(|entity| {
            is_enemy_capture_team(focus_team, entity.team)
                && matches!(
                    entity.kind,
                    CaptureEntityKind::Structure | CaptureEntityKind::Unit
                )
        })
        .map(entity_position)
        .min_by(|lhs, rhs| {
            world_distance(*lhs, from)
                .partial_cmp(&world_distance(*rhs, from))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn is_enemy_capture_team(focus_team: CaptureTeam, team: CaptureTeam) -> bool {
    !matches!(team, CaptureTeam::Neutral) && team != focus_team
}

fn world_distance(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn world_to_screen(world: Vec2, view: CaptureView) -> Vec2 {
    let local_x = world.x - view.center.x;
    let local_y = world.y - view.center.y;
    Vec2::new(
        WIDTH as f32 * 0.5 + (local_x - local_y) * view.scale,
        330.0 + (local_x + local_y) * view.scale * 0.54,
    )
}

fn fill_rect(pixels: &mut [u8], x: i32, y: i32, width: i32, height: i32, color: Rgba) {
    for yy in y.max(0)..(y + height).min(HEIGHT as i32) {
        for xx in x.max(0)..(x + width).min(WIDTH as i32) {
            blend_pixel(pixels, xx, yy, color);
        }
    }
}

fn fill_circle(pixels: &mut [u8], center: Vec2, radius: f32, color: Rgba) {
    let min_x = (center.x - radius).floor() as i32;
    let max_x = (center.x + radius).ceil() as i32;
    let min_y = (center.y - radius).floor() as i32;
    let max_y = (center.y + radius).ceil() as i32;
    let radius_squared = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center.x;
            let dy = y as f32 - center.y;
            if dx * dx + dy * dy <= radius_squared {
                blend_pixel(pixels, x, y, color);
            }
        }
    }
}

fn fill_iso_diamond(
    pixels: &mut [u8],
    center: Vec2,
    half_width: f32,
    half_height: f32,
    color: Rgba,
) {
    let min_x = (center.x - half_width).floor() as i32;
    let max_x = (center.x + half_width).ceil() as i32;
    let min_y = (center.y - half_height).floor() as i32;
    let max_y = (center.y + half_height).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = (x as f32 - center.x).abs() / half_width;
            let dy = (y as f32 - center.y).abs() / half_height;
            if dx + dy <= 1.0 {
                blend_pixel(pixels, x, y, color);
            }
        }
    }
}

fn draw_ring(pixels: &mut [u8], center: Vec2, radius: f32, color: Rgba) {
    let min_x = (center.x - radius - 2.0).floor() as i32;
    let max_x = (center.x + radius + 2.0).ceil() as i32;
    let min_y = (center.y - radius - 2.0).floor() as i32;
    let max_y = (center.y + radius + 2.0).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center.x;
            let dy = y as f32 - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if (distance - radius).abs() <= 1.4 {
                blend_pixel(pixels, x, y, color);
            }
        }
    }
}

fn draw_line(pixels: &mut [u8], start: Vec2, end: Vec2, color: Rgba, thickness: f32) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let point = start.lerp(end, t);
        fill_circle(pixels, point, thickness, color);
    }
}

fn blend_pixel(pixels: &mut [u8], x: i32, y: i32, color: Rgba) {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
        return;
    }
    let index = ((y as u32 * WIDTH + x as u32) * 4) as usize;
    let alpha = color.a as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;
    pixels[index] = (color.r as f32 * alpha + pixels[index] as f32 * inv_alpha) as u8;
    pixels[index + 1] = (color.g as f32 * alpha + pixels[index + 1] as f32 * inv_alpha) as u8;
    pixels[index + 2] = (color.b as f32 * alpha + pixels[index + 2] as f32 * inv_alpha) as u8;
    pixels[index + 3] = 255;
}
