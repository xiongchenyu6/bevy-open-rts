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

struct Chassis {
    front: f32,
    rear: f32,
    half_w: f32,
    track_w: f32,
    lower_y: [f32; 2],
    roof_y: f32,
    glacis_top_z: f32,
    rear_top_z: f32,
    wheel_r: f32,
    wheel_zs: &'static [f32],
    skirt: bool,
}

impl Chassis {
    fn track_x(&self) -> f32 {
        self.half_w + self.track_w * 0.5
    }

    /// Fenders cover only the inner part of each track so the dark track runs
    /// flank the hull from the RTS camera's high angle.
    fn fender_hw(&self) -> f32 {
        self.half_w + self.track_w * 0.4
    }

    fn roof_hw(&self) -> f32 {
        self.half_w + self.track_w * 0.1
    }

    fn track_top_y(&self) -> f32 {
        self.wheel_r * 2.0 + 0.06
    }
}

fn build_tracked_chassis(parts: &mut PartSet, c: &Chassis) {
    use TankBucket::*;
    let mid_z = (c.front + c.rear) * 0.5;
    let lower_h = c.lower_y[1] - c.lower_y[0];
    parts.add(
        Paint,
        cuboid(c.half_w * 2.0, lower_h, c.rear - c.front - 0.2),
        Vec3::new(0.0, c.lower_y[0] + lower_h * 0.5, mid_z),
    );
    // Upper hull: full-width fenders at the belt line, sloped glacis and
    // rear plate up to the roof.
    parts.add(
        Paint,
        frustum(
            c.fender_hw(),
            c.front,
            c.rear,
            c.roof_hw(),
            c.glacis_top_z,
            c.rear_top_z,
            c.lower_y[1] - 0.02,
            c.roof_y,
        ),
        Vec3::ZERO,
    );

    let top_y = c.track_top_y();
    for side in [-1.0f32, 1.0] {
        let x = side * c.track_x();
        // Track band: boxes laid along a closed idler/top/sprocket/bottom loop.
        let profile = [
            (c.front + 0.02, c.wheel_r + 0.04),
            (c.front + 0.28, top_y),
            (c.rear - 0.28, top_y),
            (c.rear - 0.02, c.wheel_r + 0.08),
            (c.rear - 0.25, 0.03),
            (c.front + 0.25, 0.03),
        ];
        for i in 0..profile.len() {
            let (z0, y0) = profile[i];
            let (z1, y1) = profile[(i + 1) % profile.len()];
            let (dz, dy) = (z1 - z0, y1 - y0);
            let len = (dz * dz + dy * dy).sqrt();
            parts.add_rot(
                Track,
                cuboid(c.track_w, 0.05, len + 0.03),
                Vec3::new(x, (y0 + y1) * 0.5, (z0 + z1) * 0.5),
                tilt_x(dy, dz),
            );
        }
        // Filler behind the band so the loop never reads hollow.
        parts.add(
            Dark,
            cuboid(c.track_w * 0.55, top_y - 0.1, c.rear - c.front - 0.5),
            Vec3::new(x, top_y * 0.5 + 0.02, mid_z),
        );
        for &z in c.wheel_zs {
            parts.add(
                Track,
                cyl_x(c.wheel_r, c.track_w * 0.8, 18),
                Vec3::new(x, c.wheel_r, z),
            );
            parts.add(
                Paint,
                cyl_x(c.wheel_r * 0.55, c.track_w * 0.86, 12),
                Vec3::new(x, c.wheel_r, z),
            );
        }
        parts.add(
            Dark,
            cyl_x(c.wheel_r * 0.75, c.track_w * 0.6, 14),
            Vec3::new(x, c.wheel_r + 0.04, c.front + 0.05),
        );
        parts.add(
            Dark,
            cyl_x(c.wheel_r * 0.8, c.track_w * 0.6, 14),
            Vec3::new(x, c.wheel_r + 0.08, c.rear - 0.05),
        );
        parts.add(
            Dark,
            ring_x(c.wheel_r * 0.8, 0.02),
            Vec3::new(x, c.wheel_r + 0.08, c.rear - 0.05),
        );
        if c.skirt {
            let skirt_bottom = c.wheel_r * 1.1;
            let skirt_top = top_y - 0.04;
            let skirt_x = side * (c.half_w + c.track_w + 0.02);
            parts.add(
                Paint,
                cuboid(0.04, skirt_top - skirt_bottom, c.rear - c.front - 0.5),
                Vec3::new(skirt_x, (skirt_top + skirt_bottom) * 0.5, mid_z),
            );
            parts.add(
                Accent,
                cuboid(0.02, 0.07, (c.rear - c.front) * 0.5),
                Vec3::new(skirt_x + side * 0.02, skirt_top - 0.1, mid_z),
            );
        } else {
            parts.add(
                Accent,
                cuboid(0.02, 0.06, (c.rear - c.front) * 0.45),
                Vec3::new(side * (c.fender_hw() + 0.005), c.lower_y[1] + 0.06, mid_z),
            );
        }
        // Exhaust boxes and headlights.
        parts.add(
            Dark,
            cuboid(0.16, 0.1, 0.08),
            Vec3::new(side * c.roof_hw() * 0.6, c.roof_y - 0.12, c.rear + 0.02),
        );
        parts.add(
            Accent,
            cuboid(0.1, 0.06, 0.05),
            Vec3::new(
                side * c.roof_hw() * 0.7,
                c.lower_y[1] + 0.08,
                c.front - 0.01,
            ),
        );
        parts.add(
            Dark,
            cuboid(0.06, 0.08, 0.06),
            Vec3::new(side * c.half_w * 0.7, c.lower_y[1] - 0.06, c.front - 0.08),
        );
    }
    // Roof edge seams so the deck reads as plated armor rather than a slab.
    let roof_len = c.rear_top_z - c.glacis_top_z;
    for side in [-1.0f32, 1.0] {
        parts.add(
            Dark,
            cuboid(0.02, 0.012, roof_len * 0.92),
            Vec3::new(
                side * (c.roof_hw() - 0.05),
                c.roof_y + 0.004,
                (c.rear_top_z + c.glacis_top_z) * 0.5,
            ),
        );
    }
    // Engine deck grilles.
    for k in 0..3 {
        parts.add(
            Dark,
            cuboid(c.roof_hw() * 1.4, 0.02, 0.06),
            Vec3::new(0.0, c.roof_y + 0.01, c.rear_top_z - 0.15 - k as f32 * 0.14),
        );
    }
    // Driver hatch seated on the glacis slope.
    let dz = c.glacis_top_z - c.front;
    let dy = c.roof_y - (c.lower_y[1] - 0.02);
    let n = Vec2::new(dz, -dy).normalize();
    let mid = Vec3::new(
        -c.roof_hw() * 0.45,
        (c.roof_y + c.lower_y[1]) * 0.5 + n.x * 0.02,
        (c.glacis_top_z + c.front) * 0.5 + n.y * 0.02,
    );
    parts.add_rot(Paint, cuboid(0.3, 0.03, 0.24), mid, tilt_x(dy, dz));
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
        cyl_z(r * 1.8, 0.18, 16),
        Vec3::new(0.0, y, z0 - 0.09),
    );
    parts.add(
        Paint,
        taper_z(r * 0.85, r * 1.15, len, 16),
        Vec3::new(0.0, y, z0 - len * 0.5),
    );
    if evac {
        parts.add(
            Paint,
            cyl_z(r * 1.5, len * 0.14, 16),
            Vec3::new(0.0, y, z0 - len * 0.6),
        );
    }
    match brake {
        Brake::Single => {
            parts.add(
                Dark,
                cyl_z(r * 1.5, 0.14, 14),
                Vec3::new(0.0, y, z0 - len + 0.07),
            );
        }
        Brake::Double => {
            parts.add(
                Dark,
                cyl_z(r * 0.8, 0.32, 12),
                Vec3::new(0.0, y, z0 - len + 0.16),
            );
            parts.add(
                Dark,
                cyl_z(r * 1.9, 0.07, 14),
                Vec3::new(0.0, y, z0 - len + 0.06),
            );
            parts.add(
                Dark,
                cyl_z(r * 1.9, 0.07, 14),
                Vec3::new(0.0, y, z0 - len + 0.24),
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
    parts.add(Paint, cyl_y(r, 0.08, 14), at);
    parts.add(Dark, cyl_y(r * 0.7, 0.03, 12), at + Vec3::Y * 0.05);
    parts.add(
        Accent,
        cuboid(r * 0.6, 0.02, r * 0.3),
        at + Vec3::new(0.0, 0.07, -r * 0.5),
    );
}

fn antenna(parts: &mut PartSet, at: Vec3, h: f32) {
    parts.add(TankBucket::Dark, cyl_y(0.008, h, 6), at + Vec3::Y * h * 0.5);
}

fn smoke_launchers(parts: &mut PartSet, at: Vec3, side: f32) {
    for k in 0..3 {
        parts.add_rot(
            TankBucket::Dark,
            cyl_z(0.025, 0.12, 8),
            at + Vec3::new(side * k as f32 * 0.06, 0.0, k as f32 * 0.02),
            Quat::from_rotation_y(side * 0.35),
        );
    }
}

fn hover_chassis(parts: &mut PartSet, plan: &[[f32; 2]], look_glow: bool) {
    use TankBucket::*;
    // Hull floats at 0.32; the skirt flares below it and thrusters glow
    // underneath so the hover reads from the top-down camera.
    parts.add(Paint, poly_loft(plan, 0.32, 0.62, 0.72, -0.05), Vec3::ZERO);
    parts.add(Dark, poly_loft(plan, 0.18, 0.34, 1.0, 0.0), Vec3::ZERO);
    let flare: Vec<[f32; 2]> = plan.iter().map(|p| [p[0] * 1.08, p[1] * 1.08]).collect();
    parts.add(Dark, poly_loft(&flare, 0.12, 0.2, 0.93, 0.0), Vec3::ZERO);
    for (x, z) in [(-0.42, -0.55), (0.42, -0.55), (-0.42, 0.55), (0.42, 0.55)] {
        parts.add(Dark, cyl_y(0.14, 0.06, 12), Vec3::new(x, 0.1, z));
        if look_glow {
            parts.add(Glow, cyl_y(0.1, 0.02, 12), Vec3::new(x, 0.065, z));
        }
    }
    for side in [-1.0f32, 1.0] {
        parts.add(
            Accent,
            cuboid(0.04, 0.06, 0.9),
            Vec3::new(side * 0.78, 0.5, 0.05),
        );
        parts.add(
            Dark,
            cuboid(0.18, 0.1, 0.1),
            Vec3::new(side * 0.35, 0.5, 0.92),
        );
    }
    for k in 0..3 {
        parts.add(
            Dark,
            cuboid(0.7, 0.02, 0.05),
            Vec3::new(0.0, 0.63, 0.45 + k as f32 * 0.12),
        );
    }
}

// ---------------------------------------------------------------------------
// Tank builders
// ---------------------------------------------------------------------------

fn look(paint: [f32; 3], glow: [f32; 3]) -> TankLook {
    TankLook {
        paint: Color::srgb(paint[0], paint[1], paint[2]),
        glow: LinearRgba::rgb(glow[0], glow[1], glow[2]),
    }
}

const MBT_CHASSIS: Chassis = Chassis {
    front: -1.15,
    rear: 1.15,
    half_w: 0.55,
    track_w: 0.34,
    lower_y: [0.16, 0.5],
    roof_y: 0.78,
    glacis_top_z: -0.45,
    rear_top_z: 1.0,
    wheel_r: 0.21,
    wheel_zs: &[-0.8, -0.4, 0.0, 0.4, 0.8],
    skirt: true,
};

fn build_tank() -> TankModelSpec {
    use TankBucket::*;
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &MBT_CHASSIS);
    let pivot = Vec3::new(0.0, 0.78, -0.05);
    turret_ring(&mut hull, pivot, 0.58);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(
            &oct_plan(-0.62, 0.55, 0.28, 0.52, 0.42),
            0.0,
            0.32,
            0.8,
            0.0,
        ),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.62, 0.02, 0.14), Vec3::new(0.0, 0.33, 0.22));
    cupola(&mut turret, Vec3::new(-0.2, 0.34, 0.12), 0.14);
    turret.add(Dark, cuboid(0.12, 0.08, 0.12), Vec3::new(0.22, 0.34, -0.1));
    turret.add(Paint, cuboid(0.42, 0.3, 0.2), Vec3::new(0.0, 0.14, -0.62));
    for side in [-1.0f32, 1.0] {
        smoke_launchers(&mut turret, Vec3::new(side * 0.4, 0.2, -0.35), side);
    }
    antenna(&mut turret, Vec3::new(0.38, 0.3, 0.45), 0.5);
    build_gun(&mut turret, -0.7, 0.14, 0.06, 1.2, Brake::Single, true);
    TankModelSpec {
        look: look([0.58, 0.60, 0.56], [0.0, 0.0, 0.0]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

const HEAVY_CHASSIS: Chassis = Chassis {
    front: -1.25,
    rear: 1.25,
    half_w: 0.62,
    track_w: 0.4,
    lower_y: [0.18, 0.56],
    roof_y: 0.88,
    glacis_top_z: -0.5,
    rear_top_z: 1.1,
    wheel_r: 0.23,
    wheel_zs: &[-0.9, -0.54, -0.18, 0.18, 0.54, 0.9],
    skirt: true,
};

fn build_hammer_siege_tank() -> TankModelSpec {
    use TankBucket::*;
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &HEAVY_CHASSIS);
    // Recoil spades folded at the rear.
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.26, 0.05, 0.36),
            Vec3::new(side * 0.5, 0.42, 1.28),
            Quat::from_rotation_x(-0.6),
        );
    }
    let pivot = Vec3::new(0.0, 0.88, 0.05);
    turret_ring(&mut hull, pivot, 0.7);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.7, 0.75, 0.42, 0.64, 0.5), 0.0, 0.42, 0.86, 0.0),
        Vec3::ZERO,
    );
    turret.add(Paint, cuboid(0.9, 0.3, 0.5), Vec3::new(0.0, 0.2, 0.7));
    turret.add(Accent, cuboid(0.7, 0.02, 0.18), Vec3::new(0.0, 0.43, 0.25));
    turret.add(Accent, cuboid(0.9, 0.02, 0.08), Vec3::new(0.0, 0.36, 0.95));
    cupola(&mut turret, Vec3::new(-0.26, 0.44, 0.3), 0.15);
    turret.add(Dark, cuboid(0.14, 0.1, 0.14), Vec3::new(0.28, 0.45, 0.05));
    for side in [-1.0f32, 1.0] {
        smoke_launchers(&mut turret, Vec3::new(side * 0.52, 0.26, -0.35), side);
        turret.add(
            Dark,
            cuboid(0.06, 0.24, 0.5),
            Vec3::new(side * 0.4, 0.2, -0.7),
        );
    }
    antenna(&mut turret, Vec3::new(-0.45, 0.4, 0.7), 0.55);
    turret.add(Paint, cuboid(0.62, 0.42, 0.24), Vec3::new(0.0, 0.18, -0.74));
    turret.add(Glow, ring_z(0.13, 0.02), Vec3::new(0.0, 0.18, -0.87));
    build_gun(&mut turret, -0.86, 0.18, 0.1, 1.3, Brake::Double, true);
    TankModelSpec {
        look: look([0.5, 0.5, 0.52], [1.0, 0.45, 0.1]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

const LONG_CHASSIS: Chassis = Chassis {
    front: -1.3,
    rear: 1.2,
    half_w: 0.55,
    track_w: 0.34,
    lower_y: [0.16, 0.48],
    roof_y: 0.72,
    glacis_top_z: -0.6,
    rear_top_z: 1.05,
    wheel_r: 0.2,
    wheel_zs: &[-0.95, -0.55, -0.15, 0.25, 0.65, 0.95],
    skirt: true,
};

fn build_railgun_tank() -> TankModelSpec {
    use TankBucket::*;
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &LONG_CHASSIS);
    // Capacitor bank on the rear deck.
    for side in [-1.0f32, 1.0] {
        hull.add(Dark, cyl_z(0.12, 0.5, 12), Vec3::new(side * 0.4, 0.8, 0.8));
        hull.add(Glow, ring_z(0.125, 0.015), Vec3::new(side * 0.4, 0.8, 0.62));
        hull.add(Glow, ring_z(0.125, 0.015), Vec3::new(side * 0.4, 0.8, 0.98));
    }
    let pivot = Vec3::new(0.0, 0.72, -0.15);
    turret_ring(&mut hull, pivot, 0.56);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.5, 0.6, 0.3, 0.5, 0.36), 0.0, 0.22, 0.82, 0.0),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.5, 0.02, 0.12), Vec3::new(0.0, 0.23, 0.25));
    turret.add(Dark, cuboid(0.16, 0.08, 0.16), Vec3::new(-0.22, 0.25, 0.15));
    turret.add(Paint, cuboid(0.44, 0.22, 0.3), Vec3::new(0.0, 0.12, -0.55));
    // Twin rails with coil rings instead of a tube.
    let rail_len = 1.7;
    for side in [-1.0f32, 1.0] {
        turret.add(
            Dark,
            cuboid(0.035, 0.07, rail_len),
            Vec3::new(side * 0.055, 0.12, -0.7 - rail_len * 0.5),
        );
    }
    turret.add(
        Dark,
        cuboid(0.15, 0.03, rail_len * 0.9),
        Vec3::new(0.0, 0.08, -0.7 - rail_len * 0.5),
    );
    for k in 0..4 {
        turret.add(
            Glow,
            ring_z(0.095, 0.02),
            Vec3::new(0.0, 0.12, -0.95 - k as f32 * 0.36),
        );
    }
    turret.add(
        Dark,
        cyl_z(0.075, 0.1, 12),
        Vec3::new(0.0, 0.12, -0.7 - rail_len + 0.05),
    );
    antenna(&mut turret, Vec3::new(0.35, 0.2, 0.5), 0.45);
    TankModelSpec {
        look: look([0.46, 0.52, 0.6], [0.2, 0.85, 1.0]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

fn build_lance_beam_tank() -> TankModelSpec {
    use TankBucket::*;
    let chassis = Chassis {
        front: -1.2,
        rear: 1.1,
        half_w: 0.52,
        track_w: 0.32,
        lower_y: [0.16, 0.46],
        roof_y: 0.7,
        glacis_top_z: -0.35,
        rear_top_z: 0.95,
        wheel_r: 0.2,
        wheel_zs: &[-0.8, -0.4, 0.0, 0.4, 0.8],
        skirt: true,
    };
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &chassis);
    // Power conduits along the hull sides feeding the emitter.
    for side in [-1.0f32, 1.0] {
        hull.add(
            Glow,
            cuboid(0.03, 0.03, 1.4),
            Vec3::new(side * 0.6, 0.72, 0.1),
        );
    }
    let pivot = Vec3::new(0.0, 0.7, -0.05);
    turret_ring(&mut hull, pivot, 0.54);
    let mut turret = PartSet::default();
    turret.add(Paint, cyl_y(0.5, 0.16, 20), Vec3::new(0.0, 0.08, 0.0));
    turret.add(
        Paint,
        Sphere::new(0.42)
            .mesh()
            .uv(18, 10)
            .scaled_by(Vec3::new(1.0, 0.65, 1.0)),
        Vec3::new(0.0, 0.16, 0.0),
    );
    turret.add(Accent, cuboid(0.14, 0.02, 0.5), Vec3::new(0.0, 0.43, 0.05));
    for side in [-1.0f32, 1.0] {
        turret.add_rot(
            Paint,
            cuboid(0.04, 0.22, 0.5),
            Vec3::new(side * 0.42, 0.28, 0.1),
            Quat::from_rotation_z(side * -0.5),
        );
    }
    // Emitter: a fat collar, a tapering lance and a glowing lens at the tip.
    turret.add(Paint, cyl_z(0.16, 0.3, 16), Vec3::new(0.0, 0.2, -0.55));
    turret.add(
        Dark,
        taper_z(0.05, 0.11, 0.9, 14),
        Vec3::new(0.0, 0.2, -1.15),
    );
    for k in 0..3 {
        turret.add(
            Glow,
            ring_z(0.1 - k as f32 * 0.015, 0.018),
            Vec3::new(0.0, 0.2, -0.8 - k as f32 * 0.25),
        );
    }
    turret.add(Glow, sphere(0.07), Vec3::new(0.0, 0.2, -1.62));
    antenna(&mut turret, Vec3::new(-0.3, 0.3, 0.3), 0.4);
    TankModelSpec {
        look: look([0.62, 0.66, 0.7], [0.25, 0.55, 1.0]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

fn build_mirage_scout_tank() -> TankModelSpec {
    use TankBucket::*;
    let chassis = Chassis {
        front: -0.85,
        rear: 0.8,
        half_w: 0.38,
        track_w: 0.24,
        lower_y: [0.14, 0.4],
        roof_y: 0.6,
        glacis_top_z: -0.3,
        rear_top_z: 0.7,
        wheel_r: 0.17,
        wheel_zs: &[-0.5, -0.17, 0.17, 0.5],
        skirt: false,
    };
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &chassis);
    let pivot = Vec3::new(0.0, 0.6, 0.0);
    turret_ring(&mut hull, pivot, 0.34);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.34, 0.34, 0.18, 0.3, 0.24), 0.0, 0.2, 0.8, 0.0),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.3, 0.02, 0.1), Vec3::new(0.0, 0.21, 0.12));
    turret.add(Paint, cuboid(0.2, 0.16, 0.14), Vec3::new(0.0, 0.1, -0.36));
    build_gun(&mut turret, -0.42, 0.1, 0.03, 0.7, Brake::Single, false);
    // Sensor mast with a glowing optic.
    turret.add(Dark, cyl_y(0.02, 0.36, 8), Vec3::new(-0.16, 0.38, 0.18));
    turret.add(Glow, sphere(0.05), Vec3::new(-0.16, 0.58, 0.18));
    antenna(&mut turret, Vec3::new(0.2, 0.2, 0.25), 0.4);
    TankModelSpec {
        look: look([0.64, 0.62, 0.54], [0.8, 0.3, 1.0]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

fn build_siege_drill_tank() -> TankModelSpec {
    use TankBucket::*;
    let chassis = Chassis {
        front: -1.05,
        rear: 1.05,
        half_w: 0.56,
        track_w: 0.34,
        lower_y: [0.18, 0.56],
        roof_y: 0.8,
        glacis_top_z: -0.6,
        rear_top_z: 0.9,
        wheel_r: 0.22,
        wheel_zs: &[-0.7, -0.32, 0.06, 0.44, 0.8],
        skirt: true,
    };
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &chassis);
    // Casemate superstructure and the hydraulic drill cradle.
    hull.add(
        Paint,
        poly_loft(&oct_plan(-0.6, 0.7, 0.4, 0.62, 0.5), 0.8, 1.1, 0.82, 0.05),
        Vec3::ZERO,
    );
    hull.add(Accent, cuboid(0.6, 0.02, 0.16), Vec3::new(0.0, 1.11, 0.25));
    cupola(&mut hull, Vec3::new(-0.25, 1.12, 0.35), 0.14);
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.08, 0.08, 0.6),
            Vec3::new(side * 0.4, 0.72, -0.95),
            Quat::from_rotation_x(0.35),
        );
        hull.add(
            Dark,
            cyl_x(0.06, 0.12, 10),
            Vec3::new(side * 0.4, 0.88, -0.68),
        );
    }
    hull.add(Dark, cyl_z(0.2, 0.26, 14), Vec3::new(0.0, 0.62, -1.1));
    let mut drill = PartSet::default();
    drill.add(
        Dark,
        taper_z(0.02, 0.3, 1.1, 16),
        Vec3::new(0.0, 0.0, -0.55),
    );
    for k in 0..4 {
        let f = 1.0 - k as f32 * 0.22;
        drill.add(
            Paint,
            ring_z(0.3 * f + 0.01, 0.03),
            Vec3::new(0.0, 0.0, -0.2 - k as f32 * 0.24),
        );
    }
    drill.add(Glow, ring_z(0.31, 0.015), Vec3::new(0.0, 0.0, -0.04));
    TankModelSpec {
        look: look([0.58, 0.5, 0.42], [1.0, 0.5, 0.15]),
        hull,
        turret: None,
        spinner: Some(SpinnerSpec {
            pivot: Vec3::new(0.0, 0.62, -1.22),
            axis: Vec3::NEG_Z,
            rate: 6.0,
            parts: drill,
        }),
    }
}

fn build_flak_hover_tank() -> TankModelSpec {
    use TankBucket::*;
    let plan = [
        [-0.42, -1.0],
        [0.42, -1.0],
        [0.74, -0.5],
        [0.74, 0.7],
        [0.5, 1.0],
        [-0.5, 1.0],
        [-0.74, 0.7],
        [-0.74, -0.5],
    ];
    let mut hull = PartSet::default();
    hover_chassis(&mut hull, &plan, true);
    let pivot = Vec3::new(0.0, 0.62, 0.05);
    turret_ring(&mut hull, pivot, 0.44);
    let mut turret = PartSet::default();
    turret.add(
        Paint,
        poly_loft(&oct_plan(-0.4, 0.45, 0.24, 0.4, 0.3), 0.0, 0.24, 0.82, 0.0),
        Vec3::ZERO,
    );
    turret.add(Accent, cuboid(0.4, 0.02, 0.1), Vec3::new(0.0, 0.25, 0.2));
    // Twin flak cannons with a tracking radar block between them.
    for side in [-1.0f32, 1.0] {
        turret.add(
            Paint,
            cuboid(0.12, 0.14, 0.3),
            Vec3::new(side * 0.16, 0.12, -0.45),
        );
        turret.add(
            Dark,
            taper_z(0.03, 0.04, 0.85, 10),
            Vec3::new(side * 0.16, 0.14, -0.6 - 0.425),
        );
        turret.add(
            Dark,
            cyl_z(0.05, 0.08, 10),
            Vec3::new(side * 0.16, 0.14, -1.42),
        );
    }
    turret.add(Dark, cuboid(0.14, 0.14, 0.12), Vec3::new(0.0, 0.3, 0.05));
    turret.add(Glow, cuboid(0.1, 0.06, 0.02), Vec3::new(0.0, 0.31, -0.02));
    antenna(&mut turret, Vec3::new(-0.28, 0.24, 0.35), 0.45);
    TankModelSpec {
        look: look([0.52, 0.6, 0.58], [0.2, 0.95, 0.8]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
    }
}

fn build_siege_artillery_vehicle() -> TankModelSpec {
    use TankBucket::*;
    let chassis = Chassis {
        front: -1.15,
        rear: 1.1,
        half_w: 0.55,
        track_w: 0.34,
        lower_y: [0.16, 0.5],
        roof_y: 0.74,
        glacis_top_z: -0.55,
        rear_top_z: 1.05,
        wheel_r: 0.2,
        wheel_zs: &[-0.8, -0.42, -0.04, 0.34, 0.72],
        skirt: false,
    };
    let mut hull = PartSet::default();
    build_tracked_chassis(&mut hull, &chassis);
    for side in [-1.0f32, 1.0] {
        hull.add_rot(
            Dark,
            cuboid(0.22, 0.05, 0.4),
            Vec3::new(side * 0.45, 0.4, 1.2),
            Quat::from_rotation_x(-0.7),
        );
    }
    // Rear-mounted gun house so the long tube clears the glacis when raised.
    let pivot = Vec3::new(0.0, 0.74, 0.35);
    turret_ring(&mut hull, pivot, 0.56);
    let mut turret = PartSet::default();
    turret.add(Paint, cuboid(1.0, 0.36, 0.9), Vec3::new(0.0, 0.18, 0.05));
    turret.add(Accent, cuboid(0.7, 0.02, 0.12), Vec3::new(0.0, 0.37, 0.3));
    turret.add(Dark, cuboid(0.16, 0.1, 0.16), Vec3::new(-0.3, 0.4, 0.2));
    turret.add(Paint, cuboid(0.5, 0.3, 0.2), Vec3::new(0.0, 0.2, -0.45));
    build_gun(&mut turret, -0.55, 0.22, 0.08, 1.5, Brake::Double, true);
    antenna(&mut turret, Vec3::new(0.4, 0.36, 0.4), 0.5);
    TankModelSpec {
        look: look([0.55, 0.57, 0.49], [0.0, 0.0, 0.0]),
        hull,
        turret: Some((pivot, turret)),
        spinner: None,
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
                assert!(pivot.y > 0.5, "{id} turret pivot sits on the roof");
                let turret = turret.merged();
                for (_, mesh) in &turret {
                    mesh_is_well_formed(mesh);
                }
                let muzzle = turret
                    .iter()
                    .map(|(_, mesh)| bounds(mesh).0.z)
                    .fold(f32::MAX, f32::min);
                assert!(
                    muzzle < -0.9,
                    "{id} gun must reach forward along -Z, got min z {muzzle}"
                );
            }
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
