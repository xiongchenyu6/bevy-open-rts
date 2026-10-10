//! Original group composition from actual game models; never paid gameplay proof.
use super::*;

pub fn export_roster_stage(
    inventory: &Path,
    directory: &Path,
) -> Result<serde_json::Value, String> {
    let configuration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(inventory).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if configuration["runtime_path"] != "ui/icons/RosterPreview.png" {
        return Err("incorrect roster output identity".into());
    }
    let entries = configuration["members"]
        .as_array()
        .filter(|e| (2..=12).contains(&e.len()))
        .ok_or("roster needs 2–12 members")?;
    let mut members = Vec::new();
    for entry in entries {
        let id = entry["entity_id"].as_str().ok_or("missing roster entity")?;
        let definition =
            registry::entity(id).ok_or_else(|| format!("unknown roster entity: {id}"))?;
        if definition.role != registry::EntityRole::Unit {
            return Err("battle group members must be units".into());
        }
        let faction_name = entry["faction"].as_str().ok_or("missing roster faction")?;
        let faction = match faction_name {
            "alliance" => SkirmishFaction::Alliance,
            "demon" => SkirmishFaction::Demon,
            "chaos" => SkirmishFaction::Chaos,
            _ => return Err("unknown roster faction".into()),
        };
        let roster = registry::faction(faction_name).unwrap();
        if !roster
            .production
            .iter()
            .any(|p| p.products.contains(&definition.id))
        {
            return Err(format!("{id} is not in {faction_name} roster"));
        }
        let xy = entry["ground_position"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or("roster position needs X/Z")?;
        let x = xy[0]
            .as_f64()
            .filter(|v| v.is_finite() && v.abs() <= 20.0)
            .ok_or("invalid roster X")? as f32;
        let z = xy[1]
            .as_f64()
            .filter(|v| v.is_finite() && v.abs() <= 20.0)
            .ok_or("invalid roster Z")? as f32;
        members.push((definition.id, faction, faction_name.to_string(), x, z));
    }
    std::fs::create_dir(directory).map_err(|e| e.to_string())?;
    let mut references = Vec::new();
    for (index, (id, faction, _, _, _)) in members.iter().enumerate() {
        let reference = game_rest_reference(id, *faction)?;
        std::fs::write(
            directory.join(format!("member-{index}-reference.json")),
            serde_json::to_vec_pretty(&reference).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        references.push(reference);
    }
    let mut runs = Vec::new();
    for run in 0..2 {
        let mut app = renderer_app();
        let mut roots = Vec::new();
        let mut evidence = Vec::new();
        for (index, (id, faction, faction_name, x, z)) in members.iter().enumerate() {
            let root = spawn_actual_model(&mut app, id, *faction);
            settle(&mut app, root, id)?;
            let excluded = hide_contextual_markers(app.world_mut(), root);
            let (signature, _) = geometry(&app, root)?;
            if signature != references[index] {
                return Err(format!(
                    "roster member {id}: model-only game-reference parity failed"
                ));
            }
            roots.push(root);
            evidence.push(serde_json::json!({"entity_id":id,"faction":faction_name,"ground_position":[x,z],"excluded_contextual_marker_count":excluded,"reference_scope":"model geometry/materials only","model_reference_verified":true,"paid_gameplay_verified":false}));
        }
        for (root, (_, _, _, x, z)) in roots.iter().zip(&members) {
            let mut transform = app
                .world_mut()
                .get_mut::<Transform>(*root)
                .ok_or("roster root has no Transform")?;
            transform.translation.x += x;
            transform.translation.z += z;
        }
        for _ in 0..12 {
            app.update();
        }
        let mut corners = Vec::new();
        for root in roots {
            corners.extend(geometry(&app, root)?.1);
        }
        let camera = frame(&mut app, &corners)?;
        let path = directory.join(format!("RosterPreview-{run}.png"));
        app.insert_resource(PortraitReadback {
            path: path.clone(),
            result: None,
        });
        let target = app.world().resource::<CaptureTarget>().0.clone();
        app.world_mut()
            .spawn(Screenshot::image(target))
            .observe(capture_rgba);
        for _ in 0..180 {
            app.update();
            if app.world().resource::<PortraitReadback>().result.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let alpha = app
            .world_mut()
            .resource_mut::<PortraitReadback>()
            .result
            .take()
            .ok_or("roster readback timeout")??;
        runs.push(serde_json::json!({"run":run,"image":path,"members":evidence,"camera":camera,"alpha":alpha,"scope":"Original asset-only composition, not a match screenshot or gameplay proof","runtime_replaced":false}));
        std::fs::write(directory.join("manifest.json"),serde_json::to_vec_pretty(&serde_json::json!({"configuration":configuration,"runs":runs,"human_readability_accepted":false,"normal_ui_verified":false,"runtime_replaced":false})).unwrap()).map_err(|e|e.to_string())?;
    }
    Ok(serde_json::json!({"runs":runs,"runtime_replaced":false}))
}
