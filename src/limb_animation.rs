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

// ---------------------------------------------------------------------------
// Vehicles & aircraft
// ---------------------------------------------------------------------------

/// Stationary attackers smoothly yaw the whole body toward their target.
/// Without this, tanks fire sideways: `move_units` only faces the travel
/// direction, and nothing rotated a standing unit toward its victim.
pub(crate) fn face_attack_targets(
    time: Res<Time>,
    targets: Query<&GlobalTransform>,
    mut attackers: Query<(&mut Transform, &AttackOrder), (With<Unit>, Without<MoveOrder>)>,
) {
    let max_step = 4.5 * time.delta_secs();
    for (mut transform, order) in &mut attackers {
        let Ok(target) = targets.get(order.target) else {
            continue;
        };
        let to_target = target.translation() - transform.translation;
        let flat = Vec3::new(to_target.x, 0.0, to_target.z);
        if flat.length_squared() < 0.01 {
            continue;
        }
        let desired = Transform::from_translation(transform.translation)
            .looking_to(flat.normalize(), Vec3::Y)
            .rotation;
        transform.rotation = transform.rotation.rotate_towards(desired, max_step);
    }
}

/// The inner "turret" node of kenney turret_single/turret_double models,
/// discovered inside a unit's or defense structure's GLB scene. Tracks the
/// current attack target with smooth traverse and kicks back on each shot —
/// defense towers keep their base still and swing only the turret.
#[derive(Component)]
pub(crate) struct TurretNode {
    pub(crate) root: Entity,
    pub(crate) rest_translation: Vec3,
    pub(crate) rest_rotation: Quat,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

pub(crate) fn tag_turret_nodes(
    mut commands: Commands,
    fresh: Query<(Entity, &Name, &Transform), Added<Name>>,
    parents: Query<&ChildOf>,
    armed: Query<(), Or<(With<Unit>, With<Structure>)>>,
) {
    for (entity, name, transform) in &fresh {
        if name.as_str() != "turret" {
            continue;
        }
        let mut cursor = entity;
        let root = loop {
            match parents.get(cursor) {
                Ok(child_of) => {
                    cursor = child_of.0;
                    if armed.contains(cursor) {
                        break Some(cursor);
                    }
                }
                Err(_) => break None,
            }
        };
        let Some(root) = root else {
            continue;
        };
        commands.entity(entity).try_insert(TurretNode {
            root,
            rest_translation: transform.translation,
            rest_rotation: transform.rotation,
            yaw: 0.0,
            pitch: 0.0,
        });
    }
}

pub(crate) fn animate_turret_nodes(
    time: Res<Time>,
    roots: Query<(
        &GlobalTransform,
        Option<&AttackOrder>,
        Option<&Weapon>,
        Has<DeployedSiegeMode>,
    )>,
    positions: Query<&GlobalTransform>,
    mut turrets: Query<(&mut TurretNode, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut turret, mut transform) in &mut turrets {
        let Ok((root_gt, order, weapon, deployed)) = roots.get(turret.root) else {
            continue;
        };
        // Desired local yaw: the target bearing relative to the body's facing;
        // no target -> traverse back to center.
        let desired = order
            .and_then(|order| positions.get(order.target).ok())
            .map_or(0.0, |target| {
                let to_target = target.translation() - root_gt.translation();
                let world_yaw = f32::atan2(-to_target.x, -to_target.z);
                let (root_yaw, _, _) = root_gt.rotation().to_euler(EulerRot::YXZ);
                wrap_angle(world_yaw - root_yaw)
            });
        let step = 5.0 * dt;
        let delta = wrap_angle(desired - turret.yaw);
        turret.yaw += delta.clamp(-step, step);
        // Deployed siege vehicles crank the barrel up into a bombardment
        // pose — the silhouette change that says "artillery mode".
        let target_pitch = if deployed { 0.30 } else { 0.0 };
        let pitch_step = 1.4 * dt;
        turret.pitch += (target_pitch - turret.pitch).clamp(-pitch_step, pitch_step);
        let spin = Quat::from_rotation_y(turret.yaw) * Quat::from_rotation_x(turret.pitch);
        let kick = weapon.map_or(0.0, weapon_fire_kick);
        transform.rotation = turret.rest_rotation * spin;
        // Recoil: slide back along the barrel's current local facing.
        transform.translation = turret.rest_translation + spin * (Vec3::Z * (kick * 0.07));
    }
}

pub(crate) fn wrap_angle(angle: f32) -> f32 {
    let mut wrapped = angle % std::f32::consts::TAU;
    if wrapped > std::f32::consts::PI {
        wrapped -= std::f32::consts::TAU;
    } else if wrapped < -std::f32::consts::PI {
        wrapped += std::f32::consts::TAU;
    }
    wrapped
}

/// One model part of an air unit, posed as a rigid airframe (bob/pitch/roll
/// around the unit origin). Rotating translation AND rotation together keeps
/// the kenney node-offset bake intact.
#[derive(Component)]
pub(crate) struct AirframePart {
    pub(crate) root: Entity,
    pub(crate) rest_translation: Vec3,
    pub(crate) rest_rotation: Quat,
}

/// Root-side motion state for aircraft banking.
#[derive(Component, Default)]
pub(crate) struct AirMotion {
    pub(crate) last_yaw: f32,
    pub(crate) roll: f32,
}

pub(crate) fn animate_airframes(
    time: Res<Time>,
    mut roots: Query<(&GlobalTransform, &mut AirMotion, Has<MoveOrder>), With<Unit>>,
    mut parts: Query<(&AirframePart, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs().max(1e-6);
    // Update per-aircraft roll from the yaw rate (bank into turns).
    for (gt, mut motion, _) in &mut roots {
        let (yaw, _, _) = gt.rotation().to_euler(EulerRot::YXZ);
        let yaw_rate = wrap_angle(yaw - motion.last_yaw) / dt;
        motion.last_yaw = yaw;
        let target_roll = (-yaw_rate * 0.28).clamp(-0.4, 0.4);
        motion.roll += (target_roll - motion.roll) * (6.0 * dt).min(1.0);
    }
    for (part, mut transform) in &mut parts {
        let Ok((_, motion, moving)) = roots.get_mut(part.root) else {
            continue;
        };
        let seed = (part.root.to_bits() % 97) as f32 * 0.37;
        let bob = (t * 2.2 + seed).sin() * if moving { 0.02 } else { 0.05 };
        let pitch = if moving { 0.12 } else { 0.0 };
        let pose = Quat::from_rotation_z(motion.roll) * Quat::from_rotation_x(pitch);
        transform.rotation = pose * part.rest_rotation;
        transform.translation = pose * part.rest_translation + Vec3::Y * bob;
    }
}

/// Shared assets for deploy-mode stabilizer legs (one mesh, one material,
/// scaled per unit) so repeated deploys don't leak new assets.
#[derive(Resource)]
pub(crate) struct OutriggerAssets {
    pub(crate) mesh: Handle<Mesh>,
    pub(crate) material: Handle<StandardMaterial>,
}

/// One hydraulic stabilizer leg folded out of a deployed vehicle. The legs
/// extend over ~0.35s on deploy, retract on undeploy, then despawn — the
/// model itself visibly transforms instead of only gaining a HUD ring.
#[derive(Component)]
pub(crate) struct DeployOutrigger {
    pub(crate) direction: Vec3,
    pub(crate) length: f32,
    pub(crate) progress: f32,
    pub(crate) extending: bool,
}

pub(crate) fn manage_deploy_outriggers(
    mut commands: Commands,
    time: Res<Time>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    cache: Option<Res<OutriggerAssets>>,
    newly_deployed: Query<(Entity, &Selectable), (Added<DeployedSiegeMode>, With<Unit>)>,
    mut undeployed: RemovedComponents<DeployedSiegeMode>,
    mut outriggers: Query<(Entity, &ChildOf, &mut DeployOutrigger, &mut Transform)>,
) {
    // Spawn four legs on every fresh deploy.
    if !newly_deployed.is_empty()
        && let (Some(mut meshes), Some(mut materials)) = (meshes, materials)
    {
        let (mesh, material) = match cache.as_deref() {
            Some(assets) => (assets.mesh.clone(), assets.material.clone()),
            None => {
                let mesh = meshes.add(Cuboid::new(0.13, 0.09, 1.0));
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.42, 0.44, 0.48),
                    metallic: 0.75,
                    perceptual_roughness: 0.4,
                    ..default()
                });
                commands.insert_resource(OutriggerAssets {
                    mesh: mesh.clone(),
                    material: material.clone(),
                });
                (mesh, material)
            }
        };
        for (root, selectable) in &newly_deployed {
            let length = (selectable.radius * 0.65).clamp(0.4, 0.85);
            for i in 0..4 {
                let angle = i as f32 * core::f32::consts::FRAC_PI_2 + core::f32::consts::FRAC_PI_4;
                let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
                let leg = commands
                    .spawn((
                        Name::new("Deploy outrigger"),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        bevy::light::NotShadowCaster,
                        DeployOutrigger {
                            direction,
                            length,
                            progress: 0.0,
                            extending: true,
                        },
                        outrigger_pose(direction, length, 0.0),
                        MatchScopedEntity,
                    ))
                    .id();
                // The vehicle can die between this system running and the
                // command applying (combat's despawn queue drains first), so
                // attach via a guarded command instead of a bare add_child.
                commands.queue(move |world: &mut World| {
                    if world.get_entity(root).is_ok() {
                        world.entity_mut(root).add_child(leg);
                    } else if let Ok(orphan) = world.get_entity_mut(leg) {
                        orphan.despawn();
                    }
                });
            }
        }
    }

    // Undeployed roots: tell their legs to retract.
    let undeployed_roots: std::collections::HashSet<Entity> = undeployed.read().collect();
    let dt = time.delta_secs();
    for (entity, child_of, mut leg, mut transform) in &mut outriggers {
        if undeployed_roots.contains(&child_of.0) {
            leg.extending = false;
        }
        let step = dt / 0.35;
        leg.progress = if leg.extending {
            (leg.progress + step).min(1.0)
        } else {
            leg.progress - step
        };
        if leg.progress <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        *transform = outrigger_pose(leg.direction, leg.length, leg.progress);
    }
}

/// Leg pose at extension `progress`: angled down-and-out from the hull's
/// shoulder toward the ground, growing to full length as it plants.
pub(crate) fn outrigger_pose(direction: Vec3, length: f32, progress: f32) -> Transform {
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let down_out = (direction - Vec3::Y * 0.6).normalize();
    let reach = length * (0.25 + 0.75 * eased);
    Transform {
        translation: direction * (0.16 + 0.5 * reach) + Vec3::Y * (0.24 - 0.1 * eased),
        rotation: Quat::from_rotation_arc(Vec3::Z, down_out),
        scale: Vec3::new(1.0, 1.0, reach),
    }
}
