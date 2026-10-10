//! Read-only order/reference views for normal-input persistence acceptance.
use super::*;

fn view(save: SaveGame) -> serde_json::Value {
    serde_json::json!({
        "clock":save.clock_sec,
        "ore":save.economies[0].ore,"crystal":save.economies[0].crystal,
        "units":save.units.iter().filter(|unit| unit.team==0).map(|unit| serde_json::json!({
            "key":unit.key,"id":unit.id,"position":unit.position,"cargo":unit.cargo,
            "active":unit.continuity.active,"queued":unit.continuity.queued,
        })).collect::<Vec<_>>(),
        "resources":save.resources,
    })
}

pub fn capture_live_order_state(app: &mut App) -> Result<serde_json::Value, String> {
    collect_save_game(app.world_mut())
        .map(view)
        .ok_or("running offline match required".into())
}

pub fn capture_disk_order_state() -> Result<serde_json::Value, String> {
    read_save_file(quicksave_path()?).map(view)
}

/// Validate projected capture input against the actual camera/window and HUD.
/// Reads only; it never changes selection, command mode, entities or resources.
pub fn capture_world_click_safety(
    app: &mut App,
    point: Vec2,
    fallback_size: Vec2,
) -> Result<(), String> {
    let world = app.world_mut();
    let viewport = world
        .query_filtered::<&Camera, With<MainCamera>>()
        .iter(world)
        .next()
        .and_then(Camera::logical_viewport_size);
    let window_size = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .map(|window| Vec2::new(window.width(), window.height()));
    let size = viewport.or(window_size).unwrap_or(fallback_size);
    if !point.is_finite()
        || !size.is_finite()
        || point.x < 0.0
        || point.y < 0.0
        || point.x >= size.x
        || point.y >= size.y
    {
        return Err(format!(
            "projected point outside actual viewport: {point:?}, size={size:?}"
        ));
    }
    if cursor_is_over_top_status_hud(point)
        || world
            .get_resource::<HudHitZones>()
            .is_some_and(|zones| zones.blocks_world(point))
    {
        return Err(format!(
            "projected point blocked by actual HUD: {point:?}, size={size:?}"
        ));
    }
    Ok(())
}
