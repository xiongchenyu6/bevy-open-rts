//! Exact contact placement, safe initial egress, and bounded shared-grid routing.
use crate::*;
use bevy::prelude::*;

pub(crate) fn unit_contact_move_target_position(
    source_position: Vec3,
    source_radius: f32,
    target_position: Vec3,
    target_radius: f32,
) -> Vec3 {
    let mut direction_from_target = Vec3::new(
        source_position.x - target_position.x,
        0.0,
        source_position.z - target_position.z,
    );
    if direction_from_target.length_squared() <= f32::EPSILON {
        direction_from_target = Vec3::X;
    } else {
        direction_from_target = direction_from_target.normalize();
    }
    target_position
        + direction_from_target * (source_radius + target_radius + UNIT_ADHERENCE_MARGIN_M)
}

pub(crate) struct ContactApproach {
    pub(crate) target: Vec3,
    pub(crate) waypoints: Vec<Vec3>,
}

/// Contact endpoints must fit the mover, not just lie on the target's ring.
/// Occupied unit positions are checked separately from static footprints.
pub(crate) fn contact_position_is_clear(
    point: Vec3,
    source_radius: f32,
    obstacles: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> bool {
    // Tolerate coordinate roundoff at tangent contact, not a wider work range.
    obstacles.iter().all(|(position, radius)| {
        xz_distance(point, *position) + 1.0e-4
            >= source_radius + radius + MOVEMENT_OBSTACLE_CLEARANCE_M
    }) && occupied.iter().all(|(position, radius)| {
        xz_distance(point, *position) + 1.0e-4 >= source_radius + radius + UNIT_SEPARATION_GAP_M
    })
}

/// A retained contact route must still fit both static and current mobile bodies.
/// A unit can occupy an old waypoint after the route was originally planned.
pub(crate) fn contact_route_is_open(
    grid: &NavGrid,
    position: Vec3,
    goal: Vec3,
    path: Option<&PlannedPath>,
    source_radius: f32,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> bool {
    let segment_clear = |from, to| {
        grid.line_clear(from, to)
            && grid.terrain_line_clear(from, to)
            && contact_escape_segment_is_clear(
                from,
                to,
                source_radius,
                obstacles,
                buildings,
                occupied,
                false,
            )
    };
    let mut anchor = position;
    if let Some(path) = path {
        for waypoint in path.waypoints.iter().skip(path.next) {
            if !segment_clear(anchor, *waypoint) {
                return false;
            }
            anchor = *waypoint;
        }
    }
    segment_clear(anchor, goal)
}

/// Choose the nearest reachable side of a structure. A worker-sized temporary
/// grid retains terrain restrictions and routes around adjacent footprints and
/// occupied contact positions; the shared grid's 0.3m inflation is too small
/// for a 0.6m worker squeezed between neighboring buildings.
/// An existing overlap may be escaped only by increasing separation. All other
/// footprint segments stay clear; this never permits crossing a hard obstacle.
fn contact_escape_leg_is_clear(
    source: Vec3,
    target: Vec3,
    source_radius: f32,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> bool {
    contact_escape_segment_is_clear(
        source,
        target,
        source_radius,
        obstacles,
        buildings,
        occupied,
        true,
    )
}

fn contact_escape_segment_is_clear(
    source: Vec3,
    target: Vec3,
    source_radius: f32,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
    finish_escape: bool,
) -> bool {
    let direction = Vec2::new(target.x - source.x, target.z - source.z);
    let static_clear = obstacles.iter().all(|&(position, radius)| {
        let physical = source_radius + radius;
        let clearance = physical + MOVEMENT_OBSTACLE_CLEARANCE_M;
        let original = xz_distance(source, position);
        let minimum = distance_point_to_xz_segment(position, source, target);
        if original + 1e-4 >= clearance {
            return minimum + 1e-4 >= clearance;
        }
        let building = buildings
            .iter()
            .any(|&(building, building_radius)| building == position && building_radius == radius);
        if original + 1e-4 < physical && !building {
            return false;
        }
        // Existing padding deficits may recover only outward. Actual body
        // overlap is exceptional only for an already-overlapped Structure.
        let offset = Vec2::new(source.x - position.x, source.z - position.z);
        offset.dot(direction) >= 0.0
            && minimum + 1e-4 >= original
            && (!finish_escape || xz_distance(target, position) + 1e-4 >= clearance)
    });
    if !static_clear {
        return false;
    }
    let length_squared = direction.length_squared();
    let mut exit_ratio = 0.0f32;
    if length_squared > f32::EPSILON {
        for &(position, radius) in buildings {
            let clearance = source_radius + radius + MOVEMENT_OBSTACLE_CLEARANCE_M;
            let offset = Vec2::new(source.x - position.x, source.z - position.z);
            if offset.length() + 1e-4 >= clearance {
                continue;
            }
            let projection = offset.dot(direction);
            let discriminant = projection * projection
                + length_squared * (clearance * clearance - offset.length_squared());
            exit_ratio = exit_ratio.max(
                ((-projection + discriminant.max(0.0).sqrt()) / length_squared).clamp(0.0, 1.0),
            );
        }
    }
    let exit = source.lerp(target, exit_ratio);
    occupied.iter().all(|&(position, radius)| {
        let physical = source_radius + radius;
        // Only the initial building-egress segment can use physical-body
        // clearance. From the exact building exit onward, the full gap remains.
        distance_point_to_xz_segment(position, source, target) + 1e-4 >= physical
            && distance_point_to_xz_segment(position, exit, target) + 1e-4
                >= physical + UNIT_SEPARATION_GAP_M
    })
}

// Bounded initial escape planning, only for an existing structure overlap.
// Store real consecutive grid edges: never substitute a straight jump to a seed.
fn curved_contact_escape_prefixes(
    source: Vec3,
    radius: f32,
    grid: &NavGrid,
    routing: &NavGrid,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> Vec<Vec<Vec3>> {
    use std::collections::{HashMap, VecDeque};
    let origin = grid.cell_of(source);
    let mut parents = HashMap::<(i32, i32), Option<(i32, i32)>>::new();
    let mut queue = VecDeque::new();
    let mut results = Vec::new();
    // The actual source may lie near a blocked cell's edge, so validate the
    // nearby seed centers directly before traversing adjacent grid edges.
    for dz in -2..=2 {
        for dx in -2..=2 {
            let cell = (origin.0 + dx, origin.1 + dz);
            let point = grid.cell_center(cell.0, cell.1);
            if grid.terrain_line_clear(source, point)
                && contact_escape_segment_is_clear(
                    source, point, radius, obstacles, buildings, occupied, false,
                )
            {
                parents.insert(cell, None);
                queue.push_back(cell);
            }
        }
    }
    while let Some(cell) = queue.pop_front() {
        let point = grid.cell_center(cell.0, cell.1);
        if !routing.is_blocked(cell.0, cell.1)
            && contact_position_is_clear(point, radius, obstacles, occupied)
        {
            let mut prefix = vec![point];
            let mut cursor = cell;
            while let Some(previous) = parents[&cursor] {
                prefix.push(grid.cell_center(previous.0, previous.1));
                cursor = previous;
            }
            prefix.reverse();
            results.push(prefix);
            // Only the overlapped prefix uses this traversal. Normal routing
            // starts here and keeps all full blocked-grid restrictions.
            continue;
        }
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let next = (cell.0 + dx, cell.1 + dz);
                if (next.0 - origin.0).abs() > 12
                    || (next.1 - origin.1).abs() > 12
                    || parents.contains_key(&next)
                {
                    continue;
                }
                let end = grid.cell_center(next.0, next.1);
                if grid.terrain_line_clear(point, end)
                    && contact_escape_segment_is_clear(
                        point, end, radius, obstacles, buildings, occupied, false,
                    )
                {
                    parents.insert(next, Some(cell));
                    queue.push_back(next);
                }
            }
        }
    }
    results
}

pub(crate) fn reachable_unit_contact_approach(
    source_position: Vec3,
    source_radius: f32,
    target_position: Vec3,
    target_radius: f32,
    grid: &NavGrid,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> Option<ContactApproach> {
    let nearest = unit_contact_move_target_position(
        source_position,
        source_radius,
        target_position,
        target_radius,
    );
    let offset = nearest - target_position;
    // Exact clearance-circle intersections delimit narrow valid contact arcs
    // that uniform sampling can miss. Keep the existing 32-candidate budget.
    let ring_radius = offset.length();
    let mut boundaries = Vec::new();
    for (position, radius, margin) in obstacles
        .iter()
        .map(|(p, r)| (*p, *r, MOVEMENT_OBSTACLE_CLEARANCE_M))
        .chain(
            occupied
                .iter()
                .map(|(p, r)| (*p, *r, UNIT_SEPARATION_GAP_M)),
        )
    {
        let delta = position - target_position;
        let distance = Vec2::new(delta.x, delta.z).length();
        if distance <= f32::EPSILON {
            continue;
        }
        let clearance = source_radius + radius + margin;
        let cosine = (ring_radius * ring_radius + distance * distance - clearance * clearance)
            / (2.0 * ring_radius * distance);
        if !(-1.0..1.0).contains(&cosine) {
            continue;
        }
        let center = delta.z.atan2(delta.x);
        let half_angle = cosine.acos();
        boundaries.extend([
            (center - half_angle).rem_euclid(std::f32::consts::TAU),
            (center + half_angle).rem_euclid(std::f32::consts::TAU),
        ]);
    }
    boundaries.sort_by(f32::total_cmp);
    boundaries.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
    let mut candidates = Vec::new();
    if !boundaries.is_empty() {
        for index in 0..boundaries.len() {
            let begin = boundaries[index];
            let end = if index + 1 < boundaries.len() {
                boundaries[index + 1]
            } else {
                boundaries[0] + std::f32::consts::TAU
            };
            let angle = (begin + end) * 0.5;
            let point = target_position + Vec3::new(angle.cos(), 0.0, angle.sin()) * ring_radius;
            if contact_position_is_clear(point, source_radius, obstacles, occupied) {
                candidates.push(point);
            }
        }
        candidates.sort_by(|a, b| {
            xz_distance(source_position, *a).total_cmp(&xz_distance(source_position, *b))
        });
        candidates.truncate(32);
    }
    for index in 0..32 {
        if candidates.len() == 32 {
            break;
        }
        candidates.push(
            target_position
                + Quat::from_rotation_y(index as f32 * std::f32::consts::TAU / 32.0) * offset,
        );
    }
    candidates.sort_by(|a, b| {
        xz_distance(source_position, *a).total_cmp(&xz_distance(source_position, *b))
    });
    // Prefer routes around mobile units. If their inflated grid cells seal a
    // narrow passage, use the static route and let normal movement resolve
    // mobile collisions. Contact endpoints must remain genuinely unoccupied.
    let mut refinement_routing = None;
    for include_mobile_obstacles in [true, false] {
        let mut routing = NavGrid {
            version: grid.version,
            origin_x: grid.origin_x,
            origin_z: grid.origin_z,
            width: grid.width,
            height: grid.height,
            blocked: grid.blocked.clone(),
            terrain_blocked: grid.terrain_blocked.clone(),
        };
        if !routing.blocked.is_empty() {
            for (position, radius, margin) in obstacles
                .iter()
                .map(|(p, r)| (*p, *r, MOVEMENT_OBSTACLE_CLEARANCE_M))
                .chain(
                    occupied
                        .iter()
                        .filter(|_| include_mobile_obstacles)
                        .map(|(p, r)| (*p, *r, UNIT_SEPARATION_GAP_M)),
                )
            {
                let reach = source_radius + radius + margin;
                let (min_x, min_z) = routing.cell_of(position - Vec3::splat(reach));
                let (max_x, max_z) = routing.cell_of(position + Vec3::splat(reach));
                for z in min_z.max(0)..=max_z.min(routing.height - 1) {
                    for x in min_x.max(0)..=max_x.min(routing.width - 1) {
                        if xz_distance(routing.cell_center(x, z), position) < reach {
                            routing.blocked[(z * routing.width + x) as usize] = true;
                        }
                    }
                }
            }
        }
        let goals = candidates
            .iter()
            .copied()
            .filter(|target| contact_position_is_clear(*target, source_radius, obstacles, occupied))
            .collect::<Vec<_>>();

        for target in goals.iter().copied() {
            if grid.terrain_line_clear(source_position, target)
                && contact_escape_leg_is_clear(
                    source_position,
                    target,
                    source_radius,
                    obstacles,
                    buildings,
                    occupied,
                )
            {
                return Some(ContactApproach {
                    target,
                    waypoints: if routing.blocked.is_empty() {
                        Vec::new()
                    } else {
                        vec![target]
                    },
                });
            }
        }
        if routing.blocked.is_empty() {
            continue;
        }
        let goals = goals
            .into_iter()
            .filter(|target| {
                !routing.is_blocked(routing.cell_of(*target).0, routing.cell_of(*target).1)
            })
            .collect::<Vec<_>>();
        let source_cell = routing.cell_of(source_position);
        let mut starts = Vec::new();
        // Same bounded12-cell neighborhood as nearest_open_cell, but retain all
        // physically reachable starts rather than one potentially isolated cell.
        for ring in 0i32..=12 {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let (x, z) = (source_cell.0 + dx, source_cell.1 + dz);
                    if routing.is_blocked(x, z) {
                        continue;
                    }
                    let point = routing.cell_center(x, z);
                    if contact_position_is_clear(point, source_radius, obstacles, occupied)
                        && grid.terrain_line_clear(source_position, point)
                        && contact_escape_leg_is_clear(
                            source_position,
                            point,
                            source_radius,
                            obstacles,
                            buildings,
                            occupied,
                        )
                    {
                        starts.push(point);
                    }
                }
            }
        }
        let prefixes = if starts.is_empty()
            && buildings.iter().any(|(position, radius)| {
                xz_distance(source_position, *position)
                    < source_radius + radius + MOVEMENT_OBSTACLE_CLEARANCE_M
            }) {
            curved_contact_escape_prefixes(
                source_position,
                source_radius,
                &grid,
                &routing,
                obstacles,
                buildings,
                occupied,
            )
        } else {
            Vec::new()
        };
        starts.extend(prefixes.iter().filter_map(|prefix| prefix.last().copied()));
        if let Some((target, mut waypoints)) =
            routing.find_contact_path(source_position, &starts, &goals)
        {
            if let Some(first) = waypoints.first() {
                if let Some(prefix) = prefixes.iter().find(|prefix| {
                    prefix
                        .last()
                        .is_some_and(|point| routing.cell_of(*point) == routing.cell_of(*first))
                }) {
                    let mut initial = prefix[..prefix.len() - 1].to_vec();
                    initial.extend(waypoints);
                    waypoints = initial;
                }
            }
            waypoints.push(target);
            let mut anchor = source_position;
            let physically_clear = waypoints.iter().all(|point| {
                let clear = grid.terrain_line_clear(anchor, *point)
                    && contact_escape_segment_is_clear(
                        anchor,
                        *point,
                        source_radius,
                        obstacles,
                        buildings,
                        occupied,
                        false,
                    );
                anchor = *point;
                clear
            });
            if physically_clear {
                return Some(ContactApproach { target, waypoints });
            }
        }
        refinement_routing = Some(routing);
    }
    local_contact::refined_contact_approach(
        source_position,
        source_radius,
        &candidates,
        grid,
        refinement_routing.as_ref()?,
        obstacles,
        buildings,
        occupied,
    )
}

#[path = "contact_planning/local_contact.rs"]
mod local_contact;

pub(crate) fn contact_action_entry_range(
    source_radius: f32,
    target_radius: f32,
    margin: f32,
) -> f32 {
    source_radius + target_radius + margin + CONTACT_ACTION_REACHED_TOLERANCE_M
}

pub(crate) fn move_order_targets_contact(
    move_order: Option<&MoveOrder>,
    target_position: Vec3,
    source_radius: f32,
    target_radius: f32,
) -> bool {
    let Some(move_order) = move_order else {
        return false;
    };
    let expected_contact_distance = source_radius + target_radius + UNIT_ADHERENCE_MARGIN_M;
    (xz_distance(move_order.target, target_position) - expected_contact_distance).abs()
        <= CONTACT_ACTION_REACHED_TOLERANCE_M * 2.0
}

#[cfg(test)]
#[path = "contact_escape_tests.rs"]
mod contact_escape_tests;

#[cfg(test)]
#[path = "contact_planning/case2641_replay_tests.rs"]
mod case2641_replay_tests;

#[cfg(test)]
#[path = "contact_planning/m3_constructor_tests.rs"]
mod m3_constructor_tests;
