//! Visual-only shared-card fixture; never a paid gameplay proof.
use super::*;
pub(super) fn render(dir: &Path, width: u32, height: u32) -> Result<(), String> {
    std::fs::create_dir(dir).map_err(|e| e.to_string())?;
    let mut app = build_capture_app(width, height);
    wait_for_startup(&mut app)?;
    bevy_open_rts::capture_setup_semantic_ui(&mut app);
    for _ in 0..60 {
        app.update();
    }
    let handle = capture_handle(&app);
    shoot(&mut app, &handle, dir.join("00_overview.png"));
    let targets = bevy_open_rts::capture_semantic_ui_targets(&mut app);
    if targets.len() != 10 {
        return Err(format!(
            "visual fixture expected10 actual shared cards, found{}",
            targets.len()
        ));
    }
    let mut observations = Vec::new();
    for (name, path, point) in targets {
        capture_set_cursor(&mut app, point);
        for _ in 0..12 {
            app.update();
        }
        let (hovered, text) =
            bevy_open_rts::capture_semantic_ui_tooltip(&mut app).ok_or("shared tooltip absent")?;
        if hovered != name || text.is_empty() {
            return Err(format!("actualhover sharedtooltip empty for{name}"));
        }
        shoot(&mut app, &handle, dir.join(format!("{name}_tooltip.png")));
        observations.push(serde_json::json!({"name":name,"shared_icon_path":path,"cursor":point.to_array(),"actual_hover_tooltip":text}));
    }
    std::fs::write(dir.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"scope":"UI visual fixture — no gameplay acceptance","process_id":std::process::id(),"width":width,"height":height,"cards":observations})).unwrap()).map_err(|e|e.to_string())?;
    println!("[capture] shared semantic UI visual-only fixture10 actual hover tooltips");
    Ok(())
}
