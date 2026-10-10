//! Staged asset portraits: actual game spawn/model processing, no simulation.
use super::*;
use bevy::camera::primitives::MeshAabb;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::world_serialization::{WorldInstance, WorldInstanceSpawner};
use std::{path::Path, time::Duration};

mod roster_export;
pub use roster_export::export_roster_stage;

#[derive(Resource, Default)]
struct PortraitReadback {
    path: std::path::PathBuf,
    result: Option<Result<serde_json::Value, String>>,
}

fn capture_rgba(event: On<ScreenshotCaptured>, mut result: ResMut<PortraitReadback>) {
    result.result = Some((|| {
        let image = event
            .image
            .clone()
            .try_into_dynamic()
            .map_err(|error| format!("RGBA conversion: {error:?}"))?
            .to_rgba8();
        let mut transparent = 0usize;
        let mut opaque = 0usize;
        let mut min = [512u32, 512];
        let mut max = [0u32, 0];
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel[3] == 0 {
                transparent += 1;
            }
            if pixel[3] >= 250 {
                opaque += 1;
            }
            if pixel[3] > 0 {
                min[0] = min[0].min(x);
                min[1] = min[1].min(y);
                max[0] = max[0].max(x);
                max[1] = max[1].max(y);
            }
        }
        if transparent == 0
            || opaque == 0
            || min[0] == 0
            || min[1] == 0
            || max[0] >= 511
            || max[1] >= 511
        {
            return Err(format!(
                "invalid alpha or clipped silhouette: transparent={transparent}, opaque={opaque}, bounds={min:?}..{max:?}"
            ));
        }
        image
            .save(&result.path)
            .map_err(|error| error.to_string())?;
        Ok(
            serde_json::json!({"transparent_pixels":transparent,"opaque_pixels":opaque,"alpha_bounds":[min,max],"conversion":"ScreenshotCaptured Image -> DynamicImage -> RGBA8 PNG; no RGB conversion/chroma key"}),
        )
    })());
}

fn renderer_app() -> App {
    let mut app = build_model_harness_capture_app(512, 512);
    app.insert_resource(ClearColor(Color::NONE))
        .init_resource::<NextSpawnId>()
        .init_resource::<PlayerColorSlots>()
        .init_resource::<TeamColorMaterials>()
        .add_systems(
            Update,
            (apply_team_color_materials, recenter_entity_models).chain(),
        );
    app.world_mut().spawn((
        DirectionalLight {
            illuminance: 16_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-6.0, 14.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    app
}

fn spawn_actual_model(app: &mut App, id: &'static str, faction: SkirmishFaction) -> Entity {
    let server = app.world().resource::<AssetServer>().clone();
    let mut next = NextSpawnId(0);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let root = {
        let mut commands = Commands::new(&mut queue, app.world());
        let def = registry::entity(id).expect("validated registry entity");
        match def.role {
            registry::EntityRole::Unit => spawn_unit_with_visual_faction(
                &mut commands,
                &server,
                &mut next,
                id,
                Team::Player(0),
                Vec3::ZERO,
                0,
                Some(faction),
                Team::Player(0),
            ),
            registry::EntityRole::Structure => spawn_structure_for_visual_faction(
                &mut commands,
                &server,
                &mut next,
                id,
                Team::Player(0),
                Team::Player(0),
                Vec3::ZERO,
                0.0,
                Some(faction),
            ),
        }
    };
    queue.apply(app.world_mut());
    root
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut all = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        all.push(entity);
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    all
}

fn model_entities(world: &World, root: Entity) -> Vec<Entity> {
    descendants(world, root)
        .into_iter()
        .filter(|entity| world.get::<ContextualFactionMarker>(*entity).is_none())
        .collect()
}

fn hide_contextual_markers(world: &mut World, root: Entity) -> usize {
    let markers: Vec<_> = descendants(world, root)
        .into_iter()
        .filter(|entity| world.get::<ContextualFactionMarker>(*entity).is_some())
        .collect();
    for entity in &markers {
        world.entity_mut(*entity).insert(Visibility::Hidden);
    }
    markers.len()
}

fn texture_ready(server: &AssetServer, image: Option<&Handle<Image>>) -> bool {
    image.is_none_or(|image| server.is_loaded_with_dependencies(image.id()))
}

fn settle(app: &mut App, root: Entity, id: &'static str) -> Result<(), String> {
    let def = registry::entity(id).unwrap();
    let expected = if is_procedural_tank(id) || ProceduralEntityModel::for_entity_id(id).is_some() {
        0
    } else {
        def.render_parts.len().max(1)
    };
    for _ in 0..900 {
        app.update();
        let world = app.world();
        let server = world.resource::<AssetServer>();
        let spawner = world.resource::<WorldInstanceSpawner>();
        let meshes = world.resource::<Assets<Mesh>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        let mut model_parts = 0;
        let mut mesh_count = 0;
        let mut ready = world.get::<ModelRecentered>(root).is_some();
        for entity in descendants(world, root) {
            if let Some(asset) = world.get::<WorldAssetRoot>(entity) {
                model_parts += 1;
                if let Some(bevy::asset::LoadState::Failed(error)) =
                    server.get_load_state(asset.0.id())
                {
                    return Err(format!("failed model asset: {error}"));
                }
                ready &= server.is_loaded_with_dependencies(asset.0.id())
                    && world
                        .get::<WorldInstance>(entity)
                        .is_some_and(|instance| spawner.instance_is_ready(**instance));
            }
            if let Some(mesh) = world.get::<Mesh3d>(entity) {
                mesh_count += 1;
                ready &= meshes.get(&mesh.0).is_some()
                    && world.get::<TeamColorProcessed>(entity).is_some();
                let material = world
                    .get::<MeshMaterial3d<StandardMaterial>>(entity)
                    .and_then(|handle| materials.get(&handle.0));
                ready &= material.is_some_and(|material| {
                    texture_ready(server, material.base_color_texture.as_ref())
                        && texture_ready(server, material.normal_map_texture.as_ref())
                        && texture_ready(server, material.metallic_roughness_texture.as_ref())
                        && texture_ready(server, material.emissive_texture.as_ref())
                        && texture_ready(server, material.occlusion_texture.as_ref())
                });
            }
        }
        if ready && mesh_count > 0 && model_parts == expected {
            for _ in 0..12 {
                app.update();
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("{id}: geometry/material streaming did not finish"))
}

fn image_path(server: &AssetServer, handle: Option<&Handle<Image>>) -> Option<String> {
    handle
        .and_then(|handle| server.get_path(handle.id()))
        .map(|path| path.to_string())
}

fn geometry(app: &App, root: Entity) -> Result<(serde_json::Value, Vec<Vec3>), String> {
    let world = app.world();
    let meshes = world.resource::<Assets<Mesh>>();
    let materials = world.resource::<Assets<StandardMaterial>>();
    let server = world.resource::<AssetServer>();
    let mut rows = Vec::new();
    let mut corners = Vec::new();
    for entity in model_entities(world, root) {
        let Some(handle) = world.get::<Mesh3d>(entity) else {
            continue;
        };
        let mesh = meshes.get(&handle.0).ok_or("missing mesh")?;
        let transform = world
            .get::<GlobalTransform>(entity)
            .ok_or("missing mesh GlobalTransform")?;
        let aabb = world
            .get::<Aabb>(entity)
            .copied()
            .or_else(|| mesh.compute_aabb())
            .ok_or("mesh has no finite position bounds")?;
        let center: Vec3 = aabb.center.into();
        let half: Vec3 = aabb.half_extents.into();
        if !center.is_finite() || !half.is_finite() {
            return Err("nonfinite mesh bounds".into());
        }
        let mut mesh_corners = Vec::new();
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let point = transform.transform_point(center + half * Vec3::new(x, y, z));
                    if !point.is_finite() {
                        return Err("nonfinite transformed corner".into());
                    }
                    corners.push(point);
                    mesh_corners.push(point.to_array());
                }
            }
        }
        let material = world
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .and_then(|handle| materials.get(&handle.0))
            .ok_or("missing mesh material")?;
        let color = material.base_color.to_srgba();
        rows.push(serde_json::json!({"name":world.get::<Name>(entity).map(|name|name.as_str()),"corners":mesh_corners,
            "attributes":mesh.attributes().map(|(attribute, values)| (attribute.name, format!("{values:?}"))).collect::<Vec<_>>(),"indices":format!("{:?}",mesh.indices()),
            "material":{"base_color":[color.red,color.green,color.blue,color.alpha],"metallic":material.metallic,"roughness":material.perceptual_roughness,"reflectance":material.reflectance,"alpha_mode":format!("{:?}",material.alpha_mode),"unlit":material.unlit,"double_sided":material.double_sided,"uv_transform":format!("{:?}",material.uv_transform),"base_color_texture":image_path(server,material.base_color_texture.as_ref()),"normal_map":image_path(server,material.normal_map_texture.as_ref()),"metallic_roughness_texture":image_path(server,material.metallic_roughness_texture.as_ref()),"emissive_texture":image_path(server,material.emissive_texture.as_ref()),"occlusion_texture":image_path(server,material.occlusion_texture.as_ref())}}));
    }
    if rows.is_empty() {
        return Err("empty portrait model".into());
    }
    rows.sort_by_key(|value| value.to_string());
    Ok((
        serde_json::json!({"mesh_count":rows.len(),"meshes":rows,"reference_scope":"model meshes/materials only; contextual faction markers excluded","excluded_contextual_marker_count":descendants(world,root).iter().filter(|entity|world.get::<ContextualFactionMarker>(**entity).is_some()).count()}),
        corners,
    ))
}

fn frame(app: &mut App, corners: &[Vec3]) -> Result<serde_json::Value, String> {
    let min = corners
        .iter()
        .copied()
        .fold(Vec3::splat(f32::MAX), Vec3::min);
    let max = corners
        .iter()
        .copied()
        .fold(Vec3::splat(f32::MIN), Vec3::max);
    let center = (min + max) * 0.5;
    let size = max - min;
    let distance = 2.0 * size.length() + 1.0;
    let direction = Vec3::new(6.0, 5.0, 7.0).normalize();
    let orientation =
        Transform::from_translation(center + direction * distance).looking_at(center, Vec3::Y);
    let right = orientation.rotation * Vec3::X;
    let up = orientation.rotation * Vec3::Y;
    let mut low = Vec2::splat(f32::MAX);
    let mut high = Vec2::splat(f32::MIN);
    for corner in corners {
        let delta = *corner - center;
        let point = Vec2::new(delta.dot(right), delta.dot(up));
        low = low.min(point);
        high = high.max(point);
    }
    let focus = center + right * ((low.x + high.x) * 0.5) + up * ((low.y + high.y) * 0.5);
    let span = (high - low).max_element() / 0.84;
    if !span.is_finite() || span <= 0.0 {
        return Err("invalid camera span".into());
    }
    let target = app.world().resource::<CaptureTarget>().0.clone();
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: span,
                    height: span,
                },
                scale: 1.0,
                near: 0.01,
                far: distance + size.length() + 1.0,
                ..OrthographicProjection::default_3d()
            }),
            RenderTarget::Image(target.into()),
            Transform::from_translation(focus + direction * distance).looking_at(focus, Vec3::Y),
        ))
        .id();
    for _ in 0..8 {
        app.update();
    }
    let camera_data = app.world().get::<Camera>(camera).unwrap();
    let camera_transform = app.world().get::<GlobalTransform>(camera).unwrap();
    let pixels = corners
        .iter()
        .map(|corner| {
            camera_data
                .world_to_viewport(camera_transform, *corner)
                .map(|point| point.to_array())
                .map_err(|error| format!("camera projection: {error:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if pixels
        .iter()
        .any(|point| point[0] < 39.0 || point[0] > 473.0 || point[1] < 39.0 || point[1] > 473.0)
    {
        return Err("portrait bounds violate eight-percent padding".into());
    }
    Ok(
        serde_json::json!({"world_min":min.to_array(),"world_max":max.to_array(),"focus":focus.to_array(),"direction":direction.to_array(),"distance":distance,"span":span,"projected_corners":pixels,"padding_fraction":0.08,"hdr":false,"floor":false,"size":[512,512]}),
    )
}

// Independent full-game context; MainMenu keeps simulation systems gated.
// The spawned entity is an asset reference, never paid-gameplay evidence.
fn game_rest_reference(
    id: &'static str,
    faction: SkirmishFaction,
) -> Result<serde_json::Value, String> {
    let mut app = build_capture_app(512, 512);
    let mut ready = false;
    for _ in 0..1800 {
        app.update();
        if capture_startup_ready(&app) {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready {
        return Err("full-game reference startup timed out".into());
    }
    capture_show_main_menu(&mut app);
    app.update();
    app.add_systems(
        Update,
        (apply_team_color_materials, recenter_entity_models).chain(),
    );
    let root = spawn_actual_model(&mut app, id, faction);
    settle(&mut app, root, id)?;
    let (signature, _) = geometry(&app, root)?;
    Ok(signature)
}

/// Export the audited registry inventory, never runtime replacements.
pub fn export_portrait_stage(
    inventory: &Path,
    directory: &Path,
) -> Result<serde_json::Value, String> {
    let inventory: serde_json::Value =
        serde_json::from_slice(&std::fs::read(inventory).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let entries = inventory["entities"]
        .as_array()
        .filter(|entries| !entries.is_empty())
        .ok_or("portrait inventory needs a nonempty entities array")?;
    let mut targets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let id = entry["entity_id"].as_str().ok_or("missing entity_id")?;
        let definition =
            registry::entity(id).ok_or_else(|| format!("unknown portrait entity: {id}"))?;
        let runtime_path = entry["runtime_path"]
            .as_str()
            .ok_or("missing runtime_path")?;
        if definition.icon != Some(runtime_path) || !seen.insert(definition.id) {
            return Err(format!("duplicate entity or incorrect portrait path: {id}"));
        }
        let faction_name = entry["portrait_faction"]
            .as_str()
            .ok_or("missing portrait_faction")?;
        let faction = match faction_name {
            "alliance" => SkirmishFaction::Alliance,
            "demon" => SkirmishFaction::Demon,
            "chaos" => SkirmishFaction::Chaos,
            _ => return Err(format!("unknown portrait faction: {faction_name}")),
        };
        targets.push((definition.id, faction, runtime_path, faction_name));
    }
    std::fs::create_dir(directory).map_err(|error| error.to_string())?;
    let mut reports = Vec::new();
    for (id, faction, runtime_path, faction_name) in targets {
        let reference = game_rest_reference(id, faction)?;
        std::fs::write(
            directory.join(format!("{id}-game-rest-reference.json")),
            serde_json::to_vec_pretty(&reference).unwrap(),
        )
        .map_err(|error| error.to_string())?;
        for run in 0..2 {
            let mut app = renderer_app();
            let root = spawn_actual_model(&mut app, id, faction);
            settle(&mut app, root, id)?;
            let excluded_markers = hide_contextual_markers(app.world_mut(), root);
            let (signature, corners) = geometry(&app, root)?;
            if signature != reference {
                return Err(format!(
                    "{id}: full-game asset-reference geometry/material parity failed"
                ));
            }
            let backend = app
                .world()
                .get_resource::<bevy::render::renderer::RenderAdapterInfo>()
                .map(|adapter| serde_json::json!({"name":adapter.0.name,"vendor":adapter.0.vendor,"device":adapter.0.device,"driver":adapter.0.driver,"driver_info":adapter.0.driver_info,"backend":format!("{:?}",adapter.0.backend)}));
            let camera = frame(&mut app, &corners)?;
            let path = directory.join(format!("{id}-{run}.png"));
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
                .ok_or("portrait readback timed out")??;
            reports.push(serde_json::json!({"id":id,"run":run,"runtime_path":runtime_path,"faction":faction_name,"palette_slot":0,"geometry":signature,"reference_scope":"model-only parity; not whole-scene parity","excluded_contextual_marker_count":excluded_markers,"camera":camera,"alpha":alpha,"image":path,"stage_only":true,"asset_only_full_game_reference_verified":true,"live_gameplay_reference_verified":false,"backend":backend,"light":{"illuminance":16000,"position":[-6,14,7]}}));
            std::fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec_pretty(&reports).unwrap(),
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(
        serde_json::json!({"portraits":reports,"runtime_replaced":false,"gameplay_progression_claimed":false}),
    )
}

#[cfg(test)]
mod marker_tests {
    use super::*;
    #[test]
    fn portraits_exclude_only_contextual_markers_and_preserve_actual_tank_and_glb_children() {
        for (faction, count) in [
            (FactionIdentityMarker::Human, 2),
            (FactionIdentityMarker::Demon, 3),
            (FactionIdentityMarker::Chaos, 2),
        ] {
            let mut world = World::new();
            world.init_resource::<Assets<Mesh>>();
            world.init_resource::<Assets<StandardMaterial>>();
            let root = world
                .spawn((Transform::default(), Visibility::default()))
                .id();
            assert!(spawn_tank_model(&mut world, root, "Tank"));
            let part = world
                .spawn((
                    ChildOf(root),
                    WorldAssetRoot(Handle::default()),
                    Transform::default(),
                ))
                .id();
            let mesh = world
                .resource_mut::<Assets<Mesh>>()
                .add(Cuboid::new(0.1, 0.1, 0.1));
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            let accessory = spawn_procedural_mesh_child(
                &mut world,
                part,
                "registered GLB accessory",
                mesh,
                material,
                Transform::default(),
            );
            let models_before = model_entities(&world, root);
            spawn_faction_identity_marker_model(&mut world, root, faction, Vec3::ZERO, 0.2);
            let markers: Vec<_> = descendants(&world, root)
                .into_iter()
                .filter(|e| world.get::<ContextualFactionMarker>(*e).is_some())
                .collect();
            assert_eq!(markers.len(), count);
            assert!(markers.iter().all(|e| {
                world
                    .get::<Visibility>(*e)
                    .is_none_or(|v| *v != Visibility::Hidden)
            }));
            assert_eq!(model_entities(&world, root), models_before);
            assert!(models_before.contains(&accessory));
            assert!(models_before.iter().any(|e| {
                world
                    .get::<Name>(*e)
                    .is_some_and(|n| n.as_str() == "tank hull")
            }));
            assert_eq!(hide_contextual_markers(&mut world, root), count);
            assert!(
                markers
                    .iter()
                    .all(|e| world.get::<Visibility>(*e) == Some(&Visibility::Hidden))
            );
            assert_eq!(model_entities(&world, root), models_before);
        }
    }
}
