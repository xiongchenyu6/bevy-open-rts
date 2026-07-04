//! The RTS camera: game-facing `RtsCamera` state, the bevy_rts_camera bridge,
//! player camera settings, and camera bookmarks.
//!
//! Pure move out of lib.rs (module-split Stage 2); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;
use bevy_rts_camera::{RtsCamera as RtsCam, RtsCameraControls};

use crate::*;

pub(crate) const CAMERA_MIN_DISTANCE: f32 = 5.5;

pub(crate) const CAMERA_DEFAULT_DISTANCE: f32 = 7.0;

pub(crate) const CAMERA_MAX_DISTANCE: f32 = 9.0;

pub(crate) const CAMERA_DEFAULT_YAW: f32 = -0.72;

pub(crate) const CAMERA_DEFAULT_PITCH: f32 = -1.02;

pub(crate) const CAMERA_BOUNDS_MARGIN: f32 = 1.2;

pub(crate) const CAMERA_EDGE_PAN_WIDTH: f32 = 0.018;

// bevy_rts_camera (perspective, ground-following) framing — RTS tilt and a height
// range mapped from the legacy CAMERA_MIN/MAX_DISTANCE zoom span. NOTE the plugin's
// `angle` is measured from straight-down: 0.0 = top-down, larger = more oblique.
// ~0.55 rad ≈ 32° off vertical ≈ godot's ~58°-below-horizontal isometric look.
pub(crate) const CAMERA_RTS_ANGLE: f32 = 0.55;

pub(crate) const CAMERA_RTS_HEIGHT_MIN: f32 = 6.0;

pub(crate) const CAMERA_RTS_HEIGHT_MAX: f32 = 11.0;

// Player-adjustable camera ranges (Options menu → 镜头). Tilt is the plugin angle
// (0 = top-down). Steps/min/max bound the +/- buttons and slider normalisation.
pub(crate) const CAMERA_TILT_MIN: f32 = 0.15;

pub(crate) const CAMERA_TILT_MAX: f32 = 1.05;

pub(crate) const CAMERA_TILT_STEP: f32 = 0.05;

pub(crate) const CAMERA_RTS_PAN_SPEED: f32 = 18.0;

pub(crate) const CAMERA_PAN_SPEED_MIN: f32 = 8.0;

pub(crate) const CAMERA_PAN_SPEED_MAX: f32 = 32.0;

pub(crate) const CAMERA_PAN_SPEED_STEP: f32 = 2.0;

pub(crate) const CAMERA_BOOKMARK_KEYS: [KeyCode; 4] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
];

#[derive(Component)]
pub(crate) struct MainCamera;

#[derive(Resource)]
pub(crate) struct RtsCamera {
    pub(crate) focus: Vec3,
    pub(crate) distance: f32,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    /// Set when game code wants to jump the camera to `focus`/`distance`; consumed
    /// by `camera_control`, which pushes the values into the `bevy_rts_camera`
    /// component with `snap = true`. When false, the bridge instead mirrors the
    /// plugin's live camera back into this resource so minimap/bookmarks stay fresh.
    pub(crate) pending_jump: bool,
}

impl Default for RtsCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            distance: CAMERA_DEFAULT_DISTANCE,
            yaw: CAMERA_DEFAULT_YAW,
            pitch: CAMERA_DEFAULT_PITCH,
            pending_jump: false,
        }
    }
}

impl RtsCamera {
    pub(crate) fn focused_on(focus: Vec3) -> Self {
        Self {
            focus,
            pending_jump: true,
            ..default()
        }
    }
}

/// Maps the legacy orthographic `distance` (5.5..9.0) onto `bevy_rts_camera`'s
/// `target_zoom` (0.0 = zoomed out, 1.0 = zoomed in).
pub(crate) fn camera_zoom_from_distance(distance: f32) -> f32 {
    let span = CAMERA_MAX_DISTANCE - CAMERA_MIN_DISTANCE;
    ((CAMERA_MAX_DISTANCE - distance) / span).clamp(0.0, 1.0)
}

pub(crate) fn camera_distance_from_zoom(zoom: f32) -> f32 {
    let span = CAMERA_MAX_DISTANCE - CAMERA_MIN_DISTANCE;
    CAMERA_MAX_DISTANCE - zoom.clamp(0.0, 1.0) * span
}

/// A shippable-looking camera pipeline for the low-poly scene: HDR + ACES
/// filmic tonemapping (analytic, no LUT asset) + a gentle bloom so the emissive
/// muzzle/impact/death flashes and team-color accents actually glow. Applied to
/// both the live match camera and the offscreen capture camera so screenshots
/// match what the player sees.
pub(crate) fn cinematic_camera_look() -> impl Bundle {
    use bevy::core_pipeline::tonemapping::Tonemapping;
    use bevy::light::ShadowFilteringMethod;
    use bevy::post_process::bloom::Bloom;
    (
        Tonemapping::AcesFitted,
        Bloom {
            intensity: 0.14,
            ..Bloom::NATURAL
        },
        // Soft PCF so cascade edges read as gentle penumbra, not stair-steps.
        ShadowFilteringMethod::Gaussian,
    )
}

/// Builds the `bevy_rts_camera` component from the game's camera state + map bounds.
pub(crate) fn rts_camera_component(state: &RtsCamera, bounds: MapBounds, tilt: f32) -> RtsCam {
    let mut cam = RtsCam {
        // Fixed isometric-ish tilt (player-adjustable via Options → 镜头).
        angle: tilt,
        target_angle: tilt,
        min_angle: tilt,
        dynamic_angle: false,
        height_min: CAMERA_RTS_HEIGHT_MIN,
        height_max: CAMERA_RTS_HEIGHT_MAX,
        bounds: camera_bounds_aabb(bounds),
        target_zoom: camera_zoom_from_distance(state.distance),
        snap: true,
        ..default()
    };
    cam.target_focus.translation = safe_camera_focus(state.focus, bounds);
    cam
}

pub(crate) fn camera_bounds_aabb(bounds: MapBounds) -> bevy::math::bounding::Aabb2d {
    let min = bounds.clamp_ground_point(Vec3::new(f32::MIN, 0.0, f32::MIN), CAMERA_BOUNDS_MARGIN);
    let max = bounds.clamp_ground_point(Vec3::new(f32::MAX, 0.0, f32::MAX), CAMERA_BOUNDS_MARGIN);
    bevy::math::bounding::Aabb2d {
        min: Vec2::new(min.x, min.z),
        max: Vec2::new(max.x, max.z),
    }
}

#[derive(Resource)]
pub(crate) struct CameraBookmarks {
    pub(crate) slots: [Option<CameraBookmark>; 4],
}

impl Default for CameraBookmarks {
    fn default() -> Self {
        Self {
            slots: [None, None, None, None],
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CameraBookmark {
    pub(crate) focus: Vec3,
    pub(crate) distance: f32,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

impl CameraBookmark {
    pub(crate) fn capture(camera: &RtsCamera) -> Self {
        Self {
            focus: camera.focus,
            distance: camera.distance,
            yaw: camera.yaw,
            pitch: camera.pitch,
        }
    }

    pub(crate) fn restore(self, camera: &mut RtsCamera) {
        camera.focus = self.focus;
        camera.distance = self.distance;
        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
        camera.pending_jump = true;
    }

    pub(crate) fn restore_safely(self, camera: &mut RtsCamera, bounds: MapBounds) {
        self.restore(camera);
        clamp_camera_view_safely(camera, bounds);
    }
}

/// Bridges the game's `RtsCamera` resource and the `bevy_rts_camera` component.
///
/// Live input (WASD/edge-pan/zoom/rotate) is owned by the plugin's
/// `RtsCameraControls`; this system runs before `RtsCameraSystemSet`. On an
/// explicit `pending_jump` it pushes `focus`/`distance` into the component with
/// `snap = true`; otherwise it mirrors the plugin's live target back into the
/// resource so the minimap and camera bookmarks read up-to-date values.
pub(crate) fn camera_control(
    map_bounds: Res<MapBounds>,
    mut camera_state: ResMut<RtsCamera>,
    mut camera_q: Query<&mut RtsCam, With<MainCamera>>,
) {
    let Ok(mut cam) = camera_q.single_mut() else {
        return;
    };
    cam.bounds = camera_bounds_aabb(*map_bounds);
    if camera_state.pending_jump {
        let focus = safe_camera_focus(camera_state.focus, *map_bounds);
        cam.target_focus.translation = focus;
        cam.target_zoom = camera_zoom_from_distance(camera_state.distance);
        cam.snap = true;
        camera_state.focus = focus;
        camera_state.distance = safe_camera_distance(camera_state.distance);
        camera_state.pending_jump = false;
    } else {
        camera_state.focus = cam.target_focus.translation;
        camera_state.distance = camera_distance_from_zoom(cam.target_zoom);
    }
}

/// Applies camera options and gates edge-pan while UI overlays own the cursor.
pub(crate) fn apply_camera_settings(
    options: Res<MenuOptionsState>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    match_menu: Res<MatchMenuState>,
    briefing: Res<MatchBriefingState>,
    ui_buttons: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<Button>,
    >,
    mut camera_q: Query<(&mut RtsCam, &mut RtsCameraControls), With<MainCamera>>,
) {
    let Ok((mut cam, mut controls)) = camera_q.single_mut() else {
        return;
    };
    cam.angle = options.camera_tilt;
    cam.target_angle = options.camera_tilt;
    cam.min_angle = options.camera_tilt;
    controls.pan_speed = options.camera_pan_speed;
    controls.edge_pan_width = effective_camera_edge_pan_width(
        &options,
        &match_menu,
        &briefing,
        window_q
            .single()
            .ok()
            .is_some_and(|window| cursor_is_over_interactive_button(window, &ui_buttons)),
    );
}

pub(crate) fn effective_camera_edge_pan_width(
    options: &MenuOptionsState,
    match_menu: &MatchMenuState,
    _briefing: &MatchBriefingState,
    interactive_ui_active: bool,
) -> f32 {
    if !options.camera_edge_pan || match_menu.visible || interactive_ui_active {
        return 0.0;
    }
    CAMERA_EDGE_PAN_WIDTH
}

pub(crate) fn safe_camera_distance(distance: f32) -> f32 {
    distance.clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE)
}

pub(crate) fn safe_camera_focus(focus: Vec3, bounds: MapBounds) -> Vec3 {
    bounds.clamp_ground_point(focus, CAMERA_BOUNDS_MARGIN)
}

pub(crate) fn set_camera_focus_safely(camera: &mut RtsCamera, focus: Vec3, bounds: MapBounds) {
    camera.focus = safe_camera_focus(focus, bounds);
    // Flag an explicit jump so `camera_control` pushes it into the plugin instead
    // of mirroring the (stale) live camera back over it.
    camera.pending_jump = true;
}

pub(crate) fn clamp_camera_focus_safely(camera: &mut RtsCamera, bounds: MapBounds) {
    camera.focus = safe_camera_focus(camera.focus, bounds);
}

pub(crate) fn clamp_camera_distance_safely(camera: &mut RtsCamera) {
    camera.distance = safe_camera_distance(camera.distance);
}

pub(crate) fn clamp_camera_view_safely(camera: &mut RtsCamera, bounds: MapBounds) {
    clamp_camera_focus_safely(camera, bounds);
    clamp_camera_distance_safely(camera);
}

pub(crate) fn handle_camera_bookmark_hotkeys(
    keyboard: &ButtonInput<KeyCode>,
    bookmarks: &mut CameraBookmarks,
    camera_state: &mut RtsCamera,
    map_bounds: MapBounds,
    alt: bool,
    ctrl: bool,
) {
    for (index, key) in CAMERA_BOOKMARK_KEYS.iter().enumerate() {
        if !keyboard.just_pressed(*key) {
            continue;
        }
        if alt && ctrl {
            bookmarks.slots[index] = Some(CameraBookmark::capture(camera_state));
            continue;
        }
        if alt && let Some(bookmark) = bookmarks.slots[index] {
            bookmark.restore_safely(camera_state, map_bounds);
        }
    }
}
