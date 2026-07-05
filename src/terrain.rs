//! Real terrain elevation: RTS-style discrete plateaus connected by ramps.
//!
//! Maps declare `terrain_plateaus` (flat raised rectangles, in height LEVELS of
//! [`TERRAIN_LEVEL_HEIGHT`]) and `terrain_ramps` (rectangles sloping one level
//! up along a cardinal direction). A per-match [`TerrainHeightField`] resource
//! samples the resulting height anywhere; cliffs (steep cell-to-cell steps)
//! block ground movement and structure placement while ramps stay walkable and
//! air ignores elevation entirely. The ground mesh is generated from the same
//! field, so what you see is exactly what the simulation uses.

use bevy::prelude::*;

use crate::*;

/// World height of one terrain level.
pub(crate) const TERRAIN_LEVEL_HEIGHT: f32 = 1.4;
/// Height-field sample spacing (matches the nav grid cell).
pub(crate) const TERRAIN_CELL_M: f32 = 0.5;
/// Max height change a ground unit may climb over one nav-cell step; ramps stay
/// under it, cliffs exceed it.
pub(crate) const TERRAIN_MAX_STEP_M: f32 = 0.45;

/// A flat raised rectangle (map-local coordinates), `level` levels high.
#[derive(Clone, Copy)]
pub(crate) struct TerrainPlateauSpec {
    pub(crate) min: (f32, f32),
    pub(crate) max: (f32, f32),
    pub(crate) level: u8,
}

/// Uphill direction of a ramp rectangle.
#[derive(Clone, Copy)]
pub(crate) enum RampDirection {
    PlusX,
    MinusX,
    PlusZ,
    MinusZ,
}

/// A rectangle sloping from `level - 1` (at the downhill edge) to `level`.
#[derive(Clone, Copy)]
pub(crate) struct TerrainRampSpec {
    pub(crate) min: (f32, f32),
    pub(crate) max: (f32, f32),
    pub(crate) level: u8,
    pub(crate) direction: RampDirection,
}

pub(crate) const EMPTY_TERRAIN_PLATEAUS: &[TerrainPlateauSpec] = &[];
pub(crate) const EMPTY_TERRAIN_RAMPS: &[TerrainRampSpec] = &[];

/// The sampled elevation of the current match's map (flat when the map has no
/// plateaus — every height is 0.0 and all movement checks short-circuit).
#[derive(Resource, Default)]
pub(crate) struct TerrainHeightField {
    pub(crate) origin_x: f32,
    pub(crate) origin_z: f32,
    pub(crate) width: i32,
    pub(crate) depth: i32,
    pub(crate) heights: Vec<f32>,
}

impl TerrainHeightField {
    pub(crate) fn is_flat(&self) -> bool {
        self.heights.is_empty()
    }

    pub(crate) fn rebuild(&mut self, map: &SkirmishMapDef) {
        if map.terrain_plateaus.is_empty() && map.terrain_ramps.is_empty() {
            *self = Self::default();
            return;
        }
        self.origin_x = -map.size.0 * 0.5;
        self.origin_z = -map.size.1 * 0.5;
        self.width = (map.size.0 / TERRAIN_CELL_M).ceil() as i32 + 1;
        self.depth = (map.size.1 / TERRAIN_CELL_M).ceil() as i32 + 1;
        self.heights = vec![0.0; (self.width * self.depth) as usize];
        for cz in 0..self.depth {
            for cx in 0..self.width {
                // Sample position in map-local coordinates.
                let local_x = cx as f32 * TERRAIN_CELL_M;
                let local_z = cz as f32 * TERRAIN_CELL_M;
                let mut height = 0.0f32;
                for plateau in map.terrain_plateaus {
                    if local_x >= plateau.min.0
                        && local_x <= plateau.max.0
                        && local_z >= plateau.min.1
                        && local_z <= plateau.max.1
                    {
                        height = height.max(plateau.level as f32 * TERRAIN_LEVEL_HEIGHT);
                    }
                }
                for ramp in map.terrain_ramps {
                    if local_x >= ramp.min.0
                        && local_x <= ramp.max.0
                        && local_z >= ramp.min.1
                        && local_z <= ramp.max.1
                    {
                        let t = match ramp.direction {
                            RampDirection::PlusX => {
                                (local_x - ramp.min.0) / (ramp.max.0 - ramp.min.0).max(0.01)
                            }
                            RampDirection::MinusX => {
                                (ramp.max.0 - local_x) / (ramp.max.0 - ramp.min.0).max(0.01)
                            }
                            RampDirection::PlusZ => {
                                (local_z - ramp.min.1) / (ramp.max.1 - ramp.min.1).max(0.01)
                            }
                            RampDirection::MinusZ => {
                                (ramp.max.1 - local_z) / (ramp.max.1 - ramp.min.1).max(0.01)
                            }
                        };
                        let low = (ramp.level.saturating_sub(1)) as f32 * TERRAIN_LEVEL_HEIGHT;
                        let high = ramp.level as f32 * TERRAIN_LEVEL_HEIGHT;
                        height = height.max(low + (high - low) * t.clamp(0.0, 1.0));
                    }
                }
                self.heights[(cz * self.width + cx) as usize] = height;
            }
        }
    }

    fn sample_cell(&self, cx: i32, cz: i32) -> f32 {
        let cx = cx.clamp(0, self.width - 1);
        let cz = cz.clamp(0, self.depth - 1);
        self.heights[(cz * self.width + cx) as usize]
    }

    /// Bilinear height at a world position (0.0 on flat maps).
    pub(crate) fn height_at(&self, position: Vec3) -> f32 {
        if self.is_flat() {
            return 0.0;
        }
        let fx = (position.x - self.origin_x) / TERRAIN_CELL_M;
        let fz = (position.z - self.origin_z) / TERRAIN_CELL_M;
        let cx = fx.floor() as i32;
        let cz = fz.floor() as i32;
        let tx = (fx - cx as f32).clamp(0.0, 1.0);
        let tz = (fz - cz as f32).clamp(0.0, 1.0);
        let h00 = self.sample_cell(cx, cz);
        let h10 = self.sample_cell(cx + 1, cz);
        let h01 = self.sample_cell(cx, cz + 1);
        let h11 = self.sample_cell(cx + 1, cz + 1);
        h00 * (1.0 - tx) * (1.0 - tz)
            + h10 * tx * (1.0 - tz)
            + h01 * (1.0 - tx) * tz
            + h11 * tx * tz
    }

    /// True when a ground step from `from` to `to` climbs/drops a cliff.
    pub(crate) fn step_blocked(&self, from: Vec3, to: Vec3) -> bool {
        if self.is_flat() {
            return false;
        }
        // Supercover the segment at cell resolution so long steps can't tunnel.
        let distance = xz_distance(from, to);
        let samples = (distance / (TERRAIN_CELL_M * 0.5)).ceil().max(1.0) as i32;
        let mut previous = self.height_at(from);
        for index in 1..=samples {
            let t = index as f32 / samples as f32;
            let here = self.height_at(from.lerp(to, t));
            if (here - previous).abs() > TERRAIN_MAX_STEP_M {
                return true;
            }
            previous = here;
        }
        false
    }

    /// Where a camera ray meets the terrain (ray-march; flat maps use the
    /// y = 0 plane exactly like before).
    pub(crate) fn raycast(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<Vec3> {
        if self.is_flat() {
            let ray = bevy::math::Ray3d::new(ray_origin, Dir3::new(ray_direction).ok()?);
            return ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y));
        }
        let direction = ray_direction.normalize_or_zero();
        if direction == Vec3::ZERO {
            return None;
        }
        let mut t = 0.0f32;
        let mut previous = ray_origin;
        let max_distance = 400.0f32;
        while t < max_distance {
            t += TERRAIN_CELL_M * 0.5;
            let point = ray_origin + direction * t;
            let ground = self.height_at(point);
            if point.y <= ground {
                // Refine between the last-above and first-below samples.
                let mut low = previous;
                let mut high = point;
                for _ in 0..8 {
                    let mid = (low + high) * 0.5;
                    if mid.y <= self.height_at(mid) {
                        high = mid;
                    } else {
                        low = mid;
                    }
                }
                let mut hit = (low + high) * 0.5;
                hit.y = self.height_at(hit);
                return Some(hit);
            }
            previous = point;
        }
        None
    }

    pub(crate) fn max_height(&self) -> f32 {
        self.heights.iter().copied().fold(0.0, f32::max)
    }
}

/// Generates the terrain mesh (positions displaced by the field, normals
/// recomputed) — only called for maps with elevation.
pub(crate) fn terrain_mesh(field: &TerrainHeightField) -> Mesh {
    grid_terrain_mesh(
        field.origin_x,
        field.origin_z,
        field.width.max(2),
        field.depth.max(2),
        TERRAIN_CELL_M,
        |cx, cz| field.sample_cell(cx, cz),
    )
}

/// Flat maps get the same grid mesh (heights all zero) so they carry the
/// vertex-color ground variation too, instead of a single featureless quad.
pub(crate) fn flat_terrain_mesh(size: (f32, f32)) -> Mesh {
    let cell = 1.0;
    let width = (size.0 / cell).ceil() as i32 + 1;
    let depth = (size.1 / cell).ceil() as i32 + 1;
    grid_terrain_mesh(-size.0 * 0.5, -size.1 * 0.5, width, depth, cell, |_, _| 0.0)
}

/// Deterministic 0..1 hash for terrain tinting (no RNG: same map, same look).
fn terrain_hash01(ix: i32, iz: i32) -> f32 {
    let mut h = (ix as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((iz as u32).wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65_535.0
}

/// Smooth value noise sampled at world position with the given wavelength.
fn terrain_value_noise(x: f32, z: f32, wavelength: f32) -> f32 {
    let fx = x / wavelength;
    let fz = z / wavelength;
    let ix = fx.floor() as i32;
    let iz = fz.floor() as i32;
    let tx = fx - ix as f32;
    let tz = fz - iz as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let h00 = terrain_hash01(ix, iz);
    let h10 = terrain_hash01(ix + 1, iz);
    let h01 = terrain_hash01(ix, iz + 1);
    let h11 = terrain_hash01(ix + 1, iz + 1);
    h00 * (1.0 - sx) * (1.0 - sz) + h10 * sx * (1.0 - sz) + h01 * (1.0 - sx) * sz + h11 * sx * sz
}

/// Subtle two-octave ground tint: broad 9m patches plus fine 2.6m grain, a few
/// percent of luminance with a slight warm/dark bias — enough to break up the
/// flat sand color without reading as a texture or a grid.
pub(crate) fn terrain_vertex_color(x: f32, z: f32) -> [f32; 4] {
    let broad = terrain_value_noise(x, z, 9.0);
    let fine = terrain_value_noise(x + 43.7, z - 17.3, 2.6);
    let n = broad * 0.72 + fine * 0.28;
    // ~0.86..1.04 luminance, darker patches drift toward brown — strong enough
    // to survive the bright ambient + filmic tonemap.
    let tint = 0.86 + n * 0.18;
    let warm = 1.0 - (1.0 - n) * 0.07;
    [tint, tint * warm, tint * warm * warm, 1.0]
}

fn grid_terrain_mesh(
    origin_x: f32,
    origin_z: f32,
    width: i32,
    depth: i32,
    cell: f32,
    height_at_cell: impl Fn(i32, i32) -> f32,
) -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    let mut positions = Vec::with_capacity((width * depth) as usize);
    let mut uvs = Vec::with_capacity((width * depth) as usize);
    let mut colors = Vec::with_capacity((width * depth) as usize);
    for cz in 0..depth {
        for cx in 0..width {
            let x = origin_x + cx as f32 * cell;
            let z = origin_z + cz as f32 * cell;
            positions.push([x, height_at_cell(cx, cz), z]);
            uvs.push([
                cx as f32 / (width - 1) as f32,
                cz as f32 / (depth - 1) as f32,
            ]);
            colors.push(terrain_vertex_color(x, z));
        }
    }
    let mut indices = Vec::new();
    for cz in 0..depth - 1 {
        for cx in 0..width - 1 {
            let a = (cz * width + cx) as u32;
            let b = a + 1;
            let c = a + width as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_normals();
    mesh
}

/// Settles freshly spawned ground entities onto the terrain surface.
pub(crate) fn settle_new_entities_on_terrain(
    field: Res<TerrainHeightField>,
    mut spawned: Query<
        (&mut Transform, Option<&MovementDomain>),
        (
            Or<(
                Added<Unit>,
                Added<Structure>,
                Added<ResourceNode>,
                Added<TerrainWall>,
                Added<SupplyCrate>,
            )>,
            Without<MainCamera>,
        ),
    >,
) {
    if field.is_flat() {
        return;
    }
    for (mut transform, domain) in &mut spawned {
        let ground = field.height_at(transform.translation);
        match domain {
            Some(MovementDomain::Air) => {
                // Air keeps its own altitude but never spawns inside a plateau.
                transform.translation.y = transform.translation.y.max(ground + 0.6);
            }
            _ => transform.translation.y = ground,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_map() -> SkirmishMapDef {
        SkirmishMapDef {
            id: "terrain_test",
            godot_path: "bevy://maps/TerrainTest",
            name: "Terrain Test",
            name_key: "MAP_NAME_TERRAIN_TEST",
            players: 2,
            size: (40.0, 40.0),
            spawn_points: &[(6.0, 20.0), (34.0, 20.0)],
            resources: &[],
            neutral_tech: EMPTY_NEUTRAL_TECH,
            supply_crates: EMPTY_NAMED_CRATES,
            terrain_walls: EMPTY_TERRAIN_WALLS,
            // A level-1 plateau on the east half with one ramp in the middle.
            terrain_plateaus: &[TerrainPlateauSpec {
                min: (24.0, 0.0),
                max: (40.0, 40.0),
                level: 1,
            }],
            terrain_ramps: &[TerrainRampSpec {
                min: (18.0, 18.0),
                max: (24.0, 22.0),
                level: 1,
                direction: RampDirection::PlusX,
            }],
        }
    }

    #[test]
    fn heights_sample_plateau_ramp_and_flat() {
        let mut field = TerrainHeightField::default();
        field.rebuild(&test_map());
        // Map-local (10, 20) = flat ground; (30, 20) = plateau; ramp midpoint
        // sits at roughly half a level.
        let map = test_map();
        let flat = map_local_to_world(&map, (10.0, 20.0));
        let top = map_local_to_world(&map, (30.0, 20.0));
        let mid_ramp = map_local_to_world(&map, (21.0, 20.0));
        assert!(field.height_at(flat).abs() < 0.01);
        assert!((field.height_at(top) - TERRAIN_LEVEL_HEIGHT).abs() < 0.01);
        let ramp_height = field.height_at(mid_ramp);
        assert!(
            ramp_height > 0.3 && ramp_height < TERRAIN_LEVEL_HEIGHT - 0.3,
            "ramp midpoint must be between levels, got {ramp_height}"
        );
    }

    #[test]
    fn cliffs_block_steps_but_ramps_do_not() {
        let mut field = TerrainHeightField::default();
        field.rebuild(&test_map());
        let map = test_map();
        // Straight into the cliff face (north of the ramp).
        let below = map_local_to_world(&map, (23.0, 8.0));
        let above = map_local_to_world(&map, (25.0, 8.0));
        assert!(field.step_blocked(below, above), "cliff face must block");
        // Along the ramp.
        let ramp_low = map_local_to_world(&map, (18.5, 20.0));
        let ramp_high = map_local_to_world(&map, (23.5, 20.0));
        assert!(
            !field.step_blocked(ramp_low, ramp_high),
            "ramp must stay walkable"
        );
        // Flat ground is always free.
        let a = map_local_to_world(&map, (5.0, 5.0));
        let b = map_local_to_world(&map, (10.0, 10.0));
        assert!(!field.step_blocked(a, b));
    }

    #[test]
    fn terrain_raycast_hits_plateau_tops() {
        let mut field = TerrainHeightField::default();
        field.rebuild(&test_map());
        let map = test_map();
        let over_plateau = map_local_to_world(&map, (30.0, 20.0)) + Vec3::Y * 20.0;
        let hit = field
            .raycast(over_plateau, Vec3::new(0.0, -1.0, 0.0))
            .expect("straight-down ray must hit");
        assert!(
            (hit.y - TERRAIN_LEVEL_HEIGHT).abs() < 0.1,
            "ray lands on the plateau top, got y={}",
            hit.y
        );
    }
}

#[cfg(test)]
mod highland_tests {
    use super::*;

    fn highland() -> &'static SkirmishMapDef {
        SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == "highland_bastion")
            .expect("highland_bastion registered")
    }

    #[test]
    fn nav_grid_blocks_cliffs_and_keeps_ramps() {
        let map = highland();
        let mut field = TerrainHeightField::default();
        field.rebuild(map);
        let mut grid = NavGrid::default();
        grid.rebuild_with_terrain(MapBounds::from_map(map), &[], &field);
        // Straight across the west base cliff (no ramp): the supercover line
        // crosses blocked cliff cells.
        let valley = map_local_to_world(map, (24.0, 10.0));
        let base_top = map_local_to_world(map, (12.0, 30.0));
        assert!(
            !grid.line_clear(valley, base_top),
            "cliff face must break line of sight for movement"
        );
        // A* still reaches the plateau (through the ramp at local x 20..27).
        let path = grid.find_path(valley, base_top);
        assert!(path.is_some(), "ramp must make the plateau reachable");
        // And the crystal high ground is reachable from the valley too.
        let crystal = map_local_to_world(map, (30.0, 10.0));
        assert!(grid.find_path(valley, crystal).is_some());
    }

    #[test]
    fn base_plateaus_are_buildable_and_ramps_are_not() {
        let map = highland();
        let mut field = TerrainHeightField::default();
        field.rebuild(map);
        let spawn = map_local_to_world(map, (12.0, 30.0));
        assert!(terrain_site_is_buildable(&field, spawn, 2.0));
        let ramp = map_local_to_world(map, (23.5, 30.0));
        assert!(!terrain_site_is_buildable(&field, ramp, 2.0));
    }
}
