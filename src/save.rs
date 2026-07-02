//! Quicksave / quickload (Ctrl+S / Ctrl+L, native only).
//!
//! RA2-style state saves: the match SETUP plus the live world (economies, clock,
//! units with health/veterancy/cargo, structures with construction progress,
//! rally points and garrisons, remaining resource-node amounts, support
//! cooldowns). In-flight orders, fog memory, planted mines and supply crates are
//! deliberately NOT saved (units idle after a load and re-scout the fog) — the
//! standard v1 trade-off. Loading re-enters the match through the normal restart
//! flow, then `apply_loaded_save` swaps the freshly spawned world for the saved
//! one, reusing the regular spawn functions.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::*;

pub(crate) const QUICKSAVE_PATH: &str = "saves/quicksave.ron";
pub(crate) const SAVE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedSettings {
    pub(crate) map_path: String,
    pub(crate) starting_ore: i32,
    pub(crate) starting_crystal: i32,
    pub(crate) active_teams: Vec<bool>,
    /// SkirmishFaction by `index()`.
    pub(crate) player_factions: Vec<usize>,
    pub(crate) player_color_slots: Vec<usize>,
    /// 0 = None, 1 = Human, 2 + AiDifficulty ordinal = Ai.
    pub(crate) player_controllers: Vec<u8>,
    pub(crate) player_spawn_slots: Vec<usize>,
    /// AiDifficulty ordinal per player.
    pub(crate) ai_difficulties: Vec<u8>,
    pub(crate) allied: Vec<Vec<bool>>,
    /// StartupLoadoutMode ordinal.
    pub(crate) startup_loadout: u8,
    pub(crate) visible_team: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedUnit {
    pub(crate) id: String,
    pub(crate) team: usize,
    pub(crate) position: [f32; 3],
    pub(crate) yaw: f32,
    pub(crate) health: f32,
    pub(crate) rank: u8,
    pub(crate) experience: u32,
    pub(crate) cargo: Option<(i32, i32)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedStructure {
    pub(crate) id: String,
    pub(crate) team: usize,
    pub(crate) position: [f32; 3],
    pub(crate) yaw: f32,
    pub(crate) health: f32,
    /// (remaining, total) build seconds when still under construction.
    pub(crate) construction: Option<(f32, f32)>,
    /// Rally target plus RallyMode ordinal (0 move, 1 attack-move).
    pub(crate) rally: Option<([f32; 3], u8)>,
    pub(crate) garrison_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SavedEconomy {
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
    pub(crate) power_sabotage_remaining: f32,
    pub(crate) production_veterancy_ranks: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SaveGame {
    pub(crate) version: u32,
    pub(crate) settings: SavedSettings,
    pub(crate) clock_sec: f32,
    pub(crate) enemy_units_destroyed: u32,
    pub(crate) enemy_structures_destroyed: u32,
    pub(crate) units_lost: u32,
    pub(crate) economies: Vec<SavedEconomy>,
    pub(crate) support_remaining: Vec<Vec<f32>>,
    pub(crate) support_charge_started: Vec<Vec<bool>>,
    pub(crate) units: Vec<SavedUnit>,
    pub(crate) structures: Vec<SavedStructure>,
    /// (ResourceKind ordinal, position, remaining amount).
    pub(crate) resources: Vec<(u8, [f32; 3], i32)>,
}

/// Save loaded from disk, consumed by `apply_loaded_save` right after the match
/// world is (re)built by the normal InMatch setup chain.
#[derive(Resource, Default)]
pub(crate) struct PendingLoadedSave(pub(crate) Option<SaveGame>);

fn controller_to_repr(controller: SkirmishPlayerController) -> u8 {
    match controller {
        SkirmishPlayerController::None => 0,
        SkirmishPlayerController::Human => 1,
        SkirmishPlayerController::Ai(difficulty) => 2 + difficulty as u8,
    }
}

fn controller_from_repr(repr: u8) -> SkirmishPlayerController {
    match repr {
        0 => SkirmishPlayerController::None,
        1 => SkirmishPlayerController::Human,
        n => SkirmishPlayerController::Ai(difficulty_from_ordinal(n.saturating_sub(2))),
    }
}

fn difficulty_from_ordinal(ordinal: u8) -> AiDifficulty {
    match ordinal {
        0 => AiDifficulty::Beginner,
        1 => AiDifficulty::Easy,
        2 => AiDifficulty::Normal,
        _ => AiDifficulty::Hard,
    }
}

fn yaw_of(transform: &Transform) -> f32 {
    transform.rotation.to_euler(EulerRot::YXZ).0
}

/// Snapshots the running match into a [`SaveGame`]. `None` when no match is
/// running (only `MatchPhase::Running` states are saveable).
pub(crate) fn collect_save_game(world: &mut World) -> Option<SaveGame> {
    if world.resource::<MatchState>().phase != MatchPhase::Running {
        return None;
    }
    let settings = world.resource::<MatchSetupSettings>();
    let saved_settings = SavedSettings {
        map_path: settings.map_path.to_string(),
        starting_ore: settings.starting_resources.ore,
        starting_crystal: settings.starting_resources.crystal,
        active_teams: settings.active_teams.clone(),
        player_factions: settings
            .player_factions
            .iter()
            .map(|faction| faction.index())
            .collect(),
        player_color_slots: settings.player_color_slots.clone(),
        player_controllers: settings
            .player_controllers
            .iter()
            .map(|controller| controller_to_repr(*controller))
            .collect(),
        player_spawn_slots: settings.player_spawn_slots.clone(),
        ai_difficulties: settings
            .ai_difficulties
            .players
            .iter()
            .map(|difficulty| *difficulty as u8)
            .collect(),
        allied: settings.team_relations.allied.clone(),
        startup_loadout: settings.startup_loadout as u8,
        visible_team: settings
            .visible_player
            .team
            .economy_index()
            .unwrap_or_default(),
    };

    let match_state = world.resource::<MatchState>();
    let (clock_sec, enemy_units_destroyed, enemy_structures_destroyed, units_lost) = (
        match_state.start_time_sec,
        match_state.enemy_units_destroyed,
        match_state.enemy_structures_destroyed,
        match_state.units_lost,
    );

    let economies = world
        .resource::<Economies>()
        .players
        .iter()
        .map(|economy| SavedEconomy {
            ore: economy.ore,
            crystal: economy.crystal,
            power_sabotage_remaining: economy.power_sabotage_remaining,
            production_veterancy_ranks: economy.production_veterancy_ranks.to_vec(),
        })
        .collect();
    let cooldowns = world.resource::<SupportCooldowns>();
    let support_remaining = cooldowns.remaining.iter().map(|row| row.to_vec()).collect();
    let support_charge_started = cooldowns
        .initial_charge_started
        .iter()
        .map(|row| row.to_vec())
        .collect();

    let mut units = Vec::new();
    let mut unit_q = world.query::<(
        &Unit,
        &Team,
        &Transform,
        &Health,
        Option<&Veterancy>,
        Option<&ResourceCargo>,
    )>();
    for (unit, team, transform, health, veterancy, cargo) in unit_q.iter(world) {
        let Team::Player(team_index) = team else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        units.push(SavedUnit {
            id: unit.id.to_string(),
            team: *team_index,
            position: transform.translation.to_array(),
            yaw: yaw_of(transform),
            health: health.current,
            rank: veterancy.map_or(0, |veterancy| veterancy.rank),
            experience: veterancy.map_or(0, |veterancy| veterancy.experience_points),
            cargo: cargo.map(|cargo| (cargo.ore, cargo.crystal)),
        });
    }

    let mut structures = Vec::new();
    let mut structure_q = world.query::<(
        &Structure,
        &Team,
        &Transform,
        &Health,
        Option<&UnderConstruction>,
        Option<&RallyPoint>,
        Option<&Garrison>,
    )>();
    for (structure, team, transform, health, construction, rally, garrison) in
        structure_q.iter(world)
    {
        let Team::Player(team_index) = team else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        structures.push(SavedStructure {
            id: structure.id.to_string(),
            team: *team_index,
            position: transform.translation.to_array(),
            yaw: yaw_of(transform),
            health: health.current,
            construction: construction
                .map(|construction| (construction.remaining, construction.total)),
            rally: rally.and_then(|rally| {
                rally
                    .target
                    .map(|target| (target.to_array(), rally.mode as u8))
            }),
            garrison_count: garrison.map_or(0, |garrison| garrison.count),
        });
    }

    let mut resources = Vec::new();
    let mut resource_q = world.query::<(&ResourceNode, &Transform)>();
    for (node, transform) in resource_q.iter(world) {
        if node.amount <= 0 {
            continue;
        }
        resources.push((
            node.kind as u8,
            transform.translation.to_array(),
            node.amount,
        ));
    }

    Some(SaveGame {
        version: SAVE_VERSION,
        settings: saved_settings,
        clock_sec,
        enemy_units_destroyed,
        enemy_structures_destroyed,
        units_lost,
        economies,
        support_remaining,
        support_charge_started,
        units,
        structures,
        resources,
    })
}

/// Rebuilds `MatchSetupSettings` from a save (the restart flow re-derives the
/// map/relations/factions from it, exactly like a fresh match start).
pub(crate) fn match_settings_from_save(save: &SavedSettings) -> Option<MatchSetupSettings> {
    let map = skirmish_map_by_path(&save.map_path)?;
    Some(MatchSetupSettings {
        map_path: map.godot_path,
        starting_resources: StartingResources {
            ore: save.starting_ore,
            crystal: save.starting_crystal,
        },
        visible_player: VisiblePlayer::per_player(Team::Player(save.visible_team)),
        ai_difficulties: AiDifficultySettings {
            players: save
                .ai_difficulties
                .iter()
                .map(|ordinal| difficulty_from_ordinal(*ordinal))
                .collect(),
        },
        team_relations: TeamRelations {
            allied: save.allied.clone(),
        },
        startup_loadout: if save.startup_loadout == StartupLoadoutMode::PlaytestExpanded as u8 {
            StartupLoadoutMode::PlaytestExpanded
        } else {
            StartupLoadoutMode::GodotSkirmish
        },
        active_teams: save.active_teams.clone(),
        player_factions: save
            .player_factions
            .iter()
            .map(|index| SkirmishFaction::ALL[*index % SkirmishFaction::ALL.len()])
            .collect(),
        player_color_slots: save.player_color_slots.clone(),
        player_controllers: save
            .player_controllers
            .iter()
            .map(|repr| controller_from_repr(*repr))
            .collect(),
        player_spawn_slots: save.player_spawn_slots.clone(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_save_file(save: &SaveGame, path: &str) -> Result<(), String> {
    let serialized = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|error| error.to_string())?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, serialized).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_save_file(path: &str) -> Result<SaveGame, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let save: SaveGame = ron::from_str(&raw).map_err(|error| error.to_string())?;
    if save.version != SAVE_VERSION {
        return Err(format!(
            "save version {} unsupported (expected {SAVE_VERSION})",
            save.version
        ));
    }
    Ok(save)
}

/// Ctrl+S: snapshot the running match to `saves/quicksave.ron`.
pub(crate) fn quicksave_hotkey(world: &mut World) {
    let keyboard = world.resource::<ButtonInput<KeyCode>>();
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyS) {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut battle_log = world.resource_mut::<BattleLog>();
        push_battle_log(
            &mut battle_log,
            t("网页版暂不支持存档", "Saving is desktop-only"),
            None,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let message = match collect_save_game(world) {
            Some(save) => match write_save_file(&save, QUICKSAVE_PATH) {
                Ok(()) => t("已存档 (Ctrl+L 读档)", "Game saved (Ctrl+L to load)").to_string(),
                Err(error) => format!("{}: {error}", t("存档失败", "Save failed")),
            },
            None => t("当前无法存档", "Cannot save right now").to_string(),
        };
        let mut battle_log = world.resource_mut::<BattleLog>();
        push_battle_log(&mut battle_log, message, None);
    }
}

/// Ctrl+L: load `saves/quicksave.ron` and restart the match into it.
pub(crate) fn quickload_hotkey(world: &mut World) {
    let keyboard = world.resource::<ButtonInput<KeyCode>>();
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyL) {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut battle_log = world.resource_mut::<BattleLog>();
        push_battle_log(
            &mut battle_log,
            t("网页版暂不支持读档", "Loading is desktop-only"),
            None,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match read_save_file(QUICKSAVE_PATH).and_then(|save| {
            match_settings_from_save(&save.settings)
                .map(|settings| (save, settings))
                .ok_or_else(|| "save references an unknown map".to_string())
        }) {
            Ok((save, settings)) => {
                *world.resource_mut::<MatchSetupSettings>() = settings;
                world.resource_mut::<PendingLoadedSave>().0 = Some(save);
                world
                    .resource_mut::<NextState<AppScreen>>()
                    .set(AppScreen::RestartingMatch);
            }
            Err(error) => {
                let message = format!("{}: {error}", t("读档失败", "Load failed"));
                let mut battle_log = world.resource_mut::<BattleLog>();
                push_battle_log(&mut battle_log, message, None);
            }
        }
    }
}

/// After the normal InMatch setup spawned a fresh world, replace it with the
/// saved one: despawn the spawned units/structures/resources and respawn from
/// the snapshot through the regular spawn functions, then restore the global
/// resources (economy, clock, cooldowns).
pub(crate) fn apply_loaded_save(world: &mut World) {
    let Some(save) = world.resource_mut::<PendingLoadedSave>().0.take() else {
        return;
    };

    let doomed: Vec<Entity> = world
        .query_filtered::<Entity, Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<SupplyCrate>,
        )>>()
        .iter(world)
        .collect();
    for entity in doomed {
        world.entity_mut(entity).despawn();
    }

    // Global resources.
    {
        let mut economies = world.resource_mut::<Economies>();
        for (index, saved) in save.economies.iter().enumerate() {
            let Some(economy) = economies.players.get_mut(index) else {
                continue;
            };
            economy.ore = saved.ore;
            economy.crystal = saved.crystal;
            economy.power_sabotage_remaining = saved.power_sabotage_remaining;
            for (slot, rank) in saved.production_veterancy_ranks.iter().enumerate() {
                if let Some(target) = economy.production_veterancy_ranks.get_mut(slot) {
                    *target = *rank;
                }
            }
        }
    }
    {
        let mut cooldowns = world.resource_mut::<SupportCooldowns>();
        for (index, row) in save.support_remaining.iter().enumerate() {
            cooldowns.ensure_team(Team::Player(index));
            if let Some(target) = cooldowns.remaining.get_mut(index) {
                for (slot, value) in row.iter().enumerate().take(target.len()) {
                    target[slot] = *value;
                }
            }
        }
        for (index, row) in save.support_charge_started.iter().enumerate() {
            if let Some(target) = cooldowns.initial_charge_started.get_mut(index) {
                for (slot, value) in row.iter().enumerate().take(target.len()) {
                    target[slot] = *value;
                }
            }
        }
    }
    {
        let mut match_state = world.resource_mut::<MatchState>();
        match_state.start_time_sec = save.clock_sec;
        match_state.enemy_units_destroyed = save.enemy_units_destroyed;
        match_state.enemy_structures_destroyed = save.enemy_structures_destroyed;
        match_state.units_lost = save.units_lost;
    }

    // Respawn the saved world through the regular spawn paths.
    let asset_server = world.resource::<AssetServer>().clone();
    let visible_team = world.resource::<VisiblePlayer>().team;
    let mut next_id = NextSpawnId(world.resource::<NextSpawnId>().0);
    let mut spawned_units: Vec<(Entity, SavedUnit)> = Vec::new();
    let mut spawned_structures: Vec<(Entity, SavedStructure)> = Vec::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        for saved in save.units {
            let Some(def) = registry::entity(&saved.id) else {
                continue;
            };
            let entity = spawn_unit(
                &mut commands,
                &asset_server,
                &mut next_id,
                def.id,
                Team::Player(saved.team),
                Vec3::from_array(saved.position),
                saved.rank,
                visible_team,
            );
            spawned_units.push((entity, saved));
        }
        for saved in save.structures {
            let Some(def) = registry::entity(&saved.id) else {
                continue;
            };
            let team = Team::Player(saved.team);
            let entity = if saved.construction.is_some() {
                spawn_structure_under_construction(
                    &mut commands,
                    &asset_server,
                    &mut next_id,
                    def.id,
                    team,
                    Vec3::from_array(saved.position),
                    None,
                    saved.yaw,
                    visible_team,
                )
            } else {
                spawn_structure_with_rotation(
                    &mut commands,
                    &asset_server,
                    &mut next_id,
                    def.id,
                    team,
                    visible_team,
                    Vec3::from_array(saved.position),
                    saved.yaw,
                )
            };
            spawned_structures.push((entity, saved));
        }
        for (kind, position, amount) in save.resources {
            let kind = if kind == ResourceKind::Crystal as u8 {
                ResourceKind::Crystal
            } else {
                ResourceKind::Ore
            };
            spawn_resource_node(
                &mut commands,
                &asset_server,
                kind,
                amount,
                Vec3::from_array(position),
            );
        }
    }
    queue.apply(world);
    world.resource_mut::<NextSpawnId>().0 = next_id.0;

    // Patch the per-entity live state the spawn functions don't take.
    for (entity, saved) in spawned_units {
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.current = saved.health.min(health.max);
        }
        if let Some(mut veterancy) = world.get_mut::<Veterancy>(entity) {
            veterancy.experience_points = saved.experience;
        }
        if let (Some((ore, crystal)), Some(mut cargo)) =
            (saved.cargo, world.get_mut::<ResourceCargo>(entity))
        {
            cargo.ore = ore;
            cargo.crystal = crystal;
        }
        if let Some(mut transform) = world.get_mut::<Transform>(entity) {
            transform.rotation = Quat::from_rotation_y(saved.yaw);
        }
    }
    for (entity, saved) in spawned_structures {
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.current = saved.health.min(health.max);
        }
        if let (Some((remaining, total)), Some(mut construction)) = (
            saved.construction,
            world.get_mut::<UnderConstruction>(entity),
        ) {
            construction.remaining = remaining;
            construction.total = total;
        }
        if let Some((target, mode)) = saved.rally
            && let Some(mut rally) = world.get_mut::<RallyPoint>(entity)
        {
            rally.target = Some(Vec3::from_array(target));
            rally.mode = if mode == RallyMode::AttackMove as u8 {
                RallyMode::AttackMove
            } else {
                RallyMode::Move
            };
        }
        if saved.garrison_count > 0
            && let Some(mut garrison) = world.get_mut::<Garrison>(entity)
        {
            garrison.count = saved.garrison_count.min(garrison.capacity);
        }
    }

    let mut battle_log = world.resource_mut::<BattleLog>();
    push_battle_log(&mut battle_log, t("读档完成", "Game loaded"), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_save() -> SaveGame {
        SaveGame {
            version: SAVE_VERSION,
            settings: SavedSettings {
                map_path: "res://source/match/maps/PlainAndSimple.tscn".to_string(),
                starting_ore: 32,
                starting_crystal: 16,
                active_teams: vec![true, true],
                player_factions: vec![0, 1],
                player_color_slots: vec![0, 1],
                player_controllers: vec![1, 2 + AiDifficulty::Normal as u8],
                player_spawn_slots: vec![0, 1],
                ai_difficulties: vec![2, 2],
                allied: vec![vec![true, false], vec![false, true]],
                startup_loadout: StartupLoadoutMode::GodotSkirmish as u8,
                visible_team: 0,
            },
            clock_sec: 123.5,
            enemy_units_destroyed: 4,
            enemy_structures_destroyed: 1,
            units_lost: 2,
            economies: vec![SavedEconomy {
                ore: 100,
                crystal: 50,
                power_sabotage_remaining: 0.0,
                production_veterancy_ranks: vec![0, 1],
            }],
            support_remaining: vec![vec![30.0; SupportPowerKind::ALL.len()]],
            support_charge_started: vec![vec![true; SupportPowerKind::ALL.len()]],
            units: vec![SavedUnit {
                id: "Worker".to_string(),
                team: 0,
                position: [1.0, 0.0, 2.0],
                yaw: 0.5,
                health: 4.0,
                rank: 1,
                experience: 10,
                cargo: Some((3, 0)),
            }],
            structures: vec![SavedStructure {
                id: "CommandCenter".to_string(),
                team: 0,
                position: [0.0, 0.0, 0.0],
                yaw: 0.0,
                health: 20.0,
                construction: None,
                rally: Some(([5.0, 0.0, 5.0], 1)),
                garrison_count: 0,
            }],
            resources: vec![(0, [8.0, 0.0, 8.0], 40)],
        }
    }

    #[test]
    fn quicksave_roundtrip_restores_world_through_restart_flow() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..40 {
            app.update();
        }
        // Distinctive state the roundtrip must carry over.
        app.world_mut().resource_mut::<Economies>().players[0].ore = 777;
        let save = collect_save_game(app.world_mut()).expect("running match must save");
        assert!(!save.units.is_empty(), "start units must be captured");
        assert!(!save.structures.is_empty(), "bases must be captured");
        assert!(
            !save.resources.is_empty(),
            "resource nodes must be captured"
        );
        assert_eq!(save.economies[0].ore, 777);

        // Load: the same path quickload_hotkey takes.
        *app.world_mut().resource_mut::<MatchSetupSettings>() =
            match_settings_from_save(&save.settings).expect("map resolves");
        app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(save.clone());
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::RestartingMatch);
        for _ in 0..40 {
            app.update();
        }

        let after = collect_save_game(app.world_mut()).expect("match running after load");
        assert_eq!(
            after.units.len(),
            save.units.len(),
            "unit count survives the load"
        );
        assert_eq!(
            after.structures.len(),
            save.structures.len(),
            "structure count survives the load"
        );
        assert_eq!(after.economies[0].ore, 777, "economy survives the load");
        assert!(
            (after.clock_sec - save.clock_sec).abs() < 5.0,
            "match clock resumes from the save"
        );
    }

    #[test]
    fn replay_jump_picks_neighbouring_keyframes() {
        let clocks = [0.0, 30.0, 60.0];
        // Mid-match: back goes to the last frame safely before now.
        assert_eq!(replay_jump_target(&clocks, 45.0, true), Some(1));
        assert_eq!(replay_jump_target(&clocks, 45.0, false), Some(2));
        // Right after a keyframe, back skips to the PREVIOUS one (epsilon).
        assert_eq!(replay_jump_target(&clocks, 30.5, true), Some(0));
        // Edges.
        assert_eq!(replay_jump_target(&clocks, 0.5, true), None);
        assert_eq!(replay_jump_target(&clocks, 61.0, false), None);
        assert_eq!(replay_jump_target(&[], 10.0, true), None);
    }

    #[test]
    fn replay_records_keyframe_zero_and_jumps_back_to_it() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..60 {
            app.update();
        }
        let frames = app.world().resource::<ReplayTimeline>().frames.len();
        assert_eq!(frames, 1, "keyframe 0 recorded at match start");
        let frame = app.world().resource::<ReplayTimeline>().frames[0].clone();
        assert!(frame.clock_sec < 1.0);

        // Jump back through the same pipeline the PageUp hotkey uses.
        *app.world_mut().resource_mut::<MatchSetupSettings>() =
            match_settings_from_save(&frame.settings).expect("map resolves");
        app.world_mut().resource_mut::<PendingLoadedSave>().0 = Some(frame.clone());
        app.world_mut()
            .resource_mut::<ReplayTimeline>()
            .next_record_at = f32::MAX;
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::RestartingMatch);
        for _ in 0..40 {
            app.update();
        }
        let state = app.world().resource::<MatchState>();
        assert_eq!(state.phase, MatchPhase::Running);
        assert!(
            state.start_time_sec < frame.clock_sec + 5.0,
            "clock rewound to the keyframe"
        );
        assert!(
            !app.world().resource::<ReplayTimeline>().frames.is_empty(),
            "timeline survives the jump for forward navigation"
        );
    }

    #[test]
    fn save_game_roundtrips_through_ron() {
        let save = sample_save();
        let text = ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default()).unwrap();
        let back: SaveGame = ron::from_str(&text).unwrap();
        assert_eq!(back, save);
    }

    #[test]
    fn saved_settings_rebuild_match_setup() {
        let save = sample_save();
        let settings = match_settings_from_save(&save.settings).expect("known map must resolve");
        assert_eq!(settings.starting_resources.ore, 32);
        assert_eq!(
            settings.player_controllers[1],
            SkirmishPlayerController::Ai(AiDifficulty::Normal)
        );
        assert_eq!(settings.visible_player.team, Team::Player(0));
        assert!(settings.team_relations.allied[0][0]);
    }
}

// ---------------------------------------------------------------------------
// Replay: an in-memory keyframe timeline (works on web too). Every
// REPLAY_KEYFRAME_INTERVAL_SEC of match clock a full snapshot is recorded;
// PageUp jumps back a keyframe, PageDown forward, both through the same
// restart+apply pipeline as quickload. Playing past a stale future keyframe
// truncates it (the timeline diverged).
// ---------------------------------------------------------------------------

pub(crate) const REPLAY_KEYFRAME_INTERVAL_SEC: f32 = 30.0;
/// Pressing "back" within this window of a keyframe skips to the one before it.
pub(crate) const REPLAY_JUMP_EPSILON_SEC: f32 = 1.5;

#[derive(Resource, Default)]
pub(crate) struct ReplayTimeline {
    pub(crate) frames: Vec<SaveGame>,
    pub(crate) next_record_at: f32,
}

/// Index of the keyframe a back/forward jump should land on, given the frame
/// clocks and the current match clock. `None` when there is nothing to jump to.
pub(crate) fn replay_jump_target(clocks: &[f32], now: f32, back: bool) -> Option<usize> {
    if back {
        clocks
            .iter()
            .rposition(|clock| *clock < now - REPLAY_JUMP_EPSILON_SEC)
    } else {
        clocks
            .iter()
            .position(|clock| *clock > now + REPLAY_JUMP_EPSILON_SEC)
    }
}

/// Records a keyframe whenever the match clock crosses the next interval; a
/// diverged future (frames recorded after the current clock, i.e. the player
/// rewound and kept playing) is truncated first.
pub(crate) fn record_replay_keyframes(world: &mut World) {
    if world.resource::<MatchState>().phase != MatchPhase::Running {
        return;
    }
    let clock = world.resource::<MatchState>().start_time_sec;
    {
        let timeline = world.resource::<ReplayTimeline>();
        if clock < timeline.next_record_at {
            return;
        }
    }
    let Some(save) = collect_save_game(world) else {
        return;
    };
    let mut timeline = world.resource_mut::<ReplayTimeline>();
    timeline
        .frames
        .retain(|frame| frame.clock_sec < clock - REPLAY_JUMP_EPSILON_SEC);
    timeline.frames.push(save);
    timeline.next_record_at = clock + REPLAY_KEYFRAME_INTERVAL_SEC;
}

/// Fresh matches (not loads) start a fresh timeline; a load keeps the timeline
/// so PageDown can still walk forward through it.
pub(crate) fn reset_replay_timeline_for_new_match(world: &mut World) {
    if world.resource::<PendingLoadedSave>().0.is_some() {
        return;
    }
    let mut timeline = world.resource_mut::<ReplayTimeline>();
    timeline.frames.clear();
    timeline.next_record_at = 0.0;
}

fn format_clock(seconds: f32) -> String {
    format!(
        "{}:{:02}",
        (seconds.max(0.0) / 60.0) as u32,
        (seconds.max(0.0) as u32) % 60
    )
}

/// PageUp / PageDown: jump one keyframe back / forward through the timeline.
pub(crate) fn replay_jump_hotkeys(world: &mut World) {
    let keyboard = world.resource::<ButtonInput<KeyCode>>();
    let back = keyboard.just_pressed(KeyCode::PageUp);
    let forward = keyboard.just_pressed(KeyCode::PageDown);
    if !back && !forward {
        return;
    }
    let now = world.resource::<MatchState>().start_time_sec;
    let target = {
        let timeline = world.resource::<ReplayTimeline>();
        let clocks: Vec<f32> = timeline
            .frames
            .iter()
            .map(|frame| frame.clock_sec)
            .collect();
        replay_jump_target(&clocks, now, back).map(|index| timeline.frames[index].clone())
    };
    match target {
        Some(frame) => {
            let message = format!(
                "{} {}",
                t("回放: 跳转到", "Replay: jumping to"),
                format_clock(frame.clock_sec)
            );
            if let Some(settings) = match_settings_from_save(&frame.settings) {
                *world.resource_mut::<MatchSetupSettings>() = settings;
                world.resource_mut::<PendingLoadedSave>().0 = Some(frame);
                // Recording resumes from the jump target on the next interval.
                world.resource_mut::<ReplayTimeline>().next_record_at = f32::MAX;
                world
                    .resource_mut::<NextState<AppScreen>>()
                    .set(AppScreen::RestartingMatch);
                let mut battle_log = world.resource_mut::<BattleLog>();
                push_battle_log(&mut battle_log, message, None);
            }
        }
        None => {
            let message = if back {
                t(
                    "回放: 已是最早的关键帧",
                    "Replay: already at the earliest keyframe",
                )
            } else {
                t("回放: 没有更晚的关键帧", "Replay: no later keyframe")
            };
            let mut battle_log = world.resource_mut::<BattleLog>();
            push_battle_log(&mut battle_log, message.to_string(), None);
        }
    }
}

/// After a replay jump (or quickload) lands, resume recording from the restored
/// clock so the next keyframe falls on the next interval.
pub(crate) fn resume_replay_recording_after_load(world: &mut World) {
    let clock = world.resource::<MatchState>().start_time_sec;
    let mut timeline = world.resource_mut::<ReplayTimeline>();
    if timeline.next_record_at == f32::MAX || timeline.next_record_at < clock {
        timeline.next_record_at = clock + REPLAY_KEYFRAME_INTERVAL_SEC;
    }
}
