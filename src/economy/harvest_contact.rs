//! Retain valid harvest approaches and reuse bounded contact routing when blocked.
use crate::*;
use bevy::prelude::*;

pub(super) fn update_harvest_contact_route(
    commands: &mut Commands,
    entity: Entity,
    source: (Vec3, f32),
    target: (Vec3, f32),
    orders: (Option<&MoveOrder>, Option<&PlannedPath>),
    grid: &NavGrid,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) {
    if move_order_targets_contact(orders.0, target.0, source.1, target.1)
        && orders.0.is_some_and(|order| {
            contact_position_is_clear(order.target, source.1, obstacles, occupied)
                && contact_route_is_open(
                    grid,
                    source.0,
                    order.target,
                    orders.1,
                    source.1,
                    obstacles,
                    buildings,
                    occupied,
                )
        })
    {
        return;
    }
    if let Some(approach) = reachable_unit_contact_approach(
        source.0, source.1, target.0, target.1, grid, obstacles, buildings, occupied,
    ) {
        commands.entity(entity).try_insert(MoveOrder {
            target: approach.target,
        });
        if approach.waypoints.is_empty() {
            commands.entity(entity).try_remove::<PlannedPath>();
        } else {
            commands.entity(entity).try_insert(PlannedPath {
                goal: approach.target,
                waypoints: approach.waypoints,
                next: 0,
            });
        }
    } else {
        // Preserve the mineral choice and carried cargo while no legal route exists.
        commands
            .entity(entity)
            .try_remove::<MoveOrder>()
            .try_remove::<PlannedPath>();
    }
}
