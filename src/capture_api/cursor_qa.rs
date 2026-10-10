//! Read-only cursor QA observations and real command-button hit targets.
use super::*;
use bevy::window::CustomCursor;

pub fn capture_cursor_state(
    app: &mut App,
    include_pixels: bool,
) -> Result<serde_json::Value, String> {
    let world = app.world_mut();
    let mut windows =
        world.query_filtered::<(&Window, &AppliedRtsCursor, &CursorIcon), With<PrimaryWindow>>();

    let handle = world
        .get_resource::<RtsCursorAssetHandle>()
        .ok_or("cursor handle not loaded")?
        .0
        .clone();
    let static_cursor = world
        .resource::<Assets<StaticCursor>>()
        .get(&handle)
        .ok_or("StaticCursor not loaded")?;
    let image = world
        .resource::<Assets<Image>>()
        .get(&static_cursor.image)
        .ok_or("cursor image not loaded")?;
    let layout = world
        .resource::<Assets<TextureAtlasLayout>>()
        .get(&static_cursor.texture_atlas_layout)
        .ok_or("cursor atlas layout not loaded")?;
    let (window, applied, icon) = windows.single(world).map_err(|e| e.to_string())?;
    let CursorIcon::Custom(CustomCursor::Image(custom)) = icon else {
        return Err("window does not have image custom cursor".into());
    };
    let atlas = custom
        .texture_atlas
        .as_ref()
        .ok_or("custom cursor lacks atlas")?;
    let mode = world.get_resource::<CommandMode>();
    let hovered = world.get_resource::<HoveredResource>();
    let zones = world.resource::<HudHitZones>();
    let desired = desired_rts_cursor_kind(mode, hovered, window, zones).atlas_index();
    let server = world.resource::<AssetServer>();
    Ok(serde_json::json!({
        "scope":"Native offscreen asset/input metadata only; OS pointer is not rendered by screenshot",
        "static_cursor_path":server.get_path(handle.id()).map(|p|p.to_string()),
        "image_path":server.get_path(static_cursor.image.id()).map(|p|p.to_string()),
        "loaded_image_size":[image.width(),image.height()],"loaded_image_format":format!("{:?}",image.texture_descriptor.format),
        "loaded_rgba":if include_pixels {image.data.clone()} else {None},
        "atlas_size":layout.size.to_array(),
        "atlas_rects":layout.textures.iter().map(|rect|[rect.min.to_array(),rect.max.to_array()]).collect::<Vec<_>>(),
        "hotspots":(0..4).map(|index|static_cursor.hotspot_or_default(index)).collect::<Vec<_>>(),
        "applied_index":applied.index,"desired_index":desired,"custom_index":atlas.index,
        "custom_image_matches_loaded":custom.handle.id()==static_cursor.image.id(),
        "custom_layout_matches_loaded":atlas.layout.id()==static_cursor.texture_atlas_layout.id(),
        "custom_hotspot":custom.hotspot,"flip_x":custom.flip_x,"flip_y":custom.flip_y,
        "cursor_position":window.cursor_position().map(|p|p.to_array()),"over_hud":cursor_is_over_hud(window,zones),
        "command_mode":mode.map(|m|serde_json::json!({"patrol":m.patrol,"attack_move":m.attack_move,"rally":m.rally_point,"support":m.support_power.is_some(),"placement":m.pending_structure_placement.map(|p|p.id)})),
        "hovered_resource":hovered.is_some_and(|h|h.0.is_some()),
        "native_offscreen_pointer_visual_verified":false
    }))
}

pub fn capture_cursor_command_button(app: &mut App, action: &str) -> Option<Vec2> {
    let world = app.world_mut();
    let mut buttons = world.query::<(
        &BuildAction,
        &CommandSlotAvailability,
        &UiGlobalTransform,
        &Node,
    )>();
    buttons
        .iter(world)
        .find_map(|(a, available, transform, node)| {
            let matches = match action {
                "patrol" => matches!(a, BuildAction::Patrol),
                "attack" => matches!(a, BuildAction::AttackMove),
                "build_power" => matches!(a, BuildAction::Build("PowerReactor")),
                _ => false,
            };
            (matches && available.enabled && node.display != Display::None)
                .then_some(transform.translation)
        })
}
