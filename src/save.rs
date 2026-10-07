//! Quicksave / quickload (Ctrl+S / Ctrl+L, native only).
//!
//! Full local match snapshots preserve mission state, production, orders and
//! explored terrain. Saved entity keys are remapped to newly spawned entities;
//! runtime Entity values never survive a load. Unsupported schemas are rejected.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::*;

mod continuity;
pub(crate) use continuity::*;

// Web builds keep saves in memory only — the disk path is native-only.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) const SAVE_VERSION: u32 = 2;

/// Save loaded from disk, consumed by `apply_loaded_save` right after the match
/// world is (re)built by the normal InMatch setup chain.
#[derive(Resource, Default)]
pub(crate) struct PendingLoadedSave(pub(crate) Option<SaveGame>);

/// Consumed by mission startup to skip the briefing after restoring a match.
#[derive(Resource)]
pub(crate) struct LoadedMatchMarker;

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
    if world.resource::<MatchState>().phase != MatchPhase::Running
        || online_match_uses_command_transport(world.get_resource::<OnlineSession>())
    {
        return None;
    }
    let settings = world.resource::<MatchSetupSettings>();
    let saved_settings = SavedSettings {
        map_path: settings.map_path.to_string(),
        starting_ore: settings.starting_resources.ore,
        starting_crystal: settings.starting_resources.crystal,
        active_teams: settings.active_teams.clone(),
        // Runtime settings intentionally allow sparse vectors. Save their
        // effective values, rather than writing incomplete schema records.
        player_factions: player_teams(settings.active_teams.len())
            .map(|team| settings.player_faction(team).index())
            .collect(),
        // An absent color keeps the runtime's neutral-grey fallback. Do not
        // invent a palette color for a previously uncolored virtual player.
        player_color_slots: settings
            .player_color_slots
            .iter()
            .copied()
            .take(settings.active_teams.len())
            .collect(),
        player_controllers: {
            let fallback = skirmish_player_controllers_from_match_setup(settings);
            (0..settings.active_teams.len())
                .map(|index| {
                    controller_to_repr(
                        settings
                            .player_controllers
                            .get(index)
                            .copied()
                            .unwrap_or(fallback[index]),
                    )
                })
                .collect()
        },
        player_spawn_slots: player_teams(settings.active_teams.len())
            .map(|team| settings.player_spawn_slot(team))
            .collect(),
        ai_difficulties: player_teams(settings.active_teams.len())
            .map(|team| world.resource::<AiDifficultySettings>().difficulty(team) as u8)
            .collect(),
        allied: player_teams(settings.active_teams.len())
            .map(|a| {
                player_teams(settings.active_teams.len())
                    .map(|b| world.resource::<TeamRelations>().are_allied(a, b))
                    .collect()
            })
            .collect(),
        startup_loadout: settings.startup_loadout as u8,
        victory_condition: VictoryCondition::ALL
            .iter()
            .position(|mode| *mode == settings.victory_condition)
            .unwrap_or(0) as u8,
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
        Entity,
        &Unit,
        &Team,
        &Transform,
        &Health,
        Option<&Veterancy>,
        Option<&ResourceCargo>,
    )>();
    for (entity, unit, team, transform, health, veterancy, cargo) in unit_q.iter(world) {
        let Team::Player(team_index) = team else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        units.push(SavedUnit {
            key: entity.to_bits(),
            continuity: collect_unit_continuity(world, entity),
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
        Entity,
        &Structure,
        &Team,
        &Transform,
        &Health,
        Option<&UnderConstruction>,
        Option<&RallyPoint>,
        Option<&Garrison>,
    )>();
    for (entity, structure, team, transform, health, construction, rally, garrison) in
        structure_q.iter(world)
    {
        let team_index = match team {
            Team::Player(index) => *index,
            Team::Neutral => usize::MAX,
        };
        if health.current <= 0.0 {
            continue;
        }
        structures.push(SavedStructure {
            key: entity.to_bits(),
            effects: collect_effects(world, entity),
            weapon_remaining: world.get::<Weapon>(entity).map_or(0.0, |w| w.cooldown_left),
            known: world.get::<FogMemoryVisible>(entity).is_some()
                || world
                    .get::<VisibilityState>(entity)
                    .is_some_and(|v| v.visible),
            id: structure.id.to_string(),
            team: team_index,
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
            construction_cost: construction.map(|c| (c.cost.ore, c.cost.crystal)),
            free_worker_origin: construction
                .and_then(|c| c.free_worker_origin.map(|v| v.to_array())),
            repair_remaining: world
                .get::<ManualStructureRepair>(entity)
                .map(|r| r.points_remaining),
            garrison_count: garrison.map_or(0, |garrison| garrison.count),
        });
    }

    let mut resources = Vec::new();
    let mut resource_q = world.query::<(Entity, &ResourceNode, &Transform)>();
    for (entity, node, transform) in resource_q.iter(world) {
        if node.amount <= 0 {
            continue;
        }
        resources.push((
            entity.to_bits(),
            node.kind as u8,
            transform.translation.to_array(),
            node.amount,
        ));
    }

    let mission_id = world
        .resource::<ActiveMission>()
        .0
        .and_then(mission_by_index)
        .map(|m| m.id.to_string());
    let mission_triggers = world.resource::<MissionTriggerState>().fired.clone();
    let build_jobs = world
        .resource::<BuildQueue>()
        .0
        .iter()
        .filter_map(|job| {
            let (id, train) = match job.action {
                BuildAction::Train(id) => (id, true),
                BuildAction::Build(id) => (id, false),
                _ => return None,
            };
            Some(SavedBuildJob {
                team: job.team.economy_index()?,
                producer: job.producer_entity.to_bits(),
                producer_id: job.producer_id.to_string(),
                id: id.to_string(),
                train,
                timer: job.timer,
                origin: job.origin.to_array(),
                cost: (job.cost.ore, job.cost.crystal),
            })
        })
        .collect();
    let explored = world
        .get_resource::<FogOverlay>()
        .map_or_else(Vec::new, |fog| fog.explored.clone());
    let fog_remnants = world
        .query::<(&FogMemoryStructureRemnant, &Transform)>()
        .iter(world)
        .map(|(remnant, transform)| (transform.translation.to_array(), remnant.radius))
        .collect();
    let crates = world
        .query::<(Entity, &SupplyCrate, &Transform)>()
        .iter(world)
        .map(|(entity, item, transform)| SavedCrate {
            key: entity.to_bits(),
            position: transform.translation.to_array(),
            effect: match item.effect {
                SupplyCrateEffect::Resources => 0,
                SupplyCrateEffect::Repair => 1,
                SupplyCrateEffect::Veterancy => 2,
            },
            pickup_radius: item.pickup_radius,
            ore: item.resource_ore,
            crystal: item.resource_crystal,
            repair_radius: item.repair_radius,
            repair_amount: item.repair_amount,
        })
        .collect();
    let tutorial = world.get_resource::<TutorialState>().cloned();
    let ai = collect_ai_director(world.resource::<AiDirector>());
    let support_entities = collect_support_entities(world);
    let structures_lost = world.resource::<MatchState>().structures_lost;
    Some(SaveGame {
        tutorial,
        ai,
        support_entities,
        structures_lost,
        version: SAVE_VERSION,
        mission_id,
        mission_triggers,
        build_jobs,
        explored,
        fog_remnants,
        crates,
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
        victory_condition: VictoryCondition::ALL
            .get(save.victory_condition as usize)
            .copied()
            .unwrap_or_default(),
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
pub(crate) fn write_save_file(
    save: &SaveGame,
    path: impl AsRef<std::path::Path>,
) -> Result<(), String> {
    let path = path.as_ref();
    validate_save(save)?;
    let serialized = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("ron.tmp");
    std::fs::write(&temporary, serialized).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_save_file(path: impl AsRef<std::path::Path>) -> Result<SaveGame, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            t(
                "没有找到存档。请先保存对局。",
                "No saved game found. Save a match first.",
            )
        } else {
            t(
                "无法读取存档。请检查用户数据文件夹的权限。",
                "Cannot read the saved game. Check your user data folder permissions.",
            )
        }
        .to_string()
    })?;
    let save: SaveGame = ron::from_str(&raw).map_err(|_| {
        t(
            "存档已损坏，无法载入。当前对局未受影响。",
            "The saved game is damaged and cannot be loaded. Your current match is unchanged.",
        )
        .to_string()
    })?;
    if save.version != SAVE_VERSION {
        return Err(t(
            "此存档与当前版本不兼容。请开始新对局。",
            "This save is incompatible with this game version. Start a new match.",
        )
        .to_string());
    }
    validate_save(&save).map_err(|_| {
        t(
            "存档已损坏，无法载入。当前对局未受影响。",
            "The saved game is damaged and cannot be loaded. Your current match is unchanged.",
        )
        .to_string()
    })?;
    Ok(save)
}

/// Ctrl+S: snapshot the running match in the writable user data directory.
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
            Some(save) => match quicksave_path().and_then(|path| write_save_file(&save, path)) {
                Ok(()) => t("已存档 (Ctrl+L 读档)", "Game saved (Ctrl+L to load)").to_string(),
                Err(error) => format!("{}: {error}", t("存档失败", "Save failed")),
            },
            None => t("当前无法存档", "Cannot save right now").to_string(),
        };
        let mut battle_log = world.resource_mut::<BattleLog>();
        push_battle_log(&mut battle_log, message, None);
    }
}

/// Ctrl+L: load the user quicksave and restart the match into it.
pub(crate) fn quickload_hotkey(world: &mut World) {
    if online_match_uses_command_transport(world.get_resource::<OnlineSession>()) {
        return;
    }
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
        match quicksave_path().and_then(read_save_file).and_then(|save| {
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

    if let Err(error) = validate_save(&save) {
        push_battle_log(
            &mut world.resource_mut::<BattleLog>(),
            format!("{}: {error}", t("读档失败", "Load failed")),
            None,
        );
        return;
    }
    // A resumed match already passed its opening instructions. Keep the
    // optional Objectives button, but do not replay the startup overlay.
    world.resource_mut::<MatchBriefingState>().dismiss();
    if world.contains_resource::<TutorialState>() {
        *world.resource_mut::<TutorialState>() = save.tutorial.clone().unwrap_or_default();
    }

    let doomed: Vec<Entity> = world
        .query_filtered::<Entity, Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<SupplyCrate>,
            With<FogMemoryStructureRemnant>,
            With<PendingOrbitalStrike>,
            With<PendingParadrop>,
            With<TemporarySupportReveal>,
            With<SupportWarning>,
        )>>()
        .iter(world)
        .collect();
    for entity in doomed {
        world.entity_mut(entity).despawn();
    }

    world.resource_mut::<ActiveMission>().0 = save.mission_id.as_ref().and_then(|id| {
        CAMPAIGN_MISSIONS
            .iter()
            .position(|mission| mission.id == id)
    });
    world.resource_mut::<MissionTriggerState>().fired = save.mission_triggers.clone();
    if let Some(mut fog) = world.get_resource_mut::<FogOverlay>() {
        fog.explored = save.explored.clone();
    }
    restore_ai_director(&mut world.resource_mut::<AiDirector>(), &save.ai);
    let mut remap = std::collections::HashMap::new();
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
        match_state.structures_lost = save.structures_lost;
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
        restore_support_entities(&mut commands, &save.support_entities);
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
            remap.insert(saved.key, entity);
            spawned_units.push((entity, saved));
        }
        for saved in save.structures {
            let Some(def) = registry::entity(&saved.id) else {
                continue;
            };
            let team = if saved.team == usize::MAX {
                Team::Neutral
            } else {
                Team::Player(saved.team)
            };
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
            remap.insert(saved.key, entity);
            spawned_structures.push((entity, saved));
        }
        for (key, kind, position, amount) in save.resources {
            let kind = if kind == ResourceKind::Crystal as u8 {
                ResourceKind::Crystal
            } else {
                ResourceKind::Ore
            };
            let entity = spawn_resource_node(
                &mut commands,
                &asset_server,
                kind,
                amount,
                Vec3::from_array(position),
            );
            remap.insert(key, entity);
        }
        for item in &save.crates {
            let effect = match item.effect {
                1 => SupplyCrateEffect::Repair,
                2 => SupplyCrateEffect::Veterancy,
                _ => SupplyCrateEffect::Resources,
            };
            let entity = spawn_supply_crate(
                &mut commands,
                &asset_server,
                effect,
                Vec3::from_array(item.position),
            );
            commands.entity(entity).insert(SupplyCrate {
                effect,
                pickup_radius: item.pickup_radius,
                resource_ore: item.ore,
                resource_crystal: item.crystal,
                repair_radius: item.repair_radius,
                repair_amount: item.repair_amount,
            });
            remap.insert(item.key, entity);
        }
        for (position, radius) in &save.fog_remnants {
            spawn_fog_memory_structure_remnant(
                &mut commands,
                Some(&asset_server),
                Vec3::from_array(*position),
                *radius,
            );
        }
    }
    queue.apply(world);
    world.resource_mut::<NextSpawnId>().0 = next_id.0;

    // Patch the per-entity live state the spawn functions don't take.
    for (entity, saved) in spawned_units {
        restore_unit_continuity(world, entity, &saved.continuity, &remap);
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
        restore_effects(world, entity, &saved.effects);
        if let Some(mut weapon) = world.get_mut::<Weapon>(entity) {
            weapon.cooldown_left = saved.weapon_remaining;
        }
        if saved.known {
            world.entity_mut(entity).insert(FogMemoryVisible);
        }
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.current = saved.health.min(health.max);
        }
        if let (Some((remaining, total)), Some(mut construction)) = (
            saved.construction,
            world.get_mut::<UnderConstruction>(entity),
        ) {
            construction.remaining = remaining;
            construction.total = total;
            if let Some((ore, crystal)) = saved.construction_cost {
                construction.cost = registry::Cost { ore, crystal };
            }
            construction.free_worker_origin = saved.free_worker_origin.map(Vec3::from_array);
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
        if let Some(points_remaining) = saved.repair_remaining {
            world
                .entity_mut(entity)
                .insert(ManualStructureRepair { points_remaining });
        }
        if saved.garrison_count > 0
            && let Some(mut garrison) = world.get_mut::<Garrison>(entity)
        {
            garrison.count = saved.garrison_count.min(garrison.capacity);
        }
    }

    world.resource_mut::<BuildQueue>().0 = save
        .build_jobs
        .iter()
        .filter_map(|job| {
            Some(BuildJob {
                team: Team::Player(job.team),
                action: if job.train {
                    BuildAction::Train(registry::entity(&job.id)?.id)
                } else {
                    BuildAction::Build(registry::entity(&job.id)?.id)
                },
                producer_entity: *remap.get(&job.producer)?,
                producer_id: registry::entity(&job.producer_id)?.id,
                timer: job.timer,
                origin: Vec3::from_array(job.origin),
                cost: registry::Cost {
                    ore: job.cost.0,
                    crystal: job.cost.1,
                },
            })
        })
        .collect();
    world.insert_resource(LoadedMatchMarker);
    let mut battle_log = world.resource_mut::<BattleLog>();
    push_battle_log(&mut battle_log, t("读档完成", "Game loaded"), None);
}

#[cfg(test)]
mod tests;

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
    if online_match_uses_command_transport(world.get_resource::<OnlineSession>()) {
        return;
    }
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

/// Shared writable directory for desktop saves, progress and configuration.
/// Never falls back to the installed game directory.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn user_data_directory() -> Result<std::path::PathBuf, String> {
    let env_path = |name| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
    };
    #[cfg(target_os = "windows")]
    let root = env_path("LOCALAPPDATA").or_else(|| env_path("APPDATA"));
    #[cfg(target_os = "macos")]
    let root = env_path("HOME").map(|home| home.join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = env_path("XDG_DATA_HOME")
        .filter(|path| path.is_absolute())
        .or_else(|| env_path("HOME").map(|home| home.join(".local/share")));
    root.filter(|path| path.is_absolute())
        .map(|root| root.join("bevy-open-rts"))
        .ok_or_else(|| "A writable user data directory could not be located".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn quicksave_path() -> Result<std::path::PathBuf, String> {
    Ok(user_data_directory()?.join("quicksave.ron"))
}
