//! Procedural limb animation for every astronaut-based unit (all infantry and
//! the worker). The kenney astronauts have no skeleton, but arms/body/legs ship
//! as separately-pivoted GLB nodes (shoulders at y≈0.47, hips at y≈0.22), so we
//! articulate those nodes directly and compose per-limb layers:
//! arms — hammer (building) / drill (mining) / recoil kick (just fired) /
//! aim hold (attacking) / gait swing (moving); legs — stride while moving;
//! body — lean/bob to match. Idle limbs return to their exact rest pose.

use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum LimbKind {
    ArmLeft,
    ArmRight,
    Body,
    LegLeft,
    LegRight,
}

/// A named astronaut limb node inside a unit's GLB scene, remembered with its
/// rest pose and owning unit so the animator can pose and restore it.
#[derive(Component)]
pub(crate) struct Limb {
    pub(crate) kind: LimbKind,
    pub(crate) root: Entity,
    pub(crate) rest_translation: Vec3,
    pub(crate) rest_rotation: Quat,
    pub(crate) seed: f32,
}

/// Tags astronaut limb nodes as they stream in from unit GLB scenes.
pub(crate) fn tag_unit_limbs(
    mut commands: Commands,
    fresh: Query<(Entity, &Name, &Transform), Added<Name>>,
    parents: Query<&ChildOf>,
    units: Query<&Unit>,
) {
    for (entity, name, transform) in &fresh {
        let kind = match name.as_str() {
            "armLeft" => LimbKind::ArmLeft,
            "armRight" => LimbKind::ArmRight,
            "body" => LimbKind::Body,
            "legLeft" => LimbKind::LegLeft,
            "legRight" => LimbKind::LegRight,
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
        let Ok(unit) = units.get(root) else {
            continue;
        };
        if unit.id != "Worker" && !is_infantry_id(unit.id) {
            continue;
        }
        commands.entity(entity).try_insert(Limb {
            kind,
            root,
            rest_translation: transform.translation,
            rest_rotation: transform.rotation,
            seed: (root.to_bits() % 97) as f32 * 0.37,
        });
    }
}

/// What the owning unit is doing this frame, from the animator's perspective.
#[derive(Clone, Copy)]
pub(crate) struct LimbActivity {
    pub(crate) building: bool,
    pub(crate) mining: bool,
    /// 1.0 right at the shot, decaying to 0.0 over the kick window.
    pub(crate) fire_kick: f32,
    pub(crate) aiming: bool,
    pub(crate) moving: bool,
}

/// How long the recoil kick lasts after a shot.
const FIRE_KICK_SECONDS: f32 = 0.22;

pub(crate) fn weapon_fire_kick(weapon: &Weapon) -> f32 {
    if weapon.cooldown_left <= 0.0 {
        return 0.0;
    }
    let since_shot = (weapon.cooldown - weapon.cooldown_left).max(0.0);
    ((FIRE_KICK_SECONDS - since_shot) / FIRE_KICK_SECONDS).clamp(0.0, 1.0)
}

/// The pose offset (rotation, vertical bob) for one limb given the unit's
/// activity at time `t`. Layered: arms take the most specific activity, legs
/// stride whenever the unit moves, the body follows the arms.
pub(crate) fn limb_pose(kind: LimbKind, activity: LimbActivity, t: f32, seed: f32) -> (Quat, f32) {
    let gait = (t * 9.0 + seed).sin();
    // Legs stride whenever the unit is on the move, whatever the arms do.
    match kind {
        LimbKind::LegLeft | LimbKind::LegRight => {
            return if activity.moving {
                let side = if kind == LimbKind::LegLeft { 1.0 } else { -1.0 };
                (Quat::from_rotation_x(gait * 0.5 * side), 0.0)
            } else {
                (Quat::IDENTITY, 0.0)
            };
        }
        _ => {}
    }
    if activity.building {
        // Hammer swing: right arm raises and strikes, left arm braces.
        let swing = (t * 9.0 + seed).sin();
        return match kind {
            LimbKind::ArmRight => (Quat::from_rotation_x(-0.85 + swing * 0.65), 0.0),
            LimbKind::ArmLeft => (Quat::from_rotation_x(-0.3 + swing * -0.15), 0.0),
            _ => (Quat::from_rotation_x(0.1), (t * 9.0 + seed).cos() * 0.008),
        };
    }
    if activity.mining {
        // Drill peck: both arms forward on the tool, fast jitter, body bobbing.
        let jitter = (t * 13.0 + seed).sin();
        return match kind {
            LimbKind::ArmRight => (Quat::from_rotation_x(-0.6 + jitter * 0.12), 0.0),
            LimbKind::ArmLeft => (Quat::from_rotation_x(-0.6 - jitter * 0.12), 0.0),
            _ => (Quat::from_rotation_x(0.16), jitter * 0.015),
        };
    }
    if activity.fire_kick > 0.0 || activity.aiming {
        // Two-handed aim hold; a shot kicks the arms up and leans the body back.
        let kick = activity.fire_kick;
        return match kind {
            LimbKind::ArmRight => (Quat::from_rotation_x(-1.0 - kick * 0.3), 0.0),
            LimbKind::ArmLeft => (Quat::from_rotation_x(-0.85 - kick * 0.2), 0.0),
            _ => (Quat::from_rotation_x(-0.06 * kick), 0.0),
        };
    }
    if activity.moving {
        // Natural gait: each arm counter-swings its same-side leg.
        return match kind {
            LimbKind::ArmLeft => (Quat::from_rotation_x(-gait * 0.4), 0.0),
            LimbKind::ArmRight => (Quat::from_rotation_x(gait * 0.4), 0.0),
            _ => (Quat::IDENTITY, (t * 18.0 + seed).sin().abs() * 0.012),
        };
    }
    (Quat::IDENTITY, 0.0)
}

/// Poses unit limbs from what the owning unit is doing; restores the exact
/// rest pose when idle.
pub(crate) fn animate_unit_limbs(
    time: Res<Time>,
    roots: Query<
        (
            Option<&HarvestOrder>,
            Has<ConstructionWorkPulse>,
            Has<MoveOrder>,
            Has<AttackOrder>,
            Option<&Weapon>,
        ),
        With<Unit>,
    >,
    mut limbs: Query<(&Limb, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (limb, mut transform) in &mut limbs {
        let Ok((harvest, building, moving, attacking, weapon)) = roots.get(limb.root) else {
            continue;
        };
        let activity = LimbActivity {
            building,
            mining: harvest.is_some_and(|order| order.state == HarvestState::Collecting),
            fire_kick: weapon.map_or(0.0, weapon_fire_kick),
            aiming: attacking,
            moving,
        };
        let (rotation, bob) = limb_pose(limb.kind, activity, t, limb.seed);
        if rotation != Quat::IDENTITY || bob != 0.0 {
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
