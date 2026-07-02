//! Unit-vs-unit separation (boids-lite) and the A* nav grid.
//!
//! Pure move out of lib.rs (module-split Stage 1); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

// Unit-vs-unit separation (boids-lite): extra clearance kept between allied unit
// circles, and the max speed at which overlapping units get pushed apart.
pub(crate) const UNIT_SEPARATION_GAP_M: f32 = 0.05;
pub(crate) const UNIT_SEPARATION_MAX_SPEED: f32 = 1.8;
// A* nav grid: cell size, how much obstacle circles are inflated (typical small
// unit radius), how close a follower must get to consume a path waypoint, and how
// far a goal may drift (chasers re-insert MoveOrder every frame) before replanning.
pub(crate) const NAV_GRID_CELL_M: f32 = 0.5;
pub(crate) const NAV_OBSTACLE_INFLATE_M: f32 = 0.3;
pub(crate) const NAV_WAYPOINT_REACHED_M: f32 = 0.35;
pub(crate) const NAV_REPLAN_GOAL_TOLERANCE_M: f32 = 0.5;

/// Pure geometry for one overlapping unit pair: the push applied to `a` (`b` gets
/// the negation) so both end just clear of each other, split evenly. `None` when
/// the circles (plus the separation gap) don't overlap.
pub(crate) fn pair_separation_push(
    a_pos: Vec3,
    a_radius: f32,
    b_pos: Vec3,
    b_radius: f32,
) -> Option<Vec3> {
    let mut delta = b_pos - a_pos;
    delta.y = 0.0;
    let min_dist = a_radius + b_radius + UNIT_SEPARATION_GAP_M;
    let dist_sq = delta.length_squared();
    if dist_sq >= min_dist * min_dist {
        return None;
    }
    let dist = dist_sq.sqrt();
    // Coincident units get a deterministic axis; the chain of pushes over the next
    // frames fans a stack out.
    let axis = if dist > 1e-4 { delta / dist } else { Vec3::X };
    Some(-axis * (min_dist - dist) * 0.5)
}

/// Boids-lite unit separation: allied terrain units that overlap get pushed apart
/// after movement, so armies spread out instead of stacking into one point (godot
/// gets this from NavigationServer avoidance; the port previously had no
/// unit-vs-unit collision at all). Enemies are exempt so crushing still works, and
/// a unit is never pushed into a structure/resource node or off the map.
pub(crate) fn separate_units(
    time: Res<Time>,
    relations: Res<TeamRelations>,
    map_bounds: Res<MapBounds>,
    mut units: Query<
        (
            &Team,
            &Unit,
            &MovementDomain,
            &Selectable,
            &mut Transform,
            &Health,
        ),
        With<Unit>,
    >,
    obstacles: Query<
        (&Transform, &Selectable, Option<&Health>),
        (
            Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
            Without<Unit>,
        ),
    >,
) {
    let max_step = UNIT_SEPARATION_MAX_SPEED * time.delta_secs();
    if max_step <= 0.0 {
        return;
    }
    struct SeparationSnapshot {
        team: Team,
        radius: f32,
        position: Vec3,
        movable: bool,
        active: bool,
    }
    // Two passes over the same (unmutated) query iterate in the same order, so the
    // snapshot indexes line up with the apply pass below.
    let snapshot: Vec<SeparationSnapshot> = units
        .iter()
        .map(
            |(team, unit, domain, selectable, transform, health)| SeparationSnapshot {
                team: *team,
                radius: selectable.radius,
                position: transform.translation,
                movable: unit.speed > 0.0,
                active: *domain == MovementDomain::Terrain && health.current > 0.0,
            },
        )
        .collect();
    let mut pushes = vec![Vec3::ZERO; snapshot.len()];
    for i in 0..snapshot.len() {
        if !snapshot[i].active {
            continue;
        }
        for j in (i + 1)..snapshot.len() {
            if !snapshot[j].active
                || relations.are_enemies(snapshot[i].team, snapshot[j].team)
                || (!snapshot[i].movable && !snapshot[j].movable)
            {
                continue;
            }
            let Some(push) = pair_separation_push(
                snapshot[i].position,
                snapshot[i].radius,
                snapshot[j].position,
                snapshot[j].radius,
            ) else {
                continue;
            };
            // An immovable unit (deployed/zero-speed) passes its share to the other.
            match (snapshot[i].movable, snapshot[j].movable) {
                (true, true) => {
                    pushes[i] += push;
                    pushes[j] -= push;
                }
                (true, false) => pushes[i] += push * 2.0,
                (false, true) => pushes[j] -= push * 2.0,
                (false, false) => unreachable!(),
            }
        }
    }
    let blocked_by_obstacle = |position: Vec3, radius: f32| {
        obstacles
            .iter()
            .filter(|(_, _, health)| !health.is_some_and(|health| health.current <= 0.0))
            .any(|(transform, selectable, _)| {
                xz_distance(position, transform.translation) < radius + selectable.radius
            })
    };
    for (index, (_, _, _, selectable, mut transform, _)) in units.iter_mut().enumerate() {
        let push = pushes[index];
        if push == Vec3::ZERO {
            continue;
        }
        let push = push.clamp_length_max(max_step);
        let mut candidate = transform.translation + push;
        candidate.y = transform.translation.y;
        candidate = map_bounds.clamp_ground_point(candidate, selectable.radius);
        if blocked_by_obstacle(candidate, selectable.radius) {
            continue;
        }
        transform.translation = candidate;
    }
}

/// Coarse walkability grid over the map, rebuilt when structures/resource nodes
/// change. Powers A* paths for terrain units whose straight line is blocked; godot
/// gets the same from NavigationServer's navmesh.
#[derive(Resource, Default)]
pub(crate) struct NavGrid {
    pub(crate) version: u64,
    pub(crate) origin_x: f32,
    pub(crate) origin_z: f32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) blocked: Vec<bool>,
}

impl NavGrid {
    pub(crate) fn rebuild(&mut self, bounds: MapBounds, obstacles: &[(Vec3, f32)]) {
        self.origin_x = -bounds.half_width;
        self.origin_z = -bounds.half_depth;
        self.width = ((bounds.half_width * 2.0) / NAV_GRID_CELL_M).ceil() as i32;
        self.height = ((bounds.half_depth * 2.0) / NAV_GRID_CELL_M).ceil() as i32;
        self.blocked = vec![false; (self.width * self.height).max(0) as usize];
        for &(position, radius) in obstacles {
            let reach = radius + NAV_OBSTACLE_INFLATE_M;
            let min_x = (((position.x - reach) - self.origin_x) / NAV_GRID_CELL_M).floor() as i32;
            let max_x = (((position.x + reach) - self.origin_x) / NAV_GRID_CELL_M).ceil() as i32;
            let min_z = (((position.z - reach) - self.origin_z) / NAV_GRID_CELL_M).floor() as i32;
            let max_z = (((position.z + reach) - self.origin_z) / NAV_GRID_CELL_M).ceil() as i32;
            for cz in min_z.max(0)..=max_z.min(self.height - 1) {
                for cx in min_x.max(0)..=max_x.min(self.width - 1) {
                    let center = self.cell_center(cx, cz);
                    let dx = center.x - position.x;
                    let dz = center.z - position.z;
                    if dx * dx + dz * dz < reach * reach {
                        self.blocked[(cz * self.width + cx) as usize] = true;
                    }
                }
            }
        }
        self.version += 1;
    }

    pub(crate) fn cell_of(&self, position: Vec3) -> (i32, i32) {
        (
            (((position.x - self.origin_x) / NAV_GRID_CELL_M) as i32).clamp(0, self.width - 1),
            (((position.z - self.origin_z) / NAV_GRID_CELL_M) as i32).clamp(0, self.height - 1),
        )
    }

    pub(crate) fn cell_center(&self, cx: i32, cz: i32) -> Vec3 {
        Vec3::new(
            self.origin_x + (cx as f32 + 0.5) * NAV_GRID_CELL_M,
            0.0,
            self.origin_z + (cz as f32 + 0.5) * NAV_GRID_CELL_M,
        )
    }

    pub(crate) fn is_blocked(&self, cx: i32, cz: i32) -> bool {
        if cx < 0 || cz < 0 || cx >= self.width || cz >= self.height {
            return true;
        }
        self.blocked[(cz * self.width + cx) as usize]
    }

    /// Supercover walk of the segment: true when no blocked cell is crossed.
    pub(crate) fn line_clear(&self, from: Vec3, to: Vec3) -> bool {
        if self.blocked.is_empty() {
            return true;
        }
        let steps = (xz_distance(from, to) / (NAV_GRID_CELL_M * 0.5))
            .ceil()
            .max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let point = from.lerp(to, t);
            let (cx, cz) = self.cell_of(point);
            if self.is_blocked(cx, cz) {
                return false;
            }
        }
        true
    }

    /// Nearest walkable cell to `cell`, spiralling outward (targets inside a base
    /// footprint resolve to its edge instead of failing).
    pub(crate) fn nearest_open_cell(&self, cell: (i32, i32)) -> Option<(i32, i32)> {
        if !self.is_blocked(cell.0, cell.1) {
            return Some(cell);
        }
        for ring in 1i32..=12 {
            let mut best: Option<((i32, i32), i32)> = None;
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let candidate = (cell.0 + dx, cell.1 + dz);
                    if !self.is_blocked(candidate.0, candidate.1) {
                        let dist = dx * dx + dz * dz;
                        if best.is_none_or(|(_, d)| dist < d) {
                            best = Some((candidate, dist));
                        }
                    }
                }
            }
            if let Some((found, _)) = best {
                return Some(found);
            }
        }
        None
    }

    /// A* over the grid (8-directional, no corner cutting), then string-pulled into
    /// a short waypoint list (world positions). `None` when unreachable.
    pub(crate) fn find_path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        if self.blocked.is_empty() {
            return None;
        }
        let start = self.nearest_open_cell(self.cell_of(from))?;
        let goal = self.nearest_open_cell(self.cell_of(to))?;
        if start == goal {
            return Some(vec![self.cell_center(goal.0, goal.1)]);
        }
        let index = |c: (i32, i32)| (c.1 * self.width + c.0) as usize;
        let octile = |a: (i32, i32), b: (i32, i32)| {
            let dx = (a.0 - b.0).abs() as f32;
            let dz = (a.1 - b.1).abs() as f32;
            dx.max(dz) + 0.414 * dx.min(dz)
        };
        let mut open = std::collections::BinaryHeap::new();
        let mut g_cost = vec![f32::INFINITY; self.blocked.len()];
        let mut came_from = vec![u32::MAX; self.blocked.len()];
        g_cost[index(start)] = 0.0;
        // BinaryHeap is a max-heap: store negated f-cost scaled to an integer key.
        let key = |f: f32| -((f * 1024.0) as i64);
        open.push((key(octile(start, goal)), start));
        let mut reached = false;
        while let Some((_, cell)) = open.pop() {
            if cell == goal {
                reached = true;
                break;
            }
            let cell_g = g_cost[index(cell)];
            for (dx, dz, step_cost) in [
                (1i32, 0i32, 1.0f32),
                (-1, 0, 1.0),
                (0, 1, 1.0),
                (0, -1, 1.0),
                (1, 1, 1.414),
                (1, -1, 1.414),
                (-1, 1, 1.414),
                (-1, -1, 1.414),
            ] {
                let next = (cell.0 + dx, cell.1 + dz);
                if self.is_blocked(next.0, next.1) {
                    continue;
                }
                // No cutting corners diagonally past a blocked cell.
                if dx != 0
                    && dz != 0
                    && (self.is_blocked(cell.0 + dx, cell.1)
                        || self.is_blocked(cell.0, cell.1 + dz))
                {
                    continue;
                }
                let tentative = cell_g + step_cost;
                if tentative < g_cost[index(next)] {
                    g_cost[index(next)] = tentative;
                    came_from[index(next)] = index(cell) as u32;
                    open.push((key(tentative + octile(next, goal)), next));
                }
            }
        }
        if !reached {
            return None;
        }
        let mut cells = vec![goal];
        let mut cursor = index(goal);
        while came_from[cursor] != u32::MAX {
            cursor = came_from[cursor] as usize;
            cells.push((cursor as i32 % self.width, cursor as i32 / self.width));
        }
        cells.reverse();
        // String pull: keep only waypoints that break line of sight.
        let world: Vec<Vec3> = cells.iter().map(|&(x, z)| self.cell_center(x, z)).collect();
        let mut waypoints = Vec::new();
        let mut anchor = 0usize;
        while anchor + 1 < world.len() {
            let mut furthest = anchor + 1;
            for probe in (anchor + 1..world.len()).rev() {
                if self.line_clear(world[anchor], world[probe]) {
                    furthest = probe;
                    break;
                }
            }
            waypoints.push(world[furthest]);
            anchor = furthest;
        }
        Some(waypoints)
    }
}

/// The A* waypoints a terrain unit is currently following toward `goal`.
#[derive(Component)]
pub(crate) struct PlannedPath {
    pub(crate) goal: Vec3,
    pub(crate) waypoints: Vec<Vec3>,
    pub(crate) next: usize,
}

/// Rebuilds the nav grid when structures/resource nodes appear or disappear.
pub(crate) fn rebuild_nav_grid(
    map_bounds: Res<MapBounds>,
    mut grid: ResMut<NavGrid>,
    obstacles: Query<
        (&Transform, &Selectable, Option<&Health>),
        (
            Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
            Without<Unit>,
        ),
    >,
    added: Query<(), Or<(Added<Structure>, Added<ResourceNode>)>>,
    mut removed_structures: RemovedComponents<Structure>,
    mut removed_resources: RemovedComponents<ResourceNode>,
) {
    let removed = removed_structures.read().count() + removed_resources.read().count();
    if grid.version > 0 && added.is_empty() && removed == 0 {
        return;
    }
    let snapshot: Vec<(Vec3, f32)> = obstacles
        .iter()
        .filter(|(_, _, health)| !health.is_some_and(|health| health.current <= 0.0))
        .map(|(transform, selectable, _)| (transform.translation, selectable.radius))
        .collect();
    grid.rebuild(*map_bounds, &snapshot);
}

/// Gives terrain units whose straight line to the MoveOrder target is blocked an
/// A* path; direct movers keep no path. Chasers re-insert MoveOrder every frame,
/// so an existing path whose goal barely moved is kept as-is.
pub(crate) fn plan_unit_paths(
    mut commands: Commands,
    grid: Res<NavGrid>,
    changed: Query<
        (
            Entity,
            &Transform,
            &MoveOrder,
            &MovementDomain,
            Option<&PlannedPath>,
        ),
        Changed<MoveOrder>,
    >,
    stale: Query<Entity, (With<PlannedPath>, Without<MoveOrder>)>,
) {
    for entity in &stale {
        commands.entity(entity).try_remove::<PlannedPath>();
    }
    for (entity, transform, order, domain, existing) in &changed {
        if *domain != MovementDomain::Terrain {
            continue;
        }
        if let Some(path) = existing {
            if xz_distance(path.goal, order.target) < NAV_REPLAN_GOAL_TOLERANCE_M {
                continue;
            }
        }
        if grid.line_clear(transform.translation, order.target) {
            if existing.is_some() {
                commands.entity(entity).try_remove::<PlannedPath>();
            }
            continue;
        }
        match grid.find_path(transform.translation, order.target) {
            Some(waypoints) if !waypoints.is_empty() => {
                commands.entity(entity).insert(PlannedPath {
                    goal: order.target,
                    waypoints,
                    next: 0,
                });
            }
            _ => {
                commands.entity(entity).try_remove::<PlannedPath>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_pushes_overlapping_units_apart_evenly() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.5, 0.0, 0.0);
        let push = pair_separation_push(a, 0.4, b, 0.4).expect("overlapping pair must push");
        // min distance = 0.4 + 0.4 + gap; each side moves half the shortfall, away from the other.
        let min_dist = 0.8 + UNIT_SEPARATION_GAP_M;
        assert!(push.x < 0.0, "a is pushed away from b (negative x)");
        assert!((push.length() - (min_dist - 0.5) * 0.5).abs() < 1e-5);
        assert_eq!(push.y, 0.0, "separation never changes height");
    }

    #[test]
    fn separation_ignores_clear_pairs_and_handles_coincident_units() {
        assert!(
            pair_separation_push(Vec3::ZERO, 0.3, Vec3::new(1.0, 0.0, 0.0), 0.3).is_none(),
            "non-overlapping circles must not push"
        );
        let push = pair_separation_push(Vec3::ZERO, 0.3, Vec3::ZERO, 0.3)
            .expect("coincident units must still separate");
        assert!(
            push.length() > 0.0,
            "coincident pair gets a deterministic push"
        );
    }

    #[test]
    fn nav_grid_routes_around_a_wall() {
        let mut grid = NavGrid::default();
        // 10x10m map with a 3m-radius blocker in the middle: a straight crossing is
        // blocked, A* must find a detour and string-pull it into few waypoints.
        let bounds = MapBounds::from_size((10.0, 10.0));
        let wall = vec![(Vec3::ZERO, 1.5)];
        grid.rebuild(bounds, &wall);
        let from = Vec3::new(-4.0, 0.0, 0.0);
        let to = Vec3::new(4.0, 0.0, 0.0);
        assert!(
            !grid.line_clear(from, to),
            "wall must block the straight line"
        );
        let path = grid.find_path(from, to).expect("detour must exist");
        assert!(!path.is_empty());
        let last = *path.last().unwrap();
        assert!(xz_distance(last, to) < 1.0, "path must end near the goal");
        // Every leg of the pulled path must itself be clear.
        let mut cursor = from;
        for waypoint in &path {
            assert!(
                grid.line_clear(cursor, *waypoint),
                "leg must not cross the wall"
            );
            cursor = *waypoint;
        }
    }

    #[test]
    fn terrain_wall_rocks_block_the_nav_grid() {
        // A wall of rock obstacles across the middle must force A* detours,
        // exactly like structures do.
        let mut grid = NavGrid::default();
        let bounds = MapBounds::from_size((20.0, 20.0));
        let rocks: Vec<(Vec3, f32)> = (0..9)
            .map(|i| (Vec3::new(-6.0 + i as f32 * 1.5, 0.0, 0.0), 0.8))
            .collect();
        grid.rebuild(bounds, &rocks);
        let from = Vec3::new(0.0, 0.0, -6.0);
        let to = Vec3::new(0.0, 0.0, 6.0);
        assert!(
            !grid.line_clear(from, to),
            "rock wall blocks the straight line"
        );
        let path = grid
            .find_path(from, to)
            .expect("gap around the wall must exist");
        let mut cursor = from;
        for waypoint in &path {
            assert!(grid.line_clear(cursor, *waypoint));
            cursor = *waypoint;
        }
    }

    #[test]
    fn nav_grid_open_map_needs_no_path() {
        let mut grid = NavGrid::default();
        grid.rebuild(MapBounds::from_size((10.0, 10.0)), &[]);
        assert!(grid.line_clear(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 0.0, 4.0)));
    }
}
