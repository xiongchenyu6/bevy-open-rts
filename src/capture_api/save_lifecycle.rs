//! Native read-only observations for paid construction and natural weather.
use super::*;

pub fn capture_live_lifecycle_state(app: &mut App) -> Result<serde_json::Value, String> {
    let save = collect_save_game(app.world_mut()).ok_or("offline match required")?;
    serde_json::to_value(save).map_err(|error| error.to_string())
}

pub fn capture_disk_lifecycle_state() -> Result<serde_json::Value, String> {
    serde_json::to_value(read_save_file(quicksave_path()?)?).map_err(|error| error.to_string())
}

pub fn capture_lifecycle_setup_button(app: &mut App, action: &str) -> Option<Vec2> {
    let world = app.world_mut();
    if action == "play" {
        return world
            .query::<(&FrontMenuButton, &UiGlobalTransform)>()
            .iter(world)
            .find(|(button, _)| button.action == FrontMenuAction::Play)
            .map(|(_, transform)| transform.translation);
    }
    let requested = match action {
        "player" => MainMenuAction::ToggleLobbySlotController(0),
        "human-player" => {
            MainMenuAction::SetLobbySlotController(0, SkirmishPlayerController::Human)
        }
        "opponent" => MainMenuAction::ToggleLobbySlotController(1),
        "beginner" => MainMenuAction::SetLobbySlotController(
            1,
            SkirmishPlayerController::Ai(AiDifficulty::Beginner),
        ),
        "start" => MainMenuAction::StartMatch,
        _ => return None,
    };
    world
        .query::<(&MainMenuButton, &UiGlobalTransform)>()
        .iter(world)
        .find(|(button, _)| button.action == requested)
        .map(|(_, transform)| transform.translation)
}
