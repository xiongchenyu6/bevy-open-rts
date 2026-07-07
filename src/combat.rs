//! Combat: weapons/health, target acquisition and chasing, unit movement,
//! crushing, mines, wreckage, veterancy, auras/shields/EMP and combat VFX.
//!
//! Pure move out of lib.rs (module split); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

#[derive(Component)]
pub(crate) struct EmpDisabled {
    pub(crate) remaining: f32,
}

#[derive(Component)]
pub(crate) struct SupportShield {
    pub(crate) remaining: f32,
    pub(crate) damage_scale: f32,
}

pub(crate) fn queue_apply_emp_disabled(commands: &mut Commands, entity: Entity, duration: f32) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        entity_mut.remove::<(
            MoveOrder,
            FollowOrder,
            AttackOrder,
            CaptureOrder,
            GarrisonOrder,
            HarvestOrder,
            RepairOrder,
            ConstructOrder,
            AttackMoveOrder,
            PatrolOrder,
            OrderQueue,
        )>();
        if let Some(mut disabled) = entity_mut.get_mut::<EmpDisabled>() {
            disabled.remaining = disabled.remaining.max(duration);
            return;
        }
        entity_mut.insert(EmpDisabled {
            remaining: duration,
        });
    });
}

pub(crate) fn queue_apply_chrono_relay(
    commands: &mut Commands,
    entity: Entity,
    duration: f32,
    speed_multiplier: f32,
) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        if let Some(mut chrono) = entity_mut.get_mut::<ChronoRelay>() {
            chrono.remaining = chrono.remaining.max(duration);
            chrono.speed_multiplier = chrono.speed_multiplier.max(speed_multiplier);
            return;
        }
        entity_mut.insert(ChronoRelay {
            remaining: duration,
            speed_multiplier,
        });
    });
}

pub(crate) fn queue_apply_support_shield(
    commands: &mut Commands,
    entity: Entity,
    duration: f32,
    damage_scale: f32,
) {
    commands.queue(move |world: &mut World| {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };
        let damage_scale = damage_scale.clamp(0.0, 1.0);
        if let Some(mut shield) = entity_mut.get_mut::<SupportShield>() {
            shield.remaining = shield.remaining.max(duration);
            shield.damage_scale = damage_scale;
            return;
        }
        entity_mut.insert(SupportShield {
            remaining: duration,
            damage_scale,
        });
    });
}

#[derive(Component)]
pub(crate) struct PassiveSupportShield {
    pub(crate) damage_scale: f32,
}

#[derive(Component)]
pub(crate) struct MobileShieldProjector {
    pub(crate) refresh_remaining: f32,
    pub(crate) radius: f32,
    pub(crate) duration: f32,
    pub(crate) damage_scale: f32,
}

#[derive(Component)]
pub(crate) struct RepairAura {
    pub(crate) rate: f32,
    pub(crate) radius: f32,
    pub(crate) mode: RepairAuraMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepairAuraMode {
    AllEligible,
    NearestEligible,
}

#[derive(Component)]
pub(crate) struct HealingAura {
    pub(crate) rate: f32,
    pub(crate) radius: f32,
}

#[derive(Component)]
pub(crate) struct ManualStructureRepair {
    pub(crate) points_remaining: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct DeployedSiegeMode {
    pub(crate) previous_hold_position: bool,
    pub(crate) base_speed: f32,
    pub(crate) base_attack_range: f32,
    pub(crate) base_attack_damage: f32,
    pub(crate) base_attack_interval: f32,
    pub(crate) base_structure_damage_multiplier: f32,
    pub(crate) base_sight_range: f32,
}

#[derive(Component)]
pub(crate) struct ScorchMark;

#[derive(Resource, Default)]
pub(crate) struct KillCredits(pub(crate) Vec<Entity>);

#[derive(Component, Clone, Copy)]
pub(crate) struct Health {
    pub(crate) current: f32,
    pub(crate) max: f32,
}

impl Health {
    pub(crate) fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub(crate) fn ratio(self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Veterancy {
    pub(crate) rank: u8,
    pub(crate) experience_points: u32,
    pub(crate) base_health: f32,
    pub(crate) base_damage: f32,
    pub(crate) base_range: f32,
    pub(crate) base_vision: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Mine {
    pub(crate) damage: f32,
    pub(crate) trigger_radius: f32,
    pub(crate) blast_radius: f32,
    pub(crate) arming_remaining: f32,
    pub(crate) source: Option<Entity>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct MineLayer {
    pub(crate) damage: f32,
    pub(crate) deploy_interval: f32,
    pub(crate) deploy_radius: f32,
    pub(crate) spacing: f32,
    pub(crate) limit: usize,
    pub(crate) cooldown: f32,
    pub(crate) deploy_index: usize,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Weapon {
    pub(crate) range: f32,
    pub(crate) damage: f32,
    pub(crate) cooldown: f32,
    pub(crate) splash_radius: f32,
    pub(crate) splash_damage_multiplier: f32,
    pub(crate) structure_damage_multiplier: f32,
    pub(crate) cooldown_left: f32,
    pub(crate) can_attack_air: bool,
    pub(crate) can_attack_ground: bool,
}

impl Weapon {
    pub(crate) fn new(
        range: f32,
        damage: f32,
        cooldown: f32,
        splash_radius: f32,
        splash_damage_multiplier: f32,
        structure_damage_multiplier: f32,
        can_attack_air: bool,
        can_attack_ground: bool,
    ) -> Self {
        Self {
            range,
            damage,
            cooldown,
            splash_radius,
            splash_damage_multiplier,
            structure_damage_multiplier,
            can_attack_air,
            can_attack_ground,
            cooldown_left: 0.0,
        }
    }
}

pub(crate) fn attach_support_effects(
    commands: &mut Commands,
    entity_id: Entity,
    def: &registry::EntityDef,
) {
    if def.support_shield_radius > 0.0 && def.support_shield_duration > 0.0 {
        commands
            .entity(entity_id)
            .try_insert(MobileShieldProjector {
                refresh_remaining: 0.0,
                radius: def.support_shield_radius,
                duration: def.support_shield_duration,
                damage_scale: def.support_shield_damage_multiplier,
            });
    }
    if def.id == "ShieldTrooper" {
        commands.entity(entity_id).try_insert(PassiveSupportShield {
            damage_scale: SHIELD_TROOPER_PASSIVE_DAMAGE_SCALE,
        });
    }
    if let Some(mode) = passive_repair_aura_mode(def) {
        commands.entity(entity_id).try_insert(RepairAura {
            rate: def.repair_rate,
            radius: def.repair_radius,
            mode,
        });
    }
    if def.healing_rate > 0.0 && def.healing_radius > 0.0 {
        commands.entity(entity_id).try_insert(HealingAura {
            rate: def.healing_rate,
            radius: def.healing_radius,
        });
    }
    if def.income_interval > 0.0 && (def.resource_income_ore > 0 || def.resource_income_crystal > 0)
    {
        commands.entity(entity_id).try_insert(IncomeSource {
            ore: def.resource_income_ore,
            crystal: def.resource_income_crystal,
            interval: def.income_interval,
            remaining: def.income_interval,
        });
    }
    if def.garrison_capacity > 0 && def.garrison_attack_damage_per_unit > 0.0 {
        commands.entity(entity_id).try_insert(Garrison {
            capacity: def.garrison_capacity,
            damage_per_unit: def.garrison_attack_damage_per_unit,
            count: 0,
        });
    }
}

pub(crate) fn passive_repair_aura_mode(def: &registry::EntityDef) -> Option<RepairAuraMode> {
    if def.repair_rate <= 0.0 || def.repair_radius <= 0.0 {
        return None;
    }
    match def.id {
        "TechRepairDepot" => Some(RepairAuraMode::AllEligible),
        "RepairPad" => Some(RepairAuraMode::NearestEligible),
        _ => None,
    }
}

pub(crate) fn can_receive_repair_aura(
    unit: Option<&Unit>,
    structure: Option<&Structure>,
    domain: &MovementDomain,
) -> bool {
    let Some(unit) = unit else {
        return false;
    };
    structure.is_none()
        && *domain == MovementDomain::Terrain
        && unit.speed > 0.0
        && !is_infantry_unit(unit)
}

#[derive(Clone, Copy)]
pub(crate) struct RepairCapability {
    pub(crate) rate: f32,
    pub(crate) radius: f32,
}

pub(crate) fn repair_capability(unit: &Unit) -> Option<RepairCapability> {
    let def = registry::entity(unit.id)?;
    (def.repair_rate > 0.0).then_some(RepairCapability {
        rate: def.repair_rate,
        radius: def.repair_radius,
    })
}

pub(crate) fn can_receive_healing_aura(unit: Option<&Unit>) -> bool {
    unit.is_some_and(is_infantry_unit)
}

pub(crate) fn support_damage_scale(
    shield: Option<&SupportShield>,
    passive_shield: Option<&PassiveSupportShield>,
) -> f32 {
    shield
        .map(|shield| shield.damage_scale)
        .or_else(|| passive_shield.map(|shield| shield.damage_scale))
        .unwrap_or(1.0)
}

pub(crate) fn faction_weapon_damage_multiplier(
    attacker_faction: Option<SkirmishFaction>,
    target_team: Team,
    target_is_structure: bool,
) -> f32 {
    if attacker_faction == Some(SkirmishFaction::Demon)
        && target_is_structure
        && target_team != Team::Neutral
    {
        DEMON_STRUCTURE_WEAPON_DAMAGE_MULTIPLIER
    } else {
        1.0
    }
}

pub(crate) fn faction_incoming_weapon_damage_scale(target_faction: Option<SkirmishFaction>) -> f32 {
    match target_faction {
        Some(SkirmishFaction::Chaos) => CHAOS_INCOMING_WEAPON_DAMAGE_SCALE,
        Some(SkirmishFaction::Alliance | SkirmishFaction::Demon) | None => 1.0,
    }
}

pub(crate) fn applied_weapon_damage(
    base_damage: f32,
    attacker_faction: Option<SkirmishFaction>,
    target_team: Team,
    target_faction: Option<SkirmishFaction>,
    target_is_structure: bool,
    shield: Option<&SupportShield>,
    passive_shield: Option<&PassiveSupportShield>,
) -> f32 {
    base_damage
        * faction_weapon_damage_multiplier(attacker_faction, target_team, target_is_structure)
        * faction_incoming_weapon_damage_scale(target_faction)
        * support_damage_scale(shield, passive_shield)
}

pub(crate) fn spawn_land_mine_procedural_model(world: &mut World, root: Entity) {
    let Some(body_mesh) = add_procedural_mesh(
        world,
        ConicalFrustum {
            radius_top: 0.34,
            radius_bottom: 0.38,
            height: 0.12,
        }
        .mesh()
        .resolution(32),
    ) else {
        return;
    };
    let Some(ring_mesh) = add_procedural_mesh(
        world,
        Torus::new(0.03, 0.31)
            .mesh()
            .minor_resolution(8)
            .major_resolution(32),
    ) else {
        return;
    };

    let Some(dark_material) = add_procedural_material(
        world,
        Color::srgb(0.055, 0.058, 0.065),
        0.7,
        0.4,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(team_material) = add_procedural_material(
        world,
        Color::srgb(0.99, 0.81, 0.48),
        0.55,
        0.35,
        LinearRgba::BLACK,
    ) else {
        return;
    };

    spawn_procedural_mesh_child(
        world,
        root,
        "LandMine Body",
        body_mesh,
        dark_material,
        Transform::from_xyz(0.0, 0.06, 0.0),
    );
    spawn_procedural_mesh_child(
        world,
        root,
        "LandMine TeamRing",
        ring_mesh,
        team_material,
        Transform::from_xyz(0.0, 0.14, 0.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
}

pub(crate) fn spawn_tesla_fence_segment_procedural_model(world: &mut World, root: Entity) {
    let Some(base_mesh) = add_procedural_mesh(world, Cuboid::new(1.55, 0.16, 0.5)) else {
        return;
    };
    let Some(post_mesh) = add_procedural_mesh(
        world,
        ConicalFrustum {
            radius_top: 0.12,
            radius_bottom: 0.16,
            height: 0.95,
        }
        .mesh()
        .resolution(24),
    ) else {
        return;
    };
    let Some(arc_mesh) = add_procedural_mesh(world, Cuboid::new(1.18, 0.06, 0.06)) else {
        return;
    };
    let Some(cap_mesh) =
        add_procedural_mesh(world, Cylinder::new(0.16, 0.08).mesh().resolution(24))
    else {
        return;
    };

    let Some(body_material) = add_procedural_material(
        world,
        Color::srgb(0.99, 0.81, 0.48),
        0.55,
        0.38,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(dark_material) = add_procedural_material(
        world,
        Color::srgb(0.08, 0.10, 0.11),
        0.65,
        0.35,
        LinearRgba::BLACK,
    ) else {
        return;
    };
    let Some(arc_material) = add_procedural_material(
        world,
        Color::srgb(0.08, 0.78, 1.0),
        0.0,
        0.25,
        LinearRgba::rgb(0.09, 1.33, 1.8),
    ) else {
        return;
    };

    spawn_procedural_mesh_child(
        world,
        root,
        "TeslaFenceSegment Base",
        base_mesh,
        dark_material,
        Transform::from_xyz(0.0, 0.08, 0.0),
    );
    for (name, x) in [
        ("TeslaFenceSegment LeftPost", -0.62),
        ("TeslaFenceSegment RightPost", 0.62),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            post_mesh.clone(),
            body_material.clone(),
            Transform::from_xyz(x, 0.58, 0.0),
        );
    }
    for (name, z) in [
        ("TeslaFenceSegment ArcBeamFront", -0.14),
        ("TeslaFenceSegment ArcBeamBack", 0.14),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            arc_mesh.clone(),
            arc_material.clone(),
            Transform::from_xyz(0.0, 0.72, z),
        );
    }
    for (name, x) in [
        ("TeslaFenceSegment LeftCap", -0.62),
        ("TeslaFenceSegment RightCap", 0.62),
    ] {
        spawn_procedural_mesh_child(
            world,
            root,
            name,
            cap_mesh.clone(),
            arc_material.clone(),
            Transform::from_xyz(x, 1.08, 0.0),
        );
    }
}

pub(crate) fn update_support_effects(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    player_factions: Option<Res<PlayerFactions>>,
    map_bounds: Res<MapBounds>,
    mut warnings: Query<(Entity, &mut SupportWarning)>,
    mut reveals: Query<(Entity, &mut TemporarySupportReveal)>,
    mut chrono_relays: Query<(Entity, &mut ChronoRelay)>,
    mut support_params: ParamSet<(
        Query<(Entity, &mut EmpDisabled)>,
        Query<(Entity, &mut SupportShield)>,
        Query<(
            Entity,
            &Transform,
            &Team,
            &Selectable,
            &Health,
            &mut MobileShieldProjector,
            Option<&EmpDisabled>,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&Unit>,
            Option<&Structure>,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            Option<&Unit>,
            Option<&Structure>,
            &MovementDomain,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
            &mut Health,
            &Selectable,
            Option<&FogMemoryVisible>,
        )>,
    )>,
    mut pending_strikes: Query<(Entity, &mut PendingOrbitalStrike, &Transform)>,
    mut pending_paradrops: Query<(Entity, &mut PendingParadrop)>,
    mut next_spawn_id: ResMut<NextSpawnId>,
    mut match_state: ResMut<MatchState>,
    mut battle_log: ResMut<BattleLog>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, mut warning) in &mut warnings {
        warning.remaining -= time.delta_secs();
        if warning.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        warning.radius = (warning.radius + time.delta_secs() * 0.16).min(10.0);
        if warning.remaining <= 0.2 {
            warning.radius = (warning.radius * 0.84).max(0.15);
        }
    }

    for (entity, mut reveal) in &mut reveals {
        reveal.remaining -= time.delta_secs();
        if reveal.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
    }

    {
        let mut emp_disables = support_params.p0();
        for (entity, mut disabled) in &mut emp_disables {
            disabled.remaining -= time.delta_secs();
            if disabled.remaining <= 0.0 {
                commands.entity(entity).try_remove::<EmpDisabled>();
                continue;
            }
        }
    }

    for (entity, mut chrono) in &mut chrono_relays {
        chrono.remaining -= time.delta_secs();
        if chrono.remaining <= 0.0 {
            commands.entity(entity).try_remove::<ChronoRelay>();
            continue;
        }
    }

    {
        let mut support_shields = support_params.p1();
        for (entity, mut shield) in &mut support_shields {
            shield.remaining -= time.delta_secs();
            if shield.remaining <= 0.0 {
                commands.entity(entity).try_remove::<SupportShield>();
                continue;
            }
        }
    }

    let projector_refreshes: Vec<(Team, Vec3, f32, f32, f32)> = {
        let mut mobile_shield_projectors = support_params.p2();
        mobile_shield_projectors
            .iter_mut()
            .filter_map(
                |(
                    _projector_entity,
                    projector_transform,
                    owner,
                    _selectable,
                    projector_health,
                    mut projector,
                    emp,
                )| {
                    if projector_health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0)
                    {
                        return None;
                    }
                    projector.refresh_remaining -= time.delta_secs();
                    if projector.refresh_remaining > 0.0 {
                        return None;
                    }
                    projector.refresh_remaining = 0.2;
                    Some((
                        *owner,
                        projector_transform.translation,
                        projector.radius,
                        projector.duration,
                        projector.damage_scale,
                    ))
                },
            )
            .collect()
    };
    if !projector_refreshes.is_empty() {
        let shield_targets = support_params.p3();
        for (owner, projector_position, projector_radius, duration, damage_scale) in
            projector_refreshes
        {
            for (
                target_entity,
                target_team,
                target_transform,
                target_selectable,
                target_health,
                target_unit,
                target_structure,
            ) in &shield_targets
            {
                if target_unit.is_none() && target_structure.is_none() {
                    continue;
                }
                if !relations.are_allied(owner, *target_team) || target_health.current <= 0.0 {
                    continue;
                }
                if xz_distance(target_transform.translation, projector_position)
                    > projector_radius + target_selectable.radius
                {
                    continue;
                }
                queue_apply_support_shield(&mut commands, target_entity, duration, damage_scale);
            }
        }
    }

    let occupiable_spawn_points: Vec<(Vec3, f32)> = {
        let health_q = support_params.p4();
        health_q
            .iter()
            .filter_map(|(_, _, transform, _, _, _, _, _, health, selectable, _)| {
                (health.current > 0.0).then_some((transform.translation, selectable.radius))
            })
            .collect()
    };

    let mut impacts: Vec<(Vec3, f32, f32, Team)> = Vec::new();
    let mut strike_entities: Vec<Entity> = Vec::new();
    for (entity, mut strike, transform) in &mut pending_strikes {
        strike.remaining -= time.delta_secs();
        if strike.remaining > 0.0 {
            continue;
        }
        impacts.push((
            transform.translation,
            strike.radius,
            strike.damage,
            strike.team,
        ));
        strike_entities.push(entity);
        let pulse_height = 0.85 + strike.impact_scale * 0.28;
        let pulse_ttl = 0.12 + strike.impact_scale * 0.06;
        commands.spawn((
            ShotPulse {
                from: transform.translation + Vec3::new(0.0, pulse_height, 0.0),
                to: transform.translation + Vec3::new(0.0, 0.2, 0.0),
                ttl: pulse_ttl,
                team: strike.team,
            },
            MatchScopedEntity,
        ));
    }

    for entity in strike_entities {
        commands.entity(entity).try_despawn();
    }

    let mut paradrops: Vec<(Vec3, Team, SkirmishFaction, &'static [&'static str])> = Vec::new();
    let mut paradrop_entities: Vec<Entity> = Vec::new();
    for (entity, mut paradrop) in &mut pending_paradrops {
        paradrop.remaining -= time.delta_secs();
        if paradrop.remaining > 0.0 {
            continue;
        }
        paradrops.push((
            paradrop.target,
            paradrop.team,
            slot_faction_from_option(player_factions.as_deref(), paradrop.team),
            paradrop.unit_paths,
        ));
        paradrop_entities.push(entity);
    }
    for entity in paradrop_entities {
        commands.entity(entity).try_despawn();
    }

    for (impact_pos, impact_radius, impact_damage, team) in impacts {
        let mut health_q = support_params.p4();
        for (
            target_entity,
            target_team,
            target_transform,
            _unit,
            structure,
            _domain,
            shield,
            passive_shield,
            mut target_health,
            selectable,
            fog_memory,
        ) in &mut health_q
        {
            if target_health.current <= 0.0 {
                continue;
            }
            if !relations.are_enemies(team, *target_team) {
                continue;
            }
            if xz_distance(target_transform.translation, impact_pos) > impact_radius {
                continue;
            }
            let damage = impact_damage * support_damage_scale(shield, passive_shield);
            target_health.current -= damage;
            if relations.are_allied(*target_team, player_team) && damage > 0.0 {
                push_under_attack_log(
                    &mut battle_log,
                    target_transform.translation,
                    structure.is_some(),
                );
            }
            if target_health.current <= 0.0 {
                if relations.are_allied(*target_team, player_team) {
                    if structure.is_some() {
                        match_state.structures_lost += 1;
                    } else {
                        match_state.units_lost += 1;
                    }
                } else if structure.is_some() {
                    match_state.enemy_structures_destroyed += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                spawn_destruction_effects(
                    &mut commands,
                    &asset_server,
                    target_transform.translation,
                    selectable.radius,
                    structure.is_some(),
                    *target_team,
                    structure.is_some() && fog_memory.is_some(),
                );
                commands.entity(target_entity).try_despawn();
            }
        }
    }

    for (target, team, faction, unit_paths) in paradrops {
        spawn_paradrop_units(
            &mut commands,
            &asset_server,
            &mut next_spawn_id,
            target,
            team,
            faction,
            player_team,
            unit_paths,
            &occupiable_spawn_points,
            *map_bounds,
        );
        commands.spawn((
            ShotPulse {
                from: target + Vec3::new(0.0, 1.2, 0.0),
                to: target + Vec3::new(0.0, 0.2, 0.0),
                ttl: 0.18,
                team,
            },
            MatchScopedEntity,
        ));
    }
}

pub(crate) fn update_repair_and_healing_auras(
    mut commands: Commands,
    time: Res<Time>,
    economies: Res<Economies>,
    relations: Res<TeamRelations>,
    player_factions: Res<PlayerFactions>,
    support_aura_sources: Query<
        (
            &Team,
            &Transform,
            Option<&RepairAura>,
            Option<&HealingAura>,
            Option<&Structure>,
            Option<&UnderConstruction>,
        ),
        Or<(With<RepairAura>, With<HealingAura>)>,
    >,
    mut health_q: Query<(
        Entity,
        &Team,
        &Transform,
        Option<&Unit>,
        Option<&Structure>,
        &MovementDomain,
        &mut Health,
        &Selectable,
    )>,
) {
    let mut repair_sources: Vec<(Team, Vec3, f32, f32, RepairAuraMode)> = Vec::new();
    let mut healing_sources: Vec<(Team, Vec3, f32, f32)> = Vec::new();
    for (team, transform, repair_aura, healing_aura, structure, under_construction) in
        &support_aura_sources
    {
        if !structure_is_constructed(under_construction) {
            continue;
        }
        if let Some(aura) = repair_aura {
            if powered_repair_offline(team, structure, &economies) {
                continue;
            }
            let support_rate =
                aura.rate * faction_support_rate_multiplier(player_factions.faction(*team));
            repair_sources.push((
                *team,
                transform.translation,
                aura.radius,
                support_rate,
                aura.mode,
            ));
        }
        if let Some(aura) = healing_aura {
            let support_rate =
                aura.rate * faction_support_rate_multiplier(player_factions.faction(*team));
            healing_sources.push((*team, transform.translation, aura.radius, support_rate));
        }
    }

    if !repair_sources.is_empty() {
        let repair_targets = health_q
            .iter_mut()
            .filter_map(
                |(
                    entity,
                    target_team,
                    target_transform,
                    target_unit,
                    target_structure,
                    target_domain,
                    target_health,
                    target_selectable,
                )| {
                    (target_health.current > 0.0
                        && target_health.current < target_health.max
                        && can_receive_repair_aura(target_unit, target_structure, target_domain))
                    .then_some((
                        entity,
                        *target_team,
                        target_transform.translation,
                        target_selectable.radius,
                    ))
                },
            )
            .collect::<Vec<_>>();
        let mut repair_events: Vec<(Entity, f32)> = Vec::new();
        for (source_team, source_position, source_radius, source_rate, mode) in &repair_sources {
            match mode {
                RepairAuraMode::AllEligible => {
                    for (target_entity, target_team, target_position, target_radius) in
                        &repair_targets
                    {
                        if !relations.are_allied(*source_team, *target_team) {
                            continue;
                        }
                        if xz_distance(*source_position, *target_position)
                            > *source_radius + *target_radius
                        {
                            continue;
                        }
                        repair_events.push((*target_entity, *source_rate * time.delta_secs()));
                    }
                }
                RepairAuraMode::NearestEligible => {
                    let mut best = None;
                    let mut best_distance = f32::MAX;
                    for (target_entity, target_team, target_position, target_radius) in
                        &repair_targets
                    {
                        if !relations.are_allied(*source_team, *target_team) {
                            continue;
                        }
                        let distance = xz_distance(*source_position, *target_position);
                        if distance <= *source_radius + *target_radius && distance < best_distance {
                            best = Some(*target_entity);
                            best_distance = distance;
                        }
                    }
                    if let Some(target_entity) = best {
                        repair_events.push((target_entity, *source_rate * time.delta_secs()));
                    }
                }
            }
        }
        for (
            entity,
            _target_team,
            target_transform,
            _target_unit,
            _target_structure,
            _target_domain,
            mut target_health,
            _target_selectable,
        ) in &mut health_q
        {
            let repaired = repair_events
                .iter()
                .filter_map(|(target, amount)| (*target == entity).then_some(*amount))
                .sum::<f32>();
            if repaired > 0.0 {
                target_health.current = (target_health.current + repaired).min(target_health.max);
                if heal_sparkle_due(time.elapsed_secs(), time.delta_secs(), entity.to_bits()) {
                    spawn_heal_sparkle(&mut commands, target_transform.translation);
                }
            }
        }
    }

    if !healing_sources.is_empty() {
        for (
            target_entity,
            target_team,
            target_transform,
            target_unit,
            _target_structure,
            _target_domain,
            mut target_health,
            target_selectable,
        ) in &mut health_q
        {
            if target_health.current <= 0.0 || target_health.current >= target_health.max {
                continue;
            }
            if !can_receive_healing_aura(target_unit) {
                continue;
            }
            let mut healed = 0.0;
            for (source_team, source_position, source_radius, source_rate) in &healing_sources {
                if !relations.are_allied(*source_team, *target_team) {
                    continue;
                }
                if xz_distance(*source_position, target_transform.translation)
                    > *source_radius + target_selectable.radius
                {
                    continue;
                }
                healed += *source_rate * time.delta_secs();
            }
            if healed > 0.0 {
                target_health.current = (target_health.current + healed).min(target_health.max);
                if heal_sparkle_due(
                    time.elapsed_secs(),
                    time.delta_secs(),
                    target_entity.to_bits(),
                ) {
                    spawn_heal_sparkle(&mut commands, target_transform.translation);
                }
            }
        }
    }
}

pub(crate) fn cleanup_dead_entities(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut match_state: ResMut<MatchState>,
    dead_entities: Query<(
        Entity,
        &Transform,
        &Team,
        &Selectable,
        &Health,
        Option<&Structure>,
        Option<&Unit>,
        Option<&FogMemoryVisible>,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    for (entity, transform, team, selectable, health, structure, unit, fog_memory) in &dead_entities
    {
        if health.current > 0.0 || (structure.is_none() && unit.is_none()) {
            continue;
        }
        let is_structure = structure.is_some();
        let remembered_fog_structure = is_structure && fog_memory.is_some();
        if relations.are_allied(*team, player_team) {
            if is_structure {
                match_state.structures_lost += 1;
            } else {
                match_state.units_lost += 1;
            }
        } else if is_structure {
            match_state.enemy_structures_destroyed += 1;
        } else {
            match_state.enemy_units_destroyed += 1;
        }
        if let Some(asset_server) = asset_server.as_deref() {
            spawn_destruction_effects(
                &mut commands,
                asset_server,
                transform.translation,
                selectable.radius,
                is_structure,
                *team,
                remembered_fog_structure,
            );
        } else if remembered_fog_structure {
            spawn_fog_memory_structure_remnant(
                &mut commands,
                None,
                transform.translation,
                selectable.radius,
            );
        } else if is_structure {
            spawn_structure_destruction_vfx(
                &mut commands,
                transform.translation,
                selectable.radius,
                *team,
            );
        }
        commands.entity(entity).try_despawn();
    }
}

pub(crate) fn can_unit_capture_target(
    unit: &Unit,
    target: Entity,
    team: Team,
    relations: &TeamRelations,
    structures: &Query<(Entity, &Structure, &Team, Option<&UnderConstruction>), With<Health>>,
) -> bool {
    if capture_time_for_unit(unit) <= 0.0 {
        return false;
    }
    let Ok((_entity, _structure, target_team, under_construction)) = structures.get(target) else {
        return false;
    };
    structure_is_constructed(under_construction)
        && can_capture_structure_team(team, *target_team, relations)
}

pub(crate) fn can_capture_structure_team(
    capturer_team: Team,
    target_team: Team,
    relations: &TeamRelations,
) -> bool {
    target_team == Team::Neutral || relations.are_enemies(capturer_team, target_team)
}

pub(crate) fn nearest_enemy_target_with_snap_radius(
    point: Vec3,
    visible_team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
    snap_radius: f32,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        _structure,
        _under_construction,
    ) in selectable_q
    {
        if !visibility.visible
            || *team == visible_team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + snap_radius && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn capture_time_for_unit(unit: &Unit) -> f32 {
    registry::entity(unit.id).map_or(0.0, |def| def.capture_time)
}

pub(crate) fn can_unit_capture(unit: &Unit) -> bool {
    capture_time_for_unit(unit) > 0.0
}

pub(crate) fn repair_selected_structures(
    commands: &mut Commands,
    team: Team,
    selected_structures: &Query<SelectedRepairStructureItem<'_>, With<Selected>>,
    economies: &mut Economies,
) -> bool {
    let mut started_any = false;
    for (entity, structure, structure_team, health, repair, under_construction) in
        selected_structures
    {
        if *structure_team != team
            || repair.is_some()
            || !structure_is_constructed(under_construction)
            || health.current <= 0.0
            || health.current >= health.max
        {
            continue;
        }
        let Some(def) = registry::entity(structure.id) else {
            continue;
        };
        let cost = structure_repair_cost(def, health);
        if !economies.get(team).can_afford(cost) {
            continue;
        }
        if !economies.get_mut(team).spend(cost) {
            continue;
        }
        commands.entity(entity).try_insert(ManualStructureRepair {
            points_remaining: missing_structure_hitpoints(health),
        });
        started_any = true;
    }
    started_any
}

pub(crate) fn structure_repair_cost(def: &registry::EntityDef, health: &Health) -> registry::Cost {
    let hp_ratio = if health.max > 0.0 {
        (missing_structure_hitpoints(health) / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    registry::Cost {
        ore: (def.cost.ore as f32 * STRUCTURE_MANUAL_REPAIR_COST_RATIO * hp_ratio).ceil() as i32,
        crystal: (def.cost.crystal as f32 * STRUCTURE_MANUAL_REPAIR_COST_RATIO * hp_ratio).ceil()
            as i32,
    }
}

pub(crate) fn missing_structure_hitpoints(health: &Health) -> f32 {
    (health.max - health.current).max(0.0)
}

pub(crate) fn update_manual_structure_repairs(
    mut commands: Commands,
    time: Res<Time>,
    mut structures: Query<(Entity, &mut Health, &mut ManualStructureRepair), With<Structure>>,
) {
    for (entity, mut health, mut repair) in &mut structures {
        if health.current <= 0.0 {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
            continue;
        }
        let repaired = (STRUCTURE_MANUAL_REPAIR_HP_PER_SECOND * time.delta_secs())
            .min(repair.points_remaining)
            .min(missing_structure_hitpoints(&health));
        if repaired <= 0.0 {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
            continue;
        }
        health.current = (health.current + repaired).min(health.max);
        repair.points_remaining -= repaired;
        if repair.points_remaining <= 0.0 || health.current >= health.max {
            commands
                .entity(entity)
                .try_remove::<ManualStructureRepair>();
        }
    }
}

pub(crate) fn nearest_enemy_pressure_distance(
    team: Team,
    position: Vec3,
    radius: f32,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
) -> f32 {
    let mut best_distance = f32::INFINITY;
    for (_, unit_team, transform, _, health, _) in units {
        if !relations.are_enemies(team, *unit_team) || health.current <= 0.0 {
            continue;
        }
        let distance = xz_distance(position, transform.translation);
        if distance <= radius {
            best_distance = best_distance.min(distance);
        }
    }
    best_distance
}

pub(crate) fn nearest_enemy_position(
    team: Team,
    origin: Vec3,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Vec3> {
    let mut nearest = None;
    let mut distance = f32::MAX;
    for (_, target_team, transform) in targets {
        if *target_team == team || *target_team == Team::Neutral {
            continue;
        }
        let d = xz_distance(origin, transform.translation);
        if d < distance {
            nearest = Some(transform.translation);
            distance = d;
        }
    }
    nearest
}

pub(crate) fn nearest_enemy_entity(
    team: Team,
    origin: Vec3,
    targets: &Query<(Entity, &Team, &Transform), With<Health>>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut distance = f32::MAX;
    for (entity, target_team, transform) in targets {
        if *target_team == team || *target_team == Team::Neutral {
            continue;
        }
        let d = xz_distance(origin, transform.translation);
        if d < distance {
            nearest = Some(entity);
            distance = d;
        }
    }
    nearest
}

pub(crate) fn update_mine_layers(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    map_bounds: Res<MapBounds>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut next_id: ResMut<NextSpawnId>,
    mut layers: Query<(
        Entity,
        &Team,
        &Transform,
        &Health,
        &mut MineLayer,
        Option<&EmpDisabled>,
        Option<&VisualFaction>,
    )>,
    mines: Query<(&Team, &Transform, &Mine)>,
) {
    let Some(mine_def) = registry::entity("LandMine") else {
        return;
    };
    let player_team = visible_player_team(visible_player.as_deref());
    for (layer_entity, team, transform, health, mut layer, emp, visual_faction) in &mut layers {
        if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
            continue;
        }
        layer.cooldown -= time.delta_secs();
        if layer.cooldown > 0.0 {
            continue;
        }
        let active_mines = mines
            .iter()
            .filter(|(mine_team, _, mine)| {
                **mine_team == *team && mine.source == Some(layer_entity)
            })
            .count();
        if active_mines >= layer.limit {
            continue;
        }
        layer.cooldown = layer.deploy_interval;
        let deploy_position =
            next_mine_deploy_position_in_bounds(transform.translation, &mut layer, *map_bounds);
        let nearby_friendly_mine = mines.iter().any(|(mine_team, mine_transform, _)| {
            *mine_team == *team
                && xz_distance(mine_transform.translation, deploy_position) <= layer.spacing
        });
        if nearby_friendly_mine {
            continue;
        }
        let mine_entity = spawn_unit_with_visual_faction(
            &mut commands,
            &asset_server,
            &mut next_id,
            "LandMine",
            *team,
            deploy_position,
            0,
            visual_faction
                .copied()
                .map(|faction| faction.0)
                .or_else(|| default_visual_faction(*team)),
            player_team,
        );
        commands.entity(mine_entity).try_insert(Mine {
            damage: layer.damage,
            trigger_radius: mine_def.mine_trigger_radius,
            blast_radius: mine_def.mine_blast_radius,
            arming_remaining: mine_def.mine_arming_delay,
            source: Some(layer_entity),
        });
    }
}

pub(crate) fn next_mine_deploy_position_in_bounds(
    origin: Vec3,
    layer: &mut MineLayer,
    bounds: MapBounds,
) -> Vec3 {
    let (x, z) = MINE_DEPLOY_OFFSETS[layer.deploy_index % MINE_DEPLOY_OFFSETS.len()];
    layer.deploy_index += 1;
    let direction = Vec2::new(x, z).normalize_or_zero();
    bounds.clamp_ground_point(
        Vec3::new(
            origin.x + direction.x * layer.deploy_radius,
            0.0,
            origin.z + direction.y * layer.deploy_radius,
        ),
        0.4,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct MineTargetSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) radius: f32,
}

pub(crate) fn update_mines(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut kill_credits: ResMut<KillCredits>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut mine_queries: ParamSet<(
        Query<(Entity, &Team, &Transform, &mut Mine, &Health)>,
        Query<
            (
                Entity,
                &Team,
                &Transform,
                &Selectable,
                &Unit,
                &MovementDomain,
                &Health,
            ),
            (With<Unit>, Without<Mine>),
        >,
        Query<(
            &mut Health,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
        )>,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let target_snapshots = {
        let targets = mine_queries.p1();
        targets
            .iter()
            .filter_map(
                |(entity, team, transform, selectable, unit, domain, health)| {
                    if health.current <= 0.0
                        || unit.speed <= 0.0
                        || *domain != MovementDomain::Terrain
                    {
                        return None;
                    }
                    Some(MineTargetSnapshot {
                        entity,
                        team: *team,
                        position: transform.translation,
                        radius: selectable.radius,
                    })
                },
            )
            .collect::<Vec<_>>()
    };

    let mut damage_events = Vec::new();
    let mut mines_to_despawn = Vec::new();
    {
        let mut mines = mine_queries.p0();
        for (mine_entity, mine_team, mine_transform, mut mine, mine_health) in &mut mines {
            if mine_health.current <= 0.0 {
                continue;
            }
            mine.arming_remaining -= time.delta_secs();
            if mine.arming_remaining > 0.0 {
                continue;
            }
            let triggered = target_snapshots.iter().any(|target| {
                mine_can_damage_target(
                    *mine_team,
                    mine_transform.translation,
                    mine.trigger_radius,
                    target,
                    &relations,
                )
            });
            if !triggered {
                continue;
            }
            let source = mine.source.unwrap_or(mine_entity);
            let mut impacted = false;
            for target in &target_snapshots {
                if !mine_can_damage_target(
                    *mine_team,
                    mine_transform.translation,
                    mine.blast_radius,
                    target,
                    &relations,
                ) {
                    continue;
                }
                damage_events.push((
                    target.entity,
                    mine.damage,
                    mine_transform.translation,
                    target.position,
                    target.radius,
                    *mine_team,
                    target.team,
                    source,
                ));
                impacted = true;
            }
            if impacted {
                latest_battle_event.focus = Some(mine_transform.translation);
                commands.spawn((
                    ShotPulse {
                        from: mine_transform.translation + Vec3::Y * 0.2,
                        to: mine_transform.translation + Vec3::Y * 1.0,
                        ttl: 0.13,
                        team: *mine_team,
                    },
                    MatchScopedEntity,
                ));
            }
            mines_to_despawn.push(mine_entity);
        }
    }

    for mine_entity in mines_to_despawn {
        commands.entity(mine_entity).try_despawn();
    }

    {
        let mut health_q = mine_queries.p2();
        for (target, damage, from, to, target_radius, team, target_team, source) in damage_events {
            let Ok((mut health, shield, passive_shield)) = health_q.get_mut(target) else {
                continue;
            };
            if health.current <= 0.0 {
                continue;
            }
            let applied_damage = damage * support_damage_scale(shield, passive_shield);
            health.current -= applied_damage;
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.45,
                    to: to + Vec3::Y * 0.45,
                    ttl: 0.16,
                    team,
                },
                MatchScopedEntity,
            ));
            spawn_impact_burst(
                &mut commands,
                to,
                target_radius,
                applied_damage,
                false,
                team,
                ImpactBurstKind::Explosive,
            );
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    match_state.units_lost += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                spawn_combat_wreckage(&mut commands, &asset_server, to, target_radius);
                commands.entity(target).try_despawn();
            }
        }
    }
}

pub(crate) fn mine_can_damage_target(
    mine_team: Team,
    mine_position: Vec3,
    radius: f32,
    target: &MineTargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    relations.are_enemies(mine_team, target.team)
        && xz_distance(mine_position, target.position) <= radius + target.radius
}

#[derive(Clone, Copy)]
pub(crate) struct RepairerSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) radius: f32,
    pub(crate) target: Entity,
    pub(crate) capability: RepairCapability,
    pub(crate) can_move: bool,
    pub(crate) disabled: bool,
    pub(crate) alive: bool,
}

pub(crate) fn try_grant_veterancy_rank(
    commands: &mut Commands,
    entity: Entity,
    rank_delta: u8,
    units: &mut Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &mut Health,
            Option<&mut Weapon>,
            &mut VisionRadius,
            &mut Veterancy,
            Option<&VisibilityState>,
        ),
        With<Unit>,
    >,
) -> bool {
    let Ok((
        _entity,
        team,
        transform,
        selectable,
        mut health,
        weapon,
        mut vision,
        mut veteran,
        visibility,
    )) = units.get_mut(entity)
    else {
        return false;
    };
    let target_rank = veteran
        .rank
        .saturating_add(rank_delta)
        .min(VETERANCY_MAX_RANK);
    if target_rank <= veteran.rank {
        return false;
    }

    let old_health_ratio = if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let idx = target_rank as usize;
    veteran.experience_points = veteran.experience_points.max(VETERANCY_KILLS_BY_RANK[idx]);
    veteran.rank = target_rank;
    health.max = (veteran.base_health * VETERANCY_HP_MULTIPLIER_BY_RANK[idx]).ceil();
    health.current = (old_health_ratio * health.max)
        .ceil()
        .clamp(1.0, health.max);
    if let Some(mut weapon) = weapon {
        weapon.damage =
            (veteran.base_damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[idx] * 10.0).round() / 10.0;
        weapon.range = veteran.base_range + VETERANCY_RANGE_BONUS_BY_RANK[idx];
    }
    vision.0 = veteran.base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[idx];
    spawn_veterancy_promotion_effect(
        commands,
        transform.translation,
        selectable.radius,
        *team,
        target_rank,
        visibility,
    );
    true
}

pub(crate) fn apply_kill_credits(
    mut commands: Commands,
    mut kill_credits: ResMut<KillCredits>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut units: Query<
        (
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &mut Health,
            Option<&mut Weapon>,
            &mut VisionRadius,
            &mut Veterancy,
            Option<&VisibilityState>,
        ),
        With<Unit>,
    >,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let credits = std::mem::take(&mut kill_credits.0);
    for source in credits {
        let Ok((
            team,
            transform,
            selectable,
            unit,
            mut health,
            weapon,
            mut vision,
            mut veteran,
            visibility,
        )) = units.get_mut(source)
        else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        veteran.experience_points = veteran.experience_points.saturating_add(1);
        let target_rank = rank_for_experience_points(veteran.experience_points);
        if target_rank <= veteran.rank {
            continue;
        }

        let old_health_ratio = if health.max > 0.0 {
            (health.current / health.max).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let idx = target_rank as usize;
        veteran.rank = target_rank;
        health.max = (veteran.base_health * VETERANCY_HP_MULTIPLIER_BY_RANK[idx]).ceil();
        health.current = (old_health_ratio * health.max)
            .ceil()
            .clamp(1.0, health.max);
        if let Some(mut weapon) = weapon {
            weapon.damage = (veteran.base_damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[idx] * 10.0)
                .round()
                / 10.0;
            weapon.range = veteran.base_range + VETERANCY_RANGE_BONUS_BY_RANK[idx];
        }
        vision.0 = veteran.base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[idx];
        spawn_veterancy_promotion_effect(
            &mut commands,
            transform.translation,
            selectable.radius,
            *team,
            target_rank,
            visibility,
        );
        if *team == player_team {
            let unit_label = localized_entity_label(unit.id);
            push_battle_log(
                &mut battle_log,
                format!(
                    "{}: {unit_label} {}{target_rank}",
                    t("单位晋升", "Unit promoted"),
                    t("等级", "Lv")
                ),
                Some(transform.translation),
            );
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::UnitPromoted);
        }
    }
}

pub(crate) fn rank_for_experience_points(points: u32) -> u8 {
    let mut rank = 0;
    for (idx, kills) in VETERANCY_KILLS_BY_RANK.iter().enumerate() {
        if points >= *kills {
            rank = idx as u8;
        }
    }
    rank.min(VETERANCY_MAX_RANK)
}

pub(crate) fn update_veterancy_regeneration(
    time: Res<Time>,
    mut units: Query<(&Veterancy, &mut Health, Option<&EmpDisabled>), With<Unit>>,
) {
    for (veteran, mut health, emp) in &mut units {
        if veteran.rank < VETERANCY_MAX_RANK
            || health.current <= 0.0
            || health.current >= health.max
            || emp.is_some_and(|emp| emp.remaining > 0.0)
        {
            continue;
        }
        health.current =
            (health.current + VETERANCY_ELITE_REGEN_PER_SECOND * time.delta_secs()).min(health.max);
    }
}

pub(crate) fn chase_attack_targets(
    mut commands: Commands,
    relations: Res<TeamRelations>,
    attackers: Query<(Entity, &Transform, &Unit, &Weapon, &AttackOrder)>,
    targets: Query<(&Transform, &Team, &MovementDomain, &Health)>,
    teams: Query<&Team>,
) {
    for (entity, transform, unit, weapon, attack_order) in &attackers {
        let Ok(attacker_team) = teams.get(entity) else {
            continue;
        };
        let Ok((target_transform, target_team, target_domain, target_health)) =
            targets.get(attack_order.target)
        else {
            clear_attack_chase_order(&mut commands, entity);
            continue;
        };
        if !attack_order_target_valid(
            attacker_team,
            target_team,
            *target_domain,
            target_health,
            weapon,
            &relations,
        ) {
            clear_attack_chase_order(&mut commands, entity);
            continue;
        }
        let distance = xz_distance(transform.translation, target_transform.translation);
        if distance > weapon.range * 0.9 {
            if unit.speed <= 0.0 {
                if distance > weapon.range {
                    clear_attack_chase_order(&mut commands, entity);
                } else {
                    commands.entity(entity).try_remove::<MoveOrder>();
                }
                continue;
            }
            commands.entity(entity).try_insert(MoveOrder {
                target: target_transform.translation,
            });
        } else {
            commands.entity(entity).try_remove::<MoveOrder>();
        }
    }
}

pub(crate) fn move_units(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut kill_credits: ResMut<KillCredits>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    terrain: Res<TerrainHeightField>,
    mut unit_queries: ParamSet<(
        Query<(
            Entity,
            &Team,
            &Unit,
            &MovementDomain,
            &Selectable,
            &mut Transform,
            &MoveOrder,
            Option<&mut PlannedPath>,
            Option<&ChronoRelay>,
            Option<&EmpDisabled>,
            &Health,
        )>,
        Query<(
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &MovementDomain,
            &Health,
        )>,
        Query<(
            &mut Health,
            Option<&SupportShield>,
            Option<&PassiveSupportShield>,
        )>,
        Query<
            (&Transform, &Selectable, Option<&Health>),
            (
                Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
                Without<Unit>,
            ),
        >,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let static_obstacles = {
        let obstacles = unit_queries.p3();
        obstacles
            .iter()
            .filter_map(|(transform, selectable, health)| {
                if health.is_some_and(|health| health.current <= 0.0) {
                    return None;
                }
                Some(MovementObstacleSnapshot {
                    position: transform.translation,
                    radius: selectable.radius,
                })
            })
            .collect::<Vec<_>>()
    };
    let crush_targets = {
        let units = unit_queries.p1();
        units
            .iter()
            .filter_map(
                |(entity, team, transform, selectable, unit, domain, health)| {
                    if health.current <= 0.0
                        || !unit.can_be_crushed
                        || unit.speed <= 0.0
                        || *domain != MovementDomain::Terrain
                    {
                        return None;
                    }
                    Some(CrushTargetSnapshot {
                        entity,
                        team: *team,
                        position: transform.translation,
                        radius: selectable.radius,
                    })
                },
            )
            .collect::<Vec<_>>()
    };

    let mut crush_events = Vec::new();
    {
        let mut movers = unit_queries.p0();
        for (
            entity,
            team,
            unit,
            domain,
            selectable,
            mut transform,
            order,
            mut planned_path,
            chrono,
            emp,
            health,
        ) in &mut movers
        {
            if health.current <= 0.0 || emp.is_some_and(|emp| emp.remaining > 0.0) {
                continue;
            }
            if unit.speed <= 0.0 {
                commands.entity(entity).try_remove::<MoveOrder>();
                continue;
            }
            let mut target = order.target;
            // Follow the A* waypoints while any remain; the final leg goes straight
            // to the order target.
            if let Some(path) = planned_path.as_mut() {
                while path.next < path.waypoints.len()
                    && xz_distance(transform.translation, path.waypoints[path.next])
                        < NAV_WAYPOINT_REACHED_M
                {
                    path.next += 1;
                }
                if path.next < path.waypoints.len() {
                    target = path.waypoints[path.next];
                } else {
                    commands.entity(entity).try_remove::<PlannedPath>();
                }
            }
            target.y = transform.translation.y;
            let delta = target - transform.translation;
            let distance = delta.length();
            if distance < MOVE_ORDER_REACHED_DISTANCE_M {
                commands.entity(entity).try_remove::<MoveOrder>();
                continue;
            }
            let speed = unit.speed * chrono.map_or(1.0, |chrono| chrono.speed_multiplier);
            let step = speed * time.delta_secs();
            let previous_position = transform.translation;
            let direction = delta.normalize();
            let move_direction = if *domain == MovementDomain::Terrain {
                movement_direction_around_static_obstacles(
                    previous_position,
                    target,
                    direction,
                    selectable.radius,
                    &static_obstacles,
                )
            } else {
                direction
            };
            let intended_position = previous_position + move_direction * step;
            // Cliff faces are impassable on foot even when steering pushes into
            // them; A* already routes around, this is the last-resort guard.
            if *domain == MovementDomain::Terrain
                && terrain.step_blocked(previous_position, intended_position)
            {
                continue;
            }
            transform.translation += move_direction * step.min(distance);
            if *domain == MovementDomain::Terrain && !terrain.is_flat() {
                transform.translation.y = terrain.height_at(transform.translation);
            }
            let actual_position = transform.translation;
            let look_at = transform.translation + move_direction;
            if xz_distance(transform.translation, look_at) > 0.05 {
                transform.look_at(look_at, Vec3::Y);
            }
            if !unit.can_crush || *domain != MovementDomain::Terrain {
                continue;
            }
            let actual_displacement = xz_distance(previous_position, actual_position);
            let intended_displacement = xz_distance(previous_position, intended_position);
            if actual_displacement.max(intended_displacement) < CRUSH_MIN_FRAME_DISPLACEMENT_M {
                continue;
            }
            for target in &crush_targets {
                if target.entity == entity
                    || !relations.are_enemies(*team, target.team)
                    || !can_crush_target(
                        previous_position,
                        actual_position,
                        intended_position,
                        selectable.radius,
                        target,
                    )
                {
                    continue;
                }
                crush_events.push((
                    target.entity,
                    entity,
                    *team,
                    target.team,
                    previous_position,
                    target.position,
                    target.radius,
                ));
            }
        }
    }

    {
        let mut health_q = unit_queries.p2();
        for (target, source, team, target_team, from, to, target_radius) in crush_events {
            let Ok((mut health, shield, passive_shield)) = health_q.get_mut(target) else {
                continue;
            };
            if health.current <= 0.0 {
                continue;
            }
            let damage = CRUSH_DAMAGE * support_damage_scale(shield, passive_shield);
            health.current -= damage;
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.45,
                    to: to + Vec3::Y * 0.45,
                    ttl: 0.18,
                    team,
                },
                MatchScopedEntity,
            ));
            latest_battle_event.focus = Some(to);
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    match_state.units_lost += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                spawn_combat_wreckage(&mut commands, &asset_server, to, target_radius);
                commands.entity(target).try_despawn();
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CrushTargetSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MovementObstacleSnapshot {
    pub(crate) position: Vec3,
    pub(crate) radius: f32,
}

pub(crate) fn movement_direction_around_static_obstacles(
    position: Vec3,
    target: Vec3,
    desired_direction: Vec3,
    mover_radius: f32,
    obstacles: &[MovementObstacleSnapshot],
) -> Vec3 {
    let desired_xz = Vec2::new(desired_direction.x, desired_direction.z).normalize_or_zero();
    if desired_xz == Vec2::ZERO {
        return desired_direction;
    }
    let target_distance = xz_distance(position, target);
    if target_distance <= f32::EPSILON {
        return desired_direction;
    }
    let lookahead = target_distance.min(MOVEMENT_OBSTACLE_LOOKAHEAD_M);
    let start = Vec2::new(position.x, position.z);
    let end = start + desired_xz * lookahead;

    let mut best_steer = None;
    for obstacle in obstacles {
        let center = Vec2::new(obstacle.position.x, obstacle.position.z);
        let clearance = mover_radius + obstacle.radius + MOVEMENT_OBSTACLE_CLEARANCE_M;
        let to_center = center - start;
        let projection = to_center.dot(desired_xz);
        if projection < -clearance || projection > lookahead {
            continue;
        }
        let segment_distance = distance_point_to_xz_segment(
            obstacle.position,
            position,
            Vec3::new(end.x, position.y, end.y),
        );
        let current_distance = start.distance(center);
        if segment_distance > clearance && current_distance > clearance {
            continue;
        }

        let side = desired_xz.perp_dot(to_center);
        let tangent = if side >= 0.0 {
            Vec2::new(desired_xz.y, -desired_xz.x)
        } else {
            Vec2::new(-desired_xz.y, desired_xz.x)
        };
        let away = (start - center).normalize_or_zero();
        let steer = (desired_xz + tangent * MOVEMENT_OBSTACLE_STEER_WEIGHT + away * 0.35)
            .normalize_or_zero();
        if steer == Vec2::ZERO {
            continue;
        }
        let urgency = clearance - segment_distance.min(current_distance);
        if best_steer.is_none_or(|(best_urgency, _)| urgency > best_urgency) {
            best_steer = Some((urgency, steer));
        }
    }

    if let Some((_, steer)) = best_steer {
        Vec3::new(steer.x, desired_direction.y, steer.y).normalize_or_zero()
    } else {
        desired_direction
    }
}

pub(crate) fn can_crush_target(
    from_position: Vec3,
    actual_position: Vec3,
    intended_position: Vec3,
    crusher_radius: f32,
    target: &CrushTargetSnapshot,
) -> bool {
    let crush_distance = crusher_radius + target.radius + CRUSH_RADIUS_MARGIN_M;
    distance_point_to_xz_segment(target.position, from_position, actual_position) <= crush_distance
        || (xz_distance(actual_position, intended_position) > CRUSH_MIN_FRAME_DISPLACEMENT_M
            && distance_point_to_xz_segment(target.position, from_position, intended_position)
                <= crush_distance)
}

#[derive(Clone, Copy)]
pub(crate) struct TargetSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) radius: f32,
    pub(crate) visible: bool,
    pub(crate) is_structure: bool,
    pub(crate) movement_domain: MovementDomain,
}

/// Defense structures slowly rotate while idle, scanning for targets (godot's
/// `RotateRandomlyWhenLookingForTargetsIdle`). A weapon's `cooldown_left` stays
/// >0 for most of an engagement (it counts down between shots) and stays 0 when
/// no target is ever acquired, so it is a good "currently idle" proxy without a
/// second target-scan query. The sweep direction (-1/0/+1) changes in ~0.5s
/// buckets, pseudo-randomly per entity, for a back-and-forth scan.
pub(crate) fn update_idle_tower_scan(
    time: Res<Time>,
    mut towers: Query<
        (Entity, &mut Transform, &Weapon, Option<&UnderConstruction>),
        With<Structure>,
    >,
) {
    let bucket = (time.elapsed_secs() * 2.0) as u64;
    for (entity, mut transform, weapon, under_construction) in &mut towers {
        if under_construction.is_some() || weapon.cooldown_left > 0.0 {
            continue;
        }
        let hash = entity
            .to_bits()
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(bucket.wrapping_mul(0x2545_F491_4F6C_DD1D));
        let multiplier = (hash % 3) as f32 - 1.0;
        if multiplier == 0.0 {
            continue;
        }
        transform
            .rotate_y(IDLE_TOWER_SCAN_DEG_PER_SEC.to_radians() * time.delta_secs() * multiplier);
    }
}

pub(crate) fn combat(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    economies: Res<Economies>,
    relations: Res<TeamRelations>,
    player_factions: Res<PlayerFactions>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut attackers: Query<(
        Entity,
        &Transform,
        &Team,
        &mut Weapon,
        &VisionRadius,
        Option<&Unit>,
        Option<&HoldPosition>,
        Option<&AttackOrder>,
        Option<&FollowOrder>,
        Option<&Garrison>,
        Option<&EmpDisabled>,
        Option<&MoveOrder>,
        Option<&Structure>,
    )>,
    mut health_q: Query<(
        Entity,
        &Transform,
        &Team,
        &Selectable,
        &MovementDomain,
        Option<&Structure>,
        &mut Health,
        Option<&SupportShield>,
        Option<&PassiveSupportShield>,
        Option<&FogMemoryVisible>,
    )>,
    mut match_state: ResMut<MatchState>,
    mut latest_battle_event: ResMut<LatestBattleEvent>,
    mut kill_credits: ResMut<KillCredits>,
    mut battle_log: ResMut<BattleLog>,
    mut audio_feedback: ResMut<AudioFeedback>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let targets: Vec<_> = health_q
        .iter()
        .filter(|(_, _, _, _, _, _, health, _, _, _)| health.current > 0.0)
        .map(
            |(entity, transform, team, selectable, movement_domain, structure, _, _, _, _)| {
                TargetSnapshot {
                    entity,
                    team: *team,
                    position: transform.translation,
                    radius: selectable.radius,
                    visible: true,
                    movement_domain: *movement_domain,
                    is_structure: structure.is_some(),
                }
            },
        )
        .collect();
    let mut damage_events = Vec::new();

    for (
        entity,
        transform,
        team,
        mut weapon,
        vision,
        unit,
        hold_position,
        attack_order,
        follow_order,
        garrison,
        emp,
        move_order,
        structure,
    ) in &mut attackers
    {
        if emp.is_some_and(|emp| emp.remaining > 0.0) {
            continue;
        }
        if follow_order.is_some_and(|follow| follow.allow_enemy) {
            continue;
        }
        if powered_combat_offline(team, structure, &economies) {
            continue;
        }
        weapon.cooldown_left = (weapon.cooldown_left - time.delta_secs()).max(0.0);
        let attack_damage = garrison.map_or(weapon.damage, |garrison| {
            garrison.count as f32 * garrison.damage_per_unit
        });
        if attack_damage <= 0.0 {
            continue;
        }
        let attacker_faction = player_factions.faction(*team);
        if is_tesla_fence_structure(structure) {
            if weapon.cooldown_left > 0.0 {
                continue;
            }
            let zap_targets = targets
                .iter()
                .copied()
                .filter(|target| {
                    can_tesla_fence_zap_target(
                        *team,
                        transform.translation,
                        weapon.range,
                        target,
                        &relations,
                    )
                })
                .collect::<Vec<_>>();
            if zap_targets.is_empty() {
                continue;
            }
            weapon.cooldown_left = weapon_cooldown_for_faction(attacker_faction, weapon.cooldown);
            for target in zap_targets {
                let impact_kind =
                    impact_burst_kind_for_attacker(&weapon, unit, structure, target.is_structure);
                damage_events.push((
                    target.entity,
                    attack_damage,
                    transform.translation,
                    target.position,
                    target.radius,
                    *team,
                    attacker_faction,
                    target.is_structure,
                    target.team,
                    player_factions.faction(target.team),
                    impact_kind,
                    entity,
                ));
            }
            continue;
        }
        let ordered_target = attack_order.and_then(|order| {
            targets
                .iter()
                .find(|target| {
                    target.entity == order.target
                        && relations.are_enemies(*team, target.team)
                        && can_attack_domain(&weapon, target.movement_domain)
                })
                .copied()
        });
        let moving_with_active_order = moving_weapon_fire_blocked(unit, move_order);
        let auto_target = if ordered_target.is_none()
            && !moving_with_active_order
            && !hold_position.is_some_and(|hold| hold.enabled)
        {
            nearest_enemy_for_auto_acquire(
                *team,
                transform.translation,
                &weapon,
                vision,
                unit,
                &targets,
                &relations,
            )
        } else {
            None
        };
        let target = ordered_target.or(auto_target);

        let Some(target) = target else {
            continue;
        };
        if ordered_target.is_none() {
            commands.entity(entity).try_insert(AttackOrder {
                target: target.entity,
            });
        }
        if moving_with_active_order {
            continue;
        }
        if xz_distance(transform.translation, target.position) > weapon.range
            || weapon.cooldown_left > 0.0
        {
            continue;
        }
        weapon.cooldown_left = weapon_cooldown_for_faction(attacker_faction, weapon.cooldown);
        let damage = weapon_damage_against_target(&weapon, attack_damage, target.is_structure);
        let impact_kind =
            impact_burst_kind_for_attacker(&weapon, unit, structure, target.is_structure);
        damage_events.push((
            target.entity,
            damage,
            transform.translation,
            target.position,
            target.radius,
            *team,
            attacker_faction,
            target.is_structure,
            target.team,
            player_factions.faction(target.team),
            impact_kind,
            entity,
        ));
        if weapon.splash_radius > 0.0 && weapon.splash_damage_multiplier > 0.0 {
            for splash_target in &targets {
                if splash_target.entity == target.entity
                    || !relations.are_enemies(*team, splash_target.team)
                    || !can_attack_domain(&weapon, splash_target.movement_domain)
                    || xz_distance(splash_target.position, target.position) > weapon.splash_radius
                {
                    continue;
                }
                let splash_damage = weapon_damage_against_target(
                    &weapon,
                    attack_damage,
                    splash_target.is_structure,
                ) * weapon.splash_damage_multiplier;
                let splash_impact_kind = impact_burst_kind_for_attacker(
                    &weapon,
                    unit,
                    structure,
                    splash_target.is_structure,
                );
                damage_events.push((
                    splash_target.entity,
                    splash_damage,
                    target.position,
                    splash_target.position,
                    splash_target.radius,
                    *team,
                    attacker_faction,
                    splash_target.is_structure,
                    splash_target.team,
                    player_factions.faction(splash_target.team),
                    splash_impact_kind,
                    entity,
                ));
            }
        }
    }

    for (
        target,
        damage,
        from,
        to,
        target_radius,
        team,
        attacker_faction,
        target_is_structure,
        target_team,
        target_faction,
        impact_kind,
        source,
    ) in damage_events
    {
        if let Ok((entity, _, _, _, _, _, mut health, shield, passive_shield, fog_memory)) =
            health_q.get_mut(target)
        {
            if health.current <= 0.0 {
                continue;
            }
            let applied_damage = applied_weapon_damage(
                damage,
                attacker_faction,
                target_team,
                target_faction,
                target_is_structure,
                shield,
                passive_shield,
            );
            health.current -= applied_damage;
            if applied_damage > 0.0 {
                commands.spawn((
                    PendingDamageNumber {
                        position: to,
                        amount: applied_damage,
                    },
                    MatchScopedEntity,
                ));
            }
            if relations.are_allied(target_team, player_team) && applied_damage > 0.0 {
                record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::WeaponHit);
                if push_under_attack_log(&mut battle_log, to, target_is_structure) {
                    record_voice_audio_feedback(
                        &mut audio_feedback,
                        if target_is_structure {
                            UnitVoiceEvent::BaseUnderAttack
                        } else {
                            UnitVoiceEvent::UnitUnderAttack
                        },
                    );
                }
            }
            // Muzzle flash at the barrel so shots read as fired, not conjured.
            spawn_combat_flash(
                &mut commands,
                from + Vec3::Y * 0.6,
                0.22,
                0.34,
                0.1,
                LinearRgba::new(1.0, 0.9, 0.55, 1.0),
            );
            // Tracer from shooter to target (longer-lived so it's noticeable)…
            commands.spawn((
                ShotPulse {
                    from: from + Vec3::Y * 0.6,
                    to: to + Vec3::Y * 0.6,
                    ttl: 0.13,
                    team,
                },
                MatchScopedEntity,
            ));
            // …plus a vertical impact flash on the target.
            commands.spawn((
                ShotPulse {
                    from: to + Vec3::Y * 0.05,
                    to: to + Vec3::Y * 1.1,
                    ttl: 0.15,
                    team,
                },
                MatchScopedEntity,
            ));
            spawn_impact_burst(
                &mut commands,
                to,
                target_radius,
                applied_damage,
                target_is_structure,
                team,
                impact_kind,
            );
            latest_battle_event.focus = Some(to);
            if health.current <= 0.0 {
                if relations.are_allied(target_team, player_team) {
                    if target_is_structure {
                        match_state.structures_lost += 1;
                    } else {
                        match_state.units_lost += 1;
                    }
                } else if target_is_structure {
                    match_state.enemy_structures_destroyed += 1;
                } else {
                    match_state.enemy_units_destroyed += 1;
                }
                kill_credits.0.push(source);
                if relations.are_allied(target_team, player_team) {
                    record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::Explosion);
                    if !target_is_structure {
                        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::UnitLost);
                    }
                }
                spawn_destruction_effects(
                    &mut commands,
                    &asset_server,
                    to,
                    target_radius,
                    target_is_structure,
                    target_team,
                    target_is_structure && fog_memory.is_some(),
                );
                commands.entity(entity).try_despawn();
            }
        }
    }
}

pub(crate) fn moving_weapon_fire_blocked(
    unit: Option<&Unit>,
    move_order: Option<&MoveOrder>,
) -> bool {
    move_order.is_some() && unit.is_some_and(|unit| unit.speed > 0.0)
}

pub(crate) fn is_tesla_fence_structure(structure: Option<&Structure>) -> bool {
    structure.is_some_and(|structure| structure.id == "TeslaFenceSegment")
}

pub(crate) fn can_tesla_fence_zap_target(
    team: Team,
    position: Vec3,
    range: f32,
    target: &TargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    relations.are_enemies(team, target.team)
        && !target.is_structure
        && target.movement_domain == MovementDomain::Terrain
        && xz_distance(position, target.position) <= range + target.radius
}

pub(crate) fn weapon_cooldown_for_faction(faction: Option<SkirmishFaction>, cooldown: f32) -> f32 {
    if faction == Some(SkirmishFaction::Alliance) {
        cooldown
    } else {
        cooldown * 1.08
    }
}

pub(crate) fn weapon_damage_against_target(
    weapon: &Weapon,
    base_damage: f32,
    target_is_structure: bool,
) -> f32 {
    if target_is_structure {
        base_damage * weapon.structure_damage_multiplier
    } else {
        base_damage
    }
}

pub(crate) fn nearest_enemy_in_range(
    team: Team,
    position: Vec3,
    range: f32,
    can_attack_air: bool,
    can_attack_ground: bool,
    targets: &[TargetSnapshot],
    relations: &TeamRelations,
) -> Option<TargetSnapshot> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for target in targets {
        if !relations.are_enemies(team, target.team) {
            continue;
        }
        if !target.visible {
            continue;
        }
        if !can_attack_domain_for_movement(
            can_attack_air,
            can_attack_ground,
            target.movement_domain,
        ) {
            continue;
        }
        let distance = xz_distance(position, target.position);
        if distance <= range && distance < nearest_distance {
            nearest = Some(*target);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn nearest_enemy_for_auto_acquire(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    vision: &VisionRadius,
    unit: Option<&Unit>,
    targets: &[TargetSnapshot],
    relations: &TeamRelations,
) -> Option<TargetSnapshot> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for target in targets {
        if !can_auto_acquire_target(team, position, weapon, vision, unit, target, relations) {
            continue;
        }
        let distance = xz_distance(position, target.position);
        if distance < nearest_distance {
            nearest = Some(*target);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn can_auto_acquire_target(
    team: Team,
    position: Vec3,
    weapon: &Weapon,
    vision: &VisionRadius,
    unit: Option<&Unit>,
    target: &TargetSnapshot,
    relations: &TeamRelations,
) -> bool {
    if !relations.are_enemies(team, target.team) {
        return false;
    }
    if !can_attack_domain(weapon, target.movement_domain) {
        return false;
    }
    let distance = xz_distance(position, target.position);
    if unit.is_some_and(|unit| unit.speed > 0.0) {
        distance <= vision.0
    } else {
        distance <= weapon.range
    }
}

pub(crate) fn can_attack_domain(weapon: &Weapon, domain: MovementDomain) -> bool {
    can_attack_domain_for_movement(weapon.can_attack_air, weapon.can_attack_ground, domain)
}

pub(crate) fn can_attack_domain_for_movement(
    can_attack_air: bool,
    can_attack_ground: bool,
    domain: MovementDomain,
) -> bool {
    match domain {
        MovementDomain::Air => can_attack_air,
        MovementDomain::Terrain => can_attack_ground,
    }
}

pub(crate) fn veterancy_rank_label(rank: u8) -> &'static str {
    match rank {
        0 => t("新兵", "Rookie"),
        1 => t("老兵", "Veteran"),
        _ => t("精英", "Elite"),
    }
}

pub(crate) fn veterancy_rank_badge(rank: u8) -> Option<&'static str> {
    match rank {
        1 => Some("V"),
        2.. => Some("E"),
        _ => None,
    }
}

pub(crate) fn structure_smoke_color(
    team: Team,
    life_ratio: f32,
    player_colors: &PlayerColorSlots,
) -> Color {
    let alpha = (0.18 + life_ratio * 0.38).clamp(0.0, 0.62);
    let [r, g, b] = player_colors.color_rgb(team);
    Color::srgba(0.12 + r * 0.08, 0.12 + g * 0.08, 0.12 + b * 0.08, alpha)
}

pub(crate) fn veterancy_promotion_color(rank: u8, life_ratio: f32) -> Color {
    let alpha = (0.22 + life_ratio * 0.6).clamp(0.0, 0.86);
    if rank >= VETERANCY_MAX_RANK {
        Color::srgba(0.18, 0.9, 1.0, alpha)
    } else {
        Color::srgba(1.0, 0.78, 0.16, alpha)
    }
}

/// A single thick health bar (drawn on the wide HudGizmos group, so it's one
/// solid strip — NOT a stack of thin lines, which the angled camera spreads into
/// separate slivers).
pub(crate) fn draw_health_bar(
    gizmos: &mut Gizmos<HudGizmos>,
    position: Vec3,
    radius: f32,
    health: Health,
    bar_right: Vec3,
) {
    let width = radius * 1.8;
    let center = Vec3::new(position.x, position.y + 1.25, position.z);
    let ratio = health.ratio();
    let half = width * 0.5;
    // Extend along the camera's right axis so the bar reads as horizontal on
    // screen (world-X alignment looked diagonal under the yawed camera).
    let left = center - bar_right * half;
    let right = center + bar_right * half;
    let fill = left + bar_right * (width * ratio);
    // Filled (green→red) then the depleted remainder (dark red) as adjacent,
    // non-overlapping segments meeting at `fill`.
    if ratio < 0.995 {
        gizmos.line(fill, right, Color::srgb(0.30, 0.05, 0.05));
    }
    if ratio > 0.005 {
        let fill_color = Color::srgb(
            0.92 + (0.22 - 0.92) * ratio,
            0.20 + (0.90 - 0.20) * ratio,
            0.16 + (0.30 - 0.16) * ratio,
        );
        gizmos.line(left, fill, fill_color);
    }
}
