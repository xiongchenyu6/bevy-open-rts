//! Read-only native save/support observations and existing menu hit targets.
use super::*;

fn view(save: SaveGame) -> serde_json::Value {
    serde_json::json!({
        "clock":save.clock_sec,"ore":save.economies[0].ore,"crystal":save.economies[0].crystal,
        "cooldowns":save.support_remaining,"support_entities":save.support_entities,
        "units":save.units.iter().filter(|u|u.team==0).map(|u|serde_json::json!({
            "key":u.key,"id":u.id,"position":u.position,"health":u.health,"effects":u.continuity.effects,
        })).collect::<Vec<_>>(),
        "structures":save.structures.iter().filter(|s|s.team==0).map(|s|serde_json::json!({
            "key":s.key,"id":s.id,"position":s.position,"health":s.health,"construction":s.construction,
        })).collect::<Vec<_>>(),
    })
}
pub fn capture_live_effect_state(app: &mut App) -> Result<serde_json::Value, String> {
    collect_save_game(app.world_mut())
        .map(view)
        .ok_or("offline live match required".into())
}
pub fn capture_disk_effect_state() -> Result<serde_json::Value, String> {
    read_save_file(quicksave_path()?).map(view)
}
pub fn capture_effect_setup_button(app: &mut App, action: &str) -> Option<Vec2> {
    let world = app.world_mut();
    if action == "play" {
        let mut query = world.query::<(&FrontMenuButton, &UiGlobalTransform)>();
        return query
            .iter(world)
            .find(|(b, _)| b.action == FrontMenuAction::Play)
            .map(|(_, t)| t.translation);
    }
    let requested = match action {
        "faction" => MainMenuAction::ToggleLobbySlotFaction(0),
        "chaos" => MainMenuAction::SetLobbySlotFaction(0, SkirmishFaction::Chaos),
        "opponent" => MainMenuAction::ToggleLobbySlotController(1),
        "beginner" => MainMenuAction::SetLobbySlotController(
            1,
            SkirmishPlayerController::Ai(AiDifficulty::Beginner),
        ),
        "start" => MainMenuAction::StartMatch,
        _ => return None,
    };
    let mut query = world.query::<(&MainMenuButton, &UiGlobalTransform)>();
    query
        .iter(world)
        .find(|(b, _)| b.action == requested)
        .map(|(_, t)| t.translation)
}
