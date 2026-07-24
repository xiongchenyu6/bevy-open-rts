//! Entity spawning: team startup, unit/structure spawn entry points, model
//! assembly (registry parts, hunyuan GLBs, procedural fallbacks, faction
//! identity markers), placement wrappers, and paradrop spawning.

use bevy::prelude::*;

use crate::*;

pub(crate) fn setup_team(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    base: Vec3,
    loadout: StartupLoadoutMode,
) {
    let startup = faction_startup_for_loadout(faction, loadout);
    for spawn in startup.structures {
        spawn_structure_for_faction(
            commands,
            asset_server,
            next_id,
            spawn.id,
            team,
            visible_team,
            base + Vec3::new(spawn.offset.0, 0.0, spawn.offset.1),
            faction,
        );
    }
    for spawn in startup.units {
        spawn_unit_for_faction(
            commands,
            asset_server,
            next_id,
            spawn.id,
            team,
            base + Vec3::new(spawn.offset.0, 0.0, spawn.offset.1),
            0,
            faction,
            visible_team,
        );
    }
}

pub(crate) fn spawn_prop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    model: &'static str,
    position: Vec3,
    scale: f32,
) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model))),
        Transform::from_translation(position).with_scale(Vec3::splat(scale)),
        MatchScopedEntity,
    ));
}

#[allow(dead_code)]
pub(crate) fn spawn_unit(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    visible_team: Team,
) -> Entity {
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        initial_veterancy_rank,
        default_visual_faction(team),
        visible_team,
    )
}

pub(crate) fn spawn_unit_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    faction: SkirmishFaction,
    visible_team: Team,
) -> Entity {
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        position,
        initial_veterancy_rank,
        Some(faction),
        visible_team,
    )
}

pub(crate) fn spawn_unit_with_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    position: Vec3,
    initial_veterancy_rank: u8,
    visual_faction: Option<SkirmishFaction>,
    visible_team: Team,
) -> Entity {
    let Some(def) = registry::entity(id) else {
        return commands.spawn_empty().id();
    };
    next_id.0 += 1;
    let network_id = NetworkEntityId::dynamic(next_id.0);
    let position = position + Vec3::Y * def.height;
    let unit_speed = if def.mine_trigger_radius > 0.0 {
        0.0
    } else {
        def.speed
    };
    let can_gain_veterancy = def
        .weapon
        .is_some_and(|weapon| unit_speed > 0.0 && weapon.damage > 0.0);
    let initial_veterancy_rank = if can_gain_veterancy {
        initial_veterancy_rank.min(VETERANCY_MAX_RANK)
    } else {
        0
    };
    let veterancy_idx = initial_veterancy_rank as usize;
    let base_vision = unit_vision_radius(def);
    let initial_health = (def.health * VETERANCY_HP_MULTIPLIER_BY_RANK[veterancy_idx]).ceil();
    let initial_vision = base_vision + VETERANCY_SIGHT_BONUS_BY_RANK[veterancy_idx];
    let initial_visible = initial_visibility_state(team, visible_team);
    let entity_id = commands
        .spawn((
            Name::new(format!("{} {}", team.label(), def.label)),
            Transform::from_translation(position).with_scale(Vec3::splat(def.scale)),
            Unit {
                id,
                speed: unit_speed,
                can_crush: def.can_crush,
                can_be_crushed: def.can_be_crushed,
            },
            HoldPosition { enabled: false },
            team,
            Selectable { radius: def.radius },
            Health::new(initial_health),
            VisionRadius(initial_vision),
            initial_visible,
            MovementDomain::from_registry(def.domain),
            network_id,
            initial_visibility(team, visible_team),
            MatchScopedEntity,
        ))
        .id();
    if let Some(faction) = visual_faction {
        commands
            .entity(entity_id)
            .try_insert(VisualFaction(faction));
    }
    spawn_entity_models(commands, asset_server, entity_id, visual_faction, def);
    if let Some(weapon) = def.weapon {
        let weapon_damage =
            (weapon.damage * VETERANCY_DAMAGE_MULTIPLIER_BY_RANK[veterancy_idx] * 10.0).round()
                / 10.0;
        let weapon_range = weapon.range + VETERANCY_RANGE_BONUS_BY_RANK[veterancy_idx];
        commands.entity(entity_id).try_insert(Weapon::new(
            weapon_range,
            weapon_damage,
            weapon.cooldown,
            weapon.splash_radius,
            weapon.splash_damage_multiplier,
            weapon.structure_damage_multiplier,
            weapon.can_attack_air,
            weapon.can_attack_ground,
        ));
        if can_gain_veterancy {
            commands.entity(entity_id).try_insert(Veterancy {
                rank: initial_veterancy_rank,
                experience_points: VETERANCY_KILLS_BY_RANK[veterancy_idx],
                base_health: def.health,
                base_damage: weapon.damage,
                base_range: weapon.range,
                base_vision,
            });
        }
    }
    if def.resource_capacity > 0 {
        commands.entity(entity_id).try_insert(ResourceCargo {
            capacity: def.resource_capacity,
            ore: 0,
            crystal: 0,
        });
    }
    if def.mine_damage > 0.0 && def.mine_trigger_radius > 0.0 && def.mine_blast_radius > 0.0 {
        commands.entity(entity_id).try_insert(Mine {
            damage: def.mine_damage,
            trigger_radius: def.mine_trigger_radius,
            blast_radius: def.mine_blast_radius,
            arming_remaining: def.mine_arming_delay,
            source: None,
        });
    }
    if def.mine_deploy_interval > 0.0 && def.mine_limit > 0 {
        commands.entity(entity_id).try_insert(MineLayer {
            damage: def.mine_damage,
            deploy_interval: def.mine_deploy_interval,
            deploy_radius: def.mine_deploy_radius,
            spacing: def.mine_spacing,
            limit: def.mine_limit,
            cooldown: 0.2,
            deploy_index: 0,
        });
    }
    attach_support_effects(commands, entity_id, def);
    entity_id
}

pub(crate) fn spawn_structure(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        0.0,
        default_visual_faction(team),
    )
}

pub(crate) fn spawn_structure_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    faction: SkirmishFaction,
) -> Entity {
    spawn_structure_with_rotation_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        0.0,
        faction,
    )
}

#[allow(dead_code)]
pub(crate) fn spawn_structure_with_rotation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        rotation_y_radians,
        default_visual_faction(team),
    )
}

pub(crate) fn spawn_structure_with_rotation_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
    faction: SkirmishFaction,
) -> Entity {
    spawn_structure_for_visual_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        visible_team,
        position,
        rotation_y_radians,
        Some(faction),
    )
}

pub(crate) fn spawn_structure_for_visual_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    id: &'static str,
    team: Team,
    visible_team: Team,
    position: Vec3,
    rotation_y_radians: f32,
    visual_faction: Option<SkirmishFaction>,
) -> Entity {
    let Some(def) = registry::entity(id) else {
        return commands.spawn_empty().id();
    };
    next_id.0 += 1;
    let network_id = NetworkEntityId::dynamic(next_id.0);
    let initial_visible = initial_visibility_state(team, visible_team);
    let entity_id = commands
        .spawn((
            Name::new(format!("{} {}", team.label(), def.label)),
            Transform::from_translation(position)
                .with_rotation(Quat::from_rotation_y(rotation_y_radians))
                .with_scale(Vec3::splat(def.scale)),
            Structure { id },
            team,
            Selectable { radius: def.radius },
            Health::new(def.health),
            VisionRadius(structure_vision_radius(def)),
            initial_visible,
            MovementDomain::from_registry(def.domain),
            network_id,
            initial_visibility(team, visible_team),
            MatchScopedEntity,
        ))
        .id();
    if let Some(faction) = visual_faction {
        commands
            .entity(entity_id)
            .try_insert(VisualFaction(faction));
    }
    spawn_entity_models(commands, asset_server, entity_id, visual_faction, def);
    if let Some(weapon) = def.weapon {
        commands.entity(entity_id).try_insert(Weapon::new(
            weapon.range,
            weapon.damage,
            weapon.cooldown,
            weapon.splash_radius,
            weapon.splash_damage_multiplier,
            weapon.structure_damage_multiplier,
            weapon.can_attack_air,
            weapon.can_attack_ground,
        ));
    }
    if is_rally_point_structure(id) {
        commands.entity(entity_id).try_insert(RallyPoint {
            target: None,
            target_unit: None,
            mode: RallyMode::Move,
        });
    }
    attach_support_effects(commands, entity_id, def);
    entity_id
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProceduralEntityModel {
    LandMine,
    TeslaFenceSegment,
}

impl ProceduralEntityModel {
    pub(crate) fn for_entity_id(id: &str) -> Option<Self> {
        match id {
            "LandMine" => Some(Self::LandMine),
            "TeslaFenceSegment" => Some(Self::TeslaFenceSegment),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn part_count(self) -> usize {
        match self {
            Self::LandMine => 2,
            Self::TeslaFenceSegment => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FactionIdentityMarker {
    Human,
    Demon,
    Chaos,
}

impl FactionIdentityMarker {
    pub(crate) fn for_faction(faction: SkirmishFaction) -> Self {
        match faction {
            SkirmishFaction::Alliance => Self::Human,
            SkirmishFaction::Demon => Self::Demon,
            SkirmishFaction::Chaos => Self::Chaos,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn for_team(team: Team) -> Option<Self> {
        team.economy_index()
            .map(|_| Self::for_faction(SkirmishFaction::from_team(team)))
    }

    #[cfg(test)]
    pub(crate) fn part_count(self) -> usize {
        match self {
            Self::Human => 2,
            Self::Demon => 3,
            Self::Chaos => 2,
        }
    }
}

pub(crate) fn default_visual_faction(team: Team) -> Option<SkirmishFaction> {
    team.economy_index()
        .map(|_| SkirmishFaction::from_team(team))
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HunyuanModelPart {
    pub(crate) entity_id: &'static str,
}

impl HunyuanModelPart {
    pub(crate) fn for_render_part(
        entity_id: &'static str,
        part: &registry::RenderPart,
    ) -> Option<Self> {
        is_hunyuan_model_path(part.model).then_some(Self { entity_id })
    }
}

#[derive(Component)]
pub(crate) struct HunyuanModelMaterialized;

#[derive(Resource, Default)]
pub(crate) struct HunyuanModelMaterialCache {
    pub(crate) by_entity: BTreeMap<&'static str, Handle<StandardMaterial>>,
}

impl HunyuanModelMaterialCache {
    pub(crate) fn handle_for(
        &mut self,
        entity_id: &'static str,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.by_entity.get(entity_id) {
            return handle.clone();
        }
        let handle = materials.add(hunyuan_model_material(entity_id));
        self.by_entity.insert(entity_id, handle.clone());
        handle
    }
}

pub(crate) fn is_hunyuan_model_path(model: &str) -> bool {
    model.starts_with("models/hunyuan3d/")
}

pub(crate) fn hunyuan_model_material(entity_id: &str) -> StandardMaterial {
    let (base, metallic, roughness, glow) = match entity_id {
        "CryoSprayer" => (Color::srgb(0.70, 0.92, 1.0), 0.72, 0.28, 0.30),
        "LongbowMissileCrawler" => (Color::srgb(0.26, 0.29, 0.32), 0.90, 0.34, 0.10),
        "FlameAssaultBuggy" => (Color::srgb(0.92, 0.28, 0.10), 0.62, 0.36, 0.45),
        "HammerSiegeTank" => (Color::srgb(0.54, 0.55, 0.50), 0.88, 0.32, 0.08),
        "HeavySiegeWalker" => (Color::srgb(0.46, 0.49, 0.52), 0.88, 0.30, 0.08),
        "RailArtilleryWalker" => (Color::srgb(0.58, 0.53, 0.44), 0.84, 0.34, 0.10),
        "FlakHoverTank" => (Color::srgb(0.40, 0.48, 0.38), 0.82, 0.38, 0.08),
        "LanceBeamTank" => (Color::srgb(0.22, 0.50, 0.92), 0.78, 0.30, 0.32),
        "RailgunTank" => (Color::srgb(0.50, 0.55, 0.58), 0.90, 0.28, 0.16),
        "FlakRocketTeam" => (Color::srgb(0.45, 0.42, 0.38), 0.72, 0.44, 0.10),
        "FlakRocketTeamMk2" => (Color::srgb(0.58, 0.44, 0.34), 0.76, 0.40, 0.12),
        "MobileShieldProjector" => (Color::srgb(0.48, 0.32, 0.86), 0.74, 0.30, 0.35),
        "ModularMissileCarrier" => (Color::srgb(0.34, 0.35, 0.36), 0.90, 0.35, 0.14),
        "TeslaCrawlerMk2" => (Color::srgb(0.16, 0.42, 0.90), 0.80, 0.26, 0.55),
        _ => (Color::srgb(0.58, 0.56, 0.50), 0.78, 0.36, 0.10),
    };
    let lin = base.to_linear();
    StandardMaterial {
        base_color: base,
        metallic,
        perceptual_roughness: roughness,
        emissive: LinearRgba::new(lin.red * glow, lin.green * glow, lin.blue * glow, 1.0),
        ..default()
    }
}

pub(crate) fn spawn_entity_models(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    if def.render_parts.is_empty() {
        if let Some(model) = ProceduralEntityModel::for_entity_id(def.id) {
            spawn_procedural_entity_model(commands, root, model);
        } else {
            let fallback = DEFAULT_MODEL_FALLBACK;
            commands.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(fallback))),
                Transform::IDENTITY,
            ));
        }
    } else {
        let airborne =
            MovementDomain::from_registry(def.domain) == MovementDomain::Air && def.speed > 0.0;
        if airborne {
            commands.entity(root).try_insert(AirMotion::default());
        }
        for part in def.render_parts {
            let transform = scaled_render_part_transform(part, entity_visual_scale(def.id));
            let mut spawned = commands.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(part.model))),
                transform,
            ));
            if airborne {
                spawned.insert(AirframePart {
                    root,
                    rest_translation: transform.translation,
                    rest_rotation: transform.rotation,
                });
            }
            if let Some(marker) = HunyuanModelPart::for_render_part(def.id, part) {
                spawned.insert(marker);
            }
        }
    }
    spawn_faction_identity_marker(commands, root, visual_faction, def);
}

pub(crate) fn spawn_entity_models_for_harness(
    world: &mut World,
    root: Instance<ModelHarnessRoot>,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    let root = root.entity();
    let asset_server = world.resource::<AssetServer>().clone();
    if def.render_parts.is_empty() {
        if let Some(model) = ProceduralEntityModel::for_entity_id(def.id) {
            match model {
                ProceduralEntityModel::LandMine => spawn_land_mine_procedural_model(world, root),
                ProceduralEntityModel::TeslaFenceSegment => {
                    spawn_tesla_fence_segment_procedural_model(world, root)
                }
            }
        } else {
            world.spawn((
                ChildOf(root),
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(DEFAULT_MODEL_FALLBACK)),
                ),
                Transform::IDENTITY,
            ));
        }
    } else {
        for part in def.render_parts {
            let mut spawned = world.spawn((
                ChildOf(root),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(part.model))),
                scaled_render_part_transform(part, entity_visual_scale(def.id)),
            ));
            if let Some(marker) = HunyuanModelPart::for_render_part(def.id, part) {
                spawned.insert(marker);
            }
        }
    }
    if let Some(marker) = visual_faction.map(FactionIdentityMarker::for_faction) {
        let inv_scale = 1.0 / def.scale.max(0.1);
        let world_size = match def.role {
            registry::EntityRole::Structure => (def.radius * 0.26).clamp(0.28, 0.68),
            registry::EntityRole::Unit => (def.radius * 0.22).clamp(0.12, 0.26),
        };
        let local_size = world_size * inv_scale;
        let local_offset = Vec3::new(0.0, 0.12 * inv_scale, -def.radius * 0.72 * inv_scale);
        spawn_faction_identity_marker_model(world, root, marker, local_offset, local_size);
    }
}

/// Uniformly scales a part's translation AND scale so a whole composition grows
/// around the entity origin with its layout intact (part translations bake the
/// kenney node-offset compensation, so both must scale together).
pub(crate) fn scaled_render_part_transform(part: &registry::RenderPart, factor: f32) -> Transform {
    Transform::from_translation(
        Vec3::new(
            part.translation[0],
            part.translation[1],
            part.translation[2],
        ) * factor,
    )
    .with_rotation(Quat::from_xyzw(
        part.rotation[0],
        part.rotation[1],
        part.rotation[2],
        part.rotation[3],
    ))
    .with_scale(Vec3::new(part.scale[0], part.scale[1], part.scale[2]) * factor)
}

pub(crate) fn spawn_faction_identity_marker(
    commands: &mut Commands,
    root: Entity,
    visual_faction: Option<SkirmishFaction>,
    def: &registry::EntityDef,
) {
    let Some(marker) = visual_faction.map(FactionIdentityMarker::for_faction) else {
        return;
    };
    let inv_scale = 1.0 / def.scale.max(0.1);
    let world_size = match def.role {
        registry::EntityRole::Structure => (def.radius * 0.26).clamp(0.28, 0.68),
        registry::EntityRole::Unit => (def.radius * 0.22).clamp(0.12, 0.26),
    };
    let local_size = world_size * inv_scale;
    let local_offset = Vec3::new(0.0, 0.12 * inv_scale, -def.radius * 0.72 * inv_scale);
    commands.queue(move |world: &mut World| {
        if world.get_entity(root).is_err() {
            return;
        }
        spawn_faction_identity_marker_model(world, root, marker, local_offset, local_size);
    });
}

pub(crate) fn spawn_faction_identity_marker_model(
    world: &mut World,
    root: Entity,
    marker: FactionIdentityMarker,
    local_offset: Vec3,
    size: f32,
) {
    match marker {
        FactionIdentityMarker::Human => {
            let Some(plate_mesh) =
                add_procedural_mesh(world, Cuboid::new(size * 1.35, size * 0.09, size * 0.46))
            else {
                return;
            };
            let Some(mast_mesh) =
                add_procedural_mesh(world, Cuboid::new(size * 0.14, size * 0.72, size * 0.14))
            else {
                return;
            };
            let Some(blue_material) = add_procedural_material(
                world,
                Color::srgb(0.16, 0.44, 0.98),
                0.35,
                0.32,
                LinearRgba::rgb(0.02, 0.07, 0.22),
            ) else {
                return;
            };
            let Some(white_material) = add_procedural_material(
                world,
                Color::srgb(0.86, 0.94, 1.0),
                0.25,
                0.22,
                LinearRgba::rgb(0.06, 0.08, 0.12),
            ) else {
                return;
            };
            spawn_procedural_mesh_child(
                world,
                root,
                "Human Faction Command Plate",
                plate_mesh,
                blue_material,
                Transform::from_translation(local_offset),
            );
            spawn_procedural_mesh_child(
                world,
                root,
                "Human Faction Signal Mast",
                mast_mesh,
                white_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.36),
            );
        }
        FactionIdentityMarker::Demon => {
            let Some(spike_mesh) = add_procedural_mesh(
                world,
                ConicalFrustum {
                    radius_top: 0.0,
                    radius_bottom: size * 0.22,
                    height: size * 0.95,
                }
                .mesh()
                .resolution(18),
            ) else {
                return;
            };
            let Some(spike_material) = add_procedural_material(
                world,
                Color::srgb(0.92, 0.12, 0.055),
                0.25,
                0.42,
                LinearRgba::rgb(0.55, 0.035, 0.01),
            ) else {
                return;
            };
            for (name, x) in [
                ("Demon Faction Left Spike", -0.42),
                ("Demon Faction Center Spike", 0.0),
                ("Demon Faction Right Spike", 0.42),
            ] {
                spawn_procedural_mesh_child(
                    world,
                    root,
                    name,
                    spike_mesh.clone(),
                    spike_material.clone(),
                    Transform::from_translation(
                        local_offset + Vec3::new(x * size, size * 0.44, 0.0),
                    ),
                );
            }
        }
        FactionIdentityMarker::Chaos => {
            let Some(ring_mesh) = add_procedural_mesh(
                world,
                Torus::new(size * 0.055, size * 0.31)
                    .mesh()
                    .minor_resolution(8)
                    .major_resolution(28),
            ) else {
                return;
            };
            let Some(core_mesh) = add_procedural_mesh(
                world,
                Cylinder::new(size * 0.16, size * 0.14)
                    .mesh()
                    .resolution(20),
            ) else {
                return;
            };
            let Some(ring_material) = add_procedural_material(
                world,
                Color::srgb(0.55, 0.22, 0.95),
                0.0,
                0.2,
                LinearRgba::rgb(0.45, 0.09, 1.2),
            ) else {
                return;
            };
            let Some(core_material) = add_procedural_material(
                world,
                Color::srgb(0.08, 0.9, 1.0),
                0.0,
                0.18,
                LinearRgba::rgb(0.05, 0.85, 1.2),
            ) else {
                return;
            };
            spawn_procedural_mesh_child(
                world,
                root,
                "Chaos Faction Energy Ring",
                ring_mesh,
                ring_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.18)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );
            spawn_procedural_mesh_child(
                world,
                root,
                "Chaos Faction Core",
                core_mesh,
                core_material,
                Transform::from_translation(local_offset + Vec3::Y * size * 0.18),
            );
        }
    }
}

/// kenney's gold accent material ("metalRed", linear 1.0/0.6285/0.2028 ==
/// sRGB ~0.99/0.81/0.48). godot repaints every material matching that albedo
/// (Unit.gd MATERIAL_ALBEDO_TO_REPLACE, eps 0.05) with the player color; this
/// mirrors it so team colors visually bind the part collages together.
pub(crate) const TEAM_PAINT_TARGET_SRGB: [f32; 3] = [0.99, 0.81, 0.48];
pub(crate) const TEAM_PAINT_EPSILON: f32 = 0.06;

/// Marker: this mesh has been considered (and possibly repainted) already.
#[derive(Component)]
pub(crate) struct TeamColorProcessed;

/// One shared repaint material per palette slot.
#[derive(Resource, Default)]
pub(crate) struct TeamColorMaterials(
    pub(crate) std::collections::HashMap<(usize, bool), Handle<StandardMaterial>>,
);

pub(crate) fn material_matches_team_paint_target(color: Color) -> bool {
    let srgba = color.to_srgba();
    (srgba.red - TEAM_PAINT_TARGET_SRGB[0]).abs() <= TEAM_PAINT_EPSILON
        && (srgba.green - TEAM_PAINT_TARGET_SRGB[1]).abs() <= TEAM_PAINT_EPSILON
        && (srgba.blue - TEAM_PAINT_TARGET_SRGB[2]).abs() <= TEAM_PAINT_EPSILON
}

/// Repaints kenney gold-accent parts in each player's color, mirroring godot's
/// player-color material replacement. GLB parts stream in over several frames,
/// so this sweeps meshes that have not been considered yet. Under-construction
/// structures are skipped (their parts wear the ghost material) and get
/// painted right after completion restores the originals.
pub(crate) fn apply_team_color_materials(
    mut commands: Commands,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut cache: ResMut<TeamColorMaterials>,
    color_slots: Res<PlayerColorSlots>,
    mut unpainted: Query<
        (Entity, &mut MeshMaterial3d<StandardMaterial>),
        Without<TeamColorProcessed>,
    >,
    parents: Query<&ChildOf>,
    roots: Query<(&Team, Has<UnderConstruction>, Has<Unit>), Or<(With<Unit>, With<Structure>)>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (entity, mut part) in &mut unpainted {
        let mut cursor = entity;
        let owner = loop {
            if let Ok(owner) = roots.get(cursor) {
                break Some(owner);
            }
            match parents.get(cursor) {
                Ok(child_of) => cursor = child_of.0,
                Err(_) => break None,
            }
        };
        let Some((team, under_construction, is_unit)) = owner else {
            commands.entity(entity).try_insert(TeamColorProcessed);
            continue;
        };
        if under_construction {
            continue;
        }
        let Some(slot) = color_slots.slot(*team) else {
            commands.entity(entity).try_insert(TeamColorProcessed);
            continue;
        };
        let is_gold = materials
            .get(&part.0)
            .is_some_and(|material| material_matches_team_paint_target(material.base_color));
        if is_gold {
            let handle = cache
                .0
                .entry((slot, is_unit))
                .or_insert_with(|| {
                    // Units get an emissive team color so even a tiny accent
                    // glows under bloom and reads as team identity at RTS zoom;
                    // structures have large accents already, so they stay a solid
                    // (non-glowing) team color to avoid washing out under bloom.
                    let emissive = if is_unit {
                        let [r, g, b] = player_color_rgb(slot);
                        LinearRgba::rgb(r * 0.7, g * 0.7, b * 0.7)
                    } else {
                        LinearRgba::NONE
                    };
                    materials.add(StandardMaterial {
                        base_color: player_color(slot),
                        emissive,
                        metallic: 0.4,
                        perceptual_roughness: 0.45,
                        ..default()
                    })
                })
                .clone();
            part.0 = handle;
        }
        commands.entity(entity).try_insert(TeamColorProcessed);
    }
}

pub(crate) fn spawn_procedural_entity_model(
    commands: &mut Commands,
    root: Entity,
    model: ProceduralEntityModel,
) {
    commands.queue(move |world: &mut World| {
        if world.get_entity(root).is_err() {
            return;
        }

        match model {
            ProceduralEntityModel::LandMine => spawn_land_mine_procedural_model(world, root),
            ProceduralEntityModel::TeslaFenceSegment => {
                spawn_tesla_fence_segment_procedural_model(world, root)
            }
        }
    });
}

pub(crate) fn add_procedural_mesh(
    world: &mut World,
    mesh: impl Into<Mesh>,
) -> Option<Handle<Mesh>> {
    let mut meshes = world.get_resource_mut::<Assets<Mesh>>()?;
    Some(meshes.add(mesh))
}

pub(crate) fn add_procedural_material(
    world: &mut World,
    base_color: Color,
    metallic: f32,
    perceptual_roughness: f32,
    emissive: LinearRgba,
) -> Option<Handle<StandardMaterial>> {
    let mut materials = world.get_resource_mut::<Assets<StandardMaterial>>()?;
    Some(materials.add(StandardMaterial {
        base_color,
        metallic,
        perceptual_roughness,
        emissive,
        ..default()
    }))
}

pub(crate) fn spawn_procedural_mesh_child(
    world: &mut World,
    root: Entity,
    name: &'static str,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) {
    world.spawn((
        Name::new(name),
        ChildOf(root),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}

pub(crate) fn unit_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        def.radius * 5.0 + 3.5
    } else {
        FOG_REVEAL_RADIUS
    }
}

pub(crate) fn structure_vision_radius(def: &registry::EntityDef) -> f32 {
    if def.sight_range > 0.0 {
        def.sight_range
    } else if def.weapon.is_some() {
        (def.radius * 4.5 + 2.5).clamp(1.5, FOG_REVEAL_RADIUS)
    } else {
        0.0
    }
}

pub(crate) fn spawn_paradrop_units(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_spawn_id: &mut NextSpawnId,
    target: Vec3,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    unit_paths: &'static [&'static str],
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) {
    let count = unit_paths.len();
    for (i, unit_path) in unit_paths.iter().enumerate() {
        let Some(def) = registry::entity(unit_path) else {
            continue;
        };
        let offset = formation_offset(i, count);
        let spawn_position = find_paradrop_spawn_position(
            target,
            offset,
            def.radius,
            occupiable_spawn_points,
            bounds,
        );
        spawn_unit_for_faction(
            commands,
            asset_server,
            next_spawn_id,
            unit_path,
            team,
            spawn_position,
            0,
            faction,
            visible_team,
        );
        // Touch-down: a soft pop and a ring of kicked-up dust per trooper.
        spawn_landing_dust(commands, spawn_position);
    }
}

pub(crate) fn find_paradrop_spawn_position(
    target: Vec3,
    preferred_offset: Vec3,
    unit_radius: f32,
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) -> Vec3 {
    let preferred = (target + preferred_offset).with_y(0.0);
    if is_spawn_position_free(preferred, unit_radius, occupiable_spawn_points, bounds) {
        return preferred;
    }

    let preferred_direction = {
        let dir = preferred_offset.xz();
        if dir.length_squared() < 1e-4 {
            Vec2::new(0.0, 1.0)
        } else {
            dir.normalize()
        }
    };

    let ring_step = 0.5;
    let max_rings = 18;
    for ring in 1..=max_rings {
        let search_radius = ring as f32 * ring_step;
        let samples = 12 + ring * 2;
        let angular_offset = preferred_direction.angle_to(Vec2::Y);
        for sample in 0..samples {
            let angle = angular_offset + sample as f32 * (std::f32::consts::TAU / samples as f32);
            let candidate = Vec3::new(
                target.x + preferred_offset.x + angle.cos() * search_radius,
                0.0,
                target.z + preferred_offset.z + angle.sin() * search_radius,
            );
            let clamped = bounds.clamp_ground_point(candidate, unit_radius);
            if is_spawn_position_free(clamped, unit_radius, occupiable_spawn_points, bounds) {
                return clamped;
            }
        }
    }

    bounds.clamp_ground_point(preferred, unit_radius)
}

pub(crate) fn is_spawn_position_free(
    candidate: Vec3,
    unit_radius: f32,
    occupiable_spawn_points: &[(Vec3, f32)],
    bounds: MapBounds,
) -> bool {
    let inner_bounds = MapBounds {
        half_width: (bounds.half_width - unit_radius).max(0.0),
        half_depth: (bounds.half_depth - unit_radius).max(0.0),
    };
    if !inner_bounds.contains_ground_point(candidate) {
        return false;
    }

    for (position, radius) in occupiable_spawn_points {
        if xz_distance(candidate, *position) <= unit_radius + *radius + 0.05 {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
pub(crate) fn place_structure_at(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    visible_team: Team,
    id: &'static str,
    point: Vec3,
    rotation_y_radians: f32,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
    economies: &mut Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
) -> Result<(Entity, &'static str), StructurePlacementValidity> {
    place_structure_at_for_faction(
        commands,
        asset_server,
        next_id,
        team,
        SkirmishFaction::from_team(team),
        visible_team,
        id,
        point,
        rotation_y_radians,
        bounds,
        terrain,
        economies,
        structures,
        occupiers,
    )
}

pub(crate) fn place_structure_at_for_faction(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    team: Team,
    faction: SkirmishFaction,
    visible_team: Team,
    id: &'static str,
    point: Vec3,
    rotation_y_radians: f32,
    bounds: MapBounds,
    terrain: &TerrainHeightField,
    economies: &mut Economies,
    structures: &Query<StructurePrereqItem<'_>>,
    occupiers: &Query<
        PlacementOccupierItem<'_>,
        Or<(
            With<Unit>,
            With<Structure>,
            With<ResourceNode>,
            With<TerrainWall>,
        )>,
    >,
) -> Result<(Entity, &'static str), StructurePlacementValidity> {
    let def = registry::entity(id).ok_or(StructurePlacementValidity::MissingTech)?;
    let validity = structure_placement_validity_for_faction(
        team, faction, id, point, bounds, terrain, economies, structures, occupiers,
    );
    if validity != StructurePlacementValidity::Valid {
        return Err(validity);
    }
    if !economies.get_mut(team).spend(def.cost) {
        return Err(StructurePlacementValidity::NotEnoughResources);
    }
    let free_worker_origin = if id == "Refinery" {
        nearest_base_construction_anchor(team, point, def.radius, structures)
    } else {
        None
    };
    let entity = spawn_structure_under_construction_for_faction(
        commands,
        asset_server,
        next_id,
        id,
        team,
        point,
        free_worker_origin,
        rotation_y_radians,
        visible_team,
        faction,
    );
    Ok((entity, id))
}

/// Recenters each selectable entity's loaded model so its visible geometry's
/// horizontal center coincides with the entity `Transform.translation` — the
/// point gizmos (selection/hover rings) and every cursor hit-test project.
///
/// Root cause this fixes: the GLB scenes (and the migrated `render_parts`
/// offsets, e.g. a turret part at [-2,0,-1.5]) place geometry off the entity
/// origin, so the *visible* model rendered far from where clicks were judged —
/// left/right-clicking the model selected/targeted nothing. Runs once per entity,
/// after its scene meshes have spawned (their `Aabb`s exist).
pub(crate) fn recenter_entity_models(
    mut commands: Commands,
    roots: Query<
        (Entity, &GlobalTransform, Option<&ModelRecenterTracking>),
        (
            // The placement ghost carries the same GLB parts as a real
            // structure; without recentering it renders offset down-right of
            // the actual build spot (the raw kenney node-offset bake).
            Or<(With<Selectable>, With<PlacementGhostRoot>)>,
            Without<ModelRecentered>,
        ),
    >,
    children_q: Query<&Children>,
    aabb_q: Query<(&GlobalTransform, &Aabb)>,
    mut model_tf: Query<&mut Transform, With<WorldAssetRoot>>,
) {
    for (root, root_gt, tracking) in &roots {
        // Combined world-space AABB of the GLB model meshes + how many meshes.
        // Measure ONLY the WorldAssetRoot (GLB) subtrees — the same children the
        // shift below moves. The faction identity banner is a procedural child of
        // the root sitting forward at -radius*0.72 in Z; including it pulled the
        // measured center toward the flag, so the building (and its selection
        // brackets) ended up offset from the entity origin.
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut count: u32 = 0;
        let model_roots: Vec<Entity> = children_q
            .get(root)
            .map(|c| c.iter().filter(|e| model_tf.contains(*e)).collect())
            .unwrap_or_default();
        if model_roots.is_empty() {
            // Procedural-only model (authored at the origin) — nothing to shift.
            commands.entity(root).insert(ModelRecentered);
            commands.entity(root).remove::<ModelRecenterTracking>();
            continue;
        }
        let mut stack: Vec<Entity> = model_roots;
        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter());
            }
            if let Ok((gt, aabb)) = aabb_q.get(entity) {
                count += 1;
                let center = Vec3::from(aabb.center);
                let half = Vec3::from(aabb.half_extents);
                for sx in [-1.0_f32, 1.0] {
                    for sy in [-1.0_f32, 1.0] {
                        for sz in [-1.0_f32, 1.0] {
                            let corner = center + Vec3::new(sx * half.x, sy * half.y, sz * half.z);
                            let world = gt.transform_point(corner);
                            min = min.min(world);
                            max = max.max(world);
                        }
                    }
                }
            }
        }
        if count == 0 {
            // Scene meshes not spawned yet; try again next frame.
            continue;
        }
        // Wait a short settle window after meshes first appear, then correct ONCE.
        // (Applying on first sight left late-loading multi-part models misaligned;
        // re-applying every frame diverged because GlobalTransform lags a frame;
        // gating on mesh-count stability failed for animated models whose count
        // jitters and never settles.)
        let frames = tracking.map(|t| t.frames).unwrap_or(0).saturating_add(1);
        if frames < MODEL_RECENTER_SETTLE_FRAMES {
            commands
                .entity(root)
                .insert(ModelRecenterTracking { frames });
            continue;
        }
        let visual_center = (min + max) * 0.5;
        let (scale, rotation, translation) = root_gt.to_scale_rotation_translation();
        let scale = scale.x.abs().max(1e-3);
        // World shift to move the visible center onto the entity origin (XZ only —
        // keep models sitting on the ground), converted into the root's LOCAL frame
        // (children's Transforms are parent-local). The root may be rotated (units
        // face their movement direction), so undo its rotation AND scale — using
        // only `/scale` left rotated units (workers) misaligned.
        let world_delta = Vec3::new(
            translation.x - visual_center.x,
            0.0,
            translation.z - visual_center.z,
        );
        let local_delta = rotation.inverse() * (world_delta / scale);
        if let Ok(children) = children_q.get(root) {
            for child in children.iter() {
                if let Ok(mut transform) = model_tf.get_mut(child) {
                    transform.translation.x += local_delta.x;
                    transform.translation.z += local_delta.z;
                }
            }
        }
        commands.entity(root).insert(ModelRecentered);
        commands.entity(root).remove::<ModelRecenterTracking>();
    }
}
