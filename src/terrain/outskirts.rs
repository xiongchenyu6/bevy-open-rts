//! Render-only ground outside the playable rectangle. It never participates in
//! camera ground following, fog, selection, navigation or gameplay occupancy.
use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};

const OUTSKIRTS_EXTENT_M: f32 = 256.0;

fn outskirts_mesh(bounds: MapBounds, extension: f32) -> Mesh {
    let w = bounds.half_width;
    let d = bounds.half_depth;
    let rectangles = [
        (-w - extension, w + extension, -d - extension, -d),
        (-w - extension, w + extension, d, d + extension),
        (-w - extension, -w, -d, d),
        (w, w + extension, -d, d),
    ];
    let mut positions = Vec::with_capacity(16);
    let mut uv = Vec::with_capacity(16);
    let mut indices = Vec::with_capacity(24);
    for (x0, x1, z0, z1) in rectangles {
        let base = positions.len() as u32;
        for (x, z) in [(x0, z0), (x0, z1), (x1, z1), (x1, z0)] {
            positions.push([x, 0.0, z]);
            uv.push([x / (w * 2.0) + 0.5, z / (d * 2.0) + 0.5]);
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 16])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(indices))
}

pub(crate) fn spawn_battlefield_outskirts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bounds: MapBounds,
    terrain_detail: Option<&Handle<Image>>,
) {
    use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
    // Repeat a separate sampler; keep the playable ground's texture unchanged.
    let texture = terrain_detail
        .and_then(|handle| images.get(handle))
        .cloned()
        .map(|mut image| {
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::MirrorRepeat,
                address_mode_v: ImageAddressMode::MirrorRepeat,
                ..ImageSamplerDescriptor::linear()
            });
            images.add(image)
        });
    commands.spawn((
        Name::new("Battlefield outskirts"),
        Mesh3d(meshes.add(outskirts_mesh(bounds, OUTSKIRTS_EXTENT_M))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.40, 0.36),
            base_color_texture: texture,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.04, 0.0),
        bevy::light::NotShadowCaster,
        MatchScopedEntity,
    ));
    // A thin, quiet rim makes the playable limit readable without covering it.
    commands.spawn((
        Name::new("Battlefield boundary rim"),
        Mesh3d(meshes.add(outskirts_mesh(bounds, 0.08))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.24, 0.25, 0.26),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.015, 0.0),
        bevy::light::NotShadowCaster,
        MatchScopedEntity,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn apron_and_rim_are_entirely_outside_the_playable_rectangle() {
        for size in [(50.0, 50.0), (64.0, 48.0), (120.0, 120.0)] {
            let bounds = MapBounds::from_size(size);
            for extension in [OUTSKIRTS_EXTENT_M, 0.08] {
                let mesh = outskirts_mesh(bounds, extension);
                let Some(VertexAttributeValues::Float32x3(vertices)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("missing positions")
                };
                let Some(Indices::U32(indices)) = mesh.indices() else {
                    panic!("missing indices")
                };
                assert_eq!(vertices.len(), 16);
                assert_eq!(indices.len(), 24);
                for triangle in indices.chunks_exact(3) {
                    let v = [
                        vertices[triangle[0] as usize],
                        vertices[triangle[1] as usize],
                        vertices[triangle[2] as usize],
                    ];
                    assert!(
                        v.iter().all(|p| p[0] <= -bounds.half_width)
                            || v.iter().all(|p| p[0] >= bounds.half_width)
                            || v.iter().all(|p| p[2] <= -bounds.half_depth)
                            || v.iter().all(|p| p[2] >= bounds.half_depth),
                        "triangle covers playable ground"
                    );
                    let a = Vec3::from_array(v[0]);
                    let b = Vec3::from_array(v[1]);
                    let c = Vec3::from_array(v[2]);
                    assert!((b - a).cross(c - a).y > 0.0, "back-facing terrain");
                }
            }
        }
    }

    #[test]
    fn outskirts_spawn_only_render_entities_and_preserve_the_ground_sampler() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut images = Assets::<Image>::default();
        let texture = images.add(Image::default());
        let before = images.get(&texture).unwrap().sampler.clone();
        spawn_battlefield_outskirts(
            &mut Commands::new(&mut queue, &world),
            &mut meshes,
            &mut materials,
            &mut images,
            MapBounds::from_size((50.0, 50.0)),
            Some(&texture),
        );
        queue.apply(&mut world);
        let owned: Vec<_> = world
            .query_filtered::<Entity, (With<MatchScopedEntity>, With<Mesh3d>)>()
            .iter(&world)
            .collect();
        assert_eq!(owned.len(), 2, "expected exactly two owned render meshes");
        assert_eq!(images.get(&texture).unwrap().sampler, before);
        for entity in owned {
            let e = world.entity(entity);
            assert!(
                e.contains::<Mesh3d>()
                    && e.contains::<MeshMaterial3d<StandardMaterial>>()
                    && e.contains::<MatchScopedEntity>()
            );
            assert!(
                !e.contains::<bevy_rts_camera::Ground>()
                    && !e.contains::<Selectable>()
                    && !e.contains::<Structure>()
                    && !e.contains::<ResourceNode>()
                    && !e.contains::<TerrainWall>()
                    && !e.contains::<FogOverlayPlane>()
            );
            assert!(e.get::<Transform>().unwrap().translation.y < 0.0);
        }
    }
}
