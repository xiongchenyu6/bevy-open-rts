//! Projection/query fixture only; not a normal player gameplay acceptance.
use crate::*;
use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
use bevy::ecs::system::SystemState;

type CameraQuery<'w, 's> =
    Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>;
type ResourceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Selectable,
        &'static VisibilityState,
        &'static ResourceNode,
    ),
>;

fn scene() -> (World, Vec2) {
    let mut world = World::new();
    let transform = Transform::from_xyz(
        0.0,
        CAMERA_RTS_HEIGHT_MAX,
        CAMERA_RTS_HEIGHT_MAX * CAMERA_RTS_ANGLE.tan(),
    )
    .looking_at(Vec3::ZERO, Vec3::Y);
    let global = GlobalTransform::from(transform);
    let camera = Camera {
        computed: ComputedCameraValues {
            clip_from_view: Mat4::perspective_infinite_reverse_rh(
                std::f32::consts::FRAC_PI_4,
                1280.0 / 696.0,
                0.1,
            ),
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(1280, 696),
                scale_factor: 1.0,
            }),
            ..default()
        },
        ..default()
    };
    let cursor = camera.world_to_viewport(&global, Vec3::ZERO).unwrap();
    world.spawn((camera, global, MainCamera));
    (world, cursor)
}
fn node(world: &mut World, position: Vec3, visible: bool) -> Entity {
    world
        .spawn((
            Transform::from_translation(position),
            Selectable { radius: 0.6 },
            VisibilityState { visible },
            ResourceNode {
                kind: ResourceKind::Ore,
                amount: 100,
            },
        ))
        .id()
}
fn pick(world: &mut World, cursor: Vec2) -> Option<Entity> {
    let mut queries = SystemState::<(CameraQuery, ResourceQuery)>::new(world);
    let (camera, resources) = queries.get(world).unwrap();
    resource_target_at_cursor(
        cursor,
        &camera,
        &resources,
        RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX,
    )
}
#[test]
fn hidden_resource_cannot_win_over_a_visible_resource_under_the_same_cursor() {
    let (mut world, cursor) = scene();
    let hidden = node(&mut world, Vec3::ZERO, false);
    let visible = node(&mut world, Vec3::new(0.25, 0.0, 0.0), true);
    assert_ne!(hidden, visible);
    assert_eq!(
        pick(&mut world, cursor),
        Some(visible),
        "Hidden closest resource must not override a visible selectable mineral"
    );
}
#[test]
fn an_unseen_resource_does_not_create_a_hover_or_harvest_target() {
    let (mut world, cursor) = scene();
    node(&mut world, Vec3::ZERO, false);
    assert_eq!(
        pick(&mut world, cursor),
        None,
        "Opaque fog must not expose a resource through screen picking"
    );
}

// Geometry fact fixture. It does NOT establish which cursor was delivered in SDK Retry03.
#[test]
fn resource_screen_capsule_and_ground_actor_picker_can_disagree() {
    let (mut world, _) = scene();
    let ore = node(&mut world, Vec3::ZERO, true);
    world.entity_mut(ore).insert(Team::Neutral);
    let mut camera_state = world.query::<(&Camera, &GlobalTransform)>();
    let (camera, transform) = camera_state.single(&world).unwrap();
    let cursor = camera
        .world_to_viewport(
            transform,
            Vec3::Y * resource_visual_height(ResourceKind::Ore),
        )
        .unwrap()
        - Vec2::Y * 18.0;
    let ray = camera.viewport_to_world(transform, cursor).unwrap();
    let ground = ray
        .plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
        .unwrap();
    assert!(
        xz_distance(ground, Vec3::ZERO) > 1.05,
        "Fixture must be outside the current ground-only mineral pick radius"
    );
    let structure = world
        .spawn((
            Transform::from_translation(ground),
            Team::Player(0),
            Selectable { radius: 2.0 },
            VisibilityState { visible: true },
            Structure {
                id: "CommandCenter",
            },
        ))
        .id();
    assert_eq!(
        pick(&mut world, cursor),
        Some(ore),
        "Real projection must place cursor within current resource screen capsule"
    );
    type RallyQuery<'w, 's> = Query<
        'w,
        's,
        (
            Entity,
            &'static Transform,
            &'static Selectable,
            &'static Team,
            &'static VisibilityState,
            Option<&'static ResourceNode>,
            Option<&'static SupplyCrate>,
            Option<&'static Health>,
            Option<&'static Unit>,
            Option<&'static Structure>,
            Option<&'static UnderConstruction>,
        ),
    >;
    let mut state = SystemState::<RallyQuery>::new(&mut world);
    let q = state.get(&world).unwrap();
    assert_eq!(
        rally_target_at(ground, Team::Player(0), &q),
        Some((structure, ground))
    );
}
