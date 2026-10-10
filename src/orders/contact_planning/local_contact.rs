//! Bounded refinement for physically clear contact gaps missed by coarse cells.
use super::*;
use std::collections::{HashMap, VecDeque};

pub(super) fn refined_contact_approach(
    source: Vec3,
    target_position: Vec3,
    radius: f32,
    goals: &[Vec3],
    grid: &NavGrid,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> Option<ContactApproach> {
    if grid.blocked.is_empty() {
        return None;
    }
    let step = NAV_GRID_CELL_M * 0.5;
    let center = source.lerp(target_position, 0.5);
    let origin = (
        ((center.x - grid.origin_x) / step).floor() as i32,
        ((center.z - grid.origin_z) / step).floor() as i32,
    );
    let source_cell = (
        ((source.x - grid.origin_x) / step).floor() as i32,
        ((source.z - grid.origin_z) / step).floor() as i32,
    );
    let target_cell = (
        ((target_position.x - grid.origin_x) / step).floor() as i32,
        ((target_position.z - grid.origin_z) / step).floor() as i32,
    );
    let point = |cell: (i32, i32)| {
        Vec3::new(
            grid.origin_x + (cell.0 as f32 + 0.5) * step,
            source.y,
            grid.origin_z + (cell.1 as f32 + 0.5) * step,
        )
    };
    let inside = |cell: (i32, i32)| {
        cell.0 >= 0
            && cell.1 >= 0
            && cell.0 < grid.width * 2
            && cell.1 < grid.height * 2
            && (cell.0 - origin.0).abs() <= 24
            && (cell.1 - origin.1).abs() <= 24
    };
    if !inside(source_cell) || !inside(target_cell) {
        return None;
    }
    let clear = |from, to| {
        grid.line_clear(from, to)
            && grid.terrain_line_clear(from, to)
            && contact_escape_segment_is_clear(
                from, to, radius, obstacles, buildings, occupied, false,
            )
    };
    let goals: Vec<_> = goals
        .iter()
        .copied()
        .filter(|goal| contact_position_is_clear(*goal, radius, obstacles, occupied))
        .collect();
    let mut parents = HashMap::new();
    let mut queue = VecDeque::new();
    for dz in -4..=4 {
        for dx in -4..=4 {
            let cell = (source_cell.0 + dx, source_cell.1 + dz);
            if inside(cell) && clear(source, point(cell)) {
                parents.insert(cell, None);
                queue.push_back(cell);
            }
        }
    }
    while let Some(cell) = queue.pop_front() {
        debug_assert!(
            parents.len() <= 49 * 49,
            "refinement must retain its fixed node budget"
        );
        let current = point(cell);
        if let Some(target) = goals
            .iter()
            .copied()
            .find(|goal| xz_distance(current, *goal) <= step * 1.5 && clear(current, *goal))
        {
            let mut waypoints = vec![current];
            let mut cursor = cell;
            while let Some(previous) = parents[&cursor] {
                waypoints.push(point(previous));
                cursor = previous;
            }
            waypoints.reverse();
            waypoints.push(target);
            return Some(ContactApproach { target, waypoints });
        }
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let next = (cell.0 + dx, cell.1 + dz);
                if inside(next) && !parents.contains_key(&next) && clear(current, point(next)) {
                    parents.insert(next, Some(cell));
                    queue.push_back(next);
                }
            }
        }
    }
    None
}
