//! Faction-defining passive mechanics — the WC3/SC-style asymmetry layer.
//!
//! 苍穹联盟 (Alliance): disciplined regulars — veterancy accrues at double
//! rate ("训练有素") and structures slowly self-repair out of combat
//! ("战地工程"). The dependable, defensive tech faction.
//!
//! 炽炎魔军 (Demon): the swarm — units are cheaper and train faster
//! ("熔炉量产"), regenerate out of combat ("魔血再生") and turn frenzied
//! below 40% health, hitting harder and faster ("血怒").
//!
//! 混沌裂隙 (Chaos): the elite — every unit carries a rechargeable energy
//! shield ("裂隙护盾") that absorbs damage before health and refills a few
//! seconds after last being hit. Replaces the old flat 10% damage reduction.

use bevy::prelude::*;

use crate::*;

pub(crate) const CHAOS_SHIELD_HEALTH_RATIO: f32 = 0.35;
pub(crate) const CHAOS_SHIELD_RECHARGE_PER_SECOND: f32 = 6.0;
pub(crate) const DEMON_REGEN_HP_PER_SECOND: f32 = 1.2;
pub(crate) const DEMON_FURY_HEALTH_RATIO: f32 = 0.4;
pub(crate) const DEMON_FURY_DAMAGE_MULTIPLIER: f32 = 1.2;
pub(crate) const DEMON_FURY_COOLDOWN_MULTIPLIER: f32 = 0.8;
pub(crate) const DEMON_UNIT_COST_SCALE: f32 = 0.85;
pub(crate) const DEMON_PRODUCTION_SPEED_MULTIPLIER: f32 = 1.25;
pub(crate) const ALLIANCE_XP_PER_KILL: u32 = 2;
pub(crate) const ALLIANCE_STRUCTURE_REPAIR_HP_PER_SECOND: f32 = 1.5;
/// Out-of-combat window shared by shield recharge, demon regen and alliance
/// field repair: nothing kicks in until this long after the last hit.
pub(crate) const FACTION_RECENT_DAMAGE_SECONDS: f32 = 4.5;

/// A rechargeable energy shield on every Chaos unit. Absorbs weapon damage
/// before health; refills once the unit has not been hit for a few seconds.
#[derive(Component, Clone, Copy)]
pub(crate) struct FactionShield {
    pub(crate) current: f32,
    pub(crate) max: f32,
}

/// The unit or structure took weapon damage recently — suppresses shield
/// recharge, demon regeneration and alliance structure field repair.
#[derive(Component, Clone, Copy)]
pub(crate) struct RecentDamage {
    pub(crate) remaining: f32,
}

impl RecentDamage {
    pub(crate) fn fresh() -> Self {
        Self {
            remaining: FACTION_RECENT_DAMAGE_SECONDS,
        }
    }
}

/// Shield capacity for a Chaos unit with the given max health.
pub(crate) fn chaos_shield_capacity(max_health: f32) -> f32 {
    (max_health * CHAOS_SHIELD_HEALTH_RATIO).ceil()
}

/// Drains `damage` from the shield first; returns the damage left over for
/// health after the shield absorbed its share.
pub(crate) fn drain_faction_shield(shield: &mut FactionShield, damage: f32) -> f32 {
    if shield.current <= 0.0 || damage <= 0.0 {
        return damage;
    }
    let absorbed = shield.current.min(damage);
    shield.current -= absorbed;
    damage - absorbed
}

/// Demon fury: below 40% health a demon unit hits harder and faster.
pub(crate) fn demon_fury_active(
    faction: Option<SkirmishFaction>,
    is_unit: bool,
    health_ratio: Option<f32>,
) -> bool {
    faction == Some(SkirmishFaction::Demon)
        && is_unit
        && health_ratio.is_some_and(|ratio| ratio > 0.0 && ratio < DEMON_FURY_HEALTH_RATIO)
}

/// Unit training cost after the faction economy passive (demon −15%).
pub(crate) fn faction_unit_cost(
    faction: Option<SkirmishFaction>,
    cost: registry::Cost,
) -> registry::Cost {
    if faction != Some(SkirmishFaction::Demon) {
        return cost;
    }
    registry::Cost {
        ore: ((cost.ore as f32) * DEMON_UNIT_COST_SCALE).round() as i32,
        crystal: ((cost.crystal as f32) * DEMON_UNIT_COST_SCALE).round() as i32,
    }
}

/// Production tick speed for the faction (demon forges run 25% faster).
pub(crate) fn faction_production_speed(faction: Option<SkirmishFaction>) -> f32 {
    if faction == Some(SkirmishFaction::Demon) {
        DEMON_PRODUCTION_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

/// Veterancy experience per kill (alliance troops learn twice as fast).
pub(crate) fn faction_xp_per_kill(faction: Option<SkirmishFaction>) -> u32 {
    if faction == Some(SkirmishFaction::Alliance) {
        ALLIANCE_XP_PER_KILL
    } else {
        1
    }
}

/// Attaches faction passives to freshly spawned units (chaos shields).
pub(crate) fn attach_faction_passives(
    mut commands: Commands,
    player_factions: Res<PlayerFactions>,
    fresh_units: Query<(Entity, &Team, &Health), Added<Unit>>,
) {
    for (entity, team, health) in &fresh_units {
        if player_factions.faction(*team) != Some(SkirmishFaction::Chaos) {
            continue;
        }
        let max = chaos_shield_capacity(health.max);
        commands
            .entity(entity)
            .try_insert(FactionShield { current: max, max });
    }
}

/// Ticks down the recent-damage suppression window.
pub(crate) fn tick_recent_damage(
    mut commands: Commands,
    time: Res<Time>,
    mut damaged: Query<(Entity, &mut RecentDamage)>,
) {
    let dt = time.delta_secs();
    for (entity, mut recent) in &mut damaged {
        recent.remaining -= dt;
        if recent.remaining <= 0.0 {
            commands.entity(entity).try_remove::<RecentDamage>();
        }
    }
}

/// Chaos shields refill once the owner has not been hit recently.
pub(crate) fn recharge_faction_shields(
    time: Res<Time>,
    mut shields: Query<(&mut FactionShield, &Health, Has<RecentDamage>)>,
) {
    let dt = time.delta_secs();
    for (mut shield, health, recently_hit) in &mut shields {
        if recently_hit || health.current <= 0.0 || shield.current >= shield.max {
            continue;
        }
        shield.current = (shield.current + CHAOS_SHIELD_RECHARGE_PER_SECOND * dt).min(shield.max);
    }
}

/// Demon units slowly knit themselves back together out of combat.
pub(crate) fn demon_unit_regeneration(
    time: Res<Time>,
    player_factions: Res<PlayerFactions>,
    mut units: Query<(&Team, &mut Health, Has<RecentDamage>), With<Unit>>,
) {
    let dt = time.delta_secs();
    for (team, mut health, recently_hit) in &mut units {
        if recently_hit
            || health.current <= 0.0
            || health.current >= health.max
            || player_factions.faction(*team) != Some(SkirmishFaction::Demon)
        {
            continue;
        }
        health.current = (health.current + DEMON_REGEN_HP_PER_SECOND * dt).min(health.max);
    }
}

/// Alliance structures self-repair out of combat (battlefield engineering).
pub(crate) fn alliance_structure_field_repair(
    time: Res<Time>,
    player_factions: Res<PlayerFactions>,
    mut structures: Query<
        (&Team, &mut Health, Has<RecentDamage>),
        (With<Structure>, Without<UnderConstruction>, Without<Unit>),
    >,
) {
    let dt = time.delta_secs();
    for (team, mut health, recently_hit) in &mut structures {
        if recently_hit
            || health.current <= 0.0
            || health.current >= health.max
            || player_factions.faction(*team) != Some(SkirmishFaction::Alliance)
        {
            continue;
        }
        health.current =
            (health.current + ALLIANCE_STRUCTURE_REPAIR_HP_PER_SECOND * dt).min(health.max);
    }
}
