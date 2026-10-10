//! Native, opt-in, read-only GLB geometry evidence. Never an actor/input setter.
use crate::*;
use bevy::camera::primitives::Aabb;
use std::collections::{HashMap, HashSet};

pub(super) type Queries<'w, 's> = (
    Query<
        'w,
        's,
        (
            Entity,
            &'static ResourceNode,
            &'static Transform,
            &'static GlobalTransform,
            &'static VisibilityState,
            &'static Visibility,
        ),
    >,
    Query<'w, 's, (&'static Team, &'static HarvestOrder)>,
    Query<'w, 's, (Entity, &'static Children)>,
    Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            Option<&'static Aabb>,
            Option<&'static ViewVisibility>,
            Option<&'static InheritedVisibility>,
            Option<&'static Name>,
        ),
    >,
    Query<'w, 's, Entity, With<WorldAssetRoot>>,
);

pub(super) fn requested() -> bool {
    std::env::var("OPEN_BEVY_RESOURCE_GEOMETRY_OBSERVE").is_ok_and(|value| value == "1")
}

pub(super) fn snapshot(
    queries: &Queries,
    cameras: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    owner: Team,
    cursor: Option<Vec2>,
    cursor_ground: Option<Vec3>,
) -> serde_json::Value {
    let Ok((camera, camera_transform)) = cameras.single() else {
        return serde_json::json!({"camera_available":false,"resources":[]});
    };
    let known: HashSet<_> = queries
        .1
        .iter()
        .filter(|(team, _)| **team == owner)
        .filter_map(|(_, harvest)| harvest.resource)
        .collect();
    let children: HashMap<_, Vec<_>> = queries
        .2
        .iter()
        .map(|(entity, children)| (entity, children.iter().collect()))
        .collect();
    let aabbs: HashMap<_, _> = queries
        .3
        .iter()
        .filter_map(|(entity, transform, aabb, ..)| aabb.map(|aabb| (entity, (*transform, *aabb))))
        .collect();
    let model_roots: HashSet<_> = queries.4.iter().collect();
    let project = |point| {
        camera
            .world_to_viewport(camera_transform, point)
            .ok()
            .map(|point| point.to_array())
    };
    let mut resources = Vec::new();
    for (entity, node, local, global, visibility, render_visibility) in &queries.0 {
        if !visibility.visible && !known.contains(&entity) {
            continue;
        }
        let scenes: Vec<_> = children
            .get(&entity)
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| model_roots.contains(child))
            .collect();
        let mut stack = scenes.clone();
        let mut meshes = Vec::new();
        while let Some(child) = stack.pop() {
            if let Some(descendants) = children.get(&child) {
                stack.extend(descendants.iter().copied());
            }
            let Ok((_, transform, Some(aabb), view, inherited, name)) = queries.3.get(child) else {
                continue;
            };
            let center = Vec3::from(aabb.center);
            let half = Vec3::from(aabb.half_extents);
            let mut corners = Vec::new();
            let mut screens = Vec::new();
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [-1.0, 1.0] {
                        let world = transform.transform_point(center + half * Vec3::new(x, y, z));
                        corners.push(world.to_array());
                        screens.push(project(world));
                    }
                }
            }
            meshes.push(serde_json::json!({
                "entity":child.to_bits(),"name":name.map(Name::as_str),
                "world_from_local":transform.to_matrix().to_cols_array(),
                "local_aabb_center":center.to_array(),"local_aabb_half_extents":half.to_array(),
                "world_corners":corners,"screen_corners":screens,
                "view_visibility":view.map(|value| value.get()),"inherited_visibility":inherited.map(|value| value.get()),
            }));
        }
        let bounds =
            crate::capture_api::entity_visual_world_bounds(entity, &children, &aabbs, &model_roots);
        let body = bounds.map(|(min,max)| {
            let center = (min+max)*0.5;
            serde_json::json!({"min":min.to_array(),"max":max.to_array(),"size":(max-min).to_array(),
                "height_above_global_anchor":max.y-global.translation().y,"center_screen":project(center),
                "ground_screen":project(Vec3::new(center.x,min.y,center.z)),"top_screen":project(Vec3::new(center.x,max.y,center.z))})
        });
        let capsule = cursor.and_then(|cursor| resource_cursor_pick_distance(cursor,camera,camera_transform,local.translation,node.kind,RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX))
            .map(|(distance,radius)|serde_json::json!({"distance_pixels":distance,"pick_radius_pixels":radius,"inside":distance<=radius,"declared_height":resource_visual_height(node.kind),"declared_half_width":resource_visual_half_width(node.kind)}));
        resources.push(serde_json::json!({
            "entity":entity.to_bits(),"kind":format!("{:?}", node.kind),"amount":node.amount,
            "visible":visibility.visible,"actual_own_harvest_reference":known.contains(&entity),
            "render_visibility":format!("{render_visibility:?}"),
            "local_anchor":local.translation.to_array(),"global_anchor":global.translation().to_array(),
            "root_world_from_local":global.to_matrix().to_cols_array(),
            "local_anchor_screen":project(local.translation),"global_anchor_screen":project(global.translation()),
            "ground_distance_from_cursor":cursor_ground.map(|point|xz_distance(point,local.translation)),
            "glb_scene_roots":scenes.iter().map(|e|e.to_bits()).collect::<Vec<_>>(),
            "loaded_mesh_count":meshes.len(),"glb_world_bounds":body,"meshes":meshes,
            "current_declared_capsule":capsule,
        }));
    }
    resources.sort_by_key(|node| node["entity"].as_u64());
    serde_json::json!({"scope":"visible or actual-own-Harvest-referenced resource GLB subtrees only; not whole scene",
        "camera_available":true,"camera_world_from_local":camera_transform.to_matrix().to_cols_array(),
        "camera_clip_from_view":camera.computed.clip_from_view.to_cols_array(),
        "logical_viewport":camera.logical_viewport_rect().map(|rect|[rect.min.to_array(),rect.max.to_array()]),
        "resources":resources})
}

#[cfg(test)]
mod tests;
