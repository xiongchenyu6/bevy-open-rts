//! Joint configuration and authoritative outcomes for real headless skirmishes.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkirmishAuditConfig {
    pub map: String,
    pub factions: [String; 2],
    pub difficulties: [String; 2],
    pub spawn_slots: [usize; 2],
    pub max_seconds: u32,
}

impl SkirmishAuditConfig {
    fn validate(
        &self,
    ) -> Result<
        (
            &'static SkirmishMapDef,
            [SkirmishFaction; 2],
            [AiDifficulty; 2],
        ),
        String,
    > {
        let map = SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == self.map || map.godot_path == self.map)
            .ok_or_else(|| format!("unknown shipped map '{}'", self.map))?;
        if self.max_seconds == 0 || self.max_seconds > 3600 {
            return Err("simulation limit must be 1..3600 seconds".into());
        }
        if self.spawn_slots[0] == self.spawn_slots[1]
            || self.spawn_slots.iter().any(|slot| *slot >= map.players)
        {
            return Err(format!(
                "spawn slots must be distinct and within 0..{}",
                map.players
            ));
        }
        Ok((
            map,
            [
                capture_parse_faction(&self.factions[0])?,
                capture_parse_faction(&self.factions[1])?,
            ],
            [
                capture_parse_ai_difficulty(&self.difficulties[0])?,
                capture_parse_ai_difficulty(&self.difficulties[1])?,
            ],
        ))
    }
}

/// Runs the shipped economy, AI, navigation, combat and victory systems.
/// No random seeds, grants or substitute simulation are introduced.
pub fn run_skirmish_audit(config: &SkirmishAuditConfig) -> Result<serde_json::Value, String> {
    let (map, factions, difficulties) = config.validate()?;
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        settings.map_path = map.godot_path;
        settings.starting_resources = StartingResources::new(32, 16);
        settings.startup_loadout = StartupLoadoutMode::GodotSkirmish;
        settings.victory_condition = VictoryCondition::Annihilation;
        settings.visible_player = VisiblePlayer::all_players(Team::Player(0));
        settings.active_teams = vec![false; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_controllers =
            vec![SkirmishPlayerController::None; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_factions = vec![SkirmishFaction::Alliance; MAX_SKIRMISH_LOBBY_SLOTS];
        settings.player_spawn_slots = default_player_spawn_slots();
        settings.team_relations = TeamRelations::default();
        for player in 0..2 {
            settings.active_teams[player] = true;
            settings.player_controllers[player] =
                SkirmishPlayerController::Ai(difficulties[player]);
            settings.player_factions[player] = factions[player];
            settings.player_spawn_slots[player] = config.spawn_slots[player];
            settings
                .ai_difficulties
                .set_difficulty(Team::Player(player), difficulties[player]);
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    let mut repair_trace = super::repair_trace::RepairTrace::open()?;
    let mut initial_positions = None;
    let mut updates = 0;
    let max_updates = config.max_seconds * 30 + 60;
    loop {
        app.update();
        if let Some(trace) = repair_trace.as_mut() {
            trace.observe(app.world_mut())?;
        }
        updates += 1;
        if initial_positions.is_none() {
            let world = app.world_mut();
            let mut query = world.query::<(&Team, &Structure, &Transform)>();
            let mut positions: Vec<_> = query.iter(world).filter_map(|(team, structure, transform)| {
                if structure.id != "CommandCenter" { return None; }
                let Team::Player(player) = team else { return None; };
                (*player < 2).then(|| serde_json::json!({"player":player,"position":transform.translation.to_array()}))
            }).collect();
            if positions.len() == 2 {
                positions.sort_by_key(|value| value["player"].as_u64().unwrap());
                initial_positions = Some(positions);
            }
        }
        let state = app.world().resource::<MatchState>();
        if !state.is_running()
            || state.start_time_sec >= config.max_seconds as f32
            || updates >= max_updates
        {
            break;
        }
    }
    if let Some(trace) = repair_trace {
        trace.finish()?;
    }
    let world = app.world_mut();
    let state = world.resource::<MatchState>();
    let completed = !state.is_running();
    let phase = match_phase_label(state.phase);
    let clock = state.start_time_sec;
    let result_reason = state.result_reason;
    let active_anchors = state.active_anchor_teams.clone();
    let remaining_anchors = state.remaining_anchors;
    let winners: Vec<usize> = if completed {
        active_anchors
            .iter()
            .enumerate()
            .filter_map(|(index, active)| active.then_some(index))
            .collect()
    } else {
        Vec::new()
    };
    let mut teams = vec![
        serde_json::json!({"player":0,"units":0,"structures":0,"pending_construction":0,"unit_types":{},"structure_types":{}}),
        serde_json::json!({"player":1,"units":0,"structures":0,"pending_construction":0,"unit_types":{},"structure_types":{}}),
    ];
    let mut units = world.query::<(&Team, &Health, &Unit)>();
    for (team, health, unit) in units.iter(world) {
        if let Team::Player(index) = team
            && *index < 2
            && health.current > 0.0
        {
            let count = teams[*index]["units"].as_u64().unwrap();
            teams[*index]["units"] = (count + 1).into();
            let count = teams[*index]["unit_types"][unit.id].as_u64().unwrap_or(0);
            teams[*index]["unit_types"][unit.id] = (count + 1).into();
        }
    }
    let mut structures = world.query::<(&Team, &Health, &Structure, Option<&UnderConstruction>)>();
    for (team, health, structure, construction) in structures.iter(world) {
        if let Team::Player(index) = team
            && *index < 2
            && health.current > 0.0
        {
            let count = teams[*index]["structures"].as_u64().unwrap();
            teams[*index]["structures"] = (count + 1).into();
            let count = teams[*index]["structure_types"][structure.id]
                .as_u64()
                .unwrap_or(0);
            teams[*index]["structure_types"][structure.id] = (count + 1).into();
            if construction.is_some() {
                let count = teams[*index]["pending_construction"].as_u64().unwrap();
                teams[*index]["pending_construction"] = (count + 1).into();
            }
        }
    }
    let economies = world.resource::<Economies>();
    for (index, team) in teams.iter_mut().enumerate() {
        let economy = economies.get(Team::Player(index));
        team["ore"] = economy.ore.into();
        team["crystal"] = economy.crystal.into();
    }
    let timeout_units = if completed {
        None
    } else {
        let mut query = world.query::<(
            Entity,
            &Team,
            &Unit,
            &Health,
            &Transform,
            Option<&MoveOrder>,
            Option<&AttackOrder>,
            Option<&AttackMoveOrder>,
            Option<&PlannedPath>,
            Option<&HarvestOrder>,
        )>();
        Some(query.iter(world).filter(|(_, _, _, health, ..)| health.current > 0.0)
            .map(|(entity, team, unit, health, position, movement, attack, attack_move, path, harvest)| {
                serde_json::json!({
                    "entity":entity.to_bits(), "team":team.economy_index(), "unit":unit.id,
                    "health":health.current, "position":position.translation.to_array(),
                    "move_target":movement.map(|order|order.target.to_array()),
                    "attack_target":attack.map(|order|order.target.to_bits()),
                    "attack_target_position":attack.and_then(|order|world.get::<Transform>(order.target))
                        .map(|target|target.translation.to_array()),
                    "attack_target_state":attack.map(|order|serde_json::json!({
                        "unit":world.get::<Unit>(order.target).map(|target|target.id),
                        "structure":world.get::<Structure>(order.target).map(|target|target.id),
                        "health":world.get::<Health>(order.target).map(|target|target.current),
                        "team":world.get::<Team>(order.target).map(|team|team.economy_index()),
                    })),
                    "weapon":world.get::<Weapon>(entity).map(|weapon|serde_json::json!({
                        "range":weapon.range, "damage":weapon.damage,
                        "cooldown_left":weapon.cooldown_left,
                        "can_attack_ground":weapon.can_attack_ground,
                        "can_attack_air":weapon.can_attack_air,
                    })),
                    "emp_remaining":world.get::<EmpDisabled>(entity).map(|emp|emp.remaining),
                    "attack_move_target":attack_move.map(|order|order.destination.to_array()),
                    "path_goal":path.map(|path|path.goal.to_array()),
                    "remaining_waypoints":path.map(|path|path.waypoints.iter().skip(path.next)
                        .map(|point|point.to_array()).collect::<Vec<_>>()),
                    "harvest_resource":harvest.and_then(|order|order.resource).map(|resource|resource.to_bits()),
                })
            }).collect::<Vec<_>>())
    };
    Ok(serde_json::json!({
        "schema":1, "config":config, "resolved_map_id":map.id, "resolved_map_path":map.godot_path,
        "active_player_slots":[0,1], "starting_resources":{"ore":32,"crystal":16},
        "startup_loadout":"GodotSkirmish", "victory_condition":"Annihilation",
        "timestep_seconds":1.0/30.0, "updates":updates, "clock_seconds":clock,
        "phase":phase, "result_reason":result_reason, "completed":completed, "timeout":!completed,
        "initial_command_center_positions":initial_positions,
        "winning_teams":winners, "active_anchor_teams":active_anchors,
        "remaining_anchors":remaining_anchors, "teams":teams, "timeout_units":timeout_units,
    }))
}
