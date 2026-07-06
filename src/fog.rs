//! Fog of war: per-entity visibility state, the shroud overlay texture, and
//! fog memory (last-seen enemy structure remnants).

use bevy::prelude::*;

use crate::*;

pub(crate) const FOG_REVEAL_RADIUS: f32 = 11.5;

pub(crate) const FOG_COMPENSATION: f32 = 2.0;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisibilityState {
    pub(crate) visible: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FogMemoryVisible;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct FogMemoryStructureRemnant {
    pub(crate) radius: f32,
}

#[derive(Component)]
pub(crate) struct VisionRadius(pub(crate) f32);

/// Fog-of-war shroud texture resolution (pixels per side) drawn over the map.
pub(crate) const FOG_OVERLAY_RES: usize = 192;

/// Alpha of the dim shroud over explored-but-not-currently-visible terrain.
/// Keep this LIGHT: it multiplies with the filmic exposure, and at 150 the
/// explored map read as nearly unexplored-black ("点亮的地图很快就黑了").
pub(crate) const FOG_OVERLAY_EXPLORED_ALPHA: u8 = 92;

/// Height above the terrain at which the shroud plane sits.
pub(crate) const FOG_OVERLAY_Y: f32 = 0.06;

/// Marker for the textured shroud plane covering the whole map.
#[derive(Component)]
pub(crate) struct FogOverlayPlane;

/// Live fog-of-war shroud: a CPU-updated texture sampled over the map. Each cell
/// is clear where the viewing player (or an ally) currently sees, dimmed where it
/// was explored before, and black where never seen (godot's shroud+fog layers).
#[derive(Resource)]
pub(crate) struct FogOverlay {
    pub(crate) handle: Handle<Image>,
    pub(crate) explored: Vec<bool>,
}

pub(crate) fn initial_visibility_state(team: Team, visible_team: Team) -> VisibilityState {
    VisibilityState {
        visible: team == visible_team,
    }
}

pub(crate) fn initial_visibility(team: Team, visible_team: Team) -> Visibility {
    if team == visible_team {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

pub(crate) fn spawn_fog_overlay(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bounds: MapBounds,
    surface_y: f32,
) {
    use bevy::image::ImageSampler;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    // Start fully shrouded (opaque black, alpha 255). `update_fog_overlay` carves
    // out the seen/explored areas each frame.
    let data = [0u8, 0, 0, 255].repeat(FOG_OVERLAY_RES * FOG_OVERLAY_RES);
    let mut image = Image::new(
        Extent3d {
            width: FOG_OVERLAY_RES as u32,
            height: FOG_OVERLAY_RES as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );
    // Linear filtering smooths the low-res shroud into soft fog edges.
    image.sampler = ImageSampler::linear();
    let handle = images.add(image);

    commands.spawn((
        Name::new("Fog of war shroud"),
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(bounds.half_width * 2.0, bounds.half_depth * 2.0),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(handle.clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, surface_y, 0.0),
        FogOverlayPlane,
        MatchScopedEntity,
    ));

    commands.insert_resource(FogOverlay {
        handle,
        explored: vec![false; FOG_OVERLAY_RES * FOG_OVERLAY_RES],
    });
}

/// Repaints the fog shroud texture from the viewing player's (and allies')
/// current vision. Clear where seen now, dim where explored before, black where
/// never seen. Hidden entirely in spectator/all-visible mode.
pub(crate) fn update_fog_overlay(
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    bounds: Option<Res<MapBounds>>,
    fog: Option<ResMut<FogOverlay>>,
    images: Option<ResMut<Assets<Image>>>,
    revealers: Query<(&Team, &Transform, &VisionRadius, &VisibilityState)>,
    mut overlay_vis: Query<&mut Visibility, With<FogOverlayPlane>>,
) {
    let (Some(bounds), Some(mut fog), Some(mut images)) = (bounds, fog, images) else {
        return;
    };
    let bounds = *bounds;
    let hide_fog = visible_player.all_players_visible();
    for mut visibility in &mut overlay_vis {
        *visibility = if hide_fog {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if hide_fog {
        return;
    }
    let visible_team = visible_player.team;
    let res = FOG_OVERLAY_RES;
    let mut visible_now = vec![false; res * res];
    let (hw, hd) = (bounds.half_width.max(0.01), bounds.half_depth.max(0.01));
    for (team, transform, vision, state) in &revealers {
        if !state.visible || !relations.are_allied(visible_team, *team) {
            continue;
        }
        let r = vision.0 + FOG_COMPENSATION;
        let pos = transform.translation;
        // Center + radius in texture pixels (map is centered on the origin).
        let cu = (pos.x + hw) / (2.0 * hw) * res as f32;
        let cv = (pos.z + hd) / (2.0 * hd) * res as f32;
        let pr_x = (r / (2.0 * hw) * res as f32).max(0.5);
        let pr_z = (r / (2.0 * hd) * res as f32).max(0.5);
        let min_x = ((cu - pr_x).floor().max(0.0)) as usize;
        let max_x = ((cu + pr_x).ceil().min(res as f32 - 1.0)) as usize;
        let min_y = ((cv - pr_z).floor().max(0.0)) as usize;
        let max_y = ((cv + pr_z).ceil().min(res as f32 - 1.0)) as usize;
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = (px as f32 + 0.5 - cu) / pr_x;
                let dy = (py as f32 + 0.5 - cv) / pr_z;
                if dx * dx + dy * dy <= 1.0 {
                    visible_now[py * res + px] = true;
                }
            }
        }
    }
    let Some(mut image) = images.get_mut(&fog.handle) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    for i in 0..res * res {
        if visible_now[i] {
            fog.explored[i] = true;
        }
        data[i * 4 + 3] = if visible_now[i] {
            0
        } else if fog.explored[i] {
            FOG_OVERLAY_EXPLORED_ALPHA
        } else {
            255
        };
    }
}

pub(crate) fn update_visibility(
    mut commands: Commands,
    visible_player: Res<VisiblePlayer>,
    relations: Res<TeamRelations>,
    mut visibility_params: ParamSet<(
        Query<(&Team, &Transform, &VisionRadius, &VisibilityState)>,
        Query<(
            Entity,
            &Team,
            &Transform,
            Option<&Structure>,
            Option<&FogMemoryVisible>,
            &mut VisibilityState,
            &mut Visibility,
        )>,
        Query<(Entity, &Transform, &FogMemoryStructureRemnant)>,
    )>,
) {
    let visible_team = visible_player.team;
    let all_players_visible = visible_player.all_players_visible();
    let mut revealers = Vec::new();
    if !all_players_visible {
        {
            let visible_revealers = visibility_params.p0();
            for (team, transform, vision_radius, visibility_state) in &visible_revealers {
                // Allies share vision: any allied unit/structure reveals fog for the
                // viewing player (godot `is_allied_with(visible_player)`).
                if relations.are_allied(visible_team, *team) && visibility_state.visible {
                    revealers.push((transform.translation, vision_radius.0));
                }
            }
        }
    }

    {
        let mut tracked_entities = visibility_params.p1();
        for (
            entity,
            team,
            transform,
            structure,
            fog_memory,
            mut visibility_state,
            mut visibility,
        ) in &mut tracked_entities
        {
            let should_be_visible = if all_players_visible
                || relations.are_allied(visible_team, *team)
            {
                true
            } else {
                revealers.iter().any(|(source, source_radius)| {
                    xz_distance(transform.translation, *source) <= *source_radius + FOG_COMPENSATION
                })
            };
            let was_visible = visibility_state.visible;
            let is_known_structure = structure.is_some() && (fog_memory.is_some() || was_visible);

            visibility_state.visible = should_be_visible;
            *visibility = if should_be_visible {
                if fog_memory.is_some() {
                    commands.entity(entity).try_remove::<FogMemoryVisible>();
                }
                Visibility::Visible
            } else {
                commands.entity(entity).try_remove::<Selected>();
                if structure.is_some() && was_visible {
                    commands.entity(entity).try_insert(FogMemoryVisible);
                }
                if is_known_structure {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            };
        }
    }

    let remnant_entities = visibility_params.p2();
    for (entity, transform, _remnant) in &remnant_entities {
        let is_rescouted = all_players_visible
            || revealers.iter().any(|(source, source_radius)| {
                xz_distance(transform.translation, *source) <= *source_radius + FOG_COMPENSATION
            });
        if is_rescouted {
            commands.entity(entity).try_despawn();
        }
    }
}

pub(crate) fn spawn_fog_memory_structure_remnant(
    commands: &mut Commands,
    asset_server: Option<&AssetServer>,
    position: Vec3,
    radius: f32,
) {
    let radius = radius.max(0.45);
    let parent = commands
        .spawn((
            Name::new("Fog memory structure remnant"),
            Transform::from_translation(Vec3::new(position.x, 0.02, position.z)),
            Visibility::Visible,
            FogMemoryStructureRemnant { radius },
            MatchScopedEntity,
        ))
        .id();
    let Some(asset_server) = asset_server else {
        return;
    };
    commands.entity(parent).with_children(|remnant| {
        remnant.spawn((
            Name::new("Fog memory scorch mark"),
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
            remnant.spawn((
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset))),
                Transform::from_translation(offset * spread)
                    .with_rotation(Quat::from_rotation_y(index as f32 * 1.9))
                    .with_scale(Vec3::splat(scale * radius)),
            ));
        }
    });
}
