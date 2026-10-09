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
