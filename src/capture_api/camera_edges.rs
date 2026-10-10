//! Read-only native camera/input QA observations. No ordinary app registration.
use super::*;

pub fn capture_camera_menu_button(app: &mut App, action: &str) -> Option<Vec2> {
    let world = app.world_mut();
    if action == "options" {
        return world
            .query::<(&FrontMenuButton, &UiGlobalTransform)>()
            .iter(world)
            .find(|(b, _)| b.action == FrontMenuAction::Options)
            .map(|(_, t)| t.translation);
    }
    let requested = match action {
        "tilt-up" => OptionsMenuAction::CameraTiltUp,
        "tilt-down" => OptionsMenuAction::CameraTiltDown,
        "back" => OptionsMenuAction::Back,
        _ => return None,
    };
    world
        .query::<(&OptionsMenuButton, &UiGlobalTransform)>()
        .iter(world)
        .find(|(button, _)| button.action == requested)
        .map(|(_, t)| t.translation)
}

pub fn capture_camera_minimap_point(app: &mut App, normalized: Vec2) -> Option<Vec2> {
    let world = app.world_mut();
    let bounds = *world.resource::<MapBounds>();
    let rect = bounds.minimap_content_rect();
    let window = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .ok()?;
    Some(
        minimap_screen_min(window)
            + Vec2::new(
                rect.left + rect.width * normalized.x,
                rect.top + rect.height * normalized.y,
            ),
    )
}

pub fn capture_camera_edge_observation(app: &mut App) -> serde_json::Value {
    let phase = capture_match_phase_label(app);
    let world = app.world_mut();
    let bounds = *world.resource::<MapBounds>();
    let state = world.resource::<RtsCamera>();
    let focus = state.focus.to_array();
    let distance = state.distance;
    let options = world.resource::<MenuOptionsState>();
    let settings = serde_json::json!({"tilt":options.camera_tilt,"edge_pan":options.camera_edge_pan,"pan_speed":options.camera_pan_speed});
    let rendered=world.query_filtered::<(&Camera,&GlobalTransform),With<MainCamera>>().iter(world).next().map(|(camera,transform)| {
        let (_,rotation,position)=transform.to_scale_rotation_translation();
        let corners=camera.logical_viewport_size().map(|size| [Vec2::ZERO,Vec2::new(size.x,0.0),size,Vec2::new(0.0,size.y)].map(|screen| {
            let point=camera.viewport_to_world(transform,screen).ok().and_then(|r|r.plane_intersection_point(Vec3::ZERO,InfinitePlane3d::new(Vec3::Y)));
            serde_json::json!({"screen":screen.to_array(),"flat_ground":point.map(|p|p.to_array()),"inside_map":point.map(|p|p.x.abs()<=bounds.half_width && p.z.abs()<=bounds.half_depth)})
        }));
        serde_json::json!({"position":position.to_array(),"rotation":rotation.to_array(),"flat_ground_viewport_corners":corners})
    });
    let radar = world
        .query_filtered::<&Text, With<MinimapStatusText>>()
        .iter(world)
        .next()
        .map(|t| t.0.clone());
    let radar_online = world
        .query_filtered::<&Visibility, With<MinimapTerrainImage>>()
        .iter(world)
        .any(|v| *v != Visibility::Hidden);
    let selected: Vec<_> = world
        .query::<(Entity, &Selectable, Option<&Selected>)>()
        .iter(world)
        .filter(|(_, _, s)| s.is_some())
        .map(|(e, _, _)| e.to_bits())
        .collect();
    let fog=world.get_resource::<FogOverlay>().map(|fog| {
        let opaque=world.get_resource::<Assets<Image>>().and_then(|images|images.get(&fog.handle)).and_then(|image|image.data.as_ref()).map(|data|data.chunks_exact(4).filter(|pixel|pixel[3]==255).count());
        serde_json::json!({"cells":fog.explored.len(),"explored":fog.explored.iter().filter(|v|**v).count(),"opaque_cells":opaque})
    });
    let outskirts:Vec<_>=world.query::<(&Name,&Transform)>().iter(world).filter(|(name,_)|name.as_str().starts_with("Battlefield ")).map(|(name,t)|serde_json::json!({"name":name.as_str(),"position":t.translation.to_array()})).collect();
    serde_json::json!({"clock":world.resource::<MatchState>().start_time_sec,"phase":phase,"focus":focus,"distance":distance,"settings":settings,"rendered":rendered,"map_half_extent":[bounds.half_width,bounds.half_depth],"radar":radar,"radar_online":radar_online,"selected":selected,"fog":fog,"outskirts":outskirts})
}

pub fn capture_camera_mouse_wheel(app: &mut App, lines: f32) {
    let world = app.world_mut();
    let Some(window) = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()
    else {
        return;
    };
    world.write_message(bevy::input::mouse::MouseWheel {
        unit: bevy::input::mouse::MouseScrollUnit::Line,
        phase: bevy::input::touch::TouchPhase::Moved,
        x: 0.0,
        y: lines,
        window,
    });
}

pub fn capture_camera_menu_observation(app: &mut App) -> serde_json::Value {
    let world = app.world_mut();
    let screen = format!("{:?}", world.resource::<State<AppScreen>>().get());
    let tilt = world.resource::<MenuOptionsState>().camera_tilt;
    let cursor = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .and_then(|w| w.cursor_position())
        .map(|p| p.to_array());
    let buttons: Vec<_> = world.query_filtered::<(&UiGlobalTransform,&ComputedNode,&Interaction,Option<&FrontMenuButton>,Option<&OptionsMenuButton>,Option<&InheritedVisibility>),With<Button>>()
        .iter(world).filter(|(_,_,_,front,options,_)| front.is_some() || options.is_some()).map(|(transform,node,interaction,front,options,visible)|serde_json::json!({
            "front_action":front.map(|b|format!("{:?}",b.action)),"options_action":options.map(|button|format!("{:?}",button.action)),"center":transform.translation.to_array(),"size":node.size().to_array(),"interaction":format!("{interaction:?}"),"visible":visible.map(|v|v.get())
        })).collect();
    let cameras: Vec<_> = world.query::<(Entity,&Camera,&RenderTarget)>().iter(world).map(|(entity,camera,target)|serde_json::json!({"entity":entity.to_bits(),"active":camera.is_active,"target":format!("{target:?}"),"viewport":camera.logical_viewport_size().map(|v|v.to_array())})).collect();
    serde_json::json!({"screen":screen,"tilt":tilt,"cursor":cursor,"buttons":buttons,"cameras":cameras})
}
