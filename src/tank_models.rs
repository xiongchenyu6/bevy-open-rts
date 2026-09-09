//! Procedural tank models. Hull, running gear, turret and gun are assembled
//! from primitives in code (the approach Claude-of-Tanks uses) instead of
//! gluing kenney space-kit crafts together. Conventions: forward is -Z, the
//! ground plane is y = 0, and the turret child is named "turret" so
//! `TurretNode` traverses, elevates and recoils it.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use crate::*;

/// Material slot a part is rendered with. `Accent` is kenney's gold, which
/// `apply_team_color_materials` repaints in the owning player's color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TankBucket {
    Paint,
    Dark,
    Accent,
    Track,
    Glow,
}

const BUCKET_ORDER: [TankBucket; 5] = [
    TankBucket::Paint,
    TankBucket::Dark,
    TankBucket::Accent,
    TankBucket::Track,
    TankBucket::Glow,
];

#[derive(Clone, Copy, Debug)]
pub(crate) struct TankLook {
    pub(crate) paint: Color,
    pub(crate) glow: LinearRgba,
}

#[derive(Default)]
pub(crate) struct PartSet(Vec<(TankBucket, Mesh)>);

impl PartSet {
    fn add(&mut self, bucket: TankBucket, mesh: Mesh, at: Vec3) {
        self.0.push((bucket, mesh.translated_by(at)));
    }

    fn add_rot(&mut self, bucket: TankBucket, mesh: Mesh, at: Vec3, rotation: Quat) {
        self.0.push((
            bucket,
            mesh.transformed_by(Transform {
                translation: at,
                rotation,
                scale: Vec3::ONE,
            }),
        ));
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// One merged mesh per bucket, in `BUCKET_ORDER`.
    pub(crate) fn merged(self) -> Vec<(TankBucket, Mesh)> {
        let mut out: Vec<(TankBucket, Mesh)> = Vec::new();
        for bucket in BUCKET_ORDER {
            let mut merged: Option<Mesh> = None;
            for (part_bucket, mesh) in &self.0 {
                if *part_bucket != bucket {
                    continue;
                }
                match merged.as_mut() {
                    None => merged = Some(mesh.clone()),
                    Some(target) => target
                        .merge(mesh)
                        .expect("tank part meshes share position/normal/uv attributes"),
                }
            }
            if let Some(mesh) = merged {
                out.push((bucket, mesh));
            }
        }
        out
    }
}

/// A rotating detail (drill bit, radar dish) parented to the hull.
pub(crate) struct SpinnerSpec {
    pub(crate) pivot: Vec3,
    pub(crate) axis: Vec3,
    pub(crate) rate: f32,
    pub(crate) parts: PartSet,
}

pub(crate) struct TankModelSpec {
    pub(crate) look: TankLook,
    pub(crate) hull: PartSet,
    pub(crate) turret: Option<(Vec3, PartSet)>,
    pub(crate) spinner: Option<SpinnerSpec>,
}

#[derive(Component)]
pub(crate) struct TankSpinner {
    pub(crate) axis: Vec3,
    pub(crate) rate: f32,
    pub(crate) rest: Quat,
}

pub(crate) fn animate_tank_spinners(
    time: Res<Time>,
    mut spinners: Query<(&TankSpinner, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (spinner, mut transform) in &mut spinners {
        transform.rotation = spinner.rest * Quat::from_axis_angle(spinner.axis, t * spinner.rate);
    }
}

// ---------------------------------------------------------------------------
// Geometry primitives
// ---------------------------------------------------------------------------

/// Flat-shaded triangle soup builder. Faces are wound so their normal points
/// away from `center`, which keeps convex lofts correct without hand-winding.
struct FlatMesh {
    center: Vec3,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl FlatMesh {
    fn new(center: Vec3) -> Self {
        Self {
            center,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn face(&mut self, mut pts: Vec<Vec3>) {
        let centroid = pts.iter().sum::<Vec3>() / pts.len() as f32;
        let mut normal = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        if normal.length_squared() < 1e-10 {
            return;
        }
        if normal.dot(centroid - self.center) < 0.0 {
            pts.reverse();
            normal = -normal;
        }
        let normal = normal.normalize();
        let base = self.positions.len() as u32;
        for (i, p) in pts.iter().enumerate() {
            self.positions.push(p.to_array());
            self.normals.push(normal.to_array());
            let uv = match i % 4 {
                0 => [0.0, 0.0],
                1 => [1.0, 0.0],
                2 => [1.0, 1.0],
                _ => [0.0, 1.0],
            };
            self.uvs.push(uv);
        }
        for i in 1..(pts.len() as u32 - 1) {
            self.indices.extend([base, base + i, base + i + 1]);
        }
    }

    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Convex loft between two rings with the same point count, capped at both
/// ends.
fn loft(bottom: &[Vec3], top: &[Vec3]) -> Mesh {
    debug_assert_eq!(bottom.len(), top.len());
    let n = bottom.len();
    let center = (bottom.iter().sum::<Vec3>() + top.iter().sum::<Vec3>()) / (2 * n) as f32;
    let mut flat = FlatMesh::new(center);
    for i in 0..n {
        let j = (i + 1) % n;
        flat.face(vec![bottom[i], bottom[j], top[j], top[i]]);
    }
    flat.face(top.to_vec());
    flat.face(bottom.to_vec());
    flat.build()
}

/// Extrudes an XZ plan between `y0` and `y1`; the top ring is scaled toward
/// the plan centroid by `inset` and shifted by `top_shift_z`.
fn poly_loft(plan: &[[f32; 2]], y0: f32, y1: f32, inset: f32, top_shift_z: f32) -> Mesh {
    let cx = plan.iter().map(|p| p[0]).sum::<f32>() / plan.len() as f32;
    let cz = plan.iter().map(|p| p[1]).sum::<f32>() / plan.len() as f32;
    let bottom: Vec<Vec3> = plan.iter().map(|p| Vec3::new(p[0], y0, p[1])).collect();
    let top: Vec<Vec3> = plan
        .iter()
        .map(|p| {
            Vec3::new(
                cx + (p[0] - cx) * inset,
                y1,
                cz + (p[1] - cz) * inset + top_shift_z,
            )
        })
        .collect();
    loft(&bottom, &top)
}

/// Sloped box: a bottom rectangle (half-width, front z, rear z) lofted to a
/// top rectangle. Sloped glacis and rear plates fall out of the offsets.
#[allow(clippy::too_many_arguments)]
fn frustum(
    bottom_hw: f32,
    bottom_front: f32,
    bottom_rear: f32,
    top_hw: f32,
    top_front: f32,
    top_rear: f32,
    y0: f32,
    y1: f32,
) -> Mesh {
    let bottom = [
        Vec3::new(-bottom_hw, y0, bottom_front),
        Vec3::new(bottom_hw, y0, bottom_front),
        Vec3::new(bottom_hw, y0, bottom_rear),
        Vec3::new(-bottom_hw, y0, bottom_rear),
    ];
    let top = [
        Vec3::new(-top_hw, y1, top_front),
        Vec3::new(top_hw, y1, top_front),
        Vec3::new(top_hw, y1, top_rear),
        Vec3::new(-top_hw, y1, top_rear),
    ];
    loft(&bottom, &top)
}

fn cuboid(w: f32, h: f32, d: f32) -> Mesh {
    Mesh::from(Cuboid::new(w, h, d))
}

fn cyl_y(r: f32, h: f32, segments: u32) -> Mesh {
    Mesh::from(Cylinder::new(r, h).mesh().resolution(segments))
}

/// Cylinder whose axis runs along X.
fn cyl_x(r: f32, len: f32, segments: u32) -> Mesh {
    cyl_y(r, len, segments).rotated_by(Quat::from_rotation_z(FRAC_PI_2))
}

/// Cylinder whose axis runs along Z.
fn cyl_z(r: f32, len: f32, segments: u32) -> Mesh {
    cyl_y(r, len, segments).rotated_by(Quat::from_rotation_x(FRAC_PI_2))
}

/// Tapered tube along -Z: `r_front` at the muzzle end (negative z).
fn taper_z(r_front: f32, r_rear: f32, len: f32, segments: u32) -> Mesh {
    Mesh::from(
        ConicalFrustum {
            radius_top: r_front,
            radius_bottom: r_rear,
            height: len,
        }
        .mesh()
        .resolution(segments),
    )
    .rotated_by(Quat::from_rotation_x(-FRAC_PI_2))
}

fn sphere(r: f32) -> Mesh {
    Sphere::new(r).mesh().uv(14, 9)
}

/// Ring around the X axis (a wheel rim / sprocket tooth ring).
fn ring_x(major: f32, minor: f32) -> Mesh {
    Mesh::from(
        Torus::new(minor, major)
            .mesh()
            .minor_resolution(6)
            .major_resolution(20),
    )
    .rotated_by(Quat::from_rotation_z(FRAC_PI_2))
}

/// Ring around the Z axis (capacitor coils along a gun).
fn ring_z(major: f32, minor: f32) -> Mesh {
    Mesh::from(
        Torus::new(minor, major)
            .mesh()
            .minor_resolution(6)
            .major_resolution(20),
    )
    .rotated_by(Quat::from_rotation_x(FRAC_PI_2))
}

/// Rotation that points a box's local +Z along the (dz, dy) direction in the
/// YZ plane; used to lay boxes along a track profile or a sloped plate.
fn tilt_x(dy: f32, dz: f32) -> Quat {
    Quat::from_rotation_x((-dy).atan2(dz))
}

// ---------------------------------------------------------------------------
// Shared assemblies
// ---------------------------------------------------------------------------

/// Tracked running gear + hull. Everything derives from overall length,
/// width and road-wheel radius so every vehicle keeps real tank proportions
/// (length about 1.9x width, roof about 3.3 wheel radii up).
struct Chassis {
    len: f32,
    width: f32,
    wheel_r: f32,
    wheels: usize,
    skirt: bool,
    /// Fraction of the length taken by the sloped glacis.
    glacis: f32,
}

impl Chassis {
    const fn new(len: f32, width: f32, wheel_r: f32, wheels: usize, skirt: bool) -> Self {
        Self {
            len,
            width,
            wheel_r,
            wheels,
            skirt,
            glacis: 0.26,
        }
    }

    fn front(&self) -> f32 {
        -self.len * 0.5
    }

    fn rear(&self) -> f32 {
        self.len * 0.5
    }

    fn track_w(&self) -> f32 {
        self.width * 0.2
    }

    fn half_w(&self) -> f32 {
        self.width * 0.5 - self.track_w()
    }

    fn track_x(&self) -> f32 {
        self.half_w() + self.track_w() * 0.5
    }

    /// Fenders cover only the inner part of each track so the dark track runs
    /// flank the hull from the RTS camera's high angle.
    fn fender_hw(&self) -> f32 {
        self.half_w() + self.track_w() * 0.4
    }

    fn roof_hw(&self) -> f32 {
        self.half_w() + self.track_w() * 0.1
    }

    fn belt_y(&self) -> f32 {
        self.wheel_r * 2.2
    }

    fn floor_y(&self) -> f32 {
        self.wheel_r * 0.7
    }

    fn roof_y(&self) -> f32 {
        self.wheel_r * 3.3
    }

    fn glacis_top_z(&self) -> f32 {
        self.front() + self.len * self.glacis
    }

    fn rear_top_z(&self) -> f32 {
        self.rear() - self.len * 0.08
    }

    fn track_top_y(&self) -> f32 {
        self.wheel_r * 2.0 + 0.04
    }

    fn wheel_zs(&self) -> Vec<f32> {
        let span = self.len - self.wheel_r * 3.6;
        let start = self.front() + self.wheel_r * 1.8;
        (0..self.wheels)
            .map(|i| start + span * i as f32 / (self.wheels - 1).max(1) as f32)
            .collect()
    }

    fn turret_pivot(&self, z: f32) -> Vec3 {
        Vec3::new(0.0, self.roof_y(), z)
    }
}

fn build_tracked_chassis(parts: &mut PartSet, c: &Chassis) {
    use TankBucket::*;
    let (front, rear) = (c.front(), c.rear());
    let mid_z = 0.0;
    let (floor, belt, roof) = (c.floor_y(), c.belt_y(), c.roof_y());
    let track_w = c.track_w();
    let half_w = c.half_w();
    parts.add(
        Paint,
        cuboid(half_w * 2.0, belt - floor, c.len - 0.16),
        Vec3::new(0.0, (floor + belt) * 0.5, mid_z),
    );
    // Upper hull: fenders at the belt line, sloped glacis and rear plate up
    // to the roof.
    parts.add(
        Paint,
        frustum(
            c.fender_hw(),
            front,
            rear,
            c.roof_hw(),
            c.glacis_top_z(),
            c.rear_top_z(),
            belt - 0.02,
            roof,
        ),
        Vec3::ZERO,
    );

    let top_y = c.track_top_y();
    let r = c.wheel_r;
    for side in [-1.0f32, 1.0] {
        let x = side * c.track_x();
        // Track band: boxes laid along a closed idler/top/sprocket/bottom loop.
        let profile = [
            (front + r * 0.1, r + 0.02),
            (front + r * 1.5, top_y),
            (rear - r * 1.5, top_y),
            (rear - r * 0.1, r + 0.05),
            (rear - r * 1.3, 0.02),
            (front + r * 1.3, 0.02),
        ];
        for i in 0..profile.len() {
            let (z0, y0) = profile[i];
            let (z1, y1) = profile[(i + 1) % profile.len()];
            let (dz, dy) = (z1 - z0, y1 - y0);
            let len = (dz * dz + dy * dy).sqrt();
            parts.add_rot(
                Track,
                cuboid(track_w, r * 0.22, len + 0.02),
                Vec3::new(x, (y0 + y1) * 0.5, (z0 + z1) * 0.5),
                tilt_x(dy, dz),
            );
        }
        // Filler behind the band so the loop never reads hollow.
        parts.add(
            Dark,
            cuboid(track_w * 0.55, top_y - r * 0.5, c.len - r * 2.6),
            Vec3::new(x, top_y * 0.5 + 0.02, mid_z),
        );
        for z in c.wheel_zs() {
            parts.add(Track, cyl_x(r, track_w * 0.8, 18), Vec3::new(x, r, z));
            parts.add(
                Paint,
                cyl_x(r * 0.55, track_w * 0.86, 12),
                Vec3::new(x, r, z),
            );
        }
        parts.add(
            Dark,
            cyl_x(r * 0.7, track_w * 0.6, 14),
            Vec3::new(x, r + 0.02, front + r * 0.35),
        );
        parts.add(
            Dark,
            cyl_x(r * 0.75, track_w * 0.6, 14),
            Vec3::new(x, r + 0.05, rear - r * 0.35),
        );
        parts.add(
            Dark,
            ring_x(r * 0.75, r * 0.08),
            Vec3::new(x, r + 0.05, rear - r * 0.35),
        );
        let skirt_x = side * (half_w + track_w + 0.015);
        if c.skirt {
            let skirt_bottom = r * 1.15;
            let skirt_top = top_y - 0.02;
            parts.add(
                Paint,
                cuboid(0.03, skirt_top - skirt_bottom, c.len - r * 2.6),
                Vec3::new(skirt_x, (skirt_top + skirt_bottom) * 0.5, mid_z),
            );
            parts.add(
                Accent,
                cuboid(0.015, r * 0.35, c.len * 0.5),
                Vec3::new(skirt_x + side * 0.015, skirt_top - r * 0.4, mid_z),
            );
        } else {
            parts.add(
                Accent,
                cuboid(0.015, r * 0.3, c.len * 0.45),
                Vec3::new(side * (c.fender_hw() + 0.005), belt + r * 0.3, mid_z),
            );
        }
        // Exhaust boxes, headlights and tow hooks.
        parts.add(
            Dark,
            cuboid(r * 0.8, r * 0.5, r * 0.4),
            Vec3::new(side * c.roof_hw() * 0.6, roof - r * 0.6, rear + r * 0.1),
        );
        parts.add(
            Accent,
            cuboid(r * 0.5, r * 0.3, r * 0.25),
            Vec3::new(side * c.roof_hw() * 0.75, belt + r * 0.35, front - 0.005),
        );
        parts.add(
            Dark,
            cuboid(r * 0.3, r * 0.4, r * 0.3),
            Vec3::new(side * half_w * 0.7, belt - r * 0.3, front - r * 0.35),
        );
    }
    // Roof edge seams so the deck reads as plated armor rather than a slab.
    let roof_len = c.rear_top_z() - c.glacis_top_z();
    for side in [-1.0f32, 1.0] {
        parts.add(
            Dark,
            cuboid(0.02, 0.012, roof_len * 0.92),
            Vec3::new(
                side * (c.roof_hw() - 0.05),
                roof + 0.004,
                (c.rear_top_z() + c.glacis_top_z()) * 0.5,
            ),
        );
    }
    // Engine deck grilles.
    for k in 0..3 {
        parts.add(
            Dark,
            cuboid(c.roof_hw() * 1.3, 0.02, r * 0.3),
            Vec3::new(
                0.0,
                roof + 0.01,
                c.rear_top_z() - r * 0.7 - k as f32 * r * 0.65,
            ),
        );
    }
    // Driver hatch seated on the glacis slope.
    let dz = c.glacis_top_z() - front;
    let dy = roof - (belt - 0.02);
    let n = Vec2::new(dz, -dy).normalize();
    let mid = Vec3::new(
        -c.roof_hw() * 0.45,
        (roof + belt) * 0.5 + n.x * 0.02,
        (c.glacis_top_z() + front) * 0.5 + n.y * 0.02,
    );
    parts.add_rot(Paint, cuboid(r * 1.4, 0.025, r * 1.1), mid, tilt_x(dy, dz));
}

/// Wheeled running gear + body for rovers and buggies: exposed tires on
/// stub axles, wheel arches, a low body and a cab.
struct Wheeled {
    len: f32,
    width: f32,
    wheel_r: f32,
    axles: usize,
}

impl Wheeled {
    fn wheel_w(&self) -> f32 {
        self.width * 0.16
    }

    fn body_hw(&self) -> f32 {
        self.width * 0.5 - self.wheel_w() - 0.02
    }

    fn floor_y(&self) -> f32 {
        self.wheel_r * 0.9
    }

    fn deck_y(&self) -> f32 {
        self.wheel_r * 2.3
    }

    fn axle_zs(&self) -> Vec<f32> {
        let span = self.len - self.wheel_r * 3.0;
        let start = -self.len * 0.5 + self.wheel_r * 1.5;
        (0..self.axles)
            .map(|i| start + span * i as f32 / (self.axles - 1).max(1) as f32)
            .collect()
    }
}

fn build_wheeled_chassis(parts: &mut PartSet, w: &Wheeled) {
    use TankBucket::*;
    let r = w.wheel_r;
    let (floor, deck) = (w.floor_y(), w.deck_y());
    let hw = w.body_hw();
    let front = -w.len * 0.5;
    let rear = w.len * 0.5;
    parts.add(
        Paint,
        cuboid(hw * 2.0, deck - floor, w.len - r * 0.6),
        Vec3::new(0.0, (floor + deck) * 0.5, 0.0),
    );
    // Sloped nose and rear plate.
    parts.add(
        Paint,
        frustum(
            hw,
            front,
            front + r * 1.2,
            hw * 0.9,
            front + r * 0.3,
            front + r * 1.2,
            floor,
            deck + r * 0.1,
        ),
        Vec3::ZERO,
    );
    // Axle beam under the body.
    parts.add(
        Dark,
        cuboid(w.width - w.wheel_w() * 0.6, r * 0.3, w.len - r * 2.0),
        Vec3::new(0.0, r * 0.95, 0.0),
    );
    for side in [-1.0f32, 1.0] {
        let x = side * (w.width * 0.5 - w.wheel_w() * 0.5);
        for z in w.axle_zs() {
            parts.add(Track, cyl_x(r, w.wheel_w(), 18), Vec3::new(x, r, z));
            parts.add(
                Paint,
                cyl_x(r * 0.5, w.wheel_w() * 1.05, 12),
                Vec3::new(x, r, z),
            );
            // Wheel arch over the tire.
            parts.add(
                Paint,
                cuboid(w.wheel_w() + 0.04, r * 0.25, r * 2.3),
                Vec3::new(x, r * 2.15, z),
            );
        }
        parts.add(
            Accent,
            cuboid(0.015, r * 0.35, w.len * 0.45),
            Vec3::new(side * (hw + 0.005), deck - r * 0.35, r * 0.2),
        );
        parts.add(
            Accent,
            cuboid(r * 0.45, r * 0.3, r * 0.2),
            Vec3::new(side * hw * 0.65, deck - r * 0.25, front - 0.005),
        );
    }
    // Bumpers.
    parts.add(
        Dark,
        cuboid(hw * 2.0, r * 0.35, r * 0.25),
        Vec3::new(0.0, floor + r * 0.3, front - r * 0.1),
    );
    parts.add(
        Dark,
        cuboid(hw * 2.0, r * 0.35, r * 0.25),
        Vec3::new(0.0, floor + r * 0.3, rear + r * 0.1),
    );
}

#[derive(Clone, Copy)]
enum Brake {
    Single,
    Double,
}

/// Gun tube along -Z from the mantlet face at `z0`, height `y`.
fn build_gun(parts: &mut PartSet, z0: f32, y: f32, r: f32, len: f32, brake: Brake, evac: bool) {
    use TankBucket::*;
    parts.add(
        Paint,
        cyl_z(r * 1.8, r * 3.0, 16),
        Vec3::new(0.0, y, z0 - r * 1.5),
    );
    parts.add(
        Paint,
        taper_z(r * 0.85, r * 1.15, len, 16),
        Vec3::new(0.0, y, z0 - len * 0.5),
    );
    if evac {
        parts.add(
            Paint,
            cyl_z(r * 1.45, len * 0.13, 16),
            Vec3::new(0.0, y, z0 - len * 0.58),
        );
    }
    match brake {
        Brake::Single => {
            parts.add(
                Dark,
                cyl_z(r * 1.5, r * 2.4, 14),
                Vec3::new(0.0, y, z0 - len + r * 1.2),
            );
        }
        Brake::Double => {
            parts.add(
                Dark,
                cyl_z(r * 0.8, r * 5.0, 12),
                Vec3::new(0.0, y, z0 - len + r * 2.5),
            );
            parts.add(
                Dark,
                cyl_z(r * 1.9, r * 1.1, 14),
                Vec3::new(0.0, y, z0 - len + r * 1.0),
            );
            parts.add(
                Dark,
                cyl_z(r * 1.9, r * 1.1, 14),
                Vec3::new(0.0, y, z0 - len + r * 3.6),
            );
        }
    }
}

/// Octagonal turret plan, forward at negative z.
fn oct_plan(front_z: f32, rear_z: f32, front_hw: f32, mid_hw: f32, rear_hw: f32) -> Vec<[f32; 2]> {
    vec![
        [-front_hw, front_z],
        [front_hw, front_z],
        [mid_hw, front_z * 0.4],
        [mid_hw, rear_z * 0.5],
        [rear_hw, rear_z],
        [-rear_hw, rear_z],
        [-mid_hw, rear_z * 0.5],
        [-mid_hw, front_z * 0.4],
    ]
}

/// Standard turret body: lofted octagon, roof accent stripe, cupola,
/// periscope, mantlet block, smoke launchers and antenna. `w` is the turret
/// half-width, `len` its total length; the front face sits at -len*0.55.
fn turret_body(parts: &mut PartSet, w: f32, len: f32, h: f32) -> f32 {
    use TankBucket::*;
    let front = -len * 0.55;
    let rear = len * 0.45;
    parts.add(
        Paint,
        poly_loft(
            &oct_plan(front, rear, w * 0.55, w, w * 0.8),
            0.0,
            h,
            0.8,
            0.0,
        ),
        Vec3::ZERO,
    );
    parts.add(
        Accent,
        cuboid(w * 1.2, 0.02, h * 0.4),
        Vec3::new(0.0, h + 0.01, rear * 0.45),
    );
    cupola(parts, Vec3::new(-w * 0.4, h + 0.02, rear * 0.2), w * 0.28);
    parts.add(
        Dark,
        cuboid(w * 0.25, h * 0.25, w * 0.25),
        Vec3::new(w * 0.42, h + 0.02, front * 0.2),
    );
    let mantlet_z = front - h * 0.3;
    parts.add(
        Paint,
        cuboid(w * 0.8, h * 0.9, h * 0.6),
        Vec3::new(0.0, h * 0.45, mantlet_z),
    );
    for side in [-1.0f32, 1.0] {
        smoke_launchers(
            parts,
            Vec3::new(side * w * 0.8, h * 0.6, front * 0.55),
            side,
            h * 0.12,
        );
    }
    antenna(parts, Vec3::new(w * 0.7, h * 0.9, rear * 0.8), h * 1.6);
    mantlet_z - h * 0.3
}

/// Dark turret race around the pivot; the ring is what separates turret
/// from hull at RTS zoom.
fn turret_ring(parts: &mut PartSet, pivot: Vec3, r: f32) {
    parts.add(
        TankBucket::Dark,
        cyl_y(r, 0.03, 24),
        pivot + Vec3::Y * 0.005,
    );
}

fn cupola(parts: &mut PartSet, at: Vec3, r: f32) {
    use TankBucket::*;
    parts.add(Paint, cyl_y(r, r * 0.5, 14), at + Vec3::Y * r * 0.25);
    parts.add(Dark, cyl_y(r * 0.7, 0.02, 12), at + Vec3::Y * r * 0.5);
}

fn antenna(parts: &mut PartSet, at: Vec3, h: f32) {
    parts.add(TankBucket::Dark, cyl_y(0.008, h, 6), at + Vec3::Y * h * 0.5);
}

fn smoke_launchers(parts: &mut PartSet, at: Vec3, side: f32, r: f32) {
    for k in 0..3 {
        parts.add_rot(
            TankBucket::Dark,
            cyl_z(r, r * 4.0, 8),
            at + Vec3::new(side * k as f32 * r * 2.4, 0.0, k as f32 * r * 0.8),
            Quat::from_rotation_y(side * 0.35),
        );
    }
}

/// Box launcher with a grid of dark tubes on its front face.
fn missile_pod(parts: &mut PartSet, at: Vec3, w: f32, h: f32, d: f32, cols: usize, rows: usize) {
    use TankBucket::*;
    parts.add(Paint, cuboid(w, h, d), at);
    parts.add(
        Accent,
        cuboid(w * 0.9, 0.015, d * 0.2),
        at + Vec3::new(0.0, h * 0.5 + 0.01, d * 0.2),
    );
    let tube_r = (w / cols as f32).min(h / rows as f32) * 0.32;
    for i in 0..cols {
        for j in 0..rows {
            let x = (i as f32 + 0.5) / cols as f32 - 0.5;
            let y = (j as f32 + 0.5) / rows as f32 - 0.5;
            parts.add(
                Dark,
                cyl_z(tube_r, 0.05, 10),
                at + Vec3::new(x * w, y * h, -d * 0.5 - 0.01),
            );
        }
    }
}

fn hover_chassis(parts: &mut PartSet, len: f32, width: f32) {
    use TankBucket::*;
    let (hz, hx) = (len * 0.5, width * 0.5);
    let plan = [
        [-hx * 0.55, -hz],
        [hx * 0.55, -hz],
        [hx, -hz * 0.5],
        [hx, hz * 0.7],
        [hx * 0.68, hz],
        [-hx * 0.68, hz],
        [-hx, hz * 0.7],
        [-hx, -hz * 0.5],
    ];
    let skirt_top = width * 0.28;
    let roof = width * 0.5;
    // Hull floats over a flared skirt; thrusters glow underneath so the
    // hover reads from the top-down camera.
    parts.add(
        Paint,
        poly_loft(&plan, skirt_top, roof, 0.72, -0.05),
        Vec3::ZERO,
    );
    parts.add(
        Dark,
        poly_loft(&plan, skirt_top * 0.55, skirt_top + 0.02, 1.0, 0.0),
        Vec3::ZERO,
    );
    let flare: Vec<[f32; 2]> = plan.iter().map(|p| [p[0] * 1.08, p[1] * 1.08]).collect();
    parts.add(
        Dark,
        poly_loft(&flare, skirt_top * 0.35, skirt_top * 0.6, 0.93, 0.0),
        Vec3::ZERO,
    );
    for (x, z) in [
        (-hx * 0.55, -hz * 0.55),
        (hx * 0.55, -hz * 0.55),
        (-hx * 0.55, hz * 0.55),
        (hx * 0.55, hz * 0.55),
    ] {
        parts.add(
            Dark,
            cyl_y(width * 0.12, 0.05, 12),
            Vec3::new(x, skirt_top * 0.3, z),
        );
        parts.add(
            Glow,
            cyl_y(width * 0.09, 0.02, 12),
            Vec3::new(x, skirt_top * 0.2, z),
        );
    }
    for side in [-1.0f32, 1.0] {
        parts.add(
            Accent,
            cuboid(0.03, 0.05, len * 0.45),
            Vec3::new(side * (hx + 0.01), roof - 0.1, 0.05),
        );
        parts.add(
            Dark,
            cuboid(width * 0.16, 0.08, 0.08),
            Vec3::new(side * hx * 0.45, roof - 0.1, hz * 0.95),
        );
    }
    for k in 0..3 {
        parts.add(
            Dark,
            cuboid(width * 0.5, 0.02, 0.04),
            Vec3::new(0.0, roof + 0.01, hz * 0.45 + k as f32 * 0.1),
        );
    }
}

// ---------------------------------------------------------------------------
// Vehicle builders
// ---------------------------------------------------------------------------

fn look(paint: [f32; 3], glow: [f32; 3]) -> TankLook {
    TankLook {
        paint: Color::srgb(paint[0], paint[1], paint[2]),
        glow: LinearRgba::rgb(glow[0], glow[1], glow[2]),
    }
}

fn spec(look: TankLook, hull: PartSet, turret: Option<(Vec3, PartSet)>) -> TankModelSpec {
    TankModelSpec {
        look,
        hull,
        turret,
        spinner: None,
    }
}

fn build_tank() -> TankModelSpec {
    let c = Chassis::new(2.6, 1.36, 0.18, 6, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let pivot = c.turret_pivot(-0.05);
    turret_ring(&mut hull, pivot, 0.5);
    let mut turret = PartSet::default();
    let gun_z = turret_body(&mut turret, 0.42, 1.0, 0.26);
    build_gun(&mut turret, gun_z, 0.12, 0.045, 1.35, Brake::Single, true);
    spec(
        look([0.58, 0.60, 0.56], [0.0, 0.0, 0.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_hammer_siege_tank() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.6, 1.45, 0.19, 7, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    // Recoil spades folded at the rear.
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.26, 0.05, 0.34),
            Vec3::new(side * 0.45, 0.4, c.rear() + 0.02),
            Quat::from_rotation_x(-0.6),
        );
    }
    let pivot = c.turret_pivot(0.1);
    turret_ring(&mut hull, pivot, 0.6);
    let mut turret = PartSet::default();
    let gun_z = turret_body(&mut turret, 0.52, 1.25, 0.34);
    turret.add(Paint, cuboid(0.8, 0.26, 0.36), Vec3::new(0.0, 0.16, 0.7));
    turret.add(Accent, cuboid(0.8, 0.015, 0.08), Vec3::new(0.0, 0.3, 0.86));
    for side in [-1.0f32, 1.0] {
        turret.add(
            Dark,
            cuboid(0.06, 0.22, 0.5),
            Vec3::new(side * 0.38, 0.16, -0.55),
        );
    }
    turret.add(Glow, ring_z(0.13, 0.02), Vec3::new(0.0, 0.16, gun_z - 0.02));
    build_gun(&mut turret, gun_z, 0.16, 0.075, 1.35, Brake::Double, true);
    spec(
        look([0.5, 0.5, 0.52], [1.0, 0.45, 0.1]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_railgun_tank() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.7, 1.34, 0.17, 7, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    // Capacitor bank on the rear deck.
    for side in [-1.0f32, 1.0] {
        hull.add(
            Dark,
            cyl_z(0.1, 0.5, 12),
            Vec3::new(side * 0.32, c.roof_y() + 0.06, 0.85),
        );
        hull.add(
            Glow,
            ring_z(0.105, 0.015),
            Vec3::new(side * 0.32, c.roof_y() + 0.06, 0.68),
        );
        hull.add(
            Glow,
            ring_z(0.105, 0.015),
            Vec3::new(side * 0.32, c.roof_y() + 0.06, 1.02),
        );
    }
    let pivot = c.turret_pivot(-0.2);
    turret_ring(&mut hull, pivot, 0.46);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.5, 0.5, 0.26, 0.42, 0.32), 0.0, 0.18, 0.82, 0.0),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.42, 0.02, 0.1), Vec3::new(0.0, 0.19, 0.2));
    turret.add(Dark, cuboid(0.14, 0.07, 0.14), Vec3::new(-0.2, 0.2, 0.12));
    turret.add(Paint, cuboid(0.36, 0.2, 0.3), Vec3::new(0.0, 0.1, -0.55));
    // Twin rails with coil rings instead of a tube.
    let rail_len = 1.9;
    for side in [-1.0f32, 1.0] {
        turret.add(
            Dark,
            cuboid(0.03, 0.06, rail_len),
            Vec3::new(side * 0.045, 0.1, -0.7 - rail_len * 0.5),
        );
    }
    turret.add(
        Dark,
        cuboid(0.12, 0.025, rail_len * 0.9),
        Vec3::new(0.0, 0.065, -0.7 - rail_len * 0.5),
    );
    for k in 0..5 {
        turret.add(
            Glow,
            ring_z(0.08, 0.018),
            Vec3::new(0.0, 0.1, -0.95 - k as f32 * 0.33),
        );
    }
    turret.add(
        Dark,
        cyl_z(0.065, 0.1, 12),
        Vec3::new(0.0, 0.1, -0.7 - rail_len + 0.05),
    );
    antenna(&mut turret, Vec3::new(0.3, 0.18, 0.4), 0.4);
    spec(
        look([0.46, 0.52, 0.6], [0.2, 0.85, 1.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_lance_beam_tank() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.5, 1.3, 0.17, 6, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    // Power conduits along the hull sides feeding the emitter.
    for side in [-1.0f32, 1.0] {
        hull.add(
            Glow,
            cuboid(0.025, 0.025, 1.4),
            Vec3::new(side * (c.roof_hw() - 0.02), c.roof_y() + 0.015, 0.1),
        );
    }
    let pivot = c.turret_pivot(-0.1);
    turret_ring(&mut hull, pivot, 0.44);
    let mut turret = PartSet::default();
    turret.add(Paint, cyl_y(0.4, 0.12, 20), Vec3::new(0.0, 0.06, 0.0));
    turret.add(
        Paint,
        Sphere::new(0.34)
            .mesh()
            .uv(18, 10)
            .scaled_by(Vec3::new(1.0, 0.65, 1.0)),
        Vec3::new(0.0, 0.12, 0.0),
    );
    turret.add(Accent, cuboid(0.12, 0.02, 0.4), Vec3::new(0.0, 0.34, 0.05));
    for side in [-1.0f32, 1.0] {
        turret.add_rot(
            Paint,
            cuboid(0.035, 0.18, 0.4),
            Vec3::new(side * 0.34, 0.22, 0.1),
            Quat::from_rotation_z(side * -0.5),
        );
    }
    // Emitter: a fat collar, a tapering lance and a glowing lens at the tip.
    turret.add(Paint, cyl_z(0.13, 0.26, 16), Vec3::new(0.0, 0.16, -0.45));
    turret.add(
        Dark,
        taper_z(0.04, 0.09, 1.1, 14),
        Vec3::new(0.0, 0.16, -1.1),
    );
    for k in 0..4 {
        turret.add(
            Glow,
            ring_z(0.085 - k as f32 * 0.012, 0.016),
            Vec3::new(0.0, 0.16, -0.7 - k as f32 * 0.25),
        );
    }
    turret.add(Glow, sphere(0.06), Vec3::new(0.0, 0.16, -1.68));
    antenna(&mut turret, Vec3::new(-0.26, 0.26, 0.28), 0.4);
    spec(
        look([0.62, 0.66, 0.7], [0.25, 0.55, 1.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_mirage_scout_tank() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(1.8, 0.92, 0.13, 5, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let pivot = c.turret_pivot(-0.02);
    turret_ring(&mut hull, pivot, 0.3);
    let mut turret = PartSet::default();
    let gun_z = turret_body(&mut turret, 0.26, 0.62, 0.17);
    build_gun(&mut turret, gun_z, 0.08, 0.026, 0.85, Brake::Single, false);
    // Sensor mast with a glowing optic.
    turret.add(Dark, cyl_y(0.018, 0.34, 8), Vec3::new(-0.12, 0.34, 0.16));
    turret.add(Glow, sphere(0.045), Vec3::new(-0.12, 0.53, 0.16));
    spec(
        look([0.64, 0.62, 0.54], [0.8, 0.3, 1.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_siege_drill_tank() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.3, 1.3, 0.19, 6, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let roof = c.roof_y();
    // Casemate superstructure and the hydraulic drill cradle.
    hull.add(
        Paint,
        poly_loft(
            &oct_plan(-0.55, 0.65, 0.34, 0.5, 0.42),
            roof,
            roof + 0.26,
            0.82,
            0.05,
        ),
        Vec3::ZERO,
    );
    hull.add(
        Accent,
        cuboid(0.5, 0.02, 0.14),
        Vec3::new(0.0, roof + 0.27, 0.25),
    );
    cupola(&mut hull, Vec3::new(-0.22, roof + 0.27, 0.32), 0.12);
    let front = c.front();
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.07, 0.07, 0.6),
            Vec3::new(side * 0.36, roof - 0.05, front + 0.1),
            Quat::from_rotation_x(0.35),
        );
        hull.add(
            Dark,
            cyl_x(0.055, 0.1, 10),
            Vec3::new(side * 0.36, roof + 0.1, front + 0.36),
        );
    }
    let cradle = Vec3::new(0.0, roof - 0.12, front - 0.02);
    hull.add(Dark, cyl_z(0.18, 0.24, 14), cradle);
    let mut drill = PartSet::default();
    drill.add(
        Dark,
        taper_z(0.02, 0.27, 1.05, 16),
        Vec3::new(0.0, 0.0, -0.525),
    );
    for k in 0..4 {
        let f = 1.0 - k as f32 * 0.22;
        drill.add(
            Paint,
            ring_z(0.27 * f + 0.01, 0.028),
            Vec3::new(0.0, 0.0, -0.18 - k as f32 * 0.23),
        );
    }
    drill.add(Glow, ring_z(0.28, 0.014), Vec3::new(0.0, 0.0, -0.03));
    TankModelSpec {
        look: look([0.58, 0.5, 0.42], [1.0, 0.5, 0.15]),
        hull,
        turret: None,
        spinner: Some(SpinnerSpec {
            pivot: cradle + Vec3::NEG_Z * 0.12,
            axis: Vec3::NEG_Z,
            rate: 6.0,
            parts: drill,
        }),
    }
}

fn build_flak_hover_tank() -> TankModelSpec {
    use TankBucket::*;
    let mut hull = PartSet::default();
    hover_chassis(&mut hull, 2.0, 1.1);
    let pivot = Vec3::new(0.0, 0.55, 0.05);
    turret_ring(&mut hull, pivot, 0.36);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.34, 0.38, 0.2, 0.33, 0.25), 0.0, 0.2, 0.82, 0.0),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.34, 0.02, 0.08), Vec3::new(0.0, 0.21, 0.18));
    // Twin flak cannons with a tracking radar block between them.
    for side in [-1.0f32, 1.0] {
        turret.add(
            Paint,
            cuboid(0.1, 0.12, 0.26),
            Vec3::new(side * 0.13, 0.1, -0.38),
        );
        turret.add(
            Dark,
            taper_z(0.025, 0.035, 0.95, 10),
            Vec3::new(side * 0.13, 0.12, -0.5 - 0.475),
        );
        turret.add(
            Dark,
            cyl_z(0.045, 0.07, 10),
            Vec3::new(side * 0.13, 0.12, -1.42),
        );
    }
    turret.add(Dark, cuboid(0.12, 0.12, 0.1), Vec3::new(0.0, 0.26, 0.05));
    turret.add(Glow, cuboid(0.09, 0.05, 0.02), Vec3::new(0.0, 0.27, -0.01));
    antenna(&mut turret, Vec3::new(-0.24, 0.2, 0.3), 0.4);
    spec(
        look([0.52, 0.6, 0.58], [0.2, 0.95, 0.8]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_siege_artillery_vehicle() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.4, 1.28, 0.17, 6, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.22, 0.05, 0.3),
            Vec3::new(side * 0.4, 0.36, c.rear() + 0.02),
            Quat::from_rotation_x(-0.7),
        );
    }
    // Rear-mounted gun house so the long tube clears the glacis when raised.
    let pivot = c.turret_pivot(0.35);
    turret_ring(&mut hull, pivot, 0.46);
    let mut turret = PartSet::default();
    turret.add(Paint, cuboid(0.84, 0.3, 0.8), Vec3::new(0.0, 0.15, 0.05));
    turret.add(Accent, cuboid(0.6, 0.02, 0.1), Vec3::new(0.0, 0.31, 0.28));
    turret.add(Dark, cuboid(0.14, 0.09, 0.14), Vec3::new(-0.26, 0.34, 0.18));
    turret.add(Paint, cuboid(0.42, 0.26, 0.2), Vec3::new(0.0, 0.17, -0.42));
    build_gun(&mut turret, -0.52, 0.19, 0.06, 1.6, Brake::Double, true);
    antenna(&mut turret, Vec3::new(0.34, 0.3, 0.38), 0.45);
    spec(
        look([0.55, 0.57, 0.49], [0.0, 0.0, 0.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_longbow_missile_crawler() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.7, 1.4, 0.18, 7, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let pivot = c.turret_pivot(0.2);
    turret_ring(&mut hull, pivot, 0.5);
    let mut turret = PartSet::default();
    turret.add(Paint, cuboid(0.7, 0.16, 0.9), Vec3::new(0.0, 0.08, 0.0));
    // Raised eight-tube launcher box angled up for long-range fire.
    let launcher = Vec3::new(0.0, 0.42, -0.05);
    turret.add(Dark, cuboid(0.16, 0.3, 0.3), Vec3::new(0.0, 0.25, 0.25));
    let mut pod = PartSet::default();
    missile_pod(&mut pod, Vec3::ZERO, 0.8, 0.36, 1.2, 4, 2);
    for (bucket, mesh) in pod.0 {
        turret.add_rot(bucket, mesh, launcher, Quat::from_rotation_x(0.22));
    }
    turret.add(Dark, cuboid(0.12, 0.09, 0.12), Vec3::new(-0.3, 0.2, 0.35));
    antenna(&mut turret, Vec3::new(0.32, 0.16, 0.4), 0.5);
    spec(
        look([0.5, 0.54, 0.5], [0.0, 0.0, 0.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_modular_missile_carrier() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.5, 1.34, 0.17, 6, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let pivot = c.turret_pivot(0.05);
    turret_ring(&mut hull, pivot, 0.44);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.4, 0.45, 0.22, 0.34, 0.28), 0.0, 0.2, 0.82, 0.0),
        Vec3::ZERO,
    );
    // Twin pods flanking a sensor mast.
    for side in [-1.0f32, 1.0] {
        missile_pod(
            &mut turret,
            Vec3::new(side * 0.36, 0.22, -0.22),
            0.3,
            0.3,
            0.9,
            2,
            2,
        );
    }
    turret.add(Dark, cyl_y(0.03, 0.3, 8), Vec3::new(0.0, 0.35, 0.15));
    turret.add(Glow, sphere(0.05), Vec3::new(0.0, 0.52, 0.15));
    turret.add(Accent, cuboid(0.3, 0.02, 0.1), Vec3::new(0.0, 0.21, 0.25));
    spec(
        look([0.5, 0.52, 0.5], [1.0, 0.6, 0.2]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_tesla_crawler() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.0, 1.1, 0.15, 5, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let pivot = c.turret_pivot(0.0);
    turret_ring(&mut hull, pivot, 0.36);
    let mut turret = PartSet::default();
    turret.add(Paint, cyl_y(0.32, 0.14, 18), Vec3::new(0.0, 0.07, 0.0));
    // Tesla coil: insulator stack with glowing windings and a discharge ball.
    turret.add(Dark, cyl_y(0.12, 0.5, 14), Vec3::new(0.0, 0.39, 0.0));
    for k in 0..4 {
        turret.add(
            Glow,
            Mesh::from(
                Torus::new(0.02, 0.13)
                    .mesh()
                    .minor_resolution(6)
                    .major_resolution(20),
            ),
            Vec3::new(0.0, 0.22 + k as f32 * 0.1, 0.0),
        );
    }
    turret.add(Glow, sphere(0.11), Vec3::new(0.0, 0.72, 0.0));
    // Forked discharge prongs reach forward so the turret visibly aims the arc.
    for side in [-1.0f32, 1.0] {
        turret.add(
            Dark,
            cuboid(0.05, 0.05, 0.7),
            Vec3::new(side * 0.22, 0.17, -0.25),
        );
        turret.add(Glow, sphere(0.04), Vec3::new(side * 0.22, 0.17, -0.6));
    }
    spec(
        look([0.44, 0.5, 0.58], [0.3, 0.6, 1.0]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_drone_mine_layer() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.0, 1.15, 0.15, 5, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let roof = c.roof_y();
    // Rear dispenser: a hopper with a row of mine tubes facing backwards.
    hull.add(
        Paint,
        cuboid(0.7, 0.22, 0.5),
        Vec3::new(0.0, roof + 0.1, 0.55),
    );
    hull.add(
        Accent,
        cuboid(0.62, 0.015, 0.1),
        Vec3::new(0.0, roof + 0.22, 0.4),
    );
    for i in 0..4 {
        hull.add(
            Dark,
            cyl_z(0.06, 0.08, 10),
            Vec3::new(-0.24 + i as f32 * 0.16, roof + 0.08, 0.82),
        );
    }
    hull.add(
        Dark,
        cuboid(0.16, 0.08, 0.16),
        Vec3::new(-0.2, roof + 0.04, -0.2),
    );
    hull.add(Glow, sphere(0.05), Vec3::new(0.2, roof + 0.07, -0.25));
    antenna(&mut hull, Vec3::new(0.3, roof + 0.2, 0.7), 0.4);
    spec(look([0.56, 0.54, 0.5], [1.0, 0.35, 0.2]), hull, None)
}

fn build_mobile_repair_crawler() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.1, 1.15, 0.16, 5, false);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let roof = c.roof_y();
    // Crane: base drum, angled boom, hook; tool rack on the rear deck.
    hull.add(
        Paint,
        cyl_y(0.18, 0.1, 14),
        Vec3::new(0.0, roof + 0.05, 0.35),
    );
    hull.add_rot(
        Dark,
        cuboid(0.07, 0.07, 1.0),
        Vec3::new(0.0, roof + 0.4, -0.05),
        Quat::from_rotation_x(0.55),
    );
    hull.add(
        Dark,
        cuboid(0.05, 0.16, 0.05),
        Vec3::new(0.0, roof + 0.58, -0.45),
    );
    hull.add(
        Accent,
        cuboid(0.4, 0.015, 0.14),
        Vec3::new(0.0, roof + 0.01, -0.35),
    );
    hull.add(
        Glow,
        cuboid(0.34, 0.02, 0.06),
        Vec3::new(0.0, roof + 0.02, -0.15),
    );
    for i in 0..3 {
        hull.add(
            Dark,
            cuboid(0.1, 0.12, 0.08),
            Vec3::new(-0.24 + i as f32 * 0.24, roof + 0.06, 0.75),
        );
    }
    spec(look([0.62, 0.58, 0.44], [0.3, 1.0, 0.4]), hull, None)
}

fn build_mobile_shield_projector() -> TankModelSpec {
    use TankBucket::*;
    let c = Chassis::new(2.3, 1.2, 0.16, 6, true);
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &c);
    let roof = c.roof_y();
    // Projector: pylon with a vertical emitter ring and a glowing core.
    hull.add(
        Paint,
        cuboid(0.5, 0.2, 0.5),
        Vec3::new(0.0, roof + 0.1, 0.3),
    );
    hull.add(
        Dark,
        cyl_y(0.08, 0.35, 12),
        Vec3::new(0.0, roof + 0.35, 0.3),
    );
    hull.add(
        Glow,
        Mesh::from(
            Torus::new(0.03, 0.3)
                .mesh()
                .minor_resolution(6)
                .major_resolution(24),
        )
        .rotated_by(Quat::from_rotation_x(FRAC_PI_2)),
        Vec3::new(0.0, roof + 0.75, 0.3),
    );
    hull.add(Glow, sphere(0.08), Vec3::new(0.0, roof + 0.75, 0.3));
    hull.add(
        Accent,
        cuboid(0.44, 0.015, 0.1),
        Vec3::new(0.0, roof + 0.21, 0.1),
    );
    for side in [-1.0f32, 1.0] {
        hull.add(
            Dark,
            cuboid(0.08, 0.14, 0.2),
            Vec3::new(side * 0.3, roof + 0.07, -0.3),
        );
        hull.add(Glow, sphere(0.04), Vec3::new(side * 0.3, roof + 0.17, -0.3));
    }
    spec(look([0.5, 0.46, 0.62], [0.6, 0.35, 1.0]), hull, None)
}

fn build_scout_rover() -> TankModelSpec {
    use TankBucket::*;
    let w = Wheeled {
        len: 1.6,
        width: 0.86,
        wheel_r: 0.15,
        axles: 2,
    };
    let mut hull = PartSet::default();
    build_wheeled_chassis(&mut hull, &w);
    let deck = w.deck_y();
    // Roll cage over the cab and a pintle gun on a ring mount.
    for side in [-1.0f32, 1.0] {
        hull.add(
            Dark,
            cyl_y(0.015, 0.22, 6),
            Vec3::new(side * 0.22, deck + 0.11, 0.1),
        );
        hull.add(
            Dark,
            cyl_y(0.015, 0.22, 6),
            Vec3::new(side * 0.22, deck + 0.11, 0.45),
        );
        hull.add(
            Dark,
            cuboid(0.03, 0.03, 0.4),
            Vec3::new(side * 0.22, deck + 0.22, 0.27),
        );
    }
    hull.add(
        Dark,
        cuboid(0.5, 0.03, 0.03),
        Vec3::new(0.0, deck + 0.22, 0.1),
    );
    hull.add(
        Glow,
        cuboid(0.3, 0.02, 0.04),
        Vec3::new(0.0, deck + 0.01, -0.3),
    );
    let pivot = Vec3::new(0.0, deck + 0.02, -0.15);
    turret_ring(&mut hull, pivot, 0.14);
    let mut turret = PartSet::default();
    turret.add(Dark, cuboid(0.08, 0.06, 0.16), Vec3::new(0.0, 0.05, 0.0));
    turret.add(Dark, cyl_z(0.018, 0.6, 8), Vec3::new(0.0, 0.06, -0.36));
    turret.add(
        Accent,
        cuboid(0.1, 0.015, 0.06),
        Vec3::new(0.0, 0.085, 0.04),
    );
    spec(
        look([0.62, 0.62, 0.56], [0.4, 1.0, 0.6]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_flame_assault_buggy() -> TankModelSpec {
    use TankBucket::*;
    let w = Wheeled {
        len: 1.7,
        width: 0.92,
        wheel_r: 0.17,
        axles: 2,
    };
    let mut hull = PartSet::default();
    build_wheeled_chassis(&mut hull, &w);
    let deck = w.deck_y();
    // Twin fuel drums on the rear deck.
    for side in [-1.0f32, 1.0] {
        hull.add(
            Dark,
            cyl_z(0.11, 0.5, 12),
            Vec3::new(side * 0.17, deck + 0.11, 0.4),
        );
        hull.add(
            Glow,
            ring_z(0.115, 0.012),
            Vec3::new(side * 0.17, deck + 0.11, 0.22),
        );
    }
    hull.add(
        Dark,
        cuboid(0.34, 0.06, 0.06),
        Vec3::new(0.0, deck + 0.14, 0.4),
    );
    let pivot = Vec3::new(0.0, deck + 0.02, -0.2);
    turret_ring(&mut hull, pivot, 0.2);
    let mut turret = PartSet::default();
    turret.add(Paint, cyl_y(0.16, 0.1, 14), Vec3::new(0.0, 0.05, 0.0));
    turret.add(Accent, cuboid(0.2, 0.015, 0.06), Vec3::new(0.0, 0.11, 0.06));
    // Flamethrower: short fat nozzle with a glowing pilot ring.
    turret.add(Dark, cyl_z(0.04, 0.5, 10), Vec3::new(0.0, 0.1, -0.3));
    turret.add(
        Dark,
        taper_z(0.07, 0.04, 0.12, 10),
        Vec3::new(0.0, 0.1, -0.6),
    );
    turret.add(Glow, ring_z(0.05, 0.012), Vec3::new(0.0, 0.1, -0.64));
    spec(
        look([0.62, 0.42, 0.3], [1.0, 0.45, 0.1]),
        hull,
        Some((pivot, turret)),
    )
}

fn build_jammer_vehicle() -> TankModelSpec {
    use TankBucket::*;
    let w = Wheeled {
        len: 2.0,
        width: 1.05,
        wheel_r: 0.17,
        axles: 3,
    };
    let mut hull = PartSet::default();
    build_wheeled_chassis(&mut hull, &w);
    let deck = w.deck_y();
    // Boxy electronics shelter with an antenna farm.
    hull.add(
        Paint,
        cuboid(0.62, 0.26, 0.9),
        Vec3::new(0.0, deck + 0.13, 0.15),
    );
    hull.add(
        Accent,
        cuboid(0.5, 0.015, 0.14),
        Vec3::new(0.0, deck + 0.27, 0.45),
    );
    for i in 0..3 {
        antenna(
            &mut hull,
            Vec3::new(-0.2 + i as f32 * 0.2, deck + 0.26, 0.55),
            0.35 + i as f32 * 0.1,
        );
    }
    hull.add(
        Glow,
        cuboid(0.4, 0.02, 0.05),
        Vec3::new(0.0, deck + 0.27, 0.0),
    );
    // Rotating dish on a mast.
    let mast = Vec3::new(0.0, deck + 0.26, -0.15);
    hull.add(Dark, cyl_y(0.03, 0.3, 8), mast + Vec3::Y * 0.15);
    let mut dish = PartSet::default();
    dish.add(Dark, cuboid(0.05, 0.05, 0.05), Vec3::ZERO);
    dish.add_rot(
        Paint,
        Mesh::from(
            ConicalFrustum {
                radius_top: 0.22,
                radius_bottom: 0.04,
                height: 0.08,
            }
            .mesh()
            .resolution(16),
        ),
        Vec3::new(0.0, 0.03, -0.1),
        Quat::from_rotation_x(-1.1),
    );
    dish.add(Glow, sphere(0.03), Vec3::new(0.0, 0.1, -0.2));
    TankModelSpec {
        look: look([0.5, 0.54, 0.58], [1.0, 0.8, 0.2]),
        hull,
        turret: None,
        spinner: Some(SpinnerSpec {
            pivot: mast + Vec3::Y * 0.32,
            axis: Vec3::Y,
            rate: 1.2,
            parts: dish,
        }),
    }
}

pub(crate) const PROCEDURAL_TANK_IDS: &[&str] = &[
    "Tank",
    "HammerSiegeTank",
    "RailgunTank",
    "LanceBeamTank",
    "MirageScoutTank",
    "SiegeDrillTank",
    "FlakHoverTank",
    "SiegeArtilleryVehicle",
    "LongbowMissileCrawler",
    "ModularMissileCarrier",
    "TeslaCrawlerMk2",
    "DroneMineLayer",
    "MobileRepairCrawler",
    "MobileShieldProjector",
    "ScoutRover",
    "FlameAssaultBuggy",
    "JammerVehicle",
];

pub(crate) fn tank_model_spec(id: &str) -> Option<TankModelSpec> {
    Some(match id {
        "Tank" => build_tank(),
        "HammerSiegeTank" => build_hammer_siege_tank(),
        "RailgunTank" => build_railgun_tank(),
        "LanceBeamTank" => build_lance_beam_tank(),
        "MirageScoutTank" => build_mirage_scout_tank(),
        "SiegeDrillTank" => build_siege_drill_tank(),
        "FlakHoverTank" => build_flak_hover_tank(),
        "SiegeArtilleryVehicle" => build_siege_artillery_vehicle(),
        "LongbowMissileCrawler" => build_longbow_missile_crawler(),
        "ModularMissileCarrier" => build_modular_missile_carrier(),
        "TeslaCrawlerMk2" => build_tesla_crawler(),
        "DroneMineLayer" => build_drone_mine_layer(),
        "MobileRepairCrawler" => build_mobile_repair_crawler(),
        "MobileShieldProjector" => build_mobile_shield_projector(),
        "ScoutRover" => build_scout_rover(),
        "FlameAssaultBuggy" => build_flame_assault_buggy(),
        "JammerVehicle" => build_jammer_vehicle(),
        _ => return None,
    })
}

pub(crate) fn is_procedural_tank(id: &str) -> bool {
    PROCEDURAL_TANK_IDS.contains(&id)
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

type MeshPair = (Handle<Mesh>, Handle<StandardMaterial>);

#[derive(Clone)]
pub(crate) struct TankModelHandles {
    hull: Vec<MeshPair>,
    turret: Option<(Vec3, Vec<MeshPair>)>,
    spinner: Option<(Vec3, Vec3, f32, Vec<MeshPair>)>,
}

/// Meshes and materials are built once per unit type and shared by every
/// instance.
#[derive(Resource, Default)]
pub(crate) struct TankModelCache {
    by_id: HashMap<&'static str, TankModelHandles>,
}

pub(crate) fn tank_material(bucket: TankBucket, look: &TankLook) -> StandardMaterial {
    match bucket {
        TankBucket::Paint => StandardMaterial {
            base_color: look.paint,
            metallic: 0.3,
            perceptual_roughness: 0.55,
            ..default()
        },
        TankBucket::Dark => StandardMaterial {
            base_color: Color::srgb(0.14, 0.15, 0.16),
            metallic: 0.75,
            perceptual_roughness: 0.4,
            ..default()
        },
        TankBucket::Accent => StandardMaterial {
            base_color: Color::srgb(
                TEAM_PAINT_TARGET_SRGB[0],
                TEAM_PAINT_TARGET_SRGB[1],
                TEAM_PAINT_TARGET_SRGB[2],
            ),
            metallic: 0.55,
            perceptual_roughness: 0.35,
            ..default()
        },
        TankBucket::Track => StandardMaterial {
            base_color: Color::srgb(0.07, 0.07, 0.075),
            metallic: 0.2,
            perceptual_roughness: 0.9,
            ..default()
        },
        TankBucket::Glow => StandardMaterial {
            base_color: Color::from(look.glow * 0.6),
            emissive: look.glow * 2.5,
            metallic: 0.0,
            perceptual_roughness: 0.3,
            ..default()
        },
    }
}

fn upload_parts(world: &mut World, look: &TankLook, parts: PartSet) -> Vec<MeshPair> {
    let mut out = Vec::new();
    for (bucket, mesh) in parts.merged() {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(tank_material(bucket, look));
        out.push((mesh, material));
    }
    out
}

fn cached_tank_model(world: &mut World, id: &'static str) -> Option<TankModelHandles> {
    if let Some(handles) = world.get_resource_or_init::<TankModelCache>().by_id.get(id) {
        return Some(handles.clone());
    }
    let spec = tank_model_spec(id)?;
    let hull = upload_parts(world, &spec.look, spec.hull);
    let turret = spec
        .turret
        .map(|(pivot, parts)| (pivot, upload_parts(world, &spec.look, parts)));
    let spinner = spec.spinner.map(|spinner| {
        (
            spinner.pivot,
            spinner.axis,
            spinner.rate,
            upload_parts(world, &spec.look, spinner.parts),
        )
    });
    let handles = TankModelHandles {
        hull,
        turret,
        spinner,
    };
    world
        .get_resource_or_init::<TankModelCache>()
        .by_id
        .insert(id, handles.clone());
    Some(handles)
}

fn spawn_mesh_children(world: &mut World, parent: Entity, label: &'static str, pairs: &[MeshPair]) {
    for (mesh, material) in pairs {
        world.spawn((
            Name::new(label),
            ChildOf(parent),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
        ));
    }
}

/// Attaches the procedural model for `id` under `root`. Returns false when
/// the entity has no procedural tank definition.
pub(crate) fn spawn_tank_model(world: &mut World, root: Entity, id: &'static str) -> bool {
    if world.get_entity(root).is_err() {
        return false;
    }
    let Some(handles) = cached_tank_model(world, id) else {
        return false;
    };
    spawn_mesh_children(world, root, "tank hull", &handles.hull);
    if let Some((pivot, parts)) = &handles.turret {
        let turret = world
            .spawn((
                Name::new("turret"),
                ChildOf(root),
                Transform::from_translation(*pivot),
                Visibility::default(),
            ))
            .id();
        spawn_mesh_children(world, turret, "tank turret", parts);
    }
    if let Some((pivot, axis, rate, parts)) = &handles.spinner {
        let spinner = world
            .spawn((
                Name::new("tank spinner"),
                ChildOf(root),
                Transform::from_translation(*pivot),
                Visibility::default(),
                TankSpinner {
                    axis: *axis,
                    rate: *rate,
                    rest: Quat::IDENTITY,
                },
            ))
            .id();
        spawn_mesh_children(world, spinner, "tank spinner part", parts);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh_is_well_formed(mesh: &Mesh) {
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len();
        let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().len();
        let uvs = mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().len();
        assert_eq!(positions, normals);
        assert_eq!(positions, uvs);
        let indices = mesh.indices().expect("indexed mesh");
        assert_eq!(indices.len() % 3, 0);
        assert!(indices.iter().all(|i| i < positions), "index out of range");
    }

    fn bounds(mesh: &Mesh) -> (Vec3, Vec3) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("position attribute");
        };
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for p in positions {
            min = min.min(Vec3::from(*p));
            max = max.max(Vec3::from(*p));
        }
        (min, max)
    }

    #[test]
    fn every_procedural_tank_builds_well_formed_merged_meshes() {
        for id in PROCEDURAL_TANK_IDS {
            let spec = tank_model_spec(id).expect(id);
            assert!(!spec.hull.is_empty(), "{id} hull has parts");
            let hull = spec.hull.merged();
            assert!(!hull.is_empty());
            for (_, mesh) in &hull {
                mesh_is_well_formed(mesh);
            }
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for (_, mesh) in &hull {
                let (lo, hi) = bounds(mesh);
                min = min.min(lo);
                max = max.max(hi);
            }
            let def = registry::entity(id).expect(id);
            assert!(
                min.y >= -0.01,
                "{id} hull dips below the ground plane: {min}"
            );
            assert!(
                max.x - min.x <= def.radius * 2.4 && max.z - min.z <= def.radius * 3.2,
                "{id} hull footprint {} x {} exceeds selection radius {}",
                max.x - min.x,
                max.z - min.z,
                def.radius
            );
            assert!(
                hull.iter().any(|(bucket, _)| *bucket == TankBucket::Accent),
                "{id} needs a team-paint accent so player color reads at RTS zoom"
            );
            if let Some((pivot, turret)) = spec.turret {
                assert!(pivot.y > 0.3, "{id} turret pivot sits on the roof");
                let turret = turret.merged();
                for (_, mesh) in &turret {
                    mesh_is_well_formed(mesh);
                }
                let muzzle = turret
                    .iter()
                    .map(|(_, mesh)| bounds(mesh).0.z)
                    .fold(f32::MAX, f32::min);
                assert!(
                    muzzle < -def.radius * 0.7,
                    "{id} gun must reach forward along -Z, got min z {muzzle}"
                );
            }
        }
    }

    #[test]
    fn tracked_vehicles_keep_tank_proportions() {
        for id in PROCEDURAL_TANK_IDS {
            let spec = tank_model_spec(id).unwrap();
            let hull = spec.hull.merged();
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for (_, mesh) in &hull {
                let (lo, hi) = bounds(mesh);
                min = min.min(lo);
                max = max.max(hi);
            }
            let size = max - min;
            let ratio = size.z / size.x;
            assert!(
                (1.5..=2.3).contains(&ratio),
                "{id} length/width {ratio:.2} is outside real vehicle proportions ({size})"
            );
        }
    }

    #[test]
    fn drill_tank_has_a_spinner_and_no_turret() {
        let spec = tank_model_spec("SiegeDrillTank").unwrap();
        assert!(spec.turret.is_none());
        let spinner = spec.spinner.expect("drill spinner");
        assert_eq!(spinner.axis, Vec3::NEG_Z);
        for (_, mesh) in spinner.parts.merged() {
            mesh_is_well_formed(&mesh);
        }
    }

    #[test]
    fn accent_material_is_repainted_by_the_team_color_system() {
        let look = look([0.5, 0.5, 0.5], [0.0, 0.0, 0.0]);
        let accent = tank_material(TankBucket::Accent, &look);
        assert!(material_matches_team_paint_target(accent.base_color));
        assert!(!material_matches_team_paint_target(
            tank_material(TankBucket::Paint, &look).base_color
        ));
    }

    #[test]
    fn spawn_tank_model_shares_handles_and_names_the_turret() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        let world = app.world_mut();
        let a = world.spawn(Transform::IDENTITY).id();
        let b = world.spawn(Transform::IDENTITY).id();
        assert!(spawn_tank_model(world, a, "Tank"));
        assert!(spawn_tank_model(world, b, "Tank"));
        assert!(!spawn_tank_model(world, a, "Worker"));

        let mut turrets = 0;
        let mut meshes = std::collections::BTreeSet::new();
        let mut query = world.query::<(&Name, Option<&Mesh3d>)>();
        for (name, mesh) in query.iter(world) {
            if name.as_str() == "turret" {
                turrets += 1;
            }
            if let Some(mesh) = mesh {
                meshes.insert(mesh.0.id());
            }
        }
        assert_eq!(turrets, 2, "each tank gets one turret node");
        let cached = world.resource::<TankModelCache>().by_id["Tank"].hull.len();
        let turret_len = world.resource::<TankModelCache>().by_id["Tank"]
            .turret
            .as_ref()
            .unwrap()
            .1
            .len();
        assert_eq!(
            meshes.len(),
            cached + turret_len,
            "second tank reuses the cached meshes instead of uploading again"
        );
    }
}
