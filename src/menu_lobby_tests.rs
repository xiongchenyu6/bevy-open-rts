//! Lobby descriptions must match the actual settings sent to the match.
use super::*;

fn four_ai_lobby() -> SkirmishMenuSelection {
    let mut selection = SkirmishMenuSelection::default();
    for slot in 0..4 {
        selection
            .set_lobby_slot_controller(slot, SkirmishPlayerController::Ai(AiDifficulty::Normal));
    }
    assert_eq!(selection.active_team_count(), 4);
    selection
}

#[test]
fn four_team_summary_and_opponents_follow_runtime_relations() {
    let selection = four_ai_lobby();
    let setup = selection.match_setup();
    assert_eq!(
        skirmish_mode_from_match_setup(&setup),
        SkirmishMatchMode::FreeForAll
    );
    assert!(main_menu_summary_text(selection).contains(SkirmishMatchMode::FreeForAll.label()));
    let opponents = skirmish_opponents_text(selection);
    for team in [Team::Player(1), Team::Player(2), Team::Player(3)] {
        assert!(
            opponents.contains(&team.label()),
            "missing actual enemy in lobby: {opponents}"
        );
    }
}

#[test]
fn allied_lobby_lists_actual_allies_and_every_enemy() {
    let mut selection = four_ai_lobby();
    selection.lobby_team_ids[1] = selection.lobby_team_ids[0];
    selection.lobby_team_ids[3] = selection.lobby_team_ids[2];
    let setup = selection.match_setup();
    assert!(
        !setup
            .team_relations
            .are_enemies(Team::Player(0), Team::Player(1))
    );
    assert!(
        setup
            .team_relations
            .are_enemies(Team::Player(0), Team::Player(2))
    );
    assert!(main_menu_summary_text(selection).contains(t("团队战", "Team battle")));
    let description = skirmish_opponents_text(selection);
    for team in [Team::Player(1), Team::Player(2), Team::Player(3)] {
        assert!(
            description.contains(&team.label()),
            "missing actual participant in lobby: {description}"
        );
    }
    assert!(description.contains(t("盟友", "Allies")));
}
