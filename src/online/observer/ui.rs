//! Passive input-frame evidence. Actor, input, layout and targeting queries are read-only.
use super::*;

pub(in crate::online) fn record(
    mut observer: ResMut<OnlineObserver>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mode: Res<CommandMode>,
    visible_player: Res<VisiblePlayer>,
    zones: Res<HudHitZones>,
    terrain: Res<TerrainHeightField>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    selected: Query<
        (
            Entity,
            Option<&NetworkEntityId>,
            &Team,
            Option<&Unit>,
            Option<&Structure>,
        ),
        With<Selected>,
    >,
    cards: Query<(
        &CommandSlot,
        &BuildAction,
        &CommandSlotAvailability,
        &Interaction,
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&InheritedVisibility>,
    )>,
) {
    if !observer.enabled {
        return;
    }
    let window = windows.single().ok();
    let mut own: Vec<_> = selected.iter().filter(|(_,_,team,_,_)| **team==visible_player.team)
        .map(|(entity,id,_,unit,structure)|serde_json::json!({"local_entity":entity.to_bits(),"network_id":id.map(|id|id.0),"type":unit.map(|u|u.id).or_else(||structure.map(|s|s.id))})).collect();
    own.sort_by_key(|v| v["local_entity"].as_u64());
    let state = serde_json::json!({"selected_owned":own,"pending_placement":mode.pending_structure_placement.as_ref().map(|p|p.id)});
    #[cfg(target_arch = "wasm32")]
    let browser_dpr = web_sys::window().map(|w| w.device_pixel_ratio());
    #[cfg(not(target_arch = "wasm32"))]
    let browser_dpr: Option<f64> = None;
    let pressed = mouse.just_pressed(MouseButton::Left);
    let released = mouse.just_released(MouseButton::Left);
    let right_pressed = mouse.just_pressed(MouseButton::Right);
    let right_released = mouse.just_released(MouseButton::Right);
    if !pressed
        && !released
        && !right_pressed
        && !right_released
        && observer.last_ui_state.is_null()
        && state["selected_owned"]
            .as_array()
            .is_some_and(|v| v.is_empty())
        && state["pending_placement"].is_null()
    {
        observer.last_ui_state = state;
        return;
    }
    if !pressed && !released && !right_pressed && !right_released && state == observer.last_ui_state
    {
        return;
    }
    let cursor = window.and_then(|w| w.cursor_position());
    let physical = window.and_then(|w| w.physical_cursor_position());
    let mut card_rows: Vec<_>=cards.iter().map(|(slot,action,availability,interaction,node,computed,transform,visible)|{
        let size=computed.size();let center=transform.translation;
        serde_json::json!({"slot":slot.0,"action":format!("{action:?}"),"enabled":availability.enabled,"interaction":format!("{interaction:?}"),"display":format!("{:?}",node.display),"inherited_visible":visible.map(|v|v.get()),"physical_center":center.to_array(),"physical_size":size.to_array(),"physical_min":(center-size*0.5).to_array(),"physical_max":(center+size*0.5).to_array(),"contains_physical_cursor":physical.is_some_and(|p|computed.contains_point(*transform,p))})
    }).collect();
    card_rows.sort_by_key(|v| v["slot"].as_u64());
    let event = serde_json::json!({"elapsed":time.elapsed_secs(),"left_pressed":pressed,"left_released":released,"right_pressed":right_pressed,"right_released":right_released,"state":state,
        "cursor_ground":window.and_then(|w|pointer_ground(w,&cameras,&terrain)).map(|p|p.to_array()),
        "window":window.map(|w|serde_json::json!({"logical_size":[w.width(),w.height()],"physical_size":[w.physical_width(),w.physical_height()],"scale_factor":w.scale_factor(),"browser_dpr":browser_dpr,"cursor_logical":cursor.map(|p|p.to_array()),"cursor_physical":physical.map(|p|p.to_array())})),
        "hud_blocks_cursor":cursor.is_some_and(|p|zones.blocks_world(p)),"hud_rects":zones.world_rects.iter().map(|(a,b)|[a.to_array(),b.to_array()]).collect::<Vec<_>>(),"cards":card_rows});
    observer.last_ui_state = state;
    observer.ui_events.push_back(event);
    while observer.ui_events.len() > 40 {
        observer.ui_events.pop_front();
    }
    observer.dirty = true;
}
