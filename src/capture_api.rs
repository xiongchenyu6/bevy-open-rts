//! The capture/test harness surface: `pub fn capture_*` drivers used by
//! `src/bin/capture.rs` and headless tests, plus the offscreen render-target
//! plumbing and the model-harness scene.
//!
//! Pure move out of lib.rs (module-split Stage 6); see IMPLEMENTATION_PLAN.md.

use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy_rts_camera::RtsCamera as RtsCam;

use crate::*;

pub fn capture_show_main_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::MainMenu);
    for _ in 0..8 {
        app.update();
    }
}

pub fn capture_show_skirmish_setup_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::SkirmishSetup);
    for _ in 0..8 {
        app.update();
    }
}

/// Enters the lobby and opens slot 0's controller dropdown, so the floating popup
/// overlay is visible for verification.
pub fn capture_show_skirmish_setup_with_dropdown(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::SkirmishSetup);
    for _ in 0..8 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<SkirmishMenuSelection>()
        .toggle_faction_dropdown(0);
    for _ in 0..6 {
        app.update();
    }
}

pub fn capture_show_campaign_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::CampaignMenu);
    for _ in 0..8 {
        app.update();
    }
}

pub fn capture_show_options_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::OptionsMenu);
    for _ in 0..8 {
        app.update();
    }
}

pub fn capture_show_credits_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::CreditsMenu);
    for _ in 0..8 {
        app.update();
    }
}

/// Offscreen render target handle used by the capture binary.
#[derive(Resource, Clone)]
pub struct CaptureTarget(pub Handle<Image>);

#[derive(Component)]
pub(crate) struct CaptureCameraReady;

/// Render-capable headless app for real screenshot/video capture.
///
/// Unlike [`GameAppMode::Headless`] (which uses `MinimalPlugins` and never
/// renders), this builds the full render pipeline with no window and an
/// offscreen image target, so captured frames show the *actual* Bevy scene
/// instead of a hand-drawn approximation.
pub fn build_capture_app(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                // A synthetic primary window (no winit / no OS surface) so the real
                // mouse-input systems, which read `Window.cursor_position()`, have a
                // window to query. Its size matches the offscreen render target so
                // cursor↔world projection lines up with what the camera renders.
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height),
                    visible: false,
                    ..default()
                }),
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::winit::WinitPlugin>(),
    )
    .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
        std::time::Duration::ZERO,
    ))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));

    app.insert_resource(ClearColor(Color::srgb(0.028, 0.034, 0.045)))
        .insert_resource(StartupLoadingPolicy {
            preload_assets: true,
        })
        .add_plugins((
            JsonAssetPlugin::<RtsDataManifest>::new(&["rts.json"]),
            RonAssetPlugin::<RtsDataManifest>::new(&["rts.ron"]),
            RonAssetPlugin::<GodotModelMapAsset>::new(&["model_map.ron"]),
        ))
        .insert_resource(RenderErrorHandler(handle_render_error));
    add_game_scenes(&mut app);

    // Create the offscreen image directly in the world so the handle is stable
    // before any system reads it (Commands-deferred creation would not be).
    let image = Image::new_target_texture(
        width,
        height,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.insert_resource(CaptureTarget(handle));
    app.add_systems(Update, retarget_capture_camera);
    // Drive the app via manual `update()` calls (not `run()`), so we must run
    // plugin finalization ourselves — this is where `RenderPlugin` creates the
    // `RenderDevice`. Without it the render systems panic on a missing device.
    app.finish();
    app.cleanup();
    app
}

/// Build a render-capable offscreen app dedicated to isolated model gallery
/// captures. This intentionally does not register the menu/match scene, so the
/// screenshots contain only the registry model under review.
pub fn build_model_harness_capture_app(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::winit::WinitPlugin>(),
    )
    .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
        std::time::Duration::ZERO,
    ))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ))
    .insert_resource(ClearColor(Color::srgb(0.028, 0.034, 0.045)))
    .insert_resource(RenderErrorHandler(handle_render_error))
    .init_resource::<HunyuanModelMaterialCache>()
    .add_systems(Update, apply_hunyuan_model_materials);

    let image = Image::new_target_texture(
        width,
        height,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.insert_resource(CaptureTarget(handle));
    app.finish();
    app.cleanup();
    app
}

#[derive(Clone, Debug)]
pub struct ModelHarnessSlot {
    pub index: usize,
    pub page: usize,
    pub row: usize,
    pub column: usize,
    pub id: &'static str,
    pub label: &'static str,
    pub role: &'static str,
    pub render_parts: usize,
    pub model_assets: usize,
}

pub fn capture_model_harness_entity_count() -> usize {
    MODEL_HARNESS_ENTITY_IDS.len()
}

pub fn capture_model_harness_page_count(per_page: usize) -> usize {
    let per_page = per_page.max(1);
    capture_model_harness_entity_count().div_ceil(per_page)
}

pub fn capture_model_harness_manifest(per_page: usize) -> Vec<ModelHarnessSlot> {
    let per_page = per_page.max(1);
    MODEL_HARNESS_ENTITY_IDS
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let def = model_harness_entity_def(*id);
            let page = index / per_page;
            let local = index % per_page;
            let columns = model_harness_columns(per_page);
            ModelHarnessSlot {
                index,
                page,
                row: local / columns,
                column: local % columns,
                id: def.id,
                label: def.label,
                role: model_harness_role(def.role),
                render_parts: def.render_parts.len(),
                model_assets: def.model_assets.len(),
            }
        })
        .collect()
}

pub fn capture_spawn_model_harness_page(
    app: &mut App,
    page: usize,
    per_page: usize,
) -> Vec<ModelHarnessSlot> {
    let per_page = per_page.max(1);
    let start = page.saturating_mul(per_page);
    let end = (start + per_page).min(MODEL_HARNESS_ENTITY_IDS.len());
    let page_defs: Vec<(usize, &'static registry::EntityDef)> = MODEL_HARNESS_ENTITY_IDS
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|(index, id)| (index, model_harness_entity_def(*id)))
        .collect();
    let columns = model_harness_columns(per_page);
    let rows = page_defs.len().div_ceil(columns).max(1);
    let spacing_x = 8.6;
    let spacing_z = 7.2;
    let width = (columns.saturating_sub(1)) as f32 * spacing_x;
    let depth = (rows.saturating_sub(1)) as f32 * spacing_z;
    let focus = Vec3::new(0.0, 0.0, 0.0);
    let camera_height = 28.0;
    let camera_depth = 22.0;
    let ortho_width = (width + 12.0)
        .max((depth + 13.0) * (MODEL_HARNESS_ASPECT_RATIO))
        .max(24.0);

    {
        let world = app.world_mut();
        world.insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 260.0,
            affects_lightmapped_meshes: true,
        });
        let target = world.resource::<CaptureTarget>().0.clone();
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let ground_mesh = meshes.add(Plane3d::default().mesh().size(width + 10.0, depth + 10.0));
        drop(meshes);
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let ground_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.2),
            perceptual_roughness: 0.92,
            ..default()
        });
        drop(materials);

        world.spawn((
            Name::new("Model Harness Camera"),
            Camera3d::default(),
            bevy::camera::Hdr,
            cinematic_camera_look(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.028, 0.034, 0.045)),
                ..default()
            },
            Projection::from(OrthographicProjection {
                scaling_mode: ScalingMode::FixedHorizontal {
                    viewport_width: ortho_width,
                },
                far: 1000.0,
                ..OrthographicProjection::default_3d()
            }),
            RenderTarget::Image(target.into()),
            Transform::from_xyz(0.0, camera_height, camera_depth).looking_at(focus, Vec3::Y),
        ));
        world.spawn((
            Name::new("Model Harness Key Light"),
            DirectionalLight {
                shadow_maps_enabled: true,
                illuminance: 16_000.0,
                ..default()
            },
            Transform::from_xyz(-6.0, 14.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        world.spawn((
            Name::new("Model Harness Ground"),
            Mesh3d(ground_mesh),
            MeshMaterial3d(ground_material),
            Transform::IDENTITY,
        ));
    }

    let mut slots = Vec::new();
    for (local, (index, def)) in page_defs.iter().enumerate() {
        let row = local / columns;
        let column = local % columns;
        let x = column as f32 * spacing_x - width * 0.5;
        let z = row as f32 * spacing_z - depth * 0.5;
        let root =
            spawn_model_harness_root(app.world_mut(), *index, def, Vec3::new(x, def.height, z));
        spawn_entity_models_for_harness(app.world_mut(), root, None, def);
        slots.push(ModelHarnessSlot {
            index: *index,
            page,
            row,
            column,
            id: def.id,
            label: def.label,
            role: model_harness_role(def.role),
            render_parts: def.render_parts.len(),
            model_assets: def.model_assets.len(),
        });
    }
    slots
}

pub(crate) fn model_harness_entity_def(id: &'static str) -> &'static registry::EntityDef {
    registry::entity(id).unwrap_or_else(|| panic!("model harness id `{id}` is not in ENTITY_DEFS"))
}

pub(crate) fn spawn_model_harness_root(
    world: &mut World,
    index: usize,
    def: &'static registry::EntityDef,
    translation: Vec3,
) -> Instance<ModelHarnessRoot> {
    let entity = world.spawn((
        Name::new(format!("Model Harness {}", def.id)),
        ModelHarnessRoot { index, id: def.id },
        Transform::from_translation(translation).with_scale(Vec3::splat(def.scale)),
        Visibility::default(),
    ));
    // The marker is inserted in the same bundle, so this entity now satisfies
    // `Instance<ModelHarnessRoot>` before it can be passed to harness-only code.
    unsafe { Instance::from_entity_unchecked(entity.id()) }
}

pub(crate) fn model_harness_columns(per_page: usize) -> usize {
    per_page.clamp(1, 3)
}

pub(crate) fn model_harness_role(role: registry::EntityRole) -> &'static str {
    match role {
        registry::EntityRole::Unit => "Unit",
        registry::EntityRole::Structure => "Structure",
    }
}

/// Capture/dev helper: orders every player unit to attack-move toward the
/// nearest enemy base, so a headless capture shows real movement and combat
/// without waiting out the AI's ~45s opening grace. This drives the same
/// `AttackMoveOrder` component the live simulation consumes — it is not a
/// headless assertion harness, it just gives the camera something real to film.
pub fn capture_player_attack_move_all(app: &mut App) {
    let player = Team::Player(0);
    let world = app.world_mut();

    let mut destination = None;
    {
        let mut structures = world.query_filtered::<(&Team, &Transform), With<Structure>>();
        for (team, transform) in structures.iter(world) {
            if *team != player && *team != Team::Neutral {
                destination = Some(transform.translation);
                break;
            }
        }
    }
    if destination.is_none() {
        let mut units = world.query_filtered::<(&Team, &Transform), With<Unit>>();
        for (team, transform) in units.iter(world) {
            if *team != player && *team != Team::Neutral {
                destination = Some(transform.translation);
                break;
            }
        }
    }
    let Some(destination) = destination else {
        return;
    };

    let player_units: Vec<Entity> = {
        let mut units = world.query_filtered::<(Entity, &Team), With<Unit>>();
        units
            .iter(world)
            .filter(|(_, team)| **team == player)
            .map(|(entity, _)| entity)
            .collect()
    };
    for entity in player_units {
        world
            .entity_mut(entity)
            .insert(AttackMoveOrder { destination });
    }
}

/// Capture/dev input helpers: drive the REAL mouse-input systems headlessly by
/// moving the synthetic window's cursor and emitting mouse-button messages, the
/// same data winit would produce. Used to verify the human core loop (select,
/// move, …) actually works, with screenshots as ground truth.
pub fn capture_set_cursor(app: &mut App, position: Vec2) {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world) {
        window.set_cursor_position(Some(position));
    }
}

pub fn capture_mouse_button(app: &mut App, button: MouseButton, pressed: bool) {
    let window = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(world).next()
    };
    let Some(window) = window else {
        return;
    };
    app.world_mut().write_message(MouseButtonInput {
        button,
        state: if pressed {
            bevy::input::ButtonState::Pressed
        } else {
            bevy::input::ButtonState::Released
        },
        window,
    });
}

/// Projects a world position to the capture camera's screen space.
pub fn capture_world_to_screen(app: &mut App, world_pos: Vec3) -> Option<Vec2> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
    let (camera, transform) = cameras.iter(world).next()?;
    camera.world_to_viewport(transform, world_pos).ok()
}

/// Moves the RTS camera focus for deterministic capture input. This uses the
/// same camera resource as the real match; the normal camera system applies it
/// on the next update.
pub fn capture_focus_camera_on(app: &mut App, focus: Vec3) {
    let bounds = *app.world().resource::<MapBounds>();
    let mut camera = app.world_mut().resource_mut::<RtsCamera>();
    set_camera_focus_safely(&mut camera, focus, bounds);
    camera.pending_jump = true;
}

/// Zooms the capture camera all the way in (for close-up model inspection).
pub fn capture_zoom_camera_closest(app: &mut App) {
    let mut camera = app.world_mut().resource_mut::<RtsCamera>();
    camera.distance = CAMERA_MIN_DISTANCE;
    camera.pending_jump = true;
}

/// Hides the opening briefing so focused proof screenshots can show the world
/// objects being inspected.
pub fn capture_dismiss_match_briefing(app: &mut App) {
    if let Some(mut briefing) = app.world_mut().get_resource_mut::<MatchBriefingState>() {
        briefing.dismiss();
    }
}

/// Diagnostic: prints every distinct mesh-material base color under each resource
/// node, so we can see the real crystal-facet albedo to match for recoloring.
pub fn capture_dump_resource_materials(app: &mut App) {
    let (children_map, _aabb, _roots) = capture_world_geometry_maps(app);
    let world = app.world_mut();
    let nodes: Vec<(Entity, ResourceKind)> = {
        let mut q = world.query::<(Entity, &ResourceNode)>();
        q.iter(world).map(|(e, n)| (e, n.kind)).collect()
    };
    let mat_handles: std::collections::HashMap<Entity, Handle<StandardMaterial>> = {
        let mut q = world.query::<(Entity, &MeshMaterial3d<StandardMaterial>)>();
        q.iter(world).map(|(e, m)| (e, m.0.clone())).collect()
    };
    let materials = world.resource::<Assets<StandardMaterial>>();
    for (root, kind) in nodes.into_iter().take(2) {
        println!("[mat] {:?} node {root:?}", kind);
        let mut stack = children_map.get(&root).cloned().unwrap_or_default();
        while let Some(e) = stack.pop() {
            if let Some(ch) = children_map.get(&e) {
                stack.extend(ch.iter().copied());
            }
            if let Some(h) = mat_handles.get(&e) {
                if let Some(m) = materials.get(h) {
                    let s = m.base_color.to_srgba();
                    let l = m.base_color.to_linear();
                    println!(
                        "[mat]   srgb=({:.3},{:.3},{:.3}) linear=({:.3},{:.3},{:.3}) metallic={:.2}",
                        s.red, s.green, s.blue, l.red, l.green, l.blue, m.metallic
                    );
                }
            }
        }
    }
}

/// World position of a player unit that is currently on-screen (projects inside
/// the viewport, clear of the top/bottom HUD margins) so a synthetic click
/// actually lands on it. Prefers workers. Returns `None` if none are framed.
pub fn capture_player_onscreen_unit_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let (width, height) = {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
        let window = windows.iter(world).next()?;
        (window.width(), window.height())
    };
    let world = app.world_mut();
    let (camera, cam_transform) = {
        let mut cameras = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
        let (camera, transform) = cameras.iter(world).next()?;
        (camera.clone(), *transform)
    };
    let mut candidates = world.query_filtered::<(&Unit, &Team, &Transform), ()>();
    let mut fallback = None;
    for (unit, team, transform) in candidates.iter(world) {
        if *team != player {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(&cam_transform, transform.translation) else {
            continue;
        };
        // Keep clear of the top status strip (<76px) and the bottom command bar
        // (>height-148px) so the click is treated as a world click, not HUD.
        let on_screen = screen.x >= 8.0
            && screen.x <= width - 8.0
            && screen.y >= 80.0
            && screen.y <= height - 152.0;
        if !on_screen {
            continue;
        }
        let is_worker = registry::entity(unit.id)
            .map(|d| d.is_worker)
            .unwrap_or(false);
        if is_worker {
            return Some(transform.translation);
        }
        fallback.get_or_insert(transform.translation);
    }
    fallback
}

/// World position of a player Worker that is currently in a click-safe area.
pub fn capture_player_onscreen_worker_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let (width, height) = {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
        let window = windows.iter(world).next()?;
        (window.width(), window.height())
    };
    let world = app.world_mut();
    let (camera, cam_transform) = {
        let mut cameras = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
        let (camera, transform) = cameras.iter(world).next()?;
        (camera.clone(), *transform)
    };
    let screen_center = Vec2::new(width * 0.5, height * 0.5);
    let mut best: Option<(f32, Vec3)> = None;
    let mut candidates = world.query_filtered::<(&Unit, &Team, &Transform), ()>();
    for (unit, team, transform) in candidates.iter(world) {
        if *team != player || !registry::entity(unit.id).is_some_and(|d| d.is_worker) {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(&cam_transform, transform.translation) else {
            continue;
        };
        let on_screen = screen.x >= 8.0
            && screen.x <= width - 8.0
            && screen.y >= 80.0
            && screen.y <= height - 152.0;
        if !on_screen {
            continue;
        }
        let score = screen.distance_squared(screen_center);
        if best.is_none_or(|(best_score, _)| score < best_score) {
            best = Some((score, transform.translation));
        }
    }
    best.map(|(_, pos)| pos)
}

/// Number of currently-selected player units (programmatic check for selection).
pub fn capture_selected_player_unit_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Team, (With<Selected>, With<Unit>)>();
    q.iter(world)
        .filter(|team| **team == Team::Player(0))
        .count()
}

/// Combined world-space AABB center of an entity's visible mesh descendants.
pub(crate) fn entity_visual_world_center(
    root: Entity,
    children_map: &std::collections::HashMap<Entity, Vec<Entity>>,
    aabb_map: &std::collections::HashMap<Entity, (GlobalTransform, Aabb)>,
    model_roots: &std::collections::HashSet<Entity>,
) -> Option<Vec3> {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut found = false;
    // Measure ONLY the GLB (WorldAssetRoot) subtrees — the parts the recenter
    // aligns to the origin. The procedural faction banner sits forward of the
    // model; including it would misreport a correctly centered building as offset.
    let mut stack: Vec<Entity> = children_map
        .get(&root)
        .map(|c| {
            c.iter()
                .copied()
                .filter(|e| model_roots.contains(e))
                .collect()
        })
        .unwrap_or_default();
    while let Some(entity) = stack.pop() {
        if let Some(children) = children_map.get(&entity) {
            stack.extend(children.iter().copied());
        }
        if let Some((gt, aabb)) = aabb_map.get(&entity) {
            found = true;
            let center = Vec3::from(aabb.center);
            let half = Vec3::from(aabb.half_extents);
            for sx in [-1.0_f32, 1.0] {
                for sy in [-1.0_f32, 1.0] {
                    for sz in [-1.0_f32, 1.0] {
                        let world = gt.transform_point(
                            center + Vec3::new(sx * half.x, sy * half.y, sz * half.z),
                        );
                        min = min.min(world);
                        max = max.max(world);
                    }
                }
            }
        }
    }
    found.then(|| (min + max) * 0.5)
}

/// Builds child + AABB lookup maps over the whole world (for visual-center math).
pub(crate) fn capture_world_geometry_maps(
    app: &mut App,
) -> (
    std::collections::HashMap<Entity, Vec<Entity>>,
    std::collections::HashMap<Entity, (GlobalTransform, Aabb)>,
    std::collections::HashSet<Entity>,
) {
    let world = app.world_mut();
    let mut children_map = std::collections::HashMap::new();
    {
        let mut q = world.query::<(Entity, &Children)>();
        for (entity, children) in q.iter(world) {
            children_map.insert(entity, children.iter().collect::<Vec<_>>());
        }
    }
    let mut aabb_map = std::collections::HashMap::new();
    {
        let mut q = world.query::<(Entity, &GlobalTransform, &Aabb)>();
        for (entity, gt, aabb) in q.iter(world) {
            aabb_map.insert(entity, (*gt, *aabb));
        }
    }
    let mut model_roots = std::collections::HashSet::new();
    {
        let mut q = world.query_filtered::<Entity, With<WorldAssetRoot>>();
        for entity in q.iter(world) {
            model_roots.insert(entity);
        }
    }
    (children_map, aabb_map, model_roots)
}

/// Worst horizontal distance (meters) between any selectable entity's VISIBLE
/// model center and its `Transform.translation` — the point gizmos and every
/// cursor hit-test project. This is a NON-self-referential alignment check: it
/// would have caught the off-origin-GLB bug where clicks missed the model.
/// Returns `(offset_m, label)` for the worst entity, or `None` if no models
/// loaded yet.
pub fn capture_worst_model_alignment_offset(app: &mut App) -> Option<(f32, String)> {
    let (children_map, aabb_map, model_roots) = capture_world_geometry_maps(app);
    let world = app.world_mut();
    let mut roots = Vec::new();
    {
        // Only entities the recenter has FINISHED settling — checking the invariant
        // "everything we corrected is aligned". Excludes units mid-settle (e.g.
        // AI-spammed workers within their first few frames), which are transient.
        let mut q = world.query_filtered::<(Entity, &GlobalTransform, Option<&Name>), (With<Selectable>, With<ModelRecentered>)>();
        for (entity, gt, name) in q.iter(world) {
            roots.push((
                entity,
                gt.translation(),
                name.map(|n| n.as_str().to_string()).unwrap_or_default(),
            ));
        }
    }
    let mut worst: Option<(f32, String)> = None;
    for (root, translation, label) in roots {
        let Some(center) = entity_visual_world_center(root, &children_map, &aabb_map, &model_roots)
        else {
            continue;
        };
        let offset = Vec2::new(center.x - translation.x, center.z - translation.z).length();
        if worst.as_ref().map_or(true, |(w, _)| offset > *w) {
            worst = Some((offset, label));
        }
    }
    worst
}

/// World-space visible-model center of an on-screen, non-empty resource node, with
/// its entity — for clicking the model where it is actually DRAWN (not its origin).
pub fn capture_onscreen_resource_model_center(app: &mut App) -> Option<(Entity, Vec3)> {
    let (children_map, aabb_map, model_roots) = capture_world_geometry_maps(app);
    let (width, height) = {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
        let window = windows.iter(world).next()?;
        (window.width(), window.height())
    };
    let world = app.world_mut();
    let (camera, cam_transform) = {
        let mut cameras = world.query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
        let (camera, transform) = cameras.iter(world).next()?;
        (camera.clone(), *transform)
    };
    let mut resources = world.query::<(Entity, &ResourceNode)>();
    let candidates: Vec<Entity> = resources
        .iter(world)
        .filter_map(|(entity, resource)| (resource.amount > 0).then_some(entity))
        .collect();
    for entity in candidates {
        let Some(center) =
            entity_visual_world_center(entity, &children_map, &aabb_map, &model_roots)
        else {
            continue;
        };
        if let Ok(screen) = camera.world_to_viewport(&cam_transform, center) {
            if screen.x >= 8.0
                && screen.x <= width - 8.0
                && screen.y >= 80.0
                && screen.y <= height - 152.0
            {
                return Some((entity, center));
            }
        }
    }
    None
}

/// The player's command-center entity, its origin (where selection brackets are
/// drawn) and its GLB visual center (where the building is actually drawn). Used
/// by the `base` capture to confirm the brackets overlay the building.
/// Spawns a row of same-type structures frozen at increasing construction
/// progress next to the player base, so a capture can show the emergence
/// animation stages side by side.
pub fn capture_spawn_construction_showcase(app: &mut App, origin: Vec3) {
    let world = app.world_mut();
    let team = Team::Player(0);
    let mut spawned = Vec::new();
    {
        let asset_server = world.resource::<AssetServer>().clone();
        let mut next_id = std::mem::take(world.resource_mut::<NextSpawnId>().as_mut());
        let mut commands = world.commands();
        for (index, progress) in [0.1f32, 0.45, 0.8].into_iter().enumerate() {
            let position = origin + Vec3::new(-7.0 + index as f32 * 7.0, 0.0, 13.0);
            let entity = spawn_structure_under_construction_with_visual_faction(
                &mut commands,
                &asset_server,
                &mut next_id,
                "Barracks",
                team,
                position,
                None,
                0.0,
                team,
                Some(SkirmishFaction::from_team(team)),
            );
            spawned.push((entity, progress));
        }
        *world.resource_mut::<NextSpawnId>() = next_id;
    }
    world.flush();
    for (entity, progress) in spawned {
        if let Some(mut under) = world.entity_mut(entity).get_mut::<UnderConstruction>() {
            let total = 10.0;
            under.total = total;
            under.remaining = total * (1.0 - progress);
        }
    }
}

/// Deterministically stages the worker limb animation: teleports a player
/// worker next to the nearest resource node and puts it straight into the
/// Collecting state, so a capture can frame the mining pose without racing
/// the real harvest loop.
pub fn capture_stage_worker_collecting(app: &mut App) -> Option<Vec3> {
    let world = app.world_mut();
    let node = {
        let mut nodes = world.query::<(Entity, &ResourceNode, &Transform)>();
        nodes
            .iter(world)
            .map(|(entity, _, transform)| (entity, transform.translation))
            .next()?
    };
    let worker = {
        let mut workers = world.query::<(Entity, &Team, &Unit)>();
        workers
            .iter(world)
            .find(|(_, team, unit)| **team == Team::Player(0) && unit.id == "Worker")
            .map(|(entity, _, _)| entity)?
    };
    let spot = node.1 + Vec3::new(0.9, 0.0, 0.9);
    world.entity_mut(worker).get_mut::<Transform>()?.translation = spot;
    world.entity_mut(worker).insert(HarvestOrder {
        resource: Some(node.0),
        state: HarvestState::Collecting,
        collect_remaining: 30.0,
        last_kind: None,
    });
    world.entity_mut(worker).remove::<MoveOrder>();
    Some(spot)
}

pub fn capture_player_command_center(app: &mut App) -> Option<(Entity, Vec3, Vec3)> {
    let (children_map, aabb_map, model_roots) = capture_world_geometry_maps(app);
    let player = Team::Player(0);
    let world = app.world_mut();
    let entity = {
        let mut q = world.query_filtered::<(Entity, &Team, &Structure), ()>();
        q.iter(world)
            .find(|(_, team, structure)| **team == player && structure.id == "CommandCenter")
            .map(|(entity, _, _)| entity)?
    };
    let origin = world.get::<Transform>(entity)?.translation;
    let center = entity_visual_world_center(entity, &children_map, &aabb_map, &model_roots)?;
    Some((entity, origin, center))
}

/// Whether the given entity currently has the `Selected` component (capture check).
pub fn capture_entity_is_selected(app: &mut App, entity: Entity) -> bool {
    app.world().get::<Selected>(entity).is_some()
}

/// Sets a rally point on the given structure (capture aid for the rally-flag visual).
pub fn capture_set_structure_rally(app: &mut App, structure: Entity, target: Vec3) {
    if let Some(mut rally) = app.world_mut().get_mut::<RallyPoint>(structure) {
        rally.target = Some(target);
        rally.target_unit = None;
        rally.mode = RallyMode::Move;
    }
}

/// IDs of currently-selected player units, for capture diagnostics.
pub fn capture_selected_player_unit_ids(app: &mut App) -> Vec<&'static str> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Unit, &Team), With<Selected>>();
    q.iter(world)
        .filter_map(|(unit, team)| (*team == Team::Player(0)).then_some(unit.id))
        .collect()
}

/// Average world position of the currently selected player units.
pub fn capture_selected_player_unit_average_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Team, &Transform), (With<Unit>, With<Selected>)>();
    let mut sum = Vec3::ZERO;
    let mut count = 0usize;
    for (team, transform) in q.iter(world) {
        if *team != player {
            continue;
        }
        sum += transform.translation;
        count += 1;
    }
    (count > 0).then_some(sum / count as f32)
}

/// Nearest enemy structure position for capture/demo movement targets.
pub fn capture_enemy_structure_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let mut enemy = None;
    let mut q = world.query_filtered::<(&Team, &Transform), With<Structure>>();
    for (team, transform) in q.iter(world) {
        if *team != player && *team != Team::Neutral {
            enemy = Some(transform.translation);
            break;
        }
    }
    enemy
}

/// Nearest living enemy elimination-anchor position for player assault captures.
pub fn capture_nearest_enemy_anchor_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let origin = {
        let mut units = world.query_filtered::<(&Team, &Transform), (With<Unit>, With<Selected>)>();
        let mut sum = Vec3::ZERO;
        let mut count = 0usize;
        for (team, transform) in units.iter(world) {
            if *team == player {
                sum += transform.translation;
                count += 1;
            }
        }
        if count > 0 {
            sum / count as f32
        } else {
            Vec3::ZERO
        }
    };

    let mut best: Option<(f32, Vec3)> = None;
    {
        let mut structures =
            world.query_filtered::<(&Structure, &Team, &Transform, &Health), With<Structure>>();
        for (structure, team, transform, health) in structures.iter(world) {
            if *team == player
                || *team == Team::Neutral
                || health.current <= 0.0
                || !is_structure_elimination_anchor(structure)
            {
                continue;
            }
            let distance = xz_distance(origin, transform.translation);
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, transform.translation));
            }
        }
    }
    {
        let mut units = world.query_filtered::<(&Unit, &Team, &Transform, &Health), With<Unit>>();
        for (unit, team, transform, health) in units.iter(world) {
            if *team == player
                || *team == Team::Neutral
                || health.current <= 0.0
                || !is_worker_elimination_anchor(unit)
            {
                continue;
            }
            let distance = xz_distance(origin, transform.translation);
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, transform.translation));
            }
        }
    }
    best.map(|(_, position)| position)
}

/// Emits a real keyboard message (the same data winit produces) so command
/// hotkeys (`command_shortcuts`) fire headlessly.
pub fn capture_key(app: &mut App, key: KeyCode, pressed: bool) {
    let window = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(world).next()
    };
    let Some(window) = window else {
        return;
    };
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: key,
            logical_key: bevy::input::keyboard::Key::Dead(None),
            state: if pressed {
                bevy::input::ButtonState::Pressed
            } else {
                bevy::input::ButtonState::Released
            },
            text: None,
            repeat: false,
            window,
        });
}

/// World position of a player production structure, if any.
pub fn capture_player_producer_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Structure, &Team, &Transform), ()>();
    for (structure, team, transform) in q.iter(world) {
        if *team == player
            && matches!(
                structure.id,
                "CommandCenter" | "Barracks" | "VehicleFactory" | "AircraftFactory"
            )
        {
            return Some(transform.translation);
        }
    }
    None
}

/// Hotkey of the first enabled "train a unit" command-panel slot for the current
/// selection, if the panel currently offers one.
pub fn capture_first_enabled_train_hotkey(app: &mut App) -> Option<KeyCode> {
    let world = app.world_mut();
    let mut q = world.query::<(&CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    for (slot, action, availability) in q.iter(world) {
        if matches!(action, BuildAction::Train(_)) && availability.enabled {
            return COMMAND_SLOT_HOTKEYS
                .get(slot.0)
                .map(|hotkey| hotkey.key_code);
        }
    }
    None
}

/// Hotkey of a specific enabled "train this unit" command-panel slot.
pub fn capture_enabled_train_hotkey_for(
    app: &mut App,
    product_id: &'static str,
) -> Option<KeyCode> {
    let world = app.world_mut();
    let mut q = world.query::<(&CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    for (slot, action, availability) in q.iter(world) {
        if matches!(action, BuildAction::Train(id) if *id == product_id) && availability.enabled {
            return COMMAND_SLOT_HOTKEYS
                .get(slot.0)
                .map(|hotkey| hotkey.key_code);
        }
    }
    None
}

/// Number of queued production jobs for the player (grows when a train/build
/// command is accepted).
pub fn capture_player_build_queue_len(app: &mut App) -> usize {
    app.world()
        .resource::<BuildQueue>()
        .0
        .iter()
        .filter(|job| job.team == Team::Player(0))
        .count()
}

/// Total player units (grows once queued production completes).
pub fn capture_player_unit_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Team, With<Unit>>();
    q.iter(world)
        .filter(|team| **team == Team::Player(0))
        .count()
}

/// Player resource totals for capture smoke tests.
pub fn capture_player_resources(app: &mut App) -> (i32, i32) {
    let economy = app.world().resource::<Economies>().get(Team::Player(0));
    (economy.ore, economy.crystal)
}

/// World position of a player worker (its command panel offers Build actions).
pub fn capture_player_worker_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Unit, &Team, &Transform), ()>();
    for (unit, team, transform) in q.iter(world) {
        let is_worker = registry::entity(unit.id)
            .map(|d| d.is_worker)
            .unwrap_or(false);
        if *team == player && is_worker {
            return Some(transform.translation);
        }
    }
    None
}

/// Hotkey of the first enabled "build a structure" command-panel slot.
pub fn capture_first_enabled_build_hotkey(app: &mut App) -> Option<KeyCode> {
    let world = app.world_mut();
    let mut q = world.query::<(&CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    for (slot, action, availability) in q.iter(world) {
        if matches!(action, BuildAction::Build(_)) && availability.enabled {
            return COMMAND_SLOT_HOTKEYS
                .get(slot.0)
                .map(|hotkey| hotkey.key_code);
        }
    }
    None
}

/// Hotkey of a specific enabled "build this structure" command-panel slot.
pub fn capture_enabled_build_hotkey_for(
    app: &mut App,
    structure_id: &'static str,
) -> Option<KeyCode> {
    let world = app.world_mut();
    let mut q = world.query::<(&CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    for (slot, action, availability) in q.iter(world) {
        if matches!(action, BuildAction::Build(id) if *id == structure_id) && availability.enabled {
            return COMMAND_SLOT_HOTKEYS
                .get(slot.0)
                .map(|hotkey| hotkey.key_code);
        }
    }
    None
}

/// Hotkey of the enabled Attack-Move command-panel slot for the current
/// selection, if the selected army can issue one.
pub fn capture_first_enabled_attack_move_hotkey(app: &mut App) -> Option<KeyCode> {
    let world = app.world_mut();
    let mut q = world.query::<(&CommandSlot, &BuildAction, &CommandSlotAvailability)>();
    for (slot, action, availability) in q.iter(world) {
        if matches!(action, BuildAction::AttackMove) && availability.enabled {
            return COMMAND_SLOT_HOTKEYS
                .get(slot.0)
                .map(|hotkey| hotkey.key_code);
        }
    }
    None
}

/// (enabled, total) Build options currently on the command panel — diagnostic
/// for whether the worker construction menu is showing at all.
pub fn capture_build_options_count(app: &mut App) -> (usize, usize) {
    let world = app.world_mut();
    let mut q = world.query::<(&BuildAction, &CommandSlotAvailability)>();
    let mut enabled = 0;
    let mut total = 0;
    for (action, availability) in q.iter(world) {
        if matches!(action, BuildAction::Build(_)) {
            total += 1;
            if availability.enabled {
                enabled += 1;
            }
        }
    }
    (enabled, total)
}

/// Count of player structures (grows when a building is placed/constructed).
pub fn capture_player_structure_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Team, With<Structure>>();
    q.iter(world)
        .filter(|team| **team == Team::Player(0))
        .count()
}

/// Count of completed player structures. Unlike `capture_player_structure_count`,
/// this excludes placed foundations that still need worker construction.
pub fn capture_player_completed_structure_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Team, Option<&UnderConstruction>), With<Structure>>();
    q.iter(world)
        .filter(|(team, under_construction)| {
            **team == Team::Player(0) && under_construction.is_none()
        })
        .count()
}

/// Position of a completed player structure by id.
pub fn capture_player_completed_structure_position(
    app: &mut App,
    structure_id: &'static str,
) -> Option<Vec3> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<
        (&Structure, &Team, &Transform, Option<&UnderConstruction>),
        With<Structure>,
    >();
    for (structure, team, transform, under_construction) in q.iter(world) {
        if *team == Team::Player(0) && structure.id == structure_id && under_construction.is_none()
        {
            return Some(transform.translation);
        }
    }
    None
}

/// Whether the player currently has a pending structure placement (build mode).
pub fn capture_player_in_placement_mode(app: &mut App) -> bool {
    app.world()
        .resource::<CommandMode>()
        .pending_structure_placement
        .is_some()
}

/// Whether the current placement-mode cursor target is a valid build spot.
pub fn capture_placement_is_valid(app: &mut App) -> bool {
    matches!(
        app.world()
            .resource::<StructurePlacementFeedback>()
            .validity,
        Some(StructurePlacementValidity::Valid)
    )
}

/// World position of the nearest visible resource node to any player unit, for
/// verifying manual (right-click) harvesting.
pub fn capture_nearest_visible_resource_position(app: &mut App) -> Option<Vec3> {
    let player = Team::Player(0);
    let world = app.world_mut();
    let anchor = {
        let mut q = world.query_filtered::<(&Team, &Transform), With<Unit>>();
        q.iter(world)
            .find(|(t, _)| **t == player)
            .map(|(_, tf)| tf.translation)
    }?;
    let mut q = world.query::<(&Transform, &VisibilityState, &ResourceNode)>();
    let mut best: Option<(f32, Vec3)> = None;
    for (tf, vis, node) in q.iter(world) {
        if !vis.visible || node.amount <= 0 {
            continue;
        }
        let d = xz_distance(anchor, tf.translation);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, tf.translation));
        }
    }
    best.map(|(_, pos)| pos)
}

/// Click position on the nearest visible resource model to `anchor`.
///
/// The returned point is above the resource ground anchor, matching the visible
/// model body. Projecting/clicking the ground anchor can miss on an angled RTS
/// camera because the crystal/ore body appears above that point on screen.
pub fn capture_nearest_visible_resource_click_position_to(
    app: &mut App,
    anchor: Vec3,
) -> Option<Vec3> {
    let world = app.world_mut();
    let mut q = world.query::<(&Transform, &VisibilityState, &ResourceNode)>();
    let mut best: Option<(f32, Vec3)> = None;
    for (tf, vis, node) in q.iter(world) {
        if !vis.visible || node.amount <= 0 {
            continue;
        }
        let d = xz_distance(anchor, tf.translation);
        let click_pos = tf.translation + Vec3::Y * resource_visual_height(node.kind) * 0.55;
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, click_pos));
        }
    }
    best.map(|(_, pos)| pos)
}

/// Number of player units currently carrying a harvest order (proves a manual
/// harvest command took effect).
pub fn capture_player_harvesting_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Team, (With<Unit>, With<HarvestOrder>)>();
    q.iter(world).filter(|t| **t == Team::Player(0)).count()
}

/// Number of player units currently assigned to construct a placed foundation.
pub fn capture_player_constructing_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Team, (With<Unit>, With<ConstructOrder>)>();
    q.iter(world).filter(|t| **t == Team::Player(0)).count()
}

/// Number of player units that currently carry a live combat order.
pub fn capture_player_combat_order_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world
        .query_filtered::<(&Team, Option<&AttackMoveOrder>, Option<&AttackOrder>), With<Unit>>();
    q.iter(world)
        .filter(|(team, attack_move, attack)| {
            **team == Team::Player(0) && (attack_move.is_some() || attack.is_some())
        })
        .count()
}

/// Count of player-owned combat-capable mobile units.
pub fn capture_player_army_unit_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Team, &Unit), With<Unit>>();
    q.iter(world)
        .filter(|(team, unit)| **team == Team::Player(0) && unit_supports_attack_move(unit))
        .count()
}

/// Current match phase label for capture smoke tests.
pub fn capture_match_phase_label(app: &mut App) -> &'static str {
    match_phase_label(app.world().resource::<MatchState>().phase)
}

/// Runs the default AI-vs-AI skirmish using the headless simulation path and
/// returns the first non-running match phase. Used by capture/CI to prove that
/// the economy -> production -> combat loop can finish a match.
pub fn capture_run_ai_match_until_resolved(max_seconds: u32) -> Option<(u32, &'static str)> {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    app.world_mut()
        .insert_resource(VisiblePlayer::all_players(Team::Player(0)));

    let steps = max_seconds.div_ceil(5).max(1);
    for step in 1..=steps {
        for _ in 0..150 {
            app.update();
        }
        let phase = app.world().resource::<MatchState>().phase;
        if !matches!(phase, MatchPhase::Running) {
            return Some((step * 5, match_phase_label(phase)));
        }
    }
    None
}

pub(crate) fn match_phase_label(phase: MatchPhase) -> &'static str {
    match phase {
        MatchPhase::Running => "Running",
        MatchPhase::HumanDefeat => "HumanDefeat",
        MatchPhase::HumanVictory => "HumanVictory",
        MatchPhase::MatchFinished => "MatchFinished",
    }
}

/// Sets every player slot's faction (0=苍穹联盟/Alliance, 1=炽炎魔军/Demon, 2=混沌裂隙/Chaos)
/// before the match scene reads `MatchSetupSettings`, so a capture can show each
/// faction's own base/units. Returns the faction label.
pub fn capture_set_all_factions(app: &mut App, index: usize) -> &'static str {
    let faction = SkirmishFaction::ALL[index % SkirmishFaction::ALL.len()];
    let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
    for slot in settings.player_factions.iter_mut() {
        *slot = faction;
    }
    faction.label()
}

/// Points the live match's `MainCamera` at the offscreen capture target.
pub(crate) fn retarget_capture_camera(
    mut commands: Commands,
    target: Res<CaptureTarget>,
    cameras: Query<Entity, (With<Camera>, Without<CaptureCameraReady>)>,
) {
    // Retarget every camera (the 3D scene camera AND the 2D UI/menu camera) to
    // the offscreen image, so captures include the menu and the in-match HUD.
    // `IsDefaultUiCamera` is required: once a camera's `RenderTarget` is an image
    // (not the primary window), Bevy's `DefaultUiCamera` fallback no longer picks
    // it, so without this marker the UI/HUD would silently drop out of captures.
    for entity in &cameras {
        commands.entity(entity).insert((
            RenderTarget::Image(target.0.clone().into()),
            bevy::ui::IsDefaultUiCamera,
            CaptureCameraReady,
        ));
    }
}

/// Parses a CLI difficulty name (beginner|easy|normal|hard).
pub(crate) fn capture_parse_ai_difficulty(name: &str) -> Result<AiDifficulty, String> {
    match name {
        "beginner" => Ok(AiDifficulty::Beginner),
        "easy" => Ok(AiDifficulty::Easy),
        "normal" => Ok(AiDifficulty::Normal),
        "hard" => Ok(AiDifficulty::Hard),
        other => Err(format!(
            "unknown difficulty '{other}' (use beginner|easy|normal|hard)"
        )),
    }
}

/// Final per-team survivors of a duel: (team index, units, structures).
pub type AiDuelCounts = Vec<(usize, usize, usize)>;

/// Runs a headless AI-vs-AI duel — Player0 uses `difficulty_a`, Player1 uses
/// `difficulty_b` on the default 1v1 setup — until the match resolves or
/// `max_seconds` of simulated time pass. Returns the elapsed simulated seconds,
/// the final phase label (Victory = Player0 won, Defeat = Player1 won, from
/// Player0's perspective) and the surviving unit/structure counts per team.
/// Powers AI-difficulty balance regression checks.
pub fn capture_run_ai_duel(
    difficulty_a: &str,
    difficulty_b: &str,
    max_seconds: u32,
) -> Result<(u32, &'static str, AiDuelCounts), String> {
    let difficulty_a = capture_parse_ai_difficulty(difficulty_a)?;
    let difficulty_b = capture_parse_ai_difficulty(difficulty_b)?;
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        if settings.player_controllers.len() < 2 {
            settings
                .player_controllers
                .resize(2, SkirmishPlayerController::None);
        }
        settings.player_controllers[0] = SkirmishPlayerController::Ai(difficulty_a);
        settings.player_controllers[1] = SkirmishPlayerController::Ai(difficulty_b);
        settings
            .ai_difficulties
            .set_difficulty(Team::Player(0), difficulty_a);
        settings
            .ai_difficulties
            .set_difficulty(Team::Player(1), difficulty_b);
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }
    app.world_mut()
        .insert_resource(VisiblePlayer::all_players(Team::Player(0)));

    let mut elapsed = max_seconds;
    let mut label = "Running";
    let steps = max_seconds.div_ceil(5).max(1);
    for step in 1..=steps {
        for _ in 0..150 {
            app.update();
        }
        let phase = app.world().resource::<MatchState>().phase;
        if !matches!(phase, MatchPhase::Running) {
            elapsed = step * 5;
            label = match_phase_label(phase);
            break;
        }
    }

    let world = app.world_mut();
    let mut counts: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    // Both duelists always appear, even when fully eliminated (0/0).
    counts.insert(0, (0, 0));
    counts.insert(1, (0, 0));
    let mut units = world.query_filtered::<(&Team, &Health), With<Unit>>();
    for (team, health) in units.iter(world) {
        if let Team::Player(index) = team
            && health.current > 0.0
        {
            counts.entry(*index).or_default().0 += 1;
        }
    }
    let mut structures = world.query_filtered::<(&Team, &Health), With<Structure>>();
    for (team, health) in structures.iter(world) {
        if let Team::Player(index) = team
            && health.current > 0.0
        {
            counts.entry(*index).or_default().1 += 1;
        }
    }
    let counts = counts
        .into_iter()
        .map(|(team, (units, structures))| (team, units, structures))
        .collect();
    Ok((elapsed, label, counts))
}

/// Outcome of a unit-vs-unit arena bout: (elapsed sim seconds, survivors A,
/// survivors B, remaining total HP A, remaining HP B).
pub type ArenaOutcome = (u32, usize, usize, f32, f32);

/// Headless N-vs-N unit arena for balance auditing: clears the default match
/// world down to the two command centers (kept so the match stays Running),
/// spawns `count` of each unit facing off mid-map, attack-moves them into each
/// other and reports who is left. Both slots use Human controllers so no AI
/// director interferes.
pub fn capture_run_arena(
    unit_a: &str,
    unit_b: &str,
    count: usize,
    max_seconds: u32,
) -> Result<ArenaOutcome, String> {
    let def_a = registry::entity(unit_a)
        .filter(|def| def.role == registry::EntityRole::Unit)
        .ok_or_else(|| format!("unknown unit '{unit_a}'"))?;
    let def_b = registry::entity(unit_b)
        .filter(|def| def.role == registry::EntityRole::Unit)
        .ok_or_else(|| format!("unknown unit '{unit_b}'"))?;

    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut settings = app.world_mut().resource_mut::<MatchSetupSettings>();
        if settings.player_controllers.len() < 2 {
            settings
                .player_controllers
                .resize(2, SkirmishPlayerController::None);
        }
        settings.player_controllers[0] = SkirmishPlayerController::Human;
        settings.player_controllers[1] = SkirmishPlayerController::Human;
        // Identical factions on both sides: faction damage/armor multipliers
        // would otherwise bias every bout.
        if settings.player_factions.len() >= 2 {
            settings.player_factions[1] = settings.player_factions[0];
        }
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..20 {
        app.update();
    }

    // Clear the default world down to the elimination anchors.
    let world = app.world_mut();
    let doomed: Vec<Entity> = world
        .query::<(Entity, Option<&Unit>, Option<&Structure>)>()
        .iter(world)
        .filter_map(|(entity, unit, structure)| {
            if unit.is_some() {
                Some(entity)
            } else if structure.is_some_and(|structure| structure.id != "CommandCenter") {
                Some(entity)
            } else {
                None
            }
        })
        .collect();
    for entity in doomed {
        world.entity_mut(entity).despawn();
    }
    // The non-controlled side is AI-directed regardless of its controller
    // (active_ai_teams keys off the visible team); an empty economy keeps its
    // command center from training reinforcements into the bout.
    for economy in &mut world.resource_mut::<Economies>().players {
        economy.ore = 0;
        economy.crystal = 0;
    }

    // Face-off lines around the map centre.
    let asset_server = world.resource::<AssetServer>().clone();
    let mut next_id = NextSpawnId(world.resource::<NextSpawnId>().0);
    let spacing = 1.2f32;
    let mut spawned: Vec<(Entity, Team, Vec3)> = Vec::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        for index in 0..count {
            let offset = (index as f32 - (count as f32 - 1.0) * 0.5) * spacing;
            let position_a = Vec3::new(-6.0, 0.0, offset);
            let position_b = Vec3::new(6.0, 0.0, offset);
            let entity_a = spawn_unit(
                &mut commands,
                &asset_server,
                &mut next_id,
                def_a.id,
                Team::Player(0),
                position_a,
                0,
                Team::Player(0),
            );
            let entity_b = spawn_unit(
                &mut commands,
                &asset_server,
                &mut next_id,
                def_b.id,
                Team::Player(1),
                position_b,
                0,
                Team::Player(0),
            );
            spawned.push((entity_a, Team::Player(0), position_b));
            spawned.push((entity_b, Team::Player(1), position_a));
        }
    }
    queue.apply(world);
    world.resource_mut::<NextSpawnId>().0 = next_id.0;
    for (entity, _, toward) in &spawned {
        world.entity_mut(*entity).insert(AttackMoveOrder {
            destination: *toward,
        });
    }

    let survivors = |world: &mut World| -> (usize, usize, f32, f32) {
        let mut a = (0usize, 0.0f32);
        let mut b = (0usize, 0.0f32);
        let mut q = world.query_filtered::<(&Team, &Health), With<Unit>>();
        for (team, health) in q.iter(world) {
            if health.current <= 0.0 {
                continue;
            }
            match team {
                Team::Player(0) => {
                    a.0 += 1;
                    a.1 += health.current;
                }
                Team::Player(1) => {
                    b.0 += 1;
                    b.1 += health.current;
                }
                _ => {}
            }
        }
        (a.0, b.0, a.1, b.1)
    };

    let steps = (max_seconds * 30).max(30);
    let mut elapsed = max_seconds;
    for step in 1..=steps {
        app.update();
        // Re-stick the bout orders once a second: the enemy-side AI director
        // keeps trying to recall its units into a rally wave, which would turn
        // the fight into a one-sided rout.
        if step % 30 == 0 {
            let world = app.world_mut();
            for (entity, _, toward) in &spawned {
                let alive = world
                    .get::<Health>(*entity)
                    .is_some_and(|health| health.current > 0.0);
                if alive {
                    world.entity_mut(*entity).insert(AttackMoveOrder {
                        destination: *toward,
                    });
                }
            }
        }
        if step % 30 == 0 {
            let (alive_a, alive_b, _, _) = survivors(app.world_mut());
            if alive_a == 0 || alive_b == 0 {
                elapsed = step / 30;
                break;
            }
        }
    }
    if std::env::var_os("RTS_ARENA_DIAG").is_some() {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(&Team, &Unit, &Health), ()>();
        for (team, unit, health) in q.iter(world) {
            if health.current > 0.0 {
                eprintln!(
                    "[arena-diag] {:?} {} hp={:.1}",
                    team, unit.id, health.current
                );
            }
        }
    }
    let (alive_a, alive_b, hp_a, hp_b) = survivors(app.world_mut());
    Ok((elapsed, alive_a, alive_b, hp_a, hp_b))
}

/// Selects a skirmish map by id for the NEXT match started in this app (used by
/// `capture map` to screenshot map layouts).
pub fn capture_select_map(app: &mut App, map_id: &str) -> Result<(), String> {
    let map = SKIRMISH_MAPS
        .iter()
        .find(|map| map.id == map_id)
        .ok_or_else(|| {
            format!(
                "unknown map '{map_id}' (known: {})",
                SKIRMISH_MAPS
                    .iter()
                    .map(|map| map.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    app.world_mut()
        .resource_mut::<MatchSetupSettings>()
        .map_path = map.godot_path;
    app.world_mut()
        .resource_mut::<SelectedSkirmishMap>()
        .godot_path = map.godot_path;
    Ok(())
}

/// Frames the whole map: spectator visibility (no fog), camera over the origin
/// raised far beyond the gameplay zoom cap so the full layout fits.
pub fn capture_frame_whole_map(app: &mut App) {
    app.world_mut()
        .insert_resource(VisiblePlayer::all_players(Team::Player(0)));
    let map_extent = {
        let selected = app.world().resource::<SelectedSkirmishMap>();
        let map = selected.definition();
        map.size.0.max(map.size.1)
    };
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&mut RtsCam, With<MainCamera>>();
    for mut camera in cameras.iter_mut(world) {
        camera.height_max = map_extent * 1.05;
        camera.height_min = map_extent * 1.05;
        camera.target_focus.translation = Vec3::ZERO;
        camera.target_zoom = 0.0;
        camera.snap = true;
    }
}
