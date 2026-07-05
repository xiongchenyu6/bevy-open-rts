//! The skirmish AI: difficulty tiers, the AI director (economy, training,
//! attack waves, support powers) and its target-scoring helpers.
//!
//! Pure move out of lib.rs (module split); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

pub(crate) const AI_SUPPLY_CRATE_COLLECTION_LIMIT: usize = 2;

pub(crate) const AI_REPAIR_MIN_MISSING_HITPOINT_RATIO: f32 = 0.25;

pub(crate) const AI_REPAIR_MAX_STARTS_PER_REFRESH: usize = 2;

pub(crate) const AI_REPAIR_REFRESH_INTERVAL_SECONDS: f32 = 0.5;

pub(crate) const AI_OPENING_ATTACK_GRACE_SECONDS: f32 = 60.0;

pub(crate) const AI_TECH_BUNKER_GARRISON_REFRESH_INTERVAL_SECONDS: f32 = 1.0;

pub(crate) const AI_TECH_BUNKER_GARRISON_SEARCH_RADIUS: f32 = 16.0;

pub(crate) const AI_SUPPORT_MIN_CLUSTER_TARGETS: usize = 2;

pub(crate) const AI_SUPPORT_ORBITAL_STRIKE_MIN_SCORE: f32 = 3.0;

pub(crate) const AI_SUPPORT_WEATHER_STORM_MIN_SCORE: f32 = 5.0;

pub(crate) const AI_SUPPORT_STRATEGIC_MISSILE_MIN_SCORE: f32 = 5.0;

pub(crate) const AI_SUPPORT_NANITE_REPAIR_MIN_MISSING_HP: f32 = 4.0;

pub(crate) const AI_SUPPORT_CHRONO_RELAY_MIN_MOBILE_UNITS: usize = 2;

pub(crate) const AI_SUPPORT_SHIELD_OVERDRIVE_MIN_SCORE: f32 = 2.0;

pub(crate) const AI_SUPPORT_SHIELD_OVERDRIVE_MOBILE_PRESSURE_BONUS: f32 = 12.0;

pub(crate) const AI_SUPPORT_SHIELD_PRESSURE_EXTRA_RADIUS: f32 = 4.0;

pub(crate) const AI_SUPPORT_SHIELD_PRESSURE_DISTANCE_WEIGHT: f32 = 0.3;

pub(crate) const AI_DRONE_SCOUT_SWITCH_MIN_SECONDS: f32 = 0.5;

pub(crate) const AI_DRONE_SCOUT_SWITCH_MAX_SECONDS: f32 = 1.0;

pub(crate) const AI_CONSTRUCTION_REFRESH_INTERVAL_SECONDS: f32 = 0.5;

pub(crate) const AI_CAPTURE_INTERVAL_SECONDS: f32 = 4.5;

pub(crate) const AI_CAPTURE_ENGINEER_LIMIT: usize = 1;

pub(crate) const AI_CAPTURE_NEUTRAL_TECH_TARGET_BONUS: f32 = 18.0;

pub(crate) const AI_SABOTEUR_INTERVAL_SECONDS: f32 = 5.0;

pub(crate) const AI_SABOTEUR_LIMIT: usize = 1;

pub(crate) const AI_SABOTEUR_ID: &str = "SaboteurInfiltrator";

pub(crate) const AI_SUPPORT_POWER_PRIORITY: [SupportPowerKind; 9] = [
    SupportPowerKind::EmpPulse,
    SupportPowerKind::NaniteRepairSwarm,
    SupportPowerKind::ShieldOverdrive,
    SupportPowerKind::ChronoRelay,
    SupportPowerKind::WeatherStorm,
    SupportPowerKind::StrategicMissile,
    SupportPowerKind::OrbitalStrike,
    SupportPowerKind::Paradrop,
    SupportPowerKind::RadarSweep,
];

#[derive(Clone, Copy)]
pub(crate) struct SupportPowerDef {
    pub(crate) requirements: &'static [&'static str],
    pub(crate) cooldown: f32,
    pub(crate) radius: f32,
    pub(crate) duration: f32,
    pub(crate) impact_delay: f32,
    pub(crate) requires_power: bool,
    pub(crate) damage: f32,
    pub(crate) damage_scale: f32,
    pub(crate) healing: f32,
    pub(crate) unit_paths: &'static [&'static str],
    pub(crate) initial_cooldown: f32,
}

#[derive(Component)]
pub(crate) struct AiAttackWaveMember;

#[derive(Clone, Copy, Default)]
pub(crate) struct AiProductionCounts {
    pub(crate) workers: usize,
    pub(crate) battle_units: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum AiStructureBuildKind {
    Economy,
    Defense,
    Offense,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AiDifficulty {
    Beginner,
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    const ALL: [Self; 4] = [Self::Beginner, Self::Easy, Self::Normal, Self::Hard];

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Self::Beginner => t("新手", "Beginner"),
            Self::Easy => t("简单", "Easy"),
            Self::Normal => t("普通", "Normal"),
            Self::Hard => t("困难", "Hard"),
        }
    }
}

pub(crate) const AI_OFFENSE_STRUCTURE_PRIORITY: &[&str] =
    &["VehicleFactory", "Barracks", "AircraftFactory"];

pub(crate) const HUMAN_AI_DEFENSE_PRIORITY: &[&str] = &[
    "AntiGroundTurret",
    "AntiAirTurret",
    "TeslaFenceSegment",
    "ArcCoilDefenseTower",
    "LanceBeamDefenseTower",
    "PrismDefenseObelisk",
    "RailCannonBunker",
];
pub(crate) const HUMAN_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("AntiGroundTurret", 1),
    ("AntiAirTurret", 1),
    ("TeslaFenceSegment", 2),
    ("ArcCoilDefenseTower", 1),
    ("LanceBeamDefenseTower", 1),
    ("PrismDefenseObelisk", 1),
    ("RailCannonBunker", 1),
];
pub(crate) const DEMON_AI_DEFENSE_PRIORITY: &[&str] = &[
    "AntiAirTurret",
    "AntiGroundTurret",
    "ArcCoilDefenseTower",
    "LanceBeamDefenseTower",
];
pub(crate) const DEMON_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("AntiAirTurret", 2),
    ("AntiGroundTurret", 2),
    ("ArcCoilDefenseTower", 1),
    ("LanceBeamDefenseTower", 1),
];
pub(crate) const CHAOS_AI_DEFENSE_PRIORITY: &[&str] = &[
    "TeslaFenceSegment",
    "ArcCoilDefenseTower",
    "PrismDefenseObelisk",
    "RailCannonBunker",
];
pub(crate) const CHAOS_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[
    ("TeslaFenceSegment", 2),
    ("ArcCoilDefenseTower", 1),
    ("PrismDefenseObelisk", 1),
    ("RailCannonBunker", 1),
];

pub(crate) const HUMAN_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: HUMAN_AI_PRODUCTION_PRIORITY,
    defense_priority: HUMAN_AI_DEFENSE_PRIORITY,
    defense_limits: HUMAN_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    opening_attack_grace: AI_OPENING_ATTACK_GRACE_SECONDS,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
    tesla_fence_limit_bonus: 0,
};

pub(crate) const DEMON_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: DEMON_AI_PRODUCTION_PRIORITY,
    defense_priority: DEMON_AI_DEFENSE_PRIORITY,
    defense_limits: DEMON_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    opening_attack_grace: AI_OPENING_ATTACK_GRACE_SECONDS,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
    tesla_fence_limit_bonus: 0,
};

pub(crate) const CHAOS_AI_PROFILE: TeamAiProfile = TeamAiProfile {
    production_priority: CHAOS_AI_PRODUCTION_PRIORITY,
    defense_priority: CHAOS_AI_DEFENSE_PRIORITY,
    defense_limits: CHAOS_AI_DEFENSE_LIMITS,
    expected_command_centers: 1,
    expected_workers: 3,
    expected_refineries: 1,
    expected_battlegroups: 2,
    expected_units_in_battlegroup: 4,
    active_offense_enabled: true,
    opening_attack_grace: AI_OPENING_ATTACK_GRACE_SECONDS,
    capture_enabled: true,
    saboteur_enabled: true,
    support_powers_enabled: true,
    production_interval: 4.0,
    attack_interval: 6.5,
    build_interval: 11.0,
    capture_interval: AI_CAPTURE_INTERVAL_SECONDS,
    saboteur_interval: AI_SABOTEUR_INTERVAL_SECONDS,
    support_interval: 3.5,
    defense_limit_bonus: 0,
    tesla_fence_limit_bonus: 0,
};

pub(crate) const BEGINNER_AI_PRODUCTION_PRIORITY: &[&str] = &[];
pub(crate) const BEGINNER_AI_DEFENSE_PRIORITY: &[&str] = &[];
pub(crate) const BEGINNER_AI_DEFENSE_LIMITS: &[(&str, usize)] = &[];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameAppMode {
    Interactive,
    Headless,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct AiDroneScout {
    pub(crate) last_target: Option<Entity>,
    pub(crate) cooldown_remaining: f32,
}

#[derive(Resource)]
pub(crate) struct AiDirector {
    pub(crate) production_timer: Vec<f32>,
    pub(crate) production_cursor: Vec<usize>,
    pub(crate) attack_timer: Vec<f32>,
    pub(crate) opening_attack_grace_applied: Vec<bool>,
    pub(crate) build_timer: Vec<f32>,
    pub(crate) construction_timer: Vec<f32>,
    pub(crate) capture_timer: Vec<f32>,
    pub(crate) saboteur_timer: Vec<f32>,
    pub(crate) support_timer: Vec<f32>,
    pub(crate) repair_timer: Vec<f32>,
}

impl AiDirector {
    pub(crate) fn ensure_team(&mut self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        if self.production_timer.len() <= index {
            self.production_timer.resize(index + 1, 2.5);
            self.production_cursor.resize(index + 1, 0);
            self.attack_timer
                .resize(index + 1, AI_OPENING_ATTACK_GRACE_SECONDS);
            self.opening_attack_grace_applied.resize(index + 1, false);
            self.build_timer.resize(index + 1, 8.0);
            self.construction_timer.resize(index + 1, 0.0);
            self.capture_timer.resize(index + 1, 3.0);
            self.saboteur_timer.resize(index + 1, 4.0);
            self.support_timer.resize(index + 1, 6.0);
            self.repair_timer.resize(index + 1, 0.0);
        }
        Some(index)
    }
}

impl Default for AiDirector {
    fn default() -> Self {
        Self {
            production_timer: Vec::new(),
            production_cursor: Vec::new(),
            attack_timer: Vec::new(),
            opening_attack_grace_applied: Vec::new(),
            build_timer: Vec::new(),
            construction_timer: Vec::new(),
            capture_timer: Vec::new(),
            saboteur_timer: Vec::new(),
            support_timer: Vec::new(),
            repair_timer: Vec::new(),
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub(crate) struct AiDifficultySettings {
    pub(crate) players: Vec<AiDifficulty>,
}

impl AiDifficultySettings {
    pub(crate) fn difficulty(&self, team: Team) -> AiDifficulty {
        team.economy_index()
            .and_then(|index| self.players.get(index).copied())
            .unwrap_or(AiDifficulty::Normal)
    }

    pub(crate) fn set_difficulty(&mut self, team: Team, difficulty: AiDifficulty) {
        if let Some(index) = team.economy_index() {
            if self.players.len() <= index {
                self.players.resize(index + 1, AiDifficulty::Normal);
            }
            self.players[index] = difficulty;
        }
    }

    pub(crate) fn default_ai_difficulty(&self, player_team: Team) -> AiDifficulty {
        active_ai_teams(Some(player_team), None)
            .next()
            .map(|team| self.difficulty(team))
            .unwrap_or(AiDifficulty::Normal)
    }
}

impl Default for AiDifficultySettings {
    fn default() -> Self {
        let _available_difficulties = AiDifficulty::ALL;
        Self {
            players: Vec::new(),
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct AiDirectorResources<'w> {
    pub(crate) terrain: Res<'w, TerrainHeightField>,
    pub(crate) map_bounds: Res<'w, MapBounds>,
    pub(crate) economies: ResMut<'w, Economies>,
    pub(crate) next_id: ResMut<'w, NextSpawnId>,
    pub(crate) director: ResMut<'w, AiDirector>,
    pub(crate) ai_settings: Res<'w, AiDifficultySettings>,
    pub(crate) player_factions: Res<'w, PlayerFactions>,
    pub(crate) active_teams: Option<Res<'w, ActiveTeams>>,
    pub(crate) relations: Res<'w, TeamRelations>,
    pub(crate) support_cooldowns: ResMut<'w, SupportCooldowns>,
    pub(crate) battle_log: ResMut<'w, BattleLog>,
    pub(crate) audio_feedback: ResMut<'w, AudioFeedback>,
}

pub(crate) fn ai_profile_requests_offensive_combat_units(profile: &TeamAiProfile) -> bool {
    profile.production_priority.iter().any(|id| {
        registry::entity(id).is_some_and(|def| {
            def.weapon.is_some() && def.speed > 0.0 && !matches!(def.id, "Worker")
        })
    })
}

pub(crate) fn ai_siege_drill_should_deploy(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    health: &Health,
    emp: Option<&EmpDisabled>,
    attack_order: Option<&AttackOrder>,
    targets: &Query<
        (
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &MovementDomain,
            &Health,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> bool {
    if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
        return false;
    }
    let Some(order) = attack_order else {
        return false;
    };
    let Ok((
        _structure,
        target_team,
        target_transform,
        _target_selectable,
        target_domain,
        target_health,
    )) = targets.get(order.target)
    else {
        return false;
    };
    if target_health.current <= 0.0 || *target_team == team || *target_team == Team::Neutral {
        return false;
    }
    if !can_attack_domain(weapon, *target_domain) {
        return false;
    }
    xz_distance(position, target_transform.translation) <= SIEGE_DRILL_DEPLOYED_ATTACK_RANGE
}

pub(crate) fn ai_drone_scout_delay(drone: Entity, target: Entity) -> f32 {
    let range = AI_DRONE_SCOUT_SWITCH_MAX_SECONDS - AI_DRONE_SCOUT_SWITCH_MIN_SECONDS;
    let fraction = (entity_pair_hash(drone, target) % 1_000) as f32 / 1_000.0;
    AI_DRONE_SCOUT_SWITCH_MIN_SECONDS + range * fraction
}

pub(crate) fn ai_supply_crate_distance_to_team_units(
    team: Team,
    crate_position: Vec3,
    team_anchors: &Query<(&Team, &Transform), Or<(With<Unit>, With<Structure>)>>,
) -> f32 {
    let mut closest_distance = f32::INFINITY;
    for (anchor_team, transform) in team_anchors {
        if *anchor_team == team {
            closest_distance =
                closest_distance.min(xz_distance(crate_position, transform.translation));
        }
    }
    closest_distance
}

pub(crate) fn ai_support_power_available(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    support_cooldowns: &SupportCooldowns,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    support_cooldowns.ready(team, power)
        && (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

pub(crate) fn ai_support_power_targets(
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Vec<SupportPowerTargetSnapshot> {
    let mut targets = units
        .iter()
        .map(
            |(entity, team, transform, _selectable, health, unit)| SupportPowerTargetSnapshot {
                entity,
                team: *team,
                position: transform.translation,
                health: *health,
                mobile: unit.speed > 0.0,
            },
        )
        .collect::<Vec<_>>();
    targets.extend(structures.iter().map(
        |(entity, _structure, team, transform, health, _under_construction)| {
            SupportPowerTargetSnapshot {
                entity,
                team: *team,
                position: transform.translation,
                health: *health,
                mobile: false,
            }
        },
    ));
    targets
}

pub(crate) fn ai_support_power_target(
    team: Team,
    power: SupportPowerKind,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Vec3> {
    let def = power.definition();
    match power {
        SupportPowerKind::NaniteRepairSwarm => best_repair_swarm_position(
            team,
            units,
            def.radius,
            def.healing,
            AI_SUPPORT_NANITE_REPAIR_MIN_MISSING_HP,
        ),
        SupportPowerKind::EmpPulse => best_mobile_unit_cluster_position(
            team,
            units,
            def.radius,
            false,
            relations,
            AI_SUPPORT_MIN_CLUSTER_TARGETS,
        ),
        SupportPowerKind::ShieldOverdrive => best_shield_overdrive_position(
            team,
            units,
            def.radius,
            relations,
            AI_SUPPORT_SHIELD_OVERDRIVE_MIN_SCORE,
        ),
        SupportPowerKind::ChronoRelay => best_mobile_unit_cluster_position(
            team,
            units,
            def.radius,
            true,
            relations,
            AI_SUPPORT_CHRONO_RELAY_MIN_MOBILE_UNITS,
        ),
        SupportPowerKind::WeatherStorm => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_WEATHER_STORM_MIN_SCORE,
        ),
        SupportPowerKind::StrategicMissile => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_STRATEGIC_MISSILE_MIN_SCORE,
        ),
        SupportPowerKind::OrbitalStrike => best_scored_strike_position(
            team,
            units,
            structures,
            relations,
            def.radius,
            AI_SUPPORT_ORBITAL_STRIKE_MIN_SCORE,
        ),
        SupportPowerKind::RadarSweep | SupportPowerKind::Paradrop => {
            any_enemy_support_target_position(team, relations, units, structures)
        }
    }
}

pub(crate) fn ai_strike_unit_score(unit: &Unit) -> f32 {
    let mut score = 1.0;
    if let Some(def) = registry::entity(unit.id) {
        if def.weapon.is_some() {
            score += 1.0;
        }
        score += ai_resource_score(def.cost) * 0.5;
    }
    score
}

pub(crate) fn ai_strike_structure_score(structure: &Structure) -> f32 {
    let mut score = 3.5;
    if let Some(def) = registry::entity(structure.id) {
        if def.weapon.is_some() {
            score += 1.0;
        }
        score += ai_resource_score(def.cost);
    }
    score
}

pub(crate) fn ai_resource_score(cost: registry::Cost) -> f32 {
    ((cost.ore + cost.crystal) as f32 / 8.0).min(2.0)
}

pub(crate) fn ai_support_unit_side_matches(
    team: Team,
    unit_team: Team,
    friendly: bool,
    relations: &TeamRelations,
) -> bool {
    if friendly {
        unit_team == team
    } else {
        relations.are_enemies(team, unit_team)
    }
}

pub(crate) fn ai_needs_more_anti_air_units(
    team: Team,
    units: impl IntoIterator<Item = (&'static str, Team)>,
) -> bool {
    let mut enemy_air_units = 0usize;
    let mut anti_air_responses = 0usize;
    for (unit_id, unit_team) in units {
        if unit_team == Team::Neutral {
            continue;
        }
        if unit_team == team {
            if ai_unit_can_attack_air(unit_id) {
                anti_air_responses += 1;
            }
        } else if ai_unit_is_air(unit_id) {
            enemy_air_units += 1;
        }
    }
    enemy_air_units > 0 && anti_air_responses < enemy_air_units
}

pub(crate) fn ai_unit_is_air(unit_id: &str) -> bool {
    registry::entity(unit_id).is_some_and(|def| matches!(def.domain, registry::MoveDomain::Air))
}

pub(crate) fn ai_unit_can_attack_air(unit_id: &str) -> bool {
    registry::entity(unit_id)
        .and_then(|def| def.weapon)
        .is_some_and(|weapon| weapon.can_attack_air)
}

pub(crate) fn ai_battle_unit_id(unit_id: &str) -> bool {
    if matches!(unit_id, "Worker" | AI_SABOTEUR_ID) {
        return false;
    }
    registry::entity(unit_id).is_some_and(|def| {
        def.speed > 0.0
            && (def.weapon.is_some()
                || def.repair_rate > 0.0
                || def.healing_rate > 0.0
                || def.support_shield_radius > 0.0
                || def.mine_deploy_radius > 0.0)
    })
}

pub(crate) fn ai_battlegroup_target_units(profile: &TeamAiProfile) -> usize {
    profile.expected_battlegroups * profile.expected_units_in_battlegroup
}

pub(crate) fn ai_battlegroup_candidate_allowed(
    candidate: &'static str,
    profile: &TeamAiProfile,
    counts: AiProductionCounts,
) -> bool {
    if ai_training_is_economy_request(candidate) || !ai_battle_unit_id(candidate) {
        return true;
    }
    let target_units = ai_battlegroup_target_units(profile);
    target_units > 0 && counts.battle_units < target_units
}

pub(crate) fn ai_battlegroup_repair_target(
    team: Team,
    repairer: Entity,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) -> Option<Entity> {
    let repairer_position = units
        .get(repairer)
        .ok()
        .map(|(_, _, transform, _, _, _)| transform.translation);
    let mut best = None;
    let mut best_missing_ratio = 0.0;
    let mut best_distance = f32::MAX;

    for (entity, unit_team, transform, _selectable, health, unit) in units {
        if entity == repairer
            || *unit_team != team
            || health.current <= 0.0
            || health.current >= health.max
            || !ai_battle_unit_id(unit.id)
        {
            continue;
        }

        let missing_ratio = if health.max > 0.0 {
            1.0 - (health.current / health.max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let distance = repairer_position
            .map(|position| xz_distance(position, transform.translation))
            .unwrap_or(0.0);
        if missing_ratio > best_missing_ratio
            || (missing_ratio == best_missing_ratio && distance < best_distance)
        {
            best = Some(entity);
            best_missing_ratio = missing_ratio;
            best_distance = distance;
        }
    }

    best
}

pub(crate) fn ai_director(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut resources: AiDirectorResources,
    structures: Query<StructurePrereqItem<'_>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    support_units: Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    capture_structures: Query<CaptureStructureTargetItem<'_>, With<Structure>>,
    ai_repair_structures: Query<AiRepairStructureItem<'_>, With<Structure>>,
    targets: Query<(Entity, &Team, &Transform), With<Health>>,
) {
    let delta = time.delta_secs();
    let player_team = visible_player_team(visible_player.as_deref());
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, resources.active_teams.as_deref()) {
        let Some(idx) = resources.director.ensure_team(team) else {
            continue;
        };
        let faction = resources.player_factions.slot_faction(team);
        let profile =
            faction_ai_profile_for_difficulty(faction, resources.ai_settings.difficulty(team));
        if !resources.director.opening_attack_grace_applied[idx] {
            resources.director.attack_timer[idx] = profile.opening_attack_grace;
            resources.director.opening_attack_grace_applied[idx] = true;
        }

        resources.director.support_timer[idx] -= delta;
        if profile.support_powers_enabled && resources.director.support_timer[idx] <= 0.0 {
            resources.director.support_timer[idx] = profile.support_interval;
            let _ = try_activate_ai_support_power(
                team,
                player_team,
                &mut commands,
                &resources.economies,
                &mut resources.support_cooldowns,
                &mut resources.battle_log,
                &mut resources.audio_feedback,
                &resources.relations,
                &structures,
                &support_units,
                &capture_structures,
            );
        } else if !profile.support_powers_enabled {
            resources.director.support_timer[idx] = profile.support_interval;
        }

        resources.director.repair_timer[idx] -= delta;
        if resources.director.repair_timer[idx] <= 0.0 {
            resources.director.repair_timer[idx] = AI_REPAIR_REFRESH_INTERVAL_SECONDS;
            let _ = repair_ai_damaged_structures(
                &mut commands,
                team,
                &ai_repair_structures,
                &mut resources.economies,
            );
        }

        resources.director.production_timer[idx] -= delta;
        let mut production_refresh_due = false;
        let mut trained_during_priority_refresh = false;
        if resources.director.production_timer[idx] <= 0.0 {
            production_refresh_due = true;
            resources.director.production_timer[idx] = profile.production_interval;
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let economy_snapshot = resources.economies.get(team).clone();
            if let Some(id) = next_ai_economy_train(
                team,
                faction,
                &profile,
                &structures,
                &economy_snapshot,
                production_counts,
            ) {
                trained_during_priority_refresh = try_spawn_ai_trained_unit(
                    &mut commands,
                    &asset_server,
                    &mut resources.economies,
                    &mut resources.next_id,
                    team,
                    faction,
                    id,
                    &structures,
                    *resources.map_bounds,
                    player_team,
                );
            }
        }

        resources.director.build_timer[idx] -= delta;
        if resources.director.build_timer[idx] <= 0.0 {
            resources.director.build_timer[idx] = profile.build_interval;
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let next_structure = next_ai_economy_structure_for_faction(
                team,
                faction,
                &profile,
                &structures,
                production_counts,
            )
            .map(|id| (id, AiStructureBuildKind::Economy))
            // Build production (offense) structures before defense, so the AI
            // always has a Barracks/VehicleFactory to train an army from instead
            // of spending its whole economy on turrets.
            .or_else(|| {
                profile
                    .active_offense_enabled
                    .then(|| next_ai_offense_structure_for_faction(team, faction, &structures))
                    .flatten()
                    .map(|id| (id, AiStructureBuildKind::Offense))
            })
            .or_else(|| {
                next_ai_defense_for_faction(team, faction, &profile, &structures)
                    .map(|id| (id, AiStructureBuildKind::Defense))
            });
            if let Some((id, build_kind)) = next_structure
                && let Some(def) = registry::entity(id)
                && requirements_met(def, team, &structures)
                && let Some(origin) =
                    ai_structure_build_origin(team, build_kind, &structures, &targets)
                && resources.economies.get_mut(team).spend(def.cost)
            {
                let spawn_at = ai_structure_build_position(
                    team,
                    origin,
                    id,
                    build_kind,
                    resources.next_id.0,
                    &targets,
                    *resources.map_bounds,
                    &resources.terrain,
                );
                spawn_structure_under_construction_for_faction(
                    &mut commands,
                    &asset_server,
                    &mut resources.next_id,
                    id,
                    team,
                    spawn_at,
                    (id == "Refinery").then_some(origin),
                    0.0,
                    player_team,
                    faction,
                );
            }
        }

        if production_refresh_due && !trained_during_priority_refresh {
            let needs_anti_air =
                ai_needs_more_anti_air_units(team, units.iter().map(|item| (item.1.id, *item.2)));
            let production_counts =
                ai_production_counts(team, units.iter().map(|item| (item.1, item.2)));
            let economy_snapshot = resources.economies.get(team).clone();
            let next_training = next_ai_train(
                team,
                faction,
                &profile,
                &structures,
                &economy_snapshot,
                &mut resources.director.production_cursor[idx],
                production_counts,
                needs_anti_air,
            );
            if let Some(id) = next_training
                && !ai_training_is_economy_request(id)
            {
                let _ = try_spawn_ai_trained_unit(
                    &mut commands,
                    &asset_server,
                    &mut resources.economies,
                    &mut resources.next_id,
                    team,
                    faction,
                    id,
                    &structures,
                    *resources.map_bounds,
                    player_team,
                );
            }
        }

        resources.director.attack_timer[idx] -= delta;
        if profile.active_offense_enabled && resources.director.attack_timer[idx] <= 0.0 {
            resources.director.attack_timer[idx] = profile.attack_interval;
            if let Some(target) = nearest_enemy_entity(team, team_home(team), &targets) {
                for (
                    entity,
                    unit,
                    unit_team,
                    _transform,
                    order_queue,
                    move_order,
                    follow_order,
                    attack_order,
                    capture_order,
                    garrison_order,
                    harvest_order,
                    repair_order,
                    construct_order,
                    attack_move_order,
                    patrol_order,
                ) in &units
                {
                    if *unit_team != team
                        || !ai_battle_unit_id(unit.id)
                        || order_queue.is_some_and(|queue| !queue.orders.is_empty())
                        || has_active_orders_in_query(
                            move_order,
                            follow_order,
                            attack_order,
                            capture_order,
                            garrison_order,
                            harvest_order,
                            repair_order,
                            construct_order,
                            attack_move_order,
                            patrol_order,
                        )
                    {
                        continue;
                    }
                    assign_ai_attack_wave_order(
                        &mut commands,
                        team,
                        entity,
                        unit,
                        target,
                        &support_units,
                    );
                }
            }
        } else if !profile.active_offense_enabled {
            resources.director.attack_timer[idx] = profile.attack_interval;
        }

        resources.director.capture_timer[idx] -= delta;
        if profile.capture_enabled && resources.director.capture_timer[idx] <= 0.0 {
            resources.director.capture_timer[idx] = profile.capture_interval;
            run_ai_capture_logic(
                team,
                faction,
                &mut commands,
                &asset_server,
                &mut resources.economies,
                &mut resources.next_id,
                player_team,
                &structures,
                &units,
                &capture_structures,
            );
        } else if !profile.capture_enabled {
            resources.director.capture_timer[idx] = profile.capture_interval;
        }

        resources.director.saboteur_timer[idx] -= delta;
        if profile.saboteur_enabled && resources.director.saboteur_timer[idx] <= 0.0 {
            resources.director.saboteur_timer[idx] = profile.saboteur_interval;
            run_ai_saboteur_logic(
                team,
                faction,
                &mut commands,
                &asset_server,
                &mut resources.economies,
                &mut resources.next_id,
                player_team,
                &structures,
                &units,
                &capture_structures,
            );
        } else if !profile.saboteur_enabled {
            resources.director.saboteur_timer[idx] = profile.saboteur_interval;
        }
    }
}

pub(crate) fn ai_training_is_economy_request(id: &str) -> bool {
    id == "Worker"
}

pub(crate) fn ai_saboteur_target_has_value(
    team: Team,
    victim_team: Team,
    saboteur_def: &registry::EntityDef,
    target_def: &registry::EntityDef,
    economies: &Economies,
) -> bool {
    if let Some(producer_id) = target_def.infiltration_production_veterancy_producer
        && saboteur_def.infiltration_production_veterancy_rank > 0
        && economies.get(team).production_veterancy_rank(producer_id)
            < saboteur_def.infiltration_production_veterancy_rank
    {
        return true;
    }
    if target_def.is_infiltration_resource_target {
        let victim = economies.get(victim_team);
        if victim.ore > 0 || victim.crystal > 0 {
            return true;
        }
    }
    if target_def.is_infiltration_power_sabotage_target
        && economies.get(victim_team).power_sabotage_remaining <= 0.0
    {
        return true;
    }
    false
}

pub(crate) fn ai_saboteur_target_score(
    victim_team: Team,
    target_def: &registry::EntityDef,
    target_position: Vec3,
    origin: Vec3,
    economies: &Economies,
) -> f32 {
    let mut score = match target_def.id {
        "Barracks" => 120.0,
        "VehicleFactory" => 116.0,
        "AircraftFactory" => 112.0,
        "AdvancedReactorPlant" => 106.0,
        "PowerReactor" => 104.0,
        "OrePurifier" => 96.0,
        "Refinery" => 88.0,
        "CommandCenter" => 82.0,
        _ => 30.0,
    };
    score += (target_def.cost.ore + target_def.cost.crystal) as f32;
    if target_def.is_infiltration_resource_target {
        let victim = economies.get(victim_team);
        score += (victim.ore + victim.crystal) as f32 * 0.5;
    }
    if target_def.is_infiltration_power_sabotage_target {
        score += target_def.power_delta.max(0) as f32 * 0.5;
    }
    score - xz_distance(origin, target_position) * 0.06
}

pub(crate) fn ai_capture_priority(structure_id: &str) -> Option<f32> {
    match structure_id {
        "CommandCenter" => Some(120.0),
        "TechLab" => Some(105.0),
        "RoboticsBay" => Some(95.0),
        "VehicleFactory" => Some(90.0),
        "AircraftFactory" => Some(88.0),
        "TechAirport" => Some(87.0),
        "TechOilDerrick" => Some(86.0),
        "TechHospital" => Some(85.0),
        "TechBunker" => Some(84.75),
        "TechRepairDepot" => Some(84.5),
        "Barracks" => Some(84.0),
        "Refinery" => Some(78.0),
        "PowerReactor" => Some(72.0),
        "RadarUplink" => Some(70.0),
        "LanceBeamDefenseTower" => Some(62.0),
        "ArcCoilDefenseTower" => Some(58.0),
        "AntiGroundTurret" => Some(48.0),
        "AntiAirTurret" => Some(44.0),
        _ => None,
    }
}

pub(crate) fn ai_production_counts<'a>(
    team: Team,
    units: impl IntoIterator<Item = (&'a Unit, &'a Team)>,
) -> AiProductionCounts {
    let mut counts = AiProductionCounts::default();
    for (unit, unit_team) in units {
        if *unit_team != team {
            continue;
        }
        if can_unit_construct_structures(unit) {
            counts.workers += 1;
        }
        if ai_battle_unit_id(unit.id) {
            counts.battle_units += 1;
        }
    }
    counts
}

pub(crate) fn ai_economy_candidate_allowed(
    candidate: &'static str,
    profile: &TeamAiProfile,
    counts: AiProductionCounts,
) -> bool {
    match candidate {
        "Worker" => counts.workers < profile.expected_workers,
        _ => true,
    }
}

pub(crate) fn ai_structure_count(
    team: Team,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
    constructed_only: bool,
) -> usize {
    structures
        .iter()
        .filter(|(structure, structure_team, _, under_construction)| {
            **structure_team == team
                && structure.id == structure_id
                && (!constructed_only || structure_is_constructed(*under_construction))
        })
        .count()
}

#[allow(dead_code)]
pub(crate) fn ai_economy_structure_allowed(
    team: Team,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    ai_economy_structure_allowed_for_faction(
        team,
        SkirmishFaction::from_team(team),
        structure_id,
        structures,
    )
}

pub(crate) fn ai_economy_structure_allowed_for_faction(
    team: Team,
    faction: SkirmishFaction,
    structure_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let Some(faction) = faction_def(faction) else {
        return false;
    };
    let Some(def) = registry::entity(structure_id) else {
        return false;
    };
    faction.can_construct(structure_id) && requirements_met(def, team, structures)
}

pub(crate) fn ai_structure_build_origin(
    team: Team,
    build_kind: AiStructureBuildKind,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    match build_kind {
        AiStructureBuildKind::Economy => ai_economy_structure_origin(team, structures),
        AiStructureBuildKind::Defense => ai_frontline_command_origin(team, structures, targets),
        AiStructureBuildKind::Offense => ai_frontline_command_origin(team, structures, targets),
    }
}

pub(crate) fn ai_economy_structure_origin(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<Vec3> {
    structures.iter().find_map(
        |(structure, structure_team, transform, under_construction)| {
            (*structure_team == team
                && structure.id == "CommandCenter"
                && structure_is_constructed(under_construction))
            .then_some(transform.translation)
        },
    )
}

pub(crate) fn ai_structure_build_position(
    team: Team,
    origin: Vec3,
    structure_id: &'static str,
    build_kind: AiStructureBuildKind,
    seed: u32,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
) -> Vec3 {
    // Retry with shifted seeds when a pick lands on a cliff edge or ramp; the
    // AI base plateau is flat, so a nearby legal spot always exists.
    let radius = registry::entity(structure_id).map_or(2.0, |def| def.radius);
    for attempt in 0..6u32 {
        let candidate = ai_structure_build_position_unchecked(
            team,
            origin,
            structure_id,
            build_kind,
            seed + attempt * 101,
            targets,
            bounds,
        );
        if terrain_site_is_buildable(terrain, candidate, radius) {
            return candidate;
        }
    }
    origin
}

pub(crate) fn ai_structure_build_position_unchecked(
    team: Team,
    origin: Vec3,
    structure_id: &'static str,
    build_kind: AiStructureBuildKind,
    seed: u32,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
    bounds: MapBounds,
) -> Vec3 {
    match build_kind {
        AiStructureBuildKind::Economy => free_position_in_bounds(origin, seed + 19, 5.0, bounds),
        AiStructureBuildKind::Defense => {
            ai_defense_position(team, origin, structure_id, seed + 7, targets, bounds)
        }
        AiStructureBuildKind::Offense => {
            ai_defense_position(team, origin, structure_id, seed + 13, targets, bounds)
        }
    }
}

pub(crate) fn ai_defense_position(
    team: Team,
    origin: Vec3,
    structure_id: &'static str,
    seed: u32,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
    bounds: MapBounds,
) -> Vec3 {
    let Some(enemy_position) = nearest_enemy_position(team, origin, targets) else {
        return free_position_in_bounds(origin, seed, 5.0, bounds);
    };
    let direction = Vec3::new(
        enemy_position.x - origin.x,
        0.0,
        enemy_position.z - origin.z,
    )
    .try_normalize()
    .unwrap_or(Vec3::Z);
    ai_defense_position_in_direction_in_bounds(origin, direction, structure_id, seed, bounds)
}

pub(crate) fn ai_defense_position_in_direction_in_bounds(
    origin: Vec3,
    direction: Vec3,
    structure_id: &'static str,
    seed: u32,
    bounds: MapBounds,
) -> Vec3 {
    let direction = Vec3::new(direction.x, 0.0, direction.z)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let lateral = Vec3::new(-direction.z, 0.0, direction.x);
    let structure_radius = registry::entity(structure_id).map_or(0.75, |def| def.radius);
    let command_radius = registry::entity("CommandCenter").map_or(1.8, |def| def.radius);
    let forward = command_radius + structure_radius * 3.0 + 1.5;
    let side_step = (structure_radius * 2.6).max(1.6);
    let side_slot = match seed % 5 {
        0 => 0.0,
        1 => side_step,
        2 => -side_step,
        3 => side_step * 2.0,
        _ => -side_step * 2.0,
    };
    let candidate = origin + direction * forward + lateral * side_slot;
    bounds.clamp_ground_point(candidate, 1.0)
}

pub(crate) fn ai_structure_under_profile_limit(
    team: Team,
    structure_id: &str,
    structures: &Query<StructurePrereqItem<'_>>,
    profile: &TeamAiProfile,
) -> bool {
    ai_structure_under_max(
        team,
        structure_id,
        structures,
        ai_structure_profile_limit(structure_id, profile),
    )
}

pub(crate) fn ai_structure_profile_limit(structure_id: &str, profile: &TeamAiProfile) -> usize {
    let base = profile
        .defense_limits
        .iter()
        .find_map(|(id, max)| (*id == structure_id).then_some(*max))
        .unwrap_or(0);
    if base == 0 {
        return 0;
    }
    let bonus = if structure_id == "TeslaFenceSegment" {
        profile.tesla_fence_limit_bonus
    } else {
        profile.defense_limit_bonus
    };
    base.saturating_add(bonus)
}

pub(crate) fn ai_structure_under_max(
    team: Team,
    structure_id: &str,
    structures: &Query<StructurePrereqItem<'_>>,
    max: usize,
) -> bool {
    if max == 0 {
        return false;
    }
    let count = structures
        .iter()
        .filter(|(structure, structure_team, _, under_construction)| {
            structure_is_constructed(*under_construction)
                && **structure_team == team
                && structure.id == structure_id
        })
        .count();
    count < max
}

pub(crate) fn ai_frontline_command_origin(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    let mut fallback = None;
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (structure, structure_team, transform, under_construction) in structures {
        if *structure_team != team
            || !structure_is_constructed(under_construction)
            || structure.id != "CommandCenter"
        {
            continue;
        }
        fallback.get_or_insert(transform.translation);
        let Some(enemy_position) = nearest_enemy_position(team, transform.translation, targets)
        else {
            continue;
        };
        let distance = xz_distance(transform.translation, enemy_position);
        if distance < best_distance {
            best_distance = distance;
            best = Some(transform.translation);
        }
    }
    best.or(fallback)
}

#[allow(dead_code)]
pub(crate) fn ai_production_origin(
    team: Team,
    product_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<(&'static str, Vec3)> {
    ai_production_origin_for_faction(
        team,
        SkirmishFaction::from_team(team),
        product_id,
        structures,
    )
}

pub(crate) fn ai_production_origin_for_faction(
    team: Team,
    faction: SkirmishFaction,
    product_id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<(&'static str, Vec3)> {
    let faction = faction_def(faction)?;
    for (structure, structure_team, transform, under_construction) in structures {
        if *structure_team == team
            && structure_is_constructed(under_construction)
            && faction.can_produce(structure.id, product_id)
        {
            return Some((structure.id, transform.translation));
        }
    }
    None
}

pub(crate) fn skirmish_ai_difficulties_from_controllers(
    controllers: &[SkirmishPlayerController],
) -> AiDifficultySettings {
    let mut settings = AiDifficultySettings::default();
    for (index, controller) in controllers.iter().copied().enumerate() {
        if let Some(difficulty) = controller.ai_difficulty() {
            settings.set_difficulty(Team::Player(index), difficulty);
        }
    }
    settings
}

#[derive(Clone, Copy)]
pub(crate) struct TeamAiProfile {
    pub(crate) production_priority: &'static [&'static str],
    pub(crate) defense_priority: &'static [&'static str],
    pub(crate) defense_limits: &'static [(&'static str, usize)],
    pub(crate) expected_command_centers: usize,
    pub(crate) expected_workers: usize,
    pub(crate) expected_refineries: usize,
    pub(crate) expected_battlegroups: usize,
    pub(crate) expected_units_in_battlegroup: usize,
    pub(crate) active_offense_enabled: bool,
    pub(crate) opening_attack_grace: f32,
    pub(crate) capture_enabled: bool,
    pub(crate) saboteur_enabled: bool,
    pub(crate) support_powers_enabled: bool,
    pub(crate) production_interval: f32,
    pub(crate) attack_interval: f32,
    pub(crate) build_interval: f32,
    pub(crate) capture_interval: f32,
    pub(crate) saboteur_interval: f32,
    pub(crate) support_interval: f32,
    pub(crate) defense_limit_bonus: usize,
    pub(crate) tesla_fence_limit_bonus: usize,
}

pub(crate) const HUMAN_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "ScoutRover",
    "RocketInfantry",
    "InterceptorVTOL",
    "MirageScoutTank",
    "FieldMedic",
    "BomberVTOL",
    "FlameAssaultBuggy",
    "ShieldTrooper",
    "RocketGunship",
    "DroneMineLayer",
    "FlakRocketTeam",
    "HeavyBombardmentAirship",
    "TeslaCrawlerMk2",
    "FlakRocketTeamMk2",
    "SiegeAirship",
    "RocketTrooperRobot",
    "HeavyMachinegunTrooper",
    "ModularMissileCarrier",
    "ShockTrooper",
    "JammerVehicle",
    "GrenadierTrooper",
    "AntiAirWalker",
    "MortarTeam",
    "FlakHoverTank",
    "CryoSprayer",
    "MobileRepairCrawler",
    "SniperScout",
    "MobileShieldProjector",
    "RailSniperTeam",
    "LongbowMissileCrawler",
    "PhaseSaboteur",
    "SiegeArtilleryVehicle",
    "PulseRifleCommando",
    "SiegeDrillTank",
    "TacticalOfficer",
    "LanceBeamTank",
    "RailgunTank",
    "HammerSiegeTank",
    "HeavySiegeWalker",
    "RailArtilleryWalker",
];

pub(crate) const DEMON_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "Tank",
    "LightRifleInfantry",
    "Helicopter",
    "FlameAssaultBuggy",
    "RocketInfantry",
    "BomberVTOL",
    "ScoutRover",
    "HeavyMachinegunTrooper",
    "HeavyBombardmentAirship",
    "FlakHoverTank",
    "ShockTrooper",
    "SiegeAirship",
    "SiegeArtilleryVehicle",
    "GrenadierTrooper",
    "SiegeDrillTank",
    "MortarTeam",
    "HammerSiegeTank",
    "PulseRifleCommando",
    "HeavySiegeWalker",
];

pub(crate) const CHAOS_AI_PRODUCTION_PRIORITY: &[&str] = &[
    "MirageScoutTank",
    "TeslaCrawlerMk2",
    "ShieldTrooper",
    "InterceptorVTOL",
    "ScoutRover",
    "FieldMedic",
    "Drone",
    "DroneMineLayer",
    "FlakRocketTeam",
    "RocketGunship",
    "FlakRocketTeamMk2",
    "HeavyBombardmentAirship",
    "RocketTrooperRobot",
    "CryoSprayer",
    "SiegeAirship",
    "ModularMissileCarrier",
    "SniperScout",
    "JammerVehicle",
    "RailSniperTeam",
    "AntiAirWalker",
    "PhaseSaboteur",
    "MobileRepairCrawler",
    "TacticalOfficer",
    "MobileShieldProjector",
    "LongbowMissileCrawler",
    "LanceBeamTank",
    "RailgunTank",
    "RailArtilleryWalker",
];

#[allow(dead_code)]
pub(crate) fn team_ai_profile(team: Team) -> &'static TeamAiProfile {
    faction_ai_profile(SkirmishFaction::from_team(team))
}

pub(crate) fn faction_ai_profile(faction: SkirmishFaction) -> &'static TeamAiProfile {
    match faction {
        SkirmishFaction::Alliance => &HUMAN_AI_PROFILE,
        SkirmishFaction::Demon => &DEMON_AI_PROFILE,
        SkirmishFaction::Chaos => &CHAOS_AI_PROFILE,
    }
}

#[allow(dead_code)]
pub(crate) fn team_ai_profile_for_difficulty(
    team: Team,
    difficulty: AiDifficulty,
) -> TeamAiProfile {
    faction_ai_profile_for_difficulty(SkirmishFaction::from_team(team), difficulty)
}

pub(crate) fn faction_ai_profile_for_difficulty(
    faction: SkirmishFaction,
    difficulty: AiDifficulty,
) -> TeamAiProfile {
    let mut profile = *faction_ai_profile(faction);
    match difficulty {
        AiDifficulty::Beginner => {
            profile.production_priority = BEGINNER_AI_PRODUCTION_PRIORITY;
            profile.defense_priority = BEGINNER_AI_DEFENSE_PRIORITY;
            profile.defense_limits = BEGINNER_AI_DEFENSE_LIMITS;
            profile.expected_command_centers = 1;
            profile.expected_workers = 3;
            profile.expected_refineries = 1;
            profile.expected_battlegroups = 0;
            profile.expected_units_in_battlegroup = 0;
            profile.active_offense_enabled = false;
            profile.opening_attack_grace = 120.0;
            profile.capture_enabled = false;
            profile.saboteur_enabled = false;
            profile.support_powers_enabled = false;
            profile.production_interval = 7.5;
            profile.build_interval = 14.0;
            profile.attack_interval = 12.0;
            profile.capture_interval = 12.0;
            profile.saboteur_interval = 12.0;
            profile.support_interval = 12.0;
            profile.defense_limit_bonus = 0;
            profile.tesla_fence_limit_bonus = 0;
        }
        AiDifficulty::Easy => {
            profile.defense_priority = BEGINNER_AI_DEFENSE_PRIORITY;
            profile.defense_limits = BEGINNER_AI_DEFENSE_LIMITS;
            profile.expected_command_centers = 1;
            profile.expected_workers = 2;
            profile.expected_refineries = 1;
            profile.expected_battlegroups = 1;
            profile.expected_units_in_battlegroup = 3;
            // Easy attacks — gently. Total passivity made it identical to
            // Beginner in threat; now it sends a small wave every so often
            // after a long settle-in grace, teaching the player to defend.
            profile.active_offense_enabled = true;
            profile.opening_attack_grace = 150.0;
            profile.capture_enabled = false;
            profile.saboteur_enabled = false;
            profile.support_powers_enabled = false;
            profile.production_interval = 6.5;
            profile.attack_interval = 18.0;
            profile.build_interval = 13.0;
            profile.capture_interval = AI_CAPTURE_INTERVAL_SECONDS + 2.0;
            profile.saboteur_interval = AI_SABOTEUR_INTERVAL_SECONDS + 3.0;
            profile.support_interval = 5.5;
            profile.defense_limit_bonus = 0;
            profile.tesla_fence_limit_bonus = 0;
        }
        AiDifficulty::Normal => {}
        AiDifficulty::Hard => {
            // Tempo-only buff over Normal, tuned by duels: extra economy
            // buildings (2nd CC/refinery), turret/fence bonuses and a too-short
            // attack interval all made Hard WEAKER than Normal — early money
            // left the army, and frequent wave re-orders churned units. Hard is
            // Normal with faster production/build ticks, one more worker and
            // bigger battlegroups.
            profile.expected_workers = 4;
            profile.expected_battlegroups = 3;
            profile.expected_units_in_battlegroup = 5;
            // Later than Normal's 60s on purpose: the duel meta has a strong
            // defender's advantage, so Hard absorbs Normal's opening wave with
            // its faster production, then counterattacks in force.
            profile.opening_attack_grace = 75.0;
            profile.production_interval = 3.0;
            profile.build_interval = 9.0;
            profile.capture_interval = (AI_CAPTURE_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.saboteur_interval = (AI_SABOTEUR_INTERVAL_SECONDS - 1.0).max(1.0);
            profile.support_interval = 2.5;
        }
    }
    if matches!(difficulty, AiDifficulty::Beginner) {
        debug_assert!(!ai_profile_requests_offensive_combat_units(&profile));
    }
    profile
}

pub(crate) fn repair_ai_damaged_structures(
    commands: &mut Commands,
    team: Team,
    structures: &Query<AiRepairStructureItem<'_>, With<Structure>>,
    economies: &mut Economies,
) -> usize {
    let mut candidates = Vec::new();
    for (entity, structure, structure_team, health, repair, under_construction) in structures {
        if *structure_team != team
            || repair.is_some()
            || !structure_is_constructed(under_construction)
            || health.current <= 0.0
            || health.current >= health.max
            || health.max <= 0.0
        {
            continue;
        }
        let missing_ratio = (missing_structure_hitpoints(health) / health.max).clamp(0.0, 1.0);
        if missing_ratio < AI_REPAIR_MIN_MISSING_HITPOINT_RATIO {
            continue;
        }
        candidates.push((health.ratio(), entity, structure.id, *health));
    }
    candidates.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut started = 0usize;
    for (_ratio, entity, structure_id, health) in candidates {
        if started >= AI_REPAIR_MAX_STARTS_PER_REFRESH {
            break;
        }
        let Some(def) = registry::entity(structure_id) else {
            continue;
        };
        let cost = structure_repair_cost(def, &health);
        if !economies.get(team).can_afford(cost) {
            continue;
        }
        if !economies.get_mut(team).spend(cost) {
            continue;
        }
        commands.entity(entity).try_insert(ManualStructureRepair {
            points_remaining: missing_structure_hitpoints(&health),
        });
        started += 1;
    }
    started
}

pub(crate) fn update_ai_siege_drill_deploy_mode(
    mut commands: Commands,
    visible_player: Option<Res<VisiblePlayer>>,
    mut drills: Query<
        (
            Entity,
            &Team,
            &mut Unit,
            &mut HoldPosition,
            &mut Weapon,
            &mut VisionRadius,
            &Transform,
            Option<&DeployedSiegeMode>,
            &Health,
            Option<&EmpDisabled>,
            Option<&AttackOrder>,
        ),
        With<Unit>,
    >,
    targets: Query<
        (
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &MovementDomain,
            &Health,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (
        entity,
        team,
        mut unit,
        mut hold,
        mut weapon,
        mut vision,
        transform,
        deployed,
        health,
        emp,
        attack_order,
    ) in &mut drills
    {
        if *team == player_team || unit.id != "SiegeDrillTank" {
            continue;
        }
        let desired_deployed = ai_siege_drill_should_deploy(
            *team,
            transform.translation,
            &weapon,
            health,
            emp,
            attack_order,
            &targets,
        );
        if desired_deployed == deployed.is_some() {
            continue;
        }
        apply_siege_drill_deploy_mode(
            &mut commands,
            entity,
            &mut unit,
            &mut hold,
            &mut weapon,
            &mut vision,
            deployed.copied(),
            desired_deployed,
            false,
        );
    }
}

pub(crate) fn auto_assign_ai_construction_workers(
    mut commands: Commands,
    time: Res<Time>,
    mut director: ResMut<AiDirector>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    workers: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Unit,
            &Health,
            Option<&OrderQueue>,
        ),
        (With<Unit>, IdleUnitOrderFilter),
    >,
    active_constructors: Query<(&Team, &Health), (With<Unit>, With<ConstructOrder>)>,
    structures: Query<
        (Entity, &Team, &Transform, &Health),
        (With<Structure>, With<UnderConstruction>),
    >,
) {
    let delta = time.delta_secs();
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, active_teams.as_deref()) {
        let Some(idx) = director.ensure_team(team) else {
            continue;
        };
        director.construction_timer[idx] -= delta;
        if director.construction_timer[idx] > 0.0 {
            continue;
        }
        director.construction_timer[idx] = AI_CONSTRUCTION_REFRESH_INTERVAL_SECONDS;
        if active_constructors
            .iter()
            .any(|(worker_team, health)| *worker_team == team && health.current > 0.0)
        {
            continue;
        }

        let mut idle_workers = Vec::new();
        for (
            worker_entity,
            worker_team,
            worker_transform,
            worker_unit,
            worker_health,
            order_queue,
        ) in &workers
        {
            if *worker_team != team
                || worker_health.current <= 0.0
                || !can_unit_construct_structures(worker_unit)
                || order_queue.is_some_and(|queue| !queue.orders.is_empty())
            {
                continue;
            }
            idle_workers.push((worker_entity, worker_transform.translation));
        }
        let unfinished_structures = structures
            .iter()
            .filter_map(
                |(structure_entity, structure_team, structure_transform, health)| {
                    (*structure_team == team && health.current > 0.0)
                        .then_some((structure_entity, structure_transform.translation))
                },
            )
            .collect::<Vec<_>>();
        let best_assignment =
            closest_construction_assignment(&idle_workers, &unfinished_structures);

        if let Some((worker_entity, structure_entity)) = best_assignment {
            issue_unit_order(
                &mut commands,
                worker_entity,
                UnitQueuedOrder::Construct(structure_entity),
            );
        }
    }
}

pub(crate) fn auto_assign_ai_supply_crate_collectors(
    mut commands: Commands,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    crates: Query<(Entity, &Transform, &SupplyCrate)>,
    team_anchors: Query<(&Team, &Transform), Or<(With<Unit>, With<Structure>)>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            &Health,
            &MovementDomain,
            &HoldPosition,
            Option<&Weapon>,
            Option<&OrderQueue>,
        ),
        With<Unit>,
    >,
    busy_units: Query<Entity, ActiveUnitOrderFilter>,
) {
    let crate_snapshots = crates
        .iter()
        .map(|(entity, transform, supply_crate)| {
            (entity, transform.translation, supply_crate.effect)
        })
        .collect::<Vec<_>>();
    if crate_snapshots.is_empty() {
        return;
    }

    for team in active_ai_teams(
        controlled_player_team(visible_player.as_deref()),
        active_teams.as_deref(),
    ) {
        let mut assignments = 0usize;
        let mut assigned_units = Vec::new();
        let mut preferred_crates = crate_snapshots.clone();
        preferred_crates.sort_by(|a, b| {
            let a_distance = ai_supply_crate_distance_to_team_units(team, a.1, &team_anchors);
            let b_distance = ai_supply_crate_distance_to_team_units(team, b.1, &team_anchors);
            a_distance.total_cmp(&b_distance)
        });

        for (_, crate_position, _effect) in preferred_crates {
            if assignments >= AI_SUPPLY_CRATE_COLLECTION_LIMIT {
                break;
            }
            let mut best = None;
            let mut best_score = f32::MAX;
            for (
                entity,
                unit,
                unit_team,
                transform,
                health,
                domain,
                hold_position,
                weapon,
                order_queue,
            ) in &units
            {
                if *unit_team != team
                    || health.current <= 0.0
                    || *domain != MovementDomain::Terrain
                    || hold_position.enabled
                    || weapon.is_none()
                    || assigned_units.contains(&entity)
                {
                    continue;
                }
                if busy_units.contains(entity)
                    || order_queue.is_some_and(|queue| !queue.orders.is_empty())
                {
                    continue;
                }
                let scout_bonus = if unit.id == "ScoutRover" { -20.0 } else { 0.0 };
                let score = xz_distance(transform.translation, crate_position) + scout_bonus;
                if score < best_score {
                    best = Some(entity);
                    best_score = score;
                }
            }

            if let Some(entity) = best {
                issue_unit_order(&mut commands, entity, UnitQueuedOrder::Move(crate_position));
                assigned_units.push(entity);
                assignments += 1;
            }
        }
    }
}

pub(crate) fn update_ai_drone_scouting(
    mut commands: Commands,
    time: Res<Time>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    relations: Res<TeamRelations>,
    mut drones: Query<
        (
            Entity,
            &Team,
            &Unit,
            &Health,
            Option<&OrderQueue>,
            Option<&mut AiDroneScout>,
        ),
        (With<Unit>, IdleUnitOrderFilter, Without<Selected>),
    >,
    targets: Query<(Entity, &Team, &Transform, &Health, &Unit), With<Unit>>,
) {
    let controlled_team = controlled_player_team(visible_player.as_deref());
    let delta = time.delta_secs();
    for (drone_entity, drone_team, drone_unit, drone_health, order_queue, scout_state) in
        &mut drones
    {
        if drone_unit.id != "Drone"
            || drone_health.current <= 0.0
            || controlled_team == Some(*drone_team)
            || !team_is_active(*drone_team, active_teams.as_deref())
            || order_queue.is_some_and(|queue| !queue.orders.is_empty())
        {
            continue;
        }

        let last_target = scout_state.as_ref().and_then(|state| state.last_target);
        let Some((target, target_position)) = choose_ai_drone_scout_target(
            *drone_team,
            drone_entity,
            last_target,
            &relations,
            &targets,
        ) else {
            continue;
        };

        if let Some(mut state) = scout_state {
            state.cooldown_remaining -= delta;
            if state.cooldown_remaining > 0.0 {
                continue;
            }
            state.last_target = Some(target);
            state.cooldown_remaining = ai_drone_scout_delay(drone_entity, target);
        } else {
            commands.entity(drone_entity).try_insert(AiDroneScout {
                last_target: Some(target),
                cooldown_remaining: ai_drone_scout_delay(drone_entity, target),
            });
        }

        issue_unit_order(
            &mut commands,
            drone_entity,
            UnitQueuedOrder::Move(target_position),
        );
    }
}

pub(crate) fn choose_ai_drone_scout_target(
    drone_team: Team,
    drone_entity: Entity,
    last_target: Option<Entity>,
    relations: &TeamRelations,
    targets: &Query<(Entity, &Team, &Transform, &Health, &Unit), With<Unit>>,
) -> Option<(Entity, Vec3)> {
    let mut best_new_target = None;
    let mut best_new_score = u64::MAX;
    let mut best_any_target = None;
    let mut best_any_score = u64::MAX;
    for (target_entity, target_team, target_transform, target_health, target_unit) in targets {
        if target_health.current <= 0.0
            || target_unit.speed <= 0.0
            || !relations.are_enemies(drone_team, *target_team)
        {
            continue;
        }
        let score = entity_pair_hash(drone_entity, target_entity);
        if score < best_any_score {
            best_any_score = score;
            best_any_target = Some((target_entity, target_transform.translation));
        }
        if Some(target_entity) != last_target && score < best_new_score {
            best_new_score = score;
            best_new_target = Some((target_entity, target_transform.translation));
        }
    }
    best_new_target.or(best_any_target)
}

pub(crate) fn update_ai_tech_bunker_garrisons(
    mut commands: Commands,
    time: Res<Time>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    mut refresh_timer: Local<Vec<f32>>,
    bunkers: Query<AiOpenBunkerItem<'_>, (With<Structure>, Without<Unit>)>,
    units: Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) {
    let delta = time.delta_secs();
    let controlled_team = controlled_player_team(visible_player.as_deref());
    for team in active_ai_teams(controlled_team, active_teams.as_deref()) {
        let idx = team.index();
        if refresh_timer.len() <= idx {
            refresh_timer.resize(idx + 1, 0.0);
        }
        refresh_timer[idx] -= delta;
        if refresh_timer[idx] > 0.0 {
            continue;
        }
        refresh_timer[idx] = AI_TECH_BUNKER_GARRISON_REFRESH_INTERVAL_SECONDS;
        garrison_ai_tech_bunkers(&mut commands, team, &bunkers, &units);
    }
}

pub(crate) fn garrison_ai_tech_bunkers(
    commands: &mut Commands,
    team: Team,
    bunkers: &Query<AiOpenBunkerItem<'_>, (With<Structure>, Without<Unit>)>,
    units: &Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) {
    let mut open_bunkers = bunkers
        .iter()
        .filter_map(
            |(entity, structure, bunker_team, transform, health, garrison, under_construction)| {
                (*bunker_team == team
                    && structure.id == "TechBunker"
                    && health.current > 0.0
                    && structure_is_constructed(under_construction)
                    && garrison.count < garrison.capacity)
                    .then_some((
                        entity,
                        transform.translation,
                        garrison.count,
                        garrison.capacity,
                    ))
            },
        )
        .collect::<Vec<_>>();
    open_bunkers.sort_by_key(|(_, _, count, _)| *count);

    let mut assigned_units = Vec::new();
    for (bunker_entity, bunker_position, count, capacity) in open_bunkers {
        for _ in count..capacity {
            let Some(unit_entity) =
                best_available_ai_garrison_unit(team, bunker_position, &assigned_units, units)
            else {
                break;
            };
            issue_unit_order(
                commands,
                unit_entity,
                UnitQueuedOrder::Garrison(bunker_entity),
            );
            assigned_units.push(unit_entity);
        }
    }
}

pub(crate) fn best_available_ai_garrison_unit(
    team: Team,
    bunker_position: Vec3,
    assigned_units: &[Entity],
    units: &Query<AiGarrisonUnitItem<'_>, (With<Unit>, Without<Structure>, IdleUnitOrderFilter)>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (entity, unit, unit_team, transform, health, order_queue) in units {
        if *unit_team != team
            || health.current <= 0.0
            || !can_unit_garrison(unit)
            || assigned_units.contains(&entity)
            || order_queue.is_some_and(|queue| !queue.orders.is_empty())
        {
            continue;
        }
        let distance = xz_distance(transform.translation, bunker_position);
        if distance <= AI_TECH_BUNKER_GARRISON_SEARCH_RADIUS && distance < best_distance {
            best = Some(entity);
            best_distance = distance;
        }
    }
    best
}

pub(crate) fn assign_ai_attack_wave_order(
    commands: &mut Commands,
    team: Team,
    entity: Entity,
    unit: &Unit,
    target: Entity,
    support_units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) {
    let Some(def) = registry::entity(unit.id) else {
        return;
    };
    let mut entity_commands = commands.entity(entity);
    entity_commands.try_insert(AiAttackWaveMember);
    if repair_capability(unit).is_some()
        && let Some(repair_target) = ai_battlegroup_repair_target(team, entity, support_units)
    {
        entity_commands.try_insert(RepairOrder {
            target: repair_target,
        });
    } else if def.weapon.is_some() {
        entity_commands.try_insert(AttackOrder { target });
    } else {
        entity_commands.try_insert(FollowOrder {
            target,
            allow_enemy: true,
            offset: Vec3::ZERO,
        });
    }
}

pub(crate) fn restore_ai_attack_wave_orders(
    mut commands: Commands,
    ai_settings: Res<AiDifficultySettings>,
    player_factions: Option<Res<PlayerFactions>>,
    visible_player: Option<Res<VisiblePlayer>>,
    active_teams: Option<Res<ActiveTeams>>,
    units: Query<
        (
            Entity,
            &Unit,
            &Team,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        (With<AiAttackWaveMember>, With<Unit>, Without<Selected>),
    >,
    support_units: Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    targets: Query<(Entity, &Team, &Transform), With<Health>>,
) {
    for team in active_ai_teams(
        controlled_player_team(visible_player.as_deref()),
        active_teams.as_deref(),
    ) {
        let profile = faction_ai_profile_for_difficulty(
            slot_faction_from_option(player_factions.as_deref(), team),
            ai_settings.difficulty(team),
        );
        if !profile.active_offense_enabled {
            continue;
        }
        let Some(target) = nearest_enemy_entity(team, team_home(team), &targets) else {
            continue;
        };
        for (
            entity,
            unit,
            unit_team,
            order_queue,
            move_order,
            follow_order,
            attack_order,
            capture_order,
            garrison_order,
            harvest_order,
            repair_order,
            construct_order,
            attack_move_order,
            patrol_order,
        ) in &units
        {
            if *unit_team != team
                || !ai_battle_unit_id(unit.id)
                || !is_unit_idle(
                    order_queue,
                    move_order,
                    follow_order,
                    attack_order,
                    capture_order,
                    garrison_order,
                    harvest_order,
                    repair_order,
                    construct_order,
                    attack_move_order,
                    patrol_order,
                )
            {
                continue;
            }
            assign_ai_attack_wave_order(&mut commands, team, entity, unit, target, &support_units);
        }
    }
}

pub(crate) fn next_ai_economy_train(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    if counts.workers < profile.expected_workers
        && let Some(worker_def) = registry::entity("Worker")
    {
        if !economy.can_afford(worker_def.cost) {
            return None;
        }
        if requirements_met(worker_def, team, structures)
            && ai_production_origin_for_faction(team, faction, "Worker", structures).is_some()
        {
            return Some("Worker");
        }
    }

    None
}

pub(crate) fn try_spawn_ai_trained_unit(
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    id: &'static str,
    structures: &Query<StructurePrereqItem<'_>>,
    map_bounds: MapBounds,
    player_team: Team,
) -> bool {
    let Some(def) = registry::entity(id) else {
        return false;
    };
    if !requirements_met(def, team, structures) {
        return false;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, id, structures)
    else {
        return false;
    };
    if !economies.get_mut(team).spend(def.cost) {
        return false;
    }

    let spawn_at = free_position_in_bounds(origin, next_id.0, 2.7, map_bounds);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        spawn_at,
        initial_rank,
        faction,
        player_team,
    );
    true
}

pub(crate) fn run_ai_capture_logic(
    team: Team,
    faction: SkirmishFaction,
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    visible_team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    capture_structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) {
    let mut engineer_count = 0usize;
    let mut idle_engineer = None;
    for (
        entity,
        unit,
        unit_team,
        transform,
        order_queue,
        move_order,
        follow_order,
        attack_order,
        capture_order,
        garrison_order,
        harvest_order,
        repair_order,
        construct_order,
        attack_move_order,
        patrol_order,
    ) in units
    {
        if *unit_team != team || unit.id != "EngineerDrone" {
            continue;
        }
        engineer_count += 1;
        if idle_engineer.is_none()
            && is_unit_idle(
                order_queue,
                move_order,
                follow_order,
                attack_order,
                capture_order,
                garrison_order,
                harvest_order,
                repair_order,
                construct_order,
                attack_move_order,
                patrol_order,
            )
        {
            idle_engineer = Some((entity, transform.translation));
        }
    }

    if let Some((entity, origin)) = idle_engineer {
        if let Some(target) = best_ai_capture_target(team, origin, capture_structures) {
            issue_unit_order(commands, entity, UnitQueuedOrder::Capture(target));
        }
        return;
    }

    if engineer_count >= AI_CAPTURE_ENGINEER_LIMIT {
        return;
    }
    let Some(def) = registry::entity("EngineerDrone") else {
        return;
    };
    if !requirements_met(def, team, structures) {
        return;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, "EngineerDrone", structures)
    else {
        return;
    };
    let Some(target) = best_ai_capture_target(team, origin, capture_structures) else {
        return;
    };
    if !economies.get_mut(team).spend(def.cost) {
        return;
    }

    let spawn_at = free_position(origin, next_id.0 + 13, 2.4);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    let engineer = spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        "EngineerDrone",
        team,
        spawn_at,
        initial_rank,
        faction,
        visible_team,
    );
    issue_unit_order(commands, engineer, UnitQueuedOrder::Capture(target));
}

pub(crate) fn run_ai_saboteur_logic(
    team: Team,
    faction: SkirmishFaction,
    commands: &mut Commands,
    asset_server: &AssetServer,
    economies: &mut Economies,
    next_id: &mut NextSpawnId,
    visible_team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<
        (
            Entity,
            &Unit,
            &Team,
            &Transform,
            Option<&OrderQueue>,
            Option<&MoveOrder>,
            Option<&FollowOrder>,
            Option<&AttackOrder>,
            Option<&CaptureOrder>,
            Option<&GarrisonOrder>,
            Option<&HarvestOrder>,
            Option<&RepairOrder>,
            Option<&ConstructOrder>,
            Option<&AttackMoveOrder>,
            Option<&PatrolOrder>,
        ),
        Without<Selected>,
    >,
    capture_structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) {
    let Some(saboteur_def) = registry::entity(AI_SABOTEUR_ID) else {
        return;
    };
    if saboteur_def.capture_time <= 0.0 {
        return;
    }

    let mut saboteur_count = 0usize;
    let mut idle_saboteur = None;
    for (
        entity,
        unit,
        unit_team,
        transform,
        order_queue,
        move_order,
        follow_order,
        attack_order,
        capture_order,
        garrison_order,
        harvest_order,
        repair_order,
        construct_order,
        attack_move_order,
        patrol_order,
    ) in units
    {
        if *unit_team != team || unit.id != AI_SABOTEUR_ID {
            continue;
        }
        saboteur_count += 1;
        if idle_saboteur.is_none()
            && order_queue.is_none_or(|queue| queue.orders.is_empty())
            && !has_active_orders_in_query(
                move_order,
                follow_order,
                attack_order,
                capture_order,
                garrison_order,
                harvest_order,
                repair_order,
                construct_order,
                attack_move_order,
                patrol_order,
            )
        {
            idle_saboteur = Some((entity, transform.translation));
        }
    }

    if let Some((entity, position)) = idle_saboteur {
        if let Some(target) =
            best_ai_saboteur_target(team, position, saboteur_def, economies, capture_structures)
        {
            issue_unit_order(commands, entity, UnitQueuedOrder::Capture(target));
        }
        return;
    }

    if saboteur_count >= AI_SABOTEUR_LIMIT
        || !requirements_met(saboteur_def, team, structures)
        || !economies.get(team).can_afford(saboteur_def.cost)
    {
        return;
    }
    let Some((producer_id, origin)) =
        ai_production_origin_for_faction(team, faction, AI_SABOTEUR_ID, structures)
    else {
        return;
    };
    let Some(target) =
        best_ai_saboteur_target(team, origin, saboteur_def, economies, capture_structures)
    else {
        return;
    };
    if !economies.get_mut(team).spend(saboteur_def.cost) {
        return;
    }

    let spawn_at = free_position(origin, next_id.0 + 17, 2.2);
    let initial_rank = economies.get(team).production_veterancy_rank(producer_id);
    let saboteur = spawn_unit_for_faction(
        commands,
        asset_server,
        next_id,
        AI_SABOTEUR_ID,
        team,
        spawn_at,
        initial_rank,
        faction,
        visible_team,
    );
    issue_unit_order(commands, saboteur, UnitQueuedOrder::Capture(target));
}

pub(crate) fn best_ai_saboteur_target(
    team: Team,
    origin: Vec3,
    saboteur_def: &registry::EntityDef,
    economies: &Economies,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_score = f32::MIN;
    for (entity, structure, structure_team, transform, health, under_construction) in structures {
        if health.current <= 0.0
            || *structure_team == team
            || *structure_team == Team::Neutral
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        let Some(target_def) = registry::entity(structure.id) else {
            continue;
        };
        if !ai_saboteur_target_has_value(team, *structure_team, saboteur_def, target_def, economies)
        {
            continue;
        }
        let score = ai_saboteur_target_score(
            *structure_team,
            target_def,
            transform.translation,
            origin,
            economies,
        );
        if score > best_score {
            best_score = score;
            best = Some(entity);
        }
    }
    best
}

pub(crate) fn best_ai_capture_target(
    team: Team,
    origin: Vec3,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Entity> {
    let mut best = None;
    let mut best_score = f32::MIN;
    for (entity, structure, structure_team, transform, health, under_construction) in structures {
        if health.current <= 0.0
            || *structure_team == team
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        let Some(target_def) = registry::entity(structure.id) else {
            continue;
        };
        let Some(priority) = ai_capture_priority(target_def.id) else {
            continue;
        };
        let owner_bonus = if *structure_team == Team::Neutral {
            AI_CAPTURE_NEUTRAL_TECH_TARGET_BONUS
        } else {
            0.0
        };
        let structure_value = (target_def.cost.ore + target_def.cost.crystal) as f32
            + target_def.power_delta.abs() as f32;
        let distance_penalty = xz_distance(origin, transform.translation) * 0.08;
        let score = priority + structure_value + owner_bonus - distance_penalty;
        if score > best_score {
            best_score = score;
            best = Some(entity);
        }
    }
    best
}

pub(crate) fn active_ai_teams(
    controlled_team: Option<Team>,
    active_teams: Option<&ActiveTeams>,
) -> impl Iterator<Item = Team> + '_ {
    let team_count = active_teams.map(|active| active.0.len()).unwrap_or(0);
    player_teams(team_count)
        .filter(move |team| Some(*team) != controlled_team)
        .filter(move |team| team_is_active(*team, active_teams))
}

pub(crate) fn next_ai_train(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    cursor: &mut usize,
    counts: AiProductionCounts,
    needs_anti_air: bool,
) -> Option<&'static str> {
    if profile.production_priority.is_empty() {
        return None;
    }
    if counts.workers < profile.expected_workers
        && let Some(worker_def) = registry::entity("Worker")
    {
        if !economy.can_afford(worker_def.cost) {
            return None;
        }
        if requirements_met(worker_def, team, structures)
            && ai_production_origin_for_faction(team, faction, "Worker", structures).is_some()
        {
            return Some("Worker");
        }
    }
    if needs_anti_air
        && let Some(candidate) = next_ai_train_matching(
            team,
            faction,
            profile,
            structures,
            economy,
            cursor,
            counts,
            |def| def.weapon.is_some_and(|weapon| weapon.can_attack_air),
        )
    {
        return Some(candidate);
    }
    next_ai_train_matching(
        team,
        faction,
        profile,
        structures,
        economy,
        cursor,
        counts,
        |_| true,
    )
}

pub(crate) fn next_ai_train_matching(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    economy: &TeamEconomy,
    cursor: &mut usize,
    counts: AiProductionCounts,
    mut predicate: impl FnMut(&registry::EntityDef) -> bool,
) -> Option<&'static str> {
    let len = profile.production_priority.len();
    let start = *cursor % len;
    for offset in 0..len {
        let index = (start + offset) % len;
        let candidate = profile.production_priority[index];
        let Some(def) = registry::entity(candidate) else {
            continue;
        };
        if !predicate(def) {
            continue;
        }
        if !ai_economy_candidate_allowed(candidate, profile, counts) {
            continue;
        }
        if !ai_battlegroup_candidate_allowed(candidate, profile, counts) {
            continue;
        }
        if !economy.can_afford(def.cost) {
            continue;
        }
        if !requirements_met(def, team, structures) {
            continue;
        }
        if ai_production_origin_for_faction(team, faction, candidate, structures).is_none() {
            continue;
        }
        *cursor = (index + 1) % len;
        return Some(candidate);
    }
    None
}

#[allow(dead_code)]
pub(crate) fn next_ai_economy_structure(
    team: Team,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    next_ai_economy_structure_for_faction(
        team,
        SkirmishFaction::from_team(team),
        profile,
        structures,
        counts,
    )
}

pub(crate) fn next_ai_economy_structure_for_faction(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
    counts: AiProductionCounts,
) -> Option<&'static str> {
    if counts.workers == 0 {
        return None;
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "CommandCenter", structures)
        && ai_structure_count(team, "CommandCenter", structures, false)
            < profile.expected_command_centers
    {
        return Some("CommandCenter");
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "Refinery", structures)
        && ai_structure_count(team, "Refinery", structures, false) < profile.expected_refineries
    {
        return Some("Refinery");
    }
    if ai_economy_structure_allowed_for_faction(team, faction, "OrePurifier", structures)
        && ai_structure_count(team, "OrePurifier", structures, false) == 0
        && has_constructed_structure(team, "Refinery", structures)
    {
        return Some("OrePurifier");
    }
    None
}

#[allow(dead_code)]
pub(crate) fn next_ai_offense_structure(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_offense_structure_for_faction(team, SkirmishFaction::from_team(team), structures)
}

pub(crate) fn next_ai_offense_structure_for_faction(
    team: Team,
    faction: SkirmishFaction,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    let faction = faction_def(faction)?;
    for &candidate in AI_OFFENSE_STRUCTURE_PRIORITY {
        let Some(def) = registry::entity(candidate) else {
            continue;
        };
        if faction.can_construct(candidate)
            && ai_structure_count(team, candidate, structures, false) == 0
            && requirements_met(def, team, structures)
        {
            return Some(candidate);
        }
    }
    None
}

#[allow(dead_code)]
pub(crate) fn next_ai_defense(
    team: Team,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    next_ai_defense_for_faction(team, SkirmishFaction::from_team(team), profile, structures)
}

pub(crate) fn next_ai_defense_for_faction(
    team: Team,
    faction: SkirmishFaction,
    profile: &TeamAiProfile,
    structures: &Query<StructurePrereqItem<'_>>,
) -> Option<&'static str> {
    let faction = faction_def(faction)?;
    for candidate in profile.defense_priority {
        if let Some(def) = registry::entity(candidate) {
            if faction.can_construct(candidate)
                && ai_structure_under_profile_limit(team, candidate, structures, profile)
                && requirements_met(def, team, structures)
            {
                return Some(candidate);
            }
        }
    }
    None
}
