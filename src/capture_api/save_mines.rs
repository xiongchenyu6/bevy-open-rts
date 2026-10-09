//! Read-only actor/mine/source and shot-pulse evidence for real-input saves.
use super::*;
use std::collections::HashSet;
fn view(save: SaveGame, visible: Option<&HashSet<u64>>) -> serde_json::Value {
    serde_json::json!({"clock":save.clock_sec,"ore":save.economies[0].ore,"crystal":save.economies[0].crystal,
        "enemy_units_destroyed":save.enemy_units_destroyed,
        "units":save.units.iter().map(|u|serde_json::json!({"key":u.key,"id":u.id,"team":u.team,
            "position":u.position,"health":u.health,"visible":visible.map(|keys|keys.contains(&u.key)),
            "active":u.continuity.active,"mine":u.continuity.mine,"layer":u.continuity.mine_layer,
        })).collect::<Vec<_>>(),
        "structures":save.structures.iter().map(|s|serde_json::json!({"key":s.key,"id":s.id,"team":s.team,
            "position":s.position,"health":s.health,"construction":s.construction,"rally":s.rally,
            "visible":visible.map(|keys|keys.contains(&s.key)),
        })).collect::<Vec<_>>(),
    })
}
pub fn capture_live_mine_state(app: &mut App) -> Result<serde_json::Value, String> {
    let world = app.world_mut();
    let save = collect_save_game(world).ok_or("offline live match required")?;
    let mut query = world.query::<(Entity, &VisibilityState)>();
    let visible = query
        .iter(world)
        .filter(|(_, v)| v.visible)
        .map(|(e, _)| e.to_bits())
        .collect::<HashSet<_>>();
    let mut state = view(save, Some(&visible));
    state["placement"] = serde_json::json!({
        "pending": world.resource::<CommandMode>().pending_structure_placement.as_ref().map(|p|p.id),
        "feedback": world.resource::<StructurePlacementFeedback>().validity.map(|v|format!("{v:?}")),
    });
    let mut pulses = world.query::<&ShotPulse>();
    state["pulses"] = serde_json::json!(pulses.iter(world).map(|p|serde_json::json!({
        "from":p.from.to_array(),"to":p.to.to_array(),"player":p.team==Team::Player(0),"ttl":p.ttl,
    })).collect::<Vec<_>>());
    Ok(state)
}
pub fn capture_disk_mine_state() -> Result<serde_json::Value, String> {
    read_save_file(quicksave_path()?).map(|s| view(s, None))
}
