//! Ambient life for structures: radar/comms dishes slowly sweep, and chimneys
//! breathe smoke puffs. Both ride the same GLB-node trick as limbs/turrets —
//! kenney's dish and chimney meshes are centered on their own named node, so
//! rotating the node spins the dish in place and the node's GlobalTransform
//! gives the true chimney mouth for smoke spawns.

use bevy::prelude::*;

use crate::*;

/// A dish node that sweeps around its own axis.
#[derive(Component)]
pub(crate) struct DishSpinner {
    pub(crate) rest_rotation: Quat,
    pub(crate) seed: f32,
}

/// A chimney node that periodically emits a smoke puff.
#[derive(Component)]
pub(crate) struct ChimneyVent {
    pub(crate) root: Entity,
    pub(crate) next_emit: f32,
}

/// One rising, fading smoke puff.
#[derive(Component)]
pub(crate) struct SmokePuff {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Tags dish and chimney nodes as GLB scenes stream in.
pub(crate) fn tag_structure_ambience(
    mut commands: Commands,
    fresh: Query<(Entity, &Name, &Transform), Added<Name>>,
    parents: Query<&ChildOf>,
    roots: Query<(), Or<(With<Structure>, With<Unit>)>>,
) {
    for (entity, name, transform) in &fresh {
        let is_dish = matches!(
            name.as_str(),
            "satelliteDish" | "satelliteDish_detailed" | "satelliteDish_large"
        );
        let is_chimney = matches!(name.as_str(), "chimney" | "chimney_detailed");
        if !is_dish && !is_chimney {
            continue;
        }
        let mut cursor = entity;
        let root = loop {
            match parents.get(cursor) {
                Ok(child_of) => {
                    cursor = child_of.0;
                    if roots.contains(cursor) {
                        break Some(cursor);
                    }
                }
                Err(_) => break None,
            }
        };
        let Some(root) = root else {
            continue;
        };
        let seed = (root.to_bits() % 89) as f32 * 0.41;
        if is_dish {
            commands.entity(entity).try_insert(DishSpinner {
                rest_rotation: transform.rotation,
                seed,
            });
        } else {
            commands.entity(entity).try_insert(ChimneyVent {
                root,
                next_emit: 0.4 + seed % 1.0,
            });
        }
    }
}

/// Slow radar sweep, each dish phased by its owner's seed.
pub(crate) fn spin_dish_nodes(time: Res<Time>, mut dishes: Query<(&DishSpinner, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (dish, mut transform) in &mut dishes {
        transform.rotation = dish.rest_rotation * Quat::from_rotation_y(t * 0.6 + dish.seed);
    }
}

/// Emits a translucent puff at each working chimney mouth every second or so.
pub(crate) fn emit_chimney_smoke(
    mut commands: Commands,
    time: Res<Time>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puff_mesh: Local<Option<Handle<Mesh>>>,
    mut vents: Query<(&mut ChimneyVent, &GlobalTransform)>,
    roots: Query<Has<UnderConstruction>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let dt = time.delta_secs();
    for (mut vent, vent_gt) in &mut vents {
        // Ghosted construction sites don't smoke yet.
        if roots.get(vent.root).unwrap_or(false) {
            continue;
        }
        vent.next_emit -= dt;
        if vent.next_emit > 0.0 {
            continue;
        }
        vent.next_emit = 1.1 + (vent_gt.translation().x * 7.3).sin().abs() * 0.5;
        let mesh = puff_mesh
            .get_or_insert_with(|| meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()))
            .clone();
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.62, 0.6, 0.58, 0.32),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        // The chimney mesh spans 0..2 in node space; spawn at the mouth.
        let mouth = vent_gt.transform_point(Vec3::Y * 2.0);
        commands.spawn((
            Name::new("Chimney smoke"),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(mouth).with_scale(Vec3::splat(0.06)),
            SmokePuff {
                age: 0.0,
                ttl: 1.6,
                material,
            },
            MatchScopedEntity,
        ));
    }
}

/// Puffs rise, swell and fade out, then despawn (with their material).
pub(crate) fn update_smoke_puffs(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puffs: Query<(Entity, &mut SmokePuff, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut puff, mut transform) in &mut puffs {
        puff.age += dt;
        let life = (puff.age / puff.ttl).clamp(0.0, 1.0);
        if life >= 1.0 {
            materials.remove(&puff.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        transform.translation.y += dt * 0.45;
        transform.scale = Vec3::splat(0.06 + life * 0.14);
        if let Some(mut material) = materials.get_mut(&puff.material) {
            material.base_color.set_alpha(0.32 * (1.0 - life));
        }
    }
}
