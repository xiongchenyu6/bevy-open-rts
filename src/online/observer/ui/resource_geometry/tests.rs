use super::*;
use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
use bevy::ecs::system::SystemState;
use bevy::math::Vec3A;

type Cameras<'w, 's> = Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>;
fn camera(world: &mut World) {
    let transform = Transform::from_xyz(0.0, 11.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y);
    world.spawn((
        MainCamera,
        GlobalTransform::from(transform),
        Camera {
            computed: ComputedCameraValues {
                clip_from_view: Mat4::perspective_infinite_reverse_rh(
                    std::f32::consts::FRAC_PI_4,
                    1276.0 / 690.0,
                    0.1,
                ),
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1276, 690),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
}
fn node(world: &mut World, visible: bool) -> Entity {
    world
        .spawn((
            ResourceNode {
                kind: ResourceKind::Ore,
                amount: 100,
            },
            Transform::default(),
            GlobalTransform::default(),
            VisibilityState { visible },
            Visibility::Visible,
        ))
        .id()
}
fn observe(world: &mut World) -> serde_json::Value {
    let mut state = SystemState::<(Queries, Cameras)>::new(world);
    let (geometry, cameras) = state.get(world).unwrap();
    snapshot(
        &geometry,
        &cameras,
        Team::Player(0),
        Some(Vec2::new(500.0, 400.0)),
        None,
    )
}
#[test]
fn geometry_includes_only_visible_or_actual_own_harvest_references() {
    let mut world = World::new();
    camera(&mut world);
    let visible = node(&mut world, true);
    let own = node(&mut world, false);
    let unknown = node(&mut world, false);
    let other = node(&mut world, false);
    for (team, target) in [(Team::Player(0), own), (Team::Player(1), other)] {
        world.spawn((
            team,
            HarvestOrder {
                resource: Some(target),
                state: HarvestState::MovingToResource,
                collect_remaining: 0.0,
                last_kind: None,
            },
        ));
    }
    let before = world.entities().len();
    let value = observe(&mut world);
    let ids: HashSet<_> = value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["entity"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, HashSet::from([visible.to_bits(), own.to_bits()]));
    assert!(!ids.contains(&unknown.to_bits()) && !ids.contains(&other.to_bits()));
    assert_eq!(
        world.entities().len(),
        before,
        "Passive observation must not spawn entities"
    );
}
#[test]
fn actual_transformed_glb_bounds_exclude_unrelated_procedural_children() {
    let mut world = World::new();
    camera(&mut world);
    let root = node(&mut world, true);
    let scene = world
        .spawn((
            WorldAssetRoot::default(),
            ChildOf(root),
            GlobalTransform::default(),
        ))
        .id();
    let transform = Transform::from_xyz(1.0, 2.0, 3.0).with_scale(Vec3::new(0.5, 2.0, 1.0));
    world.spawn((
        ChildOf(scene),
        transform,
        GlobalTransform::from(transform),
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::new(0.5, 0.25, 0.75),
        },
    ));
    world.spawn((
        ChildOf(root),
        GlobalTransform::from_translation(Vec3::new(0.0, 500.0, 0.0)),
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::ONE,
        },
    ));
    let value = observe(&mut world);
    let node = &value["resources"][0];
    assert_eq!(node["loaded_mesh_count"], 1);
    assert_eq!(
        node["glb_world_bounds"]["min"],
        serde_json::json!([0.75, 1.5, 2.25])
    );
    assert_eq!(
        node["glb_world_bounds"]["max"],
        serde_json::json!([1.25, 2.5, 3.75])
    );
    assert_eq!(
        node["meshes"][0]["world_corners"].as_array().unwrap().len(),
        8
    );
    assert_eq!(
        value["logical_viewport"],
        serde_json::json!([[0.0, 0.0], [1276.0, 690.0]])
    );
    assert_eq!(value["camera_clip_from_view"].as_array().unwrap().len(), 16);
}
#[test]
fn missing_camera_is_reported_without_fabricated_geometry() {
    let mut world = World::new();
    node(&mut world, true);
    let value = observe(&mut world);
    assert_eq!(value["camera_available"], false);
    assert_eq!(value["resources"], serde_json::json!([]));
}
