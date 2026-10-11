//! Bounded refinement for physically clear contact gaps missed by coarse cells.
use super::*;
use std::collections::{HashMap, VecDeque};

pub(super) fn refined_contact_approach(
    source: Vec3,
    radius: f32,
    goals: &[Vec3],
    grid: &NavGrid,
    routing: &NavGrid,
    obstacles: &[(Vec3, f32)],
    buildings: &[(Vec3, f32)],
    occupied: &[(Vec3, f32)],
) -> Option<ContactApproach> {
    if grid.blocked.is_empty() {
        return None;
    }
    let step = NAV_GRID_CELL_M * 0.5;
    let source_cell = (
        ((source.x - grid.origin_x) / step).floor() as i32,
        ((source.z - grid.origin_z) / step).floor() as i32,
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
            && (cell.0 - source_cell.0).abs() <= 24
            && (cell.1 - source_cell.1).abs() <= 24
    };
    if !inside(source_cell) {
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
    // Fine cells recover a physically valid exit from a coarse-grid pocket.
    // Reuse one global search for the remaining detour instead of enlarging
    // the existing 49x49 fine-node budget or jumping to an unvalidated seed.
    let mut coarse_prefixes = HashMap::new();
    for cell in parents.keys().copied() {
        let fine = point(cell);
        let coarse_cell = routing.cell_of(fine);
        let coarse = routing.cell_center(coarse_cell.0, coarse_cell.1);
        if !routing.is_blocked(coarse_cell.0, coarse_cell.1)
            && contact_position_is_clear(coarse, radius, obstacles, occupied)
            && clear(fine, coarse)
        {
            coarse_prefixes
                .entry(coarse_cell)
                .and_modify(|previous| {
                    if cell < *previous {
                        *previous = cell;
                    }
                })
                .or_insert(cell);
        }
    }
    let mut coarse_cells = coarse_prefixes.keys().copied().collect::<Vec<_>>();
    coarse_cells.sort_unstable();
    let starts = coarse_cells
        .iter()
        .map(|&(x, z)| routing.cell_center(x, z))
        .collect::<Vec<_>>();
    let coarse_goals = goals
        .iter()
        .copied()
        .filter(|goal| {
            let cell = routing.cell_of(*goal);
            !routing.is_blocked(cell.0, cell.1)
        })
        .collect::<Vec<_>>();
    let (target, global) = routing.find_contact_path(source, &starts, &coarse_goals)?;
    let first = *global.first()?;
    let mut cursor = *coarse_prefixes.get(&routing.cell_of(first))?;
    let mut waypoints = vec![point(cursor)];
    while let Some(previous) = parents[&cursor] {
        waypoints.push(point(previous));
        cursor = previous;
    }
    waypoints.reverse();
    waypoints.extend(global);
    waypoints.push(target);
    let mut anchor = source;
    for next in &waypoints {
        if !clear(anchor, *next) {
            return None;
        }
        anchor = *next;
    }
    Some(ContactApproach { target, waypoints })
}
