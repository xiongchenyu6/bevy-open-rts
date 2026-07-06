//! Procedural limb animation for workers. The kenney astronauts have no
//! skeleton, but their arms/body ship as separately-pivoted GLB nodes
//! (armLeft/armRight at shoulder height, body at the hip), so we articulate
//! those nodes directly: a hammer swing while building, a rapid drill peck
//! while mining, and an exact return to the rest pose when idle.

use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum WorkerLimbKind {
    ArmLeft,
    ArmRight,
    Body,
}

/// A named astronaut limb node inside a worker's GLB scene, remembered with
/// its rest pose and owning unit so the animator can pose and restore it.
#[derive(Component)]
pub(crate) struct WorkerLimb {
    pub(crate) kind: WorkerLimbKind,
    pub(crate) root: Entity,
    pub(crate) rest_translation: Vec3,
    pub(crate) rest_rotation: Quat,
    pub(crate) seed: f32,
}

/// Tags astronaut limb nodes as they stream in from the worker GLB scenes.
pub(crate) fn tag_worker_limbs(
    mut commands: Commands,
    fresh: Query<(Entity, &Name, &Transform), Added<Name>>,
    parents: Query<&ChildOf>,
    units: Query<&Unit>,
) {
    for (entity, name, transform) in &fresh {
        let kind = match name.as_str() {
            "armLeft" => WorkerLimbKind::ArmLeft,
            "armRight" => WorkerLimbKind::ArmRight,
            "body" => WorkerLimbKind::Body,
            _ => continue,
        };
        let mut cursor = entity;
        let root = loop {
            match parents.get(cursor) {
                Ok(child_of) => {
                    cursor = child_of.0;
                    if units.get(cursor).is_ok() {
                        break Some(cursor);
                    }
                }
                Err(_) => break None,
            }
        };
        let Some(root) = root else {
            continue;
        };
        if units.get(root).map(|unit| unit.id) != Ok("Worker") {
            continue;
        }
        commands.entity(entity).try_insert(WorkerLimb {
            kind,
            root,
            rest_translation: transform.translation,
            rest_rotation: transform.rotation,
            seed: (root.to_bits() % 97) as f32 * 0.37,
        });
    }
}

/// The pose offset for one limb in one work style at time `t`.
fn work_pose(kind: WorkerLimbKind, building: bool, t: f32, seed: f32) -> (Quat, f32) {
    if building {
        // Hammer swing: the right arm raises and strikes ~1.4×/s, the left arm
        // braces, the body leans into the work.
        let swing = (t * 9.0 + seed).sin();
        match kind {
            WorkerLimbKind::ArmRight => (Quat::from_rotation_x(-0.85 + swing * 0.65), 0.0),
            WorkerLimbKind::ArmLeft => (Quat::from_rotation_x(-0.3 + swing * -0.15), 0.0),
            WorkerLimbKind::Body => (Quat::from_rotation_x(0.1), (t * 9.0 + seed).cos() * 0.008),
        }
    } else {
        // Drill peck: both arms forward on the tool, fast small jitter, body
        // bobbing with the drill.
        let jitter = (t * 13.0 + seed).sin();
        match kind {
            WorkerLimbKind::ArmRight => (Quat::from_rotation_x(-0.6 + jitter * 0.12), 0.0),
            WorkerLimbKind::ArmLeft => (Quat::from_rotation_x(-0.6 - jitter * 0.12), 0.0),
            WorkerLimbKind::Body => (Quat::from_rotation_x(0.16), jitter * 0.015),
        }
    }
}

/// Poses worker limbs while they build (ConstructionWorkPulse) or mine
/// (HarvestOrder in the Collecting state); otherwise restores the rest pose.
pub(crate) fn animate_worker_limbs(
    time: Res<Time>,
    roots: Query<(Option<&HarvestOrder>, Has<ConstructionWorkPulse>), With<Unit>>,
    mut limbs: Query<(&WorkerLimb, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (limb, mut transform) in &mut limbs {
        let Ok((harvest, building)) = roots.get(limb.root) else {
            continue;
        };
        let mining = harvest.is_some_and(|order| order.state == HarvestState::Collecting);
        if building || mining {
            let (rotation, bob) = work_pose(limb.kind, building, t, limb.seed);
            transform.rotation = limb.rest_rotation * rotation;
            transform.translation = limb.rest_translation + Vec3::Y * bob;
        } else if transform.rotation != limb.rest_rotation
            || transform.translation != limb.rest_translation
        {
            transform.rotation = limb.rest_rotation;
            transform.translation = limb.rest_translation;
        }
    }
}
