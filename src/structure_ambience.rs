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

// ---------------------------------------------------------------------------
// Resource sparkle
// ---------------------------------------------------------------------------

/// Deterministic 0..1 hash for glint scheduling/placement (no RNG).
fn glint_hash01(a: u64, b: u32) -> f32 {
    let mut h = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b as u64)
        .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 31;
    ((h & 0xffff) as f32) / 65_535.0
}

/// Per-node glint scheduler.
#[derive(Component)]
pub(crate) struct GlintEmitter {
    pub(crate) next: f32,
    pub(crate) cycle: u32,
}

/// One short star-glint flash on a crystal.
#[derive(Component)]
pub(crate) struct ResourceGlint {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) material: Handle<StandardMaterial>,
}

/// The mineral veins breathe: the shared facet materials' emissive pulses
/// slowly (ore and crystal out of phase), which the bloom pass turns into a
/// soft living glow across every node of that mineral.
pub(crate) fn pulse_resource_tints(
    time: Res<Time>,
    tints: Option<Res<ResourceTintMaterials>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(tints), Some(mut materials)) = (tints, materials) else {
        return;
    };
    let t = time.elapsed_secs();
    for (handle, kind, phase) in [
        (&tints.ore, ResourceKind::Ore, 0.0_f32),
        (
            &tints.crystal,
            ResourceKind::Crystal,
            std::f32::consts::FRAC_PI_2,
        ),
    ] {
        let Some(mut material) = materials.get_mut(handle) else {
            continue;
        };
        let lin = kind.color().to_linear();
        let breath = 0.22 + ((t * 1.3 + phase).sin() * 0.5 + 0.5) * 0.38;
        material.emissive =
            LinearRgba::new(lin.red * breath, lin.green * breath, lin.blue * breath, 1.0);
    }
}

/// Spawns a brief bright glint at a pseudo-random spot on each resource node
/// every couple of seconds — classic RTS gem sparkle, amplified by bloom.
pub(crate) fn emit_resource_glints(
    mut commands: Commands,
    time: Res<Time>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut glint_mesh: Local<Option<Handle<Mesh>>>,
    untracked: Query<Entity, (With<ResourceNode>, Without<GlintEmitter>)>,
    mut nodes: Query<(Entity, &ResourceNode, &GlobalTransform, &mut GlintEmitter)>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    for entity in &untracked {
        commands.entity(entity).try_insert(GlintEmitter {
            next: glint_hash01(entity.to_bits(), 0) * 2.5,
            cycle: 0,
        });
    }
    let dt = time.delta_secs();
    for (entity, node, node_gt, mut emitter) in &mut nodes {
        emitter.next -= dt;
        if emitter.next > 0.0 {
            continue;
        }
        emitter.cycle = emitter.cycle.wrapping_add(1);
        let bits = entity.to_bits();
        emitter.next = 1.4 + glint_hash01(bits, emitter.cycle) * 2.2;
        // A pseudo-random spot on the crystal cluster.
        let angle = glint_hash01(bits, emitter.cycle.wrapping_mul(3)) * std::f32::consts::TAU;
        let radius = 0.18 + glint_hash01(bits, emitter.cycle.wrapping_mul(5)) * 0.3;
        let height = 0.25 + glint_hash01(bits, emitter.cycle.wrapping_mul(7)) * 0.4;
        let spot =
            node_gt.translation() + Vec3::new(angle.cos() * radius, height, angle.sin() * radius);
        let mesh = glint_mesh
            .get_or_insert_with(|| meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()))
            .clone();
        // Additive white-tinted spark in the mineral's hue.
        let lin = node.kind.color().to_linear();
        let material = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(
                lin.red * 0.4 + 0.6,
                lin.green * 0.4 + 0.6,
                lin.blue * 0.4 + 0.6,
                0.85,
            ),
            emissive: LinearRgba::new(
                lin.red * 2.0 + 1.5,
                lin.green * 2.0 + 1.5,
                lin.blue * 2.0 + 1.5,
                1.0,
            ),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Name::new("Resource glint"),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(spot).with_scale(Vec3::splat(0.01)),
            ResourceGlint {
                age: 0.0,
                ttl: 0.35,
                material,
            },
            MatchScopedEntity,
        ));
    }
}

/// Glints pop in and out (triangular scale envelope), then despawn.
pub(crate) fn update_resource_glints(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut glints: Query<(Entity, &mut ResourceGlint, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut glint, mut transform) in &mut glints {
        glint.age += dt;
        let life = (glint.age / glint.ttl).clamp(0.0, 1.0);
        if life >= 1.0 {
            materials.remove(&glint.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        // Triangular envelope: quick pop to full size at mid-life, back down.
        let envelope = 1.0 - (life * 2.0 - 1.0).abs();
        transform.scale = Vec3::splat(0.02 + envelope * 0.085);
    }
}
