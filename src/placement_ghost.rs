//! Structure placement preview: a translucent ghost of the actual building
//! follows the cursor so the player sees the real footprint, size and facing
//! before committing — not just a ring on the ground. Green tint = a legal
//! spot, red tint = blocked. The gizmo footprint ring still draws underneath.

use bevy::prelude::*;

use crate::*;

/// Tracks the single live placement-ghost entity, which building it shows, and
/// where it should sit next frame. Computed by [`compute_placement_ghost`]
/// (which holds the heavy preview queries) and consumed by
/// [`apply_placement_ghost_transform`] so the two never touch `Transform`
/// together and trip Bevy's query-conflict checker.
#[derive(Resource, Default)]
pub(crate) struct PlacementGhost {
    pub(crate) entity: Option<Entity>,
    pub(crate) id: Option<&'static str>,
    /// `(world position, yaw, valid)` when the ghost should be shown.
    pub(crate) target: Option<(Vec3, f32, bool)>,
}

/// Root of the placement ghost; `valid` drives the green/red tint.
#[derive(Component)]
pub(crate) struct PlacementGhostRoot {
    pub(crate) valid: bool,
}

/// Spawns / despawns the ghost building and records where it should sit.
pub(crate) fn compute_placement_ghost(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ghost: ResMut<PlacementGhost>,
    preview: StructurePlacementPreviewParams,
) {
    let Some(pending) = preview.command_mode.pending_structure_placement else {
        despawn_ghost(&mut commands, &mut ghost);
        return;
    };
    let Some(def) = registry::entity(pending.id) else {
        despawn_ghost(&mut commands, &mut ghost);
        return;
    };

    // Where the ghost sits: a committed drag position, else the live cursor.
    let point = pending.position.or_else(|| {
        preview.window_q.single().ok().and_then(|window| {
            if cursor_is_over_hud(window, &preview.hud_zones) {
                None
            } else {
                pointer_ground(window, &preview.camera_q, &preview.terrain)
            }
        })
    });

    // Rebuild the model whenever the selected structure changes.
    if ghost.id != Some(pending.id) {
        despawn_ghost(&mut commands, &mut ghost);
        let team = preview.visible_player.team;
        let faction = preview.player_factions.slot_faction(team);
        let root = commands
            .spawn((
                Name::new("Placement ghost"),
                Transform::from_scale(Vec3::splat(def.scale)),
                Visibility::Hidden,
                PlacementGhostRoot { valid: false },
                MatchScopedEntity,
            ))
            .id();
        spawn_entity_models(&mut commands, &asset_server, root, Some(faction), def);
        ghost.entity = Some(root);
        ghost.id = Some(pending.id);
    }

    ghost.target = point.map(|point| {
        let validity = structure_placement_validity_for_faction(
            preview.visible_player.team,
            preview
                .player_factions
                .slot_faction(preview.visible_player.team),
            pending.id,
            point,
            *preview.map_bounds,
            &preview.terrain,
            &preview.economies,
            &preview.structures,
            &preview.occupiers,
        );
        (
            point,
            pending.rotation_y_radians(),
            validity == StructurePlacementValidity::Valid,
        )
    });
}

/// Applies the position/facing/validity recorded by [`compute_placement_ghost`].
pub(crate) fn apply_placement_ghost_transform(
    ghost: Res<PlacementGhost>,
    mut roots: Query<(&mut Transform, &mut Visibility, &mut PlacementGhostRoot)>,
) {
    let Some(entity) = ghost.entity else {
        return;
    };
    let Ok((mut transform, mut visibility, mut root)) = roots.get_mut(entity) else {
        return;
    };
    match ghost.target {
        Some((point, yaw, valid)) => {
            transform.translation = point;
            transform.rotation = Quat::from_rotation_y(yaw);
            root.valid = valid;
            *visibility = Visibility::Visible;
        }
        None => *visibility = Visibility::Hidden,
    }
}

fn despawn_ghost(commands: &mut Commands, ghost: &mut PlacementGhost) {
    if let Some(entity) = ghost.entity.take() {
        commands.entity(entity).despawn();
    }
    ghost.id = None;
    ghost.target = None;
}

/// Overrides every ghost mesh with a translucent green (valid) / red (invalid)
/// material each frame — GLB parts stream in over several frames and the
/// validity flips as the cursor moves, so we re-assign rather than swap once.
pub(crate) fn tint_placement_ghost(
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut tints: Local<Option<(Handle<StandardMaterial>, Handle<StandardMaterial>)>>,
    roots: Query<(Entity, &PlacementGhostRoot)>,
    children_q: Query<&Children>,
    mut parts: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let (valid, invalid) = tints
        .get_or_insert_with(|| {
            let mut translucent = |r, g, b| {
                materials.add(StandardMaterial {
                    base_color: Color::srgba(r, g, b, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })
            };
            (translucent(0.3, 1.0, 0.5), translucent(1.0, 0.28, 0.22))
        })
        .clone();
    for (root, ghost) in &roots {
        let want = if ghost.valid { &valid } else { &invalid };
        for child in children_q.iter_descendants(root) {
            if let Ok(mut material) = parts.get_mut(child)
                && material.0 != *want
            {
                material.0 = want.clone();
            }
        }
    }
}
