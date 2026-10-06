//! Small engineered scale cues for the command center's authored corridor kit.
use bevy::prelude::*;

use crate::{entity_visual_scale, registry, scaled_render_part_transform};

#[derive(Resource)]
struct StructureDetailAssets {
    cube: Handle<Mesh>,
    steel: Handle<StandardMaterial>,
    recess: Handle<StandardMaterial>,
    concrete: Handle<StandardMaterial>,
}

/// Shared by gameplay spawning and the model harness; skipped without a renderer.
pub(crate) fn spawn_structure_details(world: &mut World, root: Entity, def: &registry::EntityDef) {
    if def.id != "CommandCenter"
        || !world.contains_resource::<bevy::render::renderer::RenderDevice>()
        || !world.contains_resource::<Assets<Mesh>>()
        || !world.contains_resource::<Assets<StandardMaterial>>()
    {
        return;
    }
    if !world.contains_resource::<StructureDetailAssets>() {
        let cube = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(1.0, 1.0, 1.0));
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let steel = materials.add(StandardMaterial {
            base_color: Color::srgb(0.27, 0.30, 0.32),
            metallic: 0.45,
            perceptual_roughness: 0.8,
            ..default()
        });
        let recess = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.12, 0.13),
            perceptual_roughness: 0.95,
            ..default()
        });
        let concrete = materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.30, 0.27),
            perceptual_roughness: 0.96,
            ..default()
        });
        world.insert_resource(StructureDetailAssets {
            cube,
            steel,
            recess,
            concrete,
        });
    }
    let assets = world.resource::<StructureDetailAssets>();
    let cube = assets.cube.clone();
    let steel = assets.steel.clone();
    let recess = assets.recess.clone();
    let concrete = assets.concrete.clone();
    for part in def.render_parts {
        // Actual roof triangle heights, including the GLB mesh-node offset:
        // the window module has a recessed center, not a flat y=1 roof.
        // Detailed/corner/cross modules keep their authored fittings and gaps.
        let (roof, conduits) = match part.model {
            "models/kenney-spacekit/corridor.glb" => (1.0, true),
            "models/kenney-spacekit/corridor_windowClosed.glb" => (0.85, false),
            _ => continue,
        };
        let assembly = scaled_render_part_transform(part, entity_visual_scale(def.id));
        let parent = world
            .spawn((
                Name::new("Command center service details"),
                ChildOf(root),
                assembly,
                Visibility::default(),
            ))
            .id();
        let mut add = |label: &'static str,
                       position: Vec3,
                       size: Vec3,
                       material: &Handle<StandardMaterial>| {
            world.spawn((
                Name::new(label),
                ChildOf(parent),
                Mesh3d(cube.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position).with_scale(size),
            ));
        };
        // Low foundation lips follow each individual corridor footprint.
        add(
            "Corridor foundation",
            Vec3::new(2.0, 0.018, 1.5),
            Vec3::new(1.055, 0.036, 1.055),
            &concrete,
        );
        // A recessed roof vent with closely spaced metal louvres supplies a
        // human-scale reference without changing the building silhouette.
        add(
            "Vent recess",
            Vec3::new(2.0, roof + 0.009, 1.5),
            Vec3::new(0.28, 0.018, 0.36),
            &recess,
        );
        for index in 0..7 {
            add(
                "Vent louvre",
                Vec3::new(2.0, roof + 0.025, 1.35 + index as f32 * 0.05),
                Vec3::new(0.26, 0.018, 0.016),
                &steel,
            );
        }
        // Pair of edge conduits, with short mounting saddles.
        for x in [1.75, 2.25].into_iter().filter(|_| conduits) {
            add(
                "Roof service conduit",
                Vec3::new(x, roof + 0.018, 1.5),
                Vec3::new(0.018, 0.018, 0.84),
                &steel,
            );
            for z in [1.18, 1.5, 1.82] {
                add(
                    "Conduit saddle",
                    Vec3::new(x, roof + 0.026, z),
                    Vec3::new(0.047, 0.025, 0.025),
                    &steel,
                );
            }
        }
    }
}
