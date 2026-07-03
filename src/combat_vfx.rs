//! Combat visual effects: shot pulses, impact bursts, wreckage, structure
//! destruction (fireball + smoke), and veterancy promotion flashes.

use bevy::prelude::*;

use crate::*;

#[derive(Component)]
pub(crate) struct CombatWreckage {
    pub(crate) remaining: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StructureDestructionVfxKind {
    ExplosionFireball,
    SmokeColumn,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct StructureDestructionVfx {
    pub(crate) kind: StructureDestructionVfxKind,
    pub(crate) remaining: f32,
    pub(crate) total: f32,
    pub(crate) radius: f32,
    pub(crate) team: Team,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct VeterancyPromotionEffect {
    pub(crate) rank: u8,
    pub(crate) remaining: f32,
    pub(crate) total: f32,
    pub(crate) radius: f32,
    pub(crate) team: Team,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ShotPulse {
    pub(crate) from: Vec3,
    pub(crate) to: Vec3,
    pub(crate) ttl: f32,
    pub(crate) team: Team,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImpactBurstKind {
    Ballistic,
    Explosive,
    Energy,
    Electric,
    Fire,
    Heavy,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ImpactBurst {
    pub(crate) remaining: f32,
    pub(crate) total: f32,
    pub(crate) radius: f32,
    pub(crate) power: f32,
    pub(crate) team: Team,
    pub(crate) kind: ImpactBurstKind,
}

pub(crate) fn update_combat_wreckage(
    mut commands: Commands,
    time: Res<Time>,
    mut wreckage: Query<(Entity, &mut CombatWreckage)>,
) {
    for (entity, mut wreckage) in &mut wreckage {
        if combat_wreckage_expired(&mut wreckage, time.delta_secs()) {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn update_structure_destruction_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut StructureDestructionVfx)>,
) {
    for (entity, mut effect) in &mut effects {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn update_veterancy_promotion_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut VeterancyPromotionEffect)>,
) {
    for (entity, mut effect) in &mut effects {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn combat_wreckage_expired(wreckage: &mut CombatWreckage, delta_secs: f32) -> bool {
    wreckage.remaining -= delta_secs;
    wreckage.remaining <= 0.0
}

/// Cached unit sphere so every emissive flash shares one mesh.
#[derive(Resource, Default)]
pub(crate) struct CombatFlashMesh(pub(crate) Option<Handle<Mesh>>);

/// A short-lived emissive blob that expands and fades — a solid, readable
/// muzzle/impact/death flash (the gizmo rings alone are hairline-thin and read
/// as dead in motion). Each flash owns its material so it can fade alone.
#[derive(Component)]
pub(crate) struct CombatFlash {
    pub(crate) remaining: f32,
    pub(crate) total: f32,
    pub(crate) start_scale: f32,
    pub(crate) end_scale: f32,
    pub(crate) material: Handle<StandardMaterial>,
    pub(crate) emissive: LinearRgba,
}

/// Spawns an expanding emissive flash at `position`. No-op in headless logic
/// tests (no render-asset stores).
pub(crate) fn spawn_combat_flash(
    commands: &mut Commands,
    position: Vec3,
    start_scale: f32,
    end_scale: f32,
    lifetime: f32,
    color: LinearRgba,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
        {
            return;
        }
        let mesh = match world.get_resource::<CombatFlashMesh>().and_then(|m| m.0.clone()) {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(Sphere::new(1.0).mesh().ico(2).unwrap_or_else(|_| Sphere::new(1.0).mesh().build()));
                if let Some(mut res) = world.get_resource_mut::<CombatFlashMesh>() {
                    res.0 = Some(handle.clone());
                }
                handle
            }
        };
        let material = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
            base_color: Color::srgba(color.red, color.green, color.blue, 0.85),
            emissive: color,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        world.spawn((
            Name::new("Combat flash"),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position + Vec3::Y * (end_scale * 0.4))
                .with_scale(Vec3::splat(start_scale)),
            CombatFlash {
                remaining: lifetime,
                total: lifetime,
                start_scale,
                end_scale,
                material,
                emissive: color,
            },
            MatchScopedEntity,
        ));
    });
}

/// Expands each flash toward `end_scale` while fading its emissive + alpha.
pub(crate) fn update_combat_flashes(
    time: Res<Time>,
    mut commands: Commands,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut flashes: Query<(Entity, &mut Transform, &mut CombatFlash)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut transform, mut flash) in &mut flashes {
        flash.remaining -= dt;
        if flash.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let life = (flash.remaining / flash.total).clamp(0.0, 1.0);
        let progress = 1.0 - life;
        let scale = flash.start_scale + (flash.end_scale - flash.start_scale) * progress;
        transform.scale = Vec3::splat(scale);
        if let Some(mut material) = materials.get_mut(&flash.material) {
            material.emissive = flash.emissive * life;
            material.base_color.set_alpha(life * 0.85);
        }
    }
}

/// Hot core color for an impact flash, matched to the burst kind.
pub(crate) fn impact_flash_color(kind: ImpactBurstKind) -> LinearRgba {
    match kind {
        ImpactBurstKind::Ballistic => LinearRgba::new(1.0, 0.86, 0.5, 1.0),
        ImpactBurstKind::Explosive | ImpactBurstKind::Heavy => LinearRgba::new(1.0, 0.55, 0.16, 1.0),
        ImpactBurstKind::Energy => LinearRgba::new(0.5, 0.95, 1.0, 1.0),
        ImpactBurstKind::Electric => LinearRgba::new(0.55, 0.8, 1.0, 1.0),
        ImpactBurstKind::Fire => LinearRgba::new(1.0, 0.42, 0.12, 1.0),
    }
}

pub(crate) fn spawn_destruction_effects(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
    is_structure: bool,
    team: Team,
    remembered_fog_structure: bool,
) {
    if is_structure && remembered_fog_structure {
        spawn_fog_memory_structure_remnant(commands, Some(asset_server), position, radius);
        return;
    }
    spawn_combat_wreckage(commands, asset_server, position, radius);
    // A bright expanding fireball so a kill visibly booms (bigger for buildings).
    let boom = if is_structure { radius * 1.35 } else { radius * 0.95 };
    spawn_combat_flash(
        commands,
        position + Vec3::Y * 0.3,
        (boom * 0.35).max(0.3),
        (boom + 0.7).clamp(0.8, 3.2),
        if is_structure { 0.55 } else { 0.38 },
        LinearRgba::new(1.0, 0.52, 0.16, 1.0),
    );
    if is_structure {
        spawn_structure_destruction_vfx(commands, position, radius, team);
    }
}

pub(crate) fn spawn_combat_wreckage(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
) {
    let radius = radius.max(0.45);
    let parent = commands
        .spawn((
            Name::new("Combat wreckage"),
            Transform::from_translation(Vec3::new(position.x, 0.02, position.z)),
            Visibility::Visible,
            CombatWreckage {
                remaining: COMBAT_WRECKAGE_LIFETIME_SECONDS,
            },
            MatchScopedEntity,
        ))
        .id();
    commands.entity(parent).with_children(|wreckage| {
        wreckage.spawn((
            Name::new("Scorch mark"),
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/craterLarge.glb"),
            )),
            Transform::from_scale(Vec3::splat((radius * 0.45).clamp(0.28, 0.8))),
            ScorchMark,
        ));
        for (index, (asset, offset, scale)) in [
            (
                "models/kenney-spacekit/rocks_smallA.glb",
                Vec3::new(0.42, 0.03, 0.18),
                0.22,
            ),
            (
                "models/kenney-spacekit/rocks_smallB.glb",
                Vec3::new(-0.34, 0.02, 0.28),
                0.2,
            ),
            (
                "models/kenney-spacekit/meteor_half.glb",
                Vec3::new(0.12, 0.04, -0.36),
                0.16,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let spread = radius * 0.62;
            wreckage.spawn((
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset))),
                Transform::from_translation(offset * spread)
                    .with_rotation(Quat::from_rotation_y(index as f32 * 1.9))
                    .with_scale(Vec3::splat(scale * radius)),
            ));
        }
    });
}

pub(crate) fn spawn_structure_destruction_vfx(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    team: Team,
) {
    let radius = radius.max(0.7);
    for (name, kind, ttl, y_offset, scale) in [
        (
            "ExplosionFireball",
            StructureDestructionVfxKind::ExplosionFireball,
            STRUCTURE_FIREBALL_LIFETIME_SECONDS,
            0.45,
            1.15,
        ),
        (
            "SmokeColumn",
            StructureDestructionVfxKind::SmokeColumn,
            STRUCTURE_SMOKE_COLUMN_LIFETIME_SECONDS,
            0.25,
            1.45,
        ),
    ] {
        commands.spawn((
            Name::new(name),
            Transform::from_translation(Vec3::new(position.x, y_offset, position.z)),
            StructureDestructionVfx {
                kind,
                remaining: ttl,
                total: ttl,
                radius: radius * scale,
                team,
            },
            MatchScopedEntity,
        ));
    }
}

pub(crate) fn impact_burst_power(
    damage: f32,
    target_radius: f32,
    target_is_structure: bool,
) -> f32 {
    let structure_bonus = if target_is_structure { 0.35 } else { 0.0 };
    ((damage.max(0.0) / 9.0).sqrt() + target_radius * 0.18 + structure_bonus).clamp(0.45, 2.2)
}

pub(crate) fn impact_burst_lifetime(power: f32) -> f32 {
    0.18 + power.clamp(0.45, 2.2) * 0.06
}

pub(crate) fn impact_burst_kind_for_attacker(
    weapon: &Weapon,
    unit: Option<&Unit>,
    structure: Option<&Structure>,
    target_is_structure: bool,
) -> ImpactBurstKind {
    let id = unit
        .map(|unit| unit.id)
        .or_else(|| structure.map(|structure| structure.id))
        .unwrap_or("");
    impact_burst_kind_for_entity_id(id, weapon, target_is_structure)
}

pub(crate) fn impact_burst_kind_for_entity_id(
    id: &str,
    weapon: &Weapon,
    target_is_structure: bool,
) -> ImpactBurstKind {
    match id {
        "FlameAssaultBuggy" => ImpactBurstKind::Fire,
        "TeslaCrawlerMk2" | "TeslaFenceSegment" | "ArcCoilDefenseTower" | "ShockTrooper" => {
            ImpactBurstKind::Electric
        }
        "PrismDefenseObelisk"
        | "LanceBeamDefenseTower"
        | "LanceBeamTank"
        | "PulseRifleCommando"
        | "ShieldTrooper"
        | "CryoSprayer" => ImpactBurstKind::Energy,
        "RocketInfantry"
        | "RocketTrooperRobot"
        | "FlakRocketTeam"
        | "FlakRocketTeamMk2"
        | "GrenadierTrooper"
        | "MortarTeam"
        | "LongbowMissileCrawler"
        | "ModularMissileCarrier"
        | "RocketGunship"
        | "BomberVTOL"
        | "HeavyBombardmentAirship"
        | "SiegeArtilleryVehicle"
        | "HammerSiegeTank"
        | "SiegeAirship" => ImpactBurstKind::Explosive,
        "RailgunTank"
        | "RailSniperTeam"
        | "RailArtilleryWalker"
        | "RailCannonBunker"
        | "HeavySiegeWalker"
        | "SiegeDrillTank" => ImpactBurstKind::Heavy,
        _ if weapon.splash_radius > 0.0 => ImpactBurstKind::Explosive,
        _ if weapon.damage >= 16.0 || target_is_structure => ImpactBurstKind::Heavy,
        _ if weapon.can_attack_air && !weapon.can_attack_ground => ImpactBurstKind::Energy,
        _ => ImpactBurstKind::Ballistic,
    }
}

pub(crate) fn spawn_impact_burst(
    commands: &mut Commands,
    position: Vec3,
    target_radius: f32,
    damage: f32,
    target_is_structure: bool,
    team: Team,
    kind: ImpactBurstKind,
) {
    if damage <= 0.0 {
        return;
    }
    let power = impact_burst_power(damage, target_radius, target_is_structure);
    let total = impact_burst_lifetime(power);
    commands.spawn((
        Name::new("Impact burst"),
        Transform::from_translation(Vec3::new(position.x, 0.08, position.z)),
        ImpactBurst {
            remaining: total,
            total,
            radius: (target_radius * 0.55 + power * 0.22).clamp(0.32, 1.45),
            power,
            team,
            kind,
        },
        MatchScopedEntity,
    ));
    // Solid emissive pop at the point of impact.
    spawn_combat_flash(
        commands,
        Vec3::new(position.x, 0.25, position.z),
        (0.1 + power * 0.05).min(0.35),
        (0.32 + power * 0.2).clamp(0.35, 1.4),
        0.2,
        impact_flash_color(kind),
    );
}

pub(crate) fn spawn_veterancy_promotion_effect(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    team: Team,
    rank: u8,
    visibility: Option<&VisibilityState>,
) {
    if visibility.is_some_and(|visibility| !visibility.visible) {
        return;
    }
    commands.spawn((
        Transform::from_translation(position + Vec3::Y * 0.08),
        VeterancyPromotionEffect {
            rank: rank.min(VETERANCY_MAX_RANK),
            remaining: VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS,
            total: VETERANCY_PROMOTION_EFFECT_LIFETIME_SECONDS,
            radius: radius.max(0.75) * 1.45,
            team,
        },
        MatchScopedEntity,
    ));
}

pub(crate) fn update_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut pulses: Query<(Entity, &mut ShotPulse)>,
) {
    for (entity, mut pulse) in &mut pulses {
        pulse.ttl -= time.delta_secs();
        if pulse.ttl <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn update_impact_bursts(
    mut commands: Commands,
    time: Res<Time>,
    mut bursts: Query<(Entity, &mut ImpactBurst)>,
) {
    for (entity, mut burst) in &mut bursts {
        burst.remaining -= time.delta_secs();
        if burst.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn draw_structure_destruction_vfx(
    gizmos: &mut Gizmos,
    position: Vec3,
    effect: &StructureDestructionVfx,
    player_colors: &PlayerColorSlots,
) {
    let life_ratio = if effect.total > 0.0 {
        (effect.remaining / effect.total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    match effect.kind {
        StructureDestructionVfxKind::ExplosionFireball => {
            let rise = (1.0 - life_ratio) * effect.radius * 1.3;
            let center = position + Vec3::Y * rise;
            let radius = effect.radius * (0.35 + (1.0 - life_ratio) * 0.45);
            let color = Color::srgba(1.0, 0.48, 0.14, 0.28 + life_ratio * 0.52);
            gizmos.circle(
                Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
                radius,
                color,
            );
            gizmos.line(
                Vec3::new(center.x, 0.08, center.z),
                center + Vec3::Y * effect.radius * 0.55,
                color,
            );
        }
        StructureDestructionVfxKind::SmokeColumn => {
            let color = structure_smoke_color(effect.team, life_ratio, player_colors);
            let base = Vec3::new(position.x, 0.1, position.z);
            let top = position + Vec3::Y * (effect.radius * (1.8 - life_ratio * 0.4));
            gizmos.line(base, top, color);
            gizmos.circle(
                Isometry3d::new(top, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
                effect.radius * (0.38 + (1.0 - life_ratio) * 0.28),
                color,
            );
        }
    }
}

pub(crate) fn draw_impact_burst(
    gizmos: &mut Gizmos,
    hud: &mut Gizmos<HudGizmos>,
    position: Vec3,
    burst: &ImpactBurst,
    player_colors: &PlayerColorSlots,
) {
    let life_ratio = if burst.total > 0.0 {
        (burst.remaining / burst.total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let progress = 1.0 - life_ratio;
    let [team_r, team_g, team_b] = player_colors.color_rgb(burst.team);
    let (hot, core, smoke, spark_count, ground_scale) = match burst.kind {
        ImpactBurstKind::Ballistic => (
            Color::srgba(
                (0.76 + team_r * 0.18).min(1.0),
                (0.72 + team_g * 0.18).min(1.0),
                (0.56 + team_b * 0.18).min(1.0),
                0.32 + life_ratio * 0.44,
            ),
            Color::srgba(1.0, 0.92, 0.66, 0.48 + life_ratio * 0.4),
            Color::srgba(0.18, 0.17, 0.15, 0.12 * life_ratio),
            5,
            0.82,
        ),
        ImpactBurstKind::Explosive => (
            Color::srgba(1.0, 0.48, 0.12, 0.42 + life_ratio * 0.46),
            Color::srgba(1.0, 0.88, 0.34, 0.50 + life_ratio * 0.48),
            Color::srgba(0.28 + team_r * 0.05, 0.24, 0.20, 0.28 * life_ratio),
            10,
            1.22,
        ),
        ImpactBurstKind::Energy => (
            Color::srgba(0.30, 0.88, 1.0, 0.36 + life_ratio * 0.46),
            Color::srgba(0.82, 1.0, 1.0, 0.56 + life_ratio * 0.42),
            Color::srgba(0.08, 0.22, 0.28, 0.18 * life_ratio),
            8,
            1.0,
        ),
        ImpactBurstKind::Electric => (
            Color::srgba(0.38, 0.64, 1.0, 0.40 + life_ratio * 0.50),
            Color::srgba(0.72, 0.98, 1.0, 0.62 + life_ratio * 0.38),
            Color::srgba(0.13, 0.08, 0.28, 0.18 * life_ratio),
            9,
            1.06,
        ),
        ImpactBurstKind::Fire => (
            Color::srgba(1.0, 0.24, 0.08, 0.42 + life_ratio * 0.46),
            Color::srgba(1.0, 0.70, 0.20, 0.54 + life_ratio * 0.44),
            Color::srgba(0.24, 0.10, 0.05, 0.26 * life_ratio),
            11,
            1.12,
        ),
        ImpactBurstKind::Heavy => (
            Color::srgba(1.0, 0.64, 0.20, 0.40 + life_ratio * 0.46),
            Color::srgba(1.0, 0.92, 0.44, 0.48 + life_ratio * 0.46),
            Color::srgba(
                0.16 + team_r * 0.06,
                0.16 + team_g * 0.06,
                0.15,
                0.34 * life_ratio,
            ),
            12,
            1.36,
        ),
    };
    let center = Vec3::new(position.x, 0.12 + burst.power * 0.04, position.z);
    let ground_radius = burst.radius * ground_scale * (0.35 + progress * 1.15);
    gizmos.circle(
        Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
        ground_radius,
        hot,
    );
    gizmos.circle(
        Isometry3d::new(
            center + Vec3::Y * 0.08,
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        burst.radius * (0.22 + progress * 0.65),
        smoke,
    );
    hud.line(
        center,
        center + Vec3::Y * (0.32 + burst.power * 0.28) * life_ratio.max(0.2),
        core,
    );
    for i in 0..spark_count {
        let angle = i as f32 * std::f32::consts::TAU / spark_count as f32 + burst.power * 0.41;
        let outward = Vec3::new(angle.cos(), 0.16 + progress * 0.18, angle.sin()).normalize();
        let start = center + outward * (burst.radius * 0.14);
        let end = center + outward * (burst.radius * (0.46 + burst.power * 0.18) * life_ratio);
        let color = if i % 2 == 0 { core } else { hot };
        hud.line(start, end, color);
    }
    match burst.kind {
        ImpactBurstKind::Electric => {
            for i in 0..4 {
                let angle = i as f32 * std::f32::consts::FRAC_PI_2 + progress * 2.4;
                let side = Vec3::new(angle.cos(), 0.0, angle.sin());
                let start = center + side * burst.radius * 0.2 + Vec3::Y * 0.12;
                let mid = center
                    + Vec3::new(-side.z, 0.0, side.x) * burst.radius * 0.22
                    + side * burst.radius * 0.5
                    + Vec3::Y * (0.32 + burst.power * 0.12);
                let end = center + side * burst.radius * 0.88 + Vec3::Y * 0.16;
                hud.line(start, mid, core);
                hud.line(mid, end, hot);
            }
        }
        ImpactBurstKind::Energy => {
            gizmos.circle(
                Isometry3d::new(
                    center + Vec3::Y * (0.15 + burst.power * 0.08),
                    Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
                ),
                burst.radius * (0.34 + progress * 0.78),
                core,
            );
        }
        ImpactBurstKind::Fire => {
            for i in 0..5 {
                let angle = i as f32 * std::f32::consts::TAU / 5.0 + progress;
                let base = center + Vec3::new(angle.cos(), 0.0, angle.sin()) * burst.radius * 0.24;
                hud.line(
                    base,
                    base + Vec3::Y * (0.34 + burst.power * 0.20) * life_ratio.max(0.25),
                    if i % 2 == 0 { hot } else { core },
                );
            }
        }
        ImpactBurstKind::Explosive | ImpactBurstKind::Heavy => {
            gizmos.circle(
                Isometry3d::new(
                    center + Vec3::Y * 0.03,
                    Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
                ),
                burst.radius * (0.55 + progress * 0.95),
                smoke,
            );
        }
        ImpactBurstKind::Ballistic => {}
    }
}

pub(crate) fn draw_veterancy_promotion_effect(
    gizmos: &mut Gizmos,
    position: Vec3,
    effect: &VeterancyPromotionEffect,
    player_colors: &PlayerColorSlots,
) {
    let life_ratio = if effect.total > 0.0 {
        (effect.remaining / effect.total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let rank_color = veterancy_promotion_color(effect.rank, life_ratio);
    let team_color = player_colors.color(effect.team);
    let center = Vec3::new(position.x, position.y + 0.08, position.z);
    let expanding_radius = effect.radius * (1.05 + (1.0 - life_ratio) * 0.5);

    draw_ring(gizmos, center, expanding_radius, rank_color);
    draw_ring(
        gizmos,
        center,
        effect.radius * 0.7,
        Color::srgba(1.0, 1.0, 1.0, 0.2 + life_ratio * 0.35),
    );
    gizmos.line(
        center,
        center + Vec3::Y * (effect.radius * (1.5 + (1.0 - life_ratio) * 0.35)),
        team_color,
    );
    let crown_y = center.y + effect.radius * 1.65;
    let badge_width = effect.radius
        * if effect.rank >= VETERANCY_MAX_RANK {
            0.55
        } else {
            0.42
        };
    gizmos.line(
        Vec3::new(center.x - badge_width, crown_y, center.z),
        Vec3::new(center.x + badge_width, crown_y, center.z),
        rank_color,
    );
}
