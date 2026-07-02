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

pub(crate) const AI_OPENING_ATTACK_GRACE_SECONDS: f32 = 45.0;

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
