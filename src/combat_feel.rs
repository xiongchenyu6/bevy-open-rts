//! Combat feel: camera shake, shockwaves, hit flashes, secondary explosions,
//! weapon-class muzzle flashes and layered impact audio.
//!
//! Everything here is presentation only. The systems *read* the transient
//! effect entities the simulation already emits ([`ImpactBurst`],
//! [`StructureDestructionVfx`], [`CombatWreckage`]) plus unit health, and
//! spawn local, unreplicated visuals/audio in response. Nothing in this module
//! writes simulation components, the economy, orders, or the online snapshot,
//! so the host and every client derive the same feel from the same replicated
//! events without any new wire data. There is deliberately no hit-stop: an
//! RTS must never stall its clock for one explosion.

use crate::*;

// ---------------------------------------------------------------------------
// Camera shake
// ---------------------------------------------------------------------------

/// Largest camera displacement, in world units, at full trauma. Small on
/// purpose: the camera sits 6-11 units up, so this is roughly 1-2% of the view.
pub(crate) const CAMERA_SHAKE_MAX_OFFSET: f32 = 0.16;

/// Trauma lost per second; a full-strength kick settles in about 0.6 s.
pub(crate) const CAMERA_SHAKE_DECAY_PER_SECOND: f32 = 1.7;

/// Shake "stamina". Every kick spends budget and budget refills slowly, so a
/// sustained bombardment fades to a murmur instead of rattling the screen for
/// the whole fight.
pub(crate) const CAMERA_SHAKE_BUDGET_MAX: f32 = 1.0;

pub(crate) const CAMERA_SHAKE_BUDGET_REGEN_PER_SECOND: f32 = 0.45;

/// Events closer to the camera focus than this shake at full strength…
pub(crate) const CAMERA_SHAKE_NEAR_DISTANCE: f32 = 5.0;

/// …and events farther than this do not shake the camera at all.
pub(crate) const CAMERA_SHAKE_FAR_DISTANCE: f32 = 24.0;

/// Kicks weaker than this are not worth moving the camera for.
const CAMERA_SHAKE_MIN_KICK: f32 = 0.03;

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct CameraShake {
    pub(crate) trauma: f32,
    pub(crate) budget: f32,
    pub(crate) clock: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            budget: CAMERA_SHAKE_BUDGET_MAX,
            clock: 0.0,
        }
    }
}

impl CameraShake {
    /// Requests a shake of `strength` (0..1, already distance-attenuated).
    /// Kicks never stack past the strongest one and are limited by the budget.
    pub(crate) fn kick(&mut self, strength: f32) {
        let strength = strength.clamp(0.0, 1.0).min(self.budget);
        if strength < CAMERA_SHAKE_MIN_KICK {
            return;
        }
        self.budget -= strength;
        self.trauma = self.trauma.max(strength);
    }

    pub(crate) fn tick(&mut self, delta_secs: f32) {
        self.clock += delta_secs;
        self.trauma = (self.trauma - CAMERA_SHAKE_DECAY_PER_SECOND * delta_secs).max(0.0);
        self.budget = (self.budget + CAMERA_SHAKE_BUDGET_REGEN_PER_SECOND * delta_secs)
            .min(CAMERA_SHAKE_BUDGET_MAX);
    }

    /// Camera-plane displacement (x = right, y = up) for the current trauma.
    /// Squared trauma keeps small kicks subtle and big ones punchy.
    pub(crate) fn offset(&self) -> Vec2 {
        let amplitude = CAMERA_SHAKE_MAX_OFFSET * self.trauma * self.trauma;
        if amplitude <= 0.0 {
            return Vec2::ZERO;
        }
        let t = self.clock;
        Vec2::new(
            (t * 71.0).sin() * 0.6 + (t * 113.0 + 1.3).sin() * 0.4,
            (t * 83.0 + 0.7).sin() * 0.6 + (t * 127.0 + 2.1).sin() * 0.4,
        ) * amplitude
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// 1.0 inside `near`, falling smoothly to 0.0 at `far`.
pub(crate) fn focus_falloff(distance: f32, near: f32, far: f32) -> f32 {
    if distance <= near {
        return 1.0;
    }
    if distance >= far {
        return 0.0;
    }
    let t = 1.0 - (distance - near) / (far - near);
    t * t * (3.0 - 2.0 * t)
}

/// How hard an impact burst shakes the camera before distance attenuation.
/// Only big hits qualify; rifle fire and ordinary cannon shots return 0.
pub(crate) fn impact_shake_strength(burst: &ImpactBurst) -> f32 {
    if is_strike_burst(burst) {
        return 0.9;
    }
    match burst.kind {
        ImpactBurstKind::Siege => 0.3,
        ImpactBurstKind::Explosive | ImpactBurstKind::Heavy if burst.power >= 1.7 => 0.12,
        _ => 0.0,
    }
}

pub(crate) fn structure_destruction_shake_strength(radius: f32) -> f32 {
    (0.38 + radius * 0.1).clamp(0.4, 0.7)
}

/// Decays the shake and nudges the match camera. `bevy_rts_camera` rewrites
/// the camera transform from its own state every frame, so the offset never
/// accumulates and never feeds back into camera focus, picking state or
/// anything the simulation reads.
pub(crate) fn apply_camera_shake(
    time: Res<Time>,
    options: Option<Res<MenuOptionsState>>,
    mut shake: ResMut<CameraShake>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    // The player can switch shake off in Options; drop any pending trauma so
    // re-enabling it mid-match never replays an old blast.
    if options.is_some_and(|options| !options.screen_shake) {
        shake.clear();
        return;
    }
    shake.tick(time.delta_secs());
    let offset = shake.offset();
    if offset == Vec2::ZERO {
        return;
    }
    for mut transform in &mut cameras {
        let displacement = transform.right() * offset.x + transform.up() * offset.y;
        transform.translation += displacement;
    }
}

// ---------------------------------------------------------------------------
// Shockwave rings and secondary explosions
// ---------------------------------------------------------------------------

/// Bursts at least this wide are support-power strikes, not weapon hits
/// (ordinary [`spawn_impact_burst`] radii are clamped to 1.45).
pub(crate) const STRIKE_BURST_MIN_RADIUS: f32 = 1.6;

pub(crate) fn is_strike_burst(burst: &ImpactBurst) -> bool {
    burst.radius >= STRIKE_BURST_MIN_RADIUS
}

/// Marks where a support-power strike lands. Spawned by the authoritative
/// simulation as a plain [`ImpactBurst`], so it reaches online clients through
/// the existing transient-event replication; [`emit_combat_feel`] turns it
/// into the full explosion on every peer.
pub(crate) fn spawn_strike_impact(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    team: Team,
) {
    let total = 0.5;
    commands.spawn((
        Name::new("Strike impact"),
        Transform::from_translation(Vec3::new(position.x, 0.08, position.z)),
        ImpactBurst {
            remaining: total,
            total,
            radius: radius.max(STRIKE_BURST_MIN_RADIUS),
            power: 2.2,
            team,
            kind: ImpactBurstKind::Siege,
        },
        MatchScopedEntity,
    ));
}

/// Cached flat ring so every shockwave shares one mesh.
#[derive(Resource, Default)]
pub(crate) struct ShockwaveRingMesh(pub(crate) Option<Handle<Mesh>>);

/// A solid ground ring that races outward and fades — the readable version of
/// the hairline gizmo circles.
#[derive(Component)]
pub(crate) struct ShockwaveRing {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) start_radius: f32,
    pub(crate) end_radius: f32,
    pub(crate) material: Handle<StandardMaterial>,
    pub(crate) peak_alpha: f32,
}

pub(crate) fn spawn_shockwave_ring(
    commands: &mut Commands,
    position: Vec3,
    start_radius: f32,
    end_radius: f32,
    ttl: f32,
    color: LinearRgba,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
        {
            return;
        }
        let mesh = match world
            .get_resource::<ShockwaveRingMesh>()
            .and_then(|mesh| mesh.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(Annulus::new(0.9, 1.0).mesh().resolution(48));
                if let Some(mut cache) = world.get_resource_mut::<ShockwaveRingMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let peak_alpha = 0.5;
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(color.red, color.green, color.blue, peak_alpha),
                emissive: color,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            });
        world.spawn((
            Name::new("Shockwave ring"),
            bevy::light::NotShadowCaster,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(position.x, 0.07, position.z))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(start_radius)),
            ShockwaveRing {
                age: 0.0,
                ttl,
                start_radius,
                end_radius,
                material,
                peak_alpha,
            },
            MatchScopedEntity,
        ));
    });
}

pub(crate) fn update_shockwave_rings(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut rings: Query<(Entity, &mut ShockwaveRing, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (entity, mut ring, mut transform) in &mut rings {
        ring.age += time.delta_secs();
        let progress = (ring.age / ring.ttl).clamp(0.0, 1.0);
        if progress >= 1.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        // Ease-out: the wave bursts out fast and coasts to a stop.
        let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
        let radius = ring.start_radius + (ring.end_radius - ring.start_radius) * eased;
        transform.scale = Vec3::splat(radius);
        if let Some(mut material) = materials.get_mut(&ring.material) {
            let fade = 1.0 - progress;
            material.base_color.set_alpha(ring.peak_alpha * fade * fade);
        }
    }
}

/// A delayed follow-up pop: collapsing buildings and strikes cook off over a
/// few beats instead of resolving in a single frame.
#[derive(Component)]
pub(crate) struct SecondaryExplosion {
    pub(crate) delay: f32,
    pub(crate) scale: f32,
}

pub(crate) fn spawn_secondary_explosions(
    commands: &mut Commands,
    position: Vec3,
    spread: f32,
    count: u32,
) {
    let seed = (position.x.to_bits() as u64) << 32 | position.z.to_bits() as u64;
    for index in 0..count {
        let angle = vfx_hash01(seed, index * 7 + 1) * std::f32::consts::TAU;
        let reach = spread * (0.25 + vfx_hash01(seed, index * 7 + 2) * 0.6);
        commands.spawn((
            Name::new("Secondary explosion"),
            Transform::from_translation(
                position + Vec3::new(angle.cos() * reach, 0.0, angle.sin() * reach),
            ),
            SecondaryExplosion {
                delay: 0.12 + index as f32 * 0.14 + vfx_hash01(seed, index * 7 + 3) * 0.08,
                scale: 0.55 + vfx_hash01(seed, index * 7 + 4) * 0.4,
            },
            MatchScopedEntity,
        ));
    }
}

pub(crate) fn update_secondary_explosions(
    mut commands: Commands,
    time: Res<Time>,
    camera: Option<Res<RtsCamera>>,
    mut mixer: ResMut<CombatAudioMixer>,
    mut pending: Query<(Entity, &mut SecondaryExplosion, &Transform)>,
) {
    for (entity, mut explosion, transform) in &mut pending {
        explosion.delay -= time.delta_secs();
        if explosion.delay > 0.0 {
            continue;
        }
        let position = transform.translation;
        spawn_combat_flash(
            &mut commands,
            position + Vec3::Y * 0.35,
            0.2 * explosion.scale,
            1.1 * explosion.scale,
            0.3,
            LinearRgba::new(1.2, 0.62, 0.2, 1.0),
        );
        spawn_debris_burst(&mut commands, position, 3, 3.0, DebrisPalette::Ember);
        if let Some(camera) = camera.as_deref() {
            mixer.request(
                CombatSample::Blast,
                1.1 + explosion.scale * 0.25,
                0.4 * audio_falloff(camera, position),
            );
        }
        commands.entity(entity).try_despawn();
    }
}

// ---------------------------------------------------------------------------
// Weapon-class muzzle flashes
// ---------------------------------------------------------------------------

/// Muzzle flash at the shooter, shaped by what is being fired: cannons bark
/// and kick dust, rockets leave a backblast, beams and arcs glow in their own
/// hue, flamers bloom. Sits a little toward the target so it reads at the
/// barrel instead of inside the hull.
pub(crate) fn spawn_muzzle_flash(
    commands: &mut Commands,
    from: Vec3,
    to: Vec3,
    kind: ImpactBurstKind,
) {
    let aim = Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalize_or_zero();
    let muzzle = from + aim * 0.5 + Vec3::Y * 0.6;
    let (start, end, lifetime, color) = match kind {
        ImpactBurstKind::Ballistic => (0.16, 0.28, 0.08, LinearRgba::new(1.0, 0.9, 0.55, 1.0)),
        ImpactBurstKind::Heavy => (0.2, 0.36, 0.12, LinearRgba::new(1.2, 0.95, 0.6, 1.0)),
        ImpactBurstKind::Explosive => (0.16, 0.3, 0.14, LinearRgba::new(1.1, 0.7, 0.32, 1.0)),
        ImpactBurstKind::Energy => (0.2, 0.3, 0.16, LinearRgba::new(0.45, 1.1, 1.3, 1.0)),
        ImpactBurstKind::Electric => (0.18, 0.36, 0.12, LinearRgba::new(0.7, 0.9, 1.5, 1.0)),
        ImpactBurstKind::Fire => (0.2, 0.42, 0.22, LinearRgba::new(1.2, 0.45, 0.1, 1.0)),
        ImpactBurstKind::Siege => (0.3, 0.6, 0.16, LinearRgba::new(1.25, 0.95, 0.6, 1.0)),
    };
    spawn_combat_flash(commands, muzzle, start, end, lifetime, color);
    match kind {
        // Big guns kick up dust at the mount.
        ImpactBurstKind::Siege | ImpactBurstKind::Heavy => spawn_landing_dust(commands, from),
        // Rockets and shells leave a pale backblast behind the launcher.
        ImpactBurstKind::Explosive => spawn_combat_flash_with_alpha(
            commands,
            from - aim * 0.3 + Vec3::Y * 0.5,
            0.14,
            0.46,
            0.3,
            LinearRgba::new(0.62, 0.6, 0.56, 1.0),
            0.4,
        ),
        // Flamers spit a second bloom further down the stream.
        ImpactBurstKind::Fire => spawn_combat_flash(
            commands,
            from + aim * 0.75 + Vec3::Y * 0.55,
            0.14,
            0.4,
            0.2,
            LinearRgba::new(1.2, 0.7, 0.18, 1.0),
        ),
        ImpactBurstKind::Ballistic | ImpactBurstKind::Energy | ImpactBurstKind::Electric => {}
    }
}

// ---------------------------------------------------------------------------
// Class-distinct tracers
// ---------------------------------------------------------------------------

/// One straight piece of a tracer, in world space, with its colour and alpha.
pub(crate) type TracerSegment = (Vec3, Vec3, LinearRgba);

/// How a weapon shot is drawn between muzzle and target, by weapon class.
/// `life` runs 1.0 (just fired) to 0.0 (gone). Bullets are a short dash that
/// travels, shells a fat slug with a fading wake, rockets a smoke trail behind
/// a hot head, beams a steady twin line, arcs a jagged bolt, flamers a fan.
pub(crate) fn tracer_segments(
    kind: ImpactBurstKind,
    from: Vec3,
    to: Vec3,
    life: f32,
) -> Vec<TracerSegment> {
    let life = life.clamp(0.0, 1.0);
    let travel = 1.0 - life;
    let span = to - from;
    let length = span.length();
    if length <= f32::EPSILON {
        return Vec::new();
    }
    let dir = span / length;
    let side = Vec3::new(-dir.z, 0.0, dir.x).normalize_or(Vec3::X);
    let at = |t: f32| from + span * t.clamp(0.0, 1.0);
    let rgba = |r: f32, g: f32, b: f32, a: f32| LinearRgba::new(r, g, b, a.clamp(0.0, 1.0));
    match kind {
        ImpactBurstKind::Ballistic => {
            let head = (travel * 1.6).min(1.0);
            vec![(
                at(head - 0.28),
                at(head),
                rgba(1.0, 0.93, 0.6, 0.35 + 0.65 * life),
            )]
        }
        ImpactBurstKind::Heavy | ImpactBurstKind::Siege => {
            let width = if kind == ImpactBurstKind::Siege {
                0.07
            } else {
                0.045
            };
            let head = (travel * 1.4).min(1.0);
            let slug = rgba(1.0, 0.82, 0.42, 0.4 + 0.6 * life);
            vec![
                (at(0.0), at(head), rgba(0.85, 0.8, 0.72, 0.3 * life)),
                (at(head - 0.2) + side * width, at(head) + side * width, slug),
                (at(head - 0.2) - side * width, at(head) - side * width, slug),
                (at(head - 0.2), at(head), slug),
            ]
        }
        ImpactBurstKind::Explosive => {
            let head = (travel * 1.25).min(1.0);
            let lift = |t: f32| at(t) + Vec3::Y * (4.0 * t * (1.0 - t) * (length * 0.06).min(0.8));
            let mut segments = Vec::with_capacity(5);
            let steps = 4;
            for step in 0..steps {
                let a = head * step as f32 / steps as f32;
                let b = head * (step + 1) as f32 / steps as f32;
                let smoke = 0.18 + 0.32 * (step as f32 / steps as f32);
                segments.push((
                    lift(a),
                    lift(b),
                    rgba(0.78, 0.76, 0.72, smoke * (0.4 + 0.6 * life)),
                ));
            }
            segments.push((
                lift(head - 0.06),
                lift(head),
                rgba(1.0, 0.6, 0.2, 0.5 + 0.5 * life),
            ));
            segments
        }
        ImpactBurstKind::Energy => {
            let glow = rgba(0.4, 0.95, 1.0, 0.9 * life);
            vec![
                (from + side * 0.03, to + side * 0.03, glow),
                (from - side * 0.03, to - side * 0.03, glow),
                (from, to, rgba(0.9, 1.0, 1.0, life)),
            ]
        }
        ImpactBurstKind::Electric => {
            let joints = 6;
            let mut previous = from;
            let mut segments = Vec::with_capacity(joints);
            for joint in 1..=joints {
                let t = joint as f32 / joints as f32;
                // Deterministic zig-zag that re-rolls as the bolt ages.
                let phase = (joint as f32 * 12.9898 + (life * 5.0).floor() * 78.233).sin();
                let jitter = if joint == joints { 0.0 } else { phase * 0.22 };
                let point = at(t) + side * jitter + Vec3::Y * jitter * 0.5;
                segments.push((previous, point, rgba(0.62, 0.84, 1.0, 0.3 + 0.7 * life)));
                previous = point;
            }
            segments
        }
        ImpactBurstKind::Fire => {
            let reach = (0.35 + travel * 0.9).min(1.0);
            let spread = 0.12 * length.min(4.0);
            vec![
                (from, at(reach), rgba(1.0, 0.62, 0.14, 0.85 * life)),
                (
                    from,
                    at(reach * 0.9) + side * spread,
                    rgba(1.0, 0.4, 0.08, 0.6 * life),
                ),
                (
                    from,
                    at(reach * 0.9) - side * spread,
                    rgba(1.0, 0.4, 0.08, 0.6 * life),
                ),
            ]
        }
    }
}

// ---------------------------------------------------------------------------
// Layered impact audio
// ---------------------------------------------------------------------------

/// Marks a playing combat sound so the mixer can cap concurrent voices.
#[derive(Component)]
pub(crate) struct CombatSfx;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CombatSample {
    /// Short metallic hit.
    Hit,
    /// Explosion body; pitched down for the big ones.
    Blast,
}

impl CombatSample {
    pub(crate) fn audio_path(self) -> &'static str {
        match self {
            Self::Hit => "sfx/weapon_hit.wav",
            Self::Blast => "sfx/explosion_small.wav",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombatSoundRequest {
    pub(crate) sample: CombatSample,
    pub(crate) speed: f32,
    pub(crate) volume: f32,
}

/// Loudest requests win each frame.
pub(crate) const COMBAT_AUDIO_MAX_STARTS_PER_FRAME: usize = 3;

/// Hard cap on simultaneously playing combat sounds.
pub(crate) const COMBAT_AUDIO_MAX_VOICES: usize = 12;

/// Minimum gap between two light hit sounds, so a firing line is a rattle
/// rather than one wall of noise.
pub(crate) const COMBAT_AUDIO_HIT_INTERVAL_SECONDS: f32 = 0.07;

/// Requests quieter than this are inaudible under the mix; skip them.
const COMBAT_AUDIO_MIN_VOLUME: f32 = 0.04;

/// Collects this frame's combat sounds and decides which actually play.
#[derive(Resource, Default)]
pub(crate) struct CombatAudioMixer {
    pub(crate) pending: Vec<CombatSoundRequest>,
    pub(crate) hit_cooldown: f32,
}

impl CombatAudioMixer {
    pub(crate) fn request(&mut self, sample: CombatSample, speed: f32, volume: f32) {
        if volume < COMBAT_AUDIO_MIN_VOLUME {
            return;
        }
        self.pending.push(CombatSoundRequest {
            sample,
            speed,
            volume: volume.min(1.0),
        });
    }

    /// Drains the queue and returns the sounds to start this frame, loudest
    /// first, honoring the per-frame, hit-interval and voice caps.
    pub(crate) fn mix(&mut self, delta_secs: f32, active_voices: usize) -> Vec<CombatSoundRequest> {
        self.hit_cooldown = (self.hit_cooldown - delta_secs).max(0.0);
        let mut pending = std::mem::take(&mut self.pending);
        pending.sort_by(|a, b| b.volume.total_cmp(&a.volume));
        let free_voices = COMBAT_AUDIO_MAX_VOICES.saturating_sub(active_voices);
        let mut started = Vec::new();
        for request in pending {
            if started.len() >= COMBAT_AUDIO_MAX_STARTS_PER_FRAME.min(free_voices) {
                break;
            }
            if request.sample == CombatSample::Hit {
                if self.hit_cooldown > 0.0 {
                    continue;
                }
                self.hit_cooldown = COMBAT_AUDIO_HIT_INTERVAL_SECONDS;
            }
            started.push(request);
        }
        started
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
        self.hit_cooldown = 0.0;
    }
}

/// Loudness of a battlefield sound heard from the camera focus.
fn audio_falloff(camera: &RtsCamera, position: Vec3) -> f32 {
    focus_falloff(xz_distance(camera.focus, position), 7.0, 32.0)
}

/// Small deterministic pitch wobble so repeated hits do not sound stamped.
fn pitch_jitter(position: Vec3) -> f32 {
    let seed = (position.x.to_bits() as u64) << 32 | position.z.to_bits() as u64;
    0.93 + vfx_hash01(seed, 41) * 0.14
}

/// Sound layers for a weapon hit of `kind`: (sample, playback speed, volume).
pub(crate) fn impact_sound_layers(
    kind: ImpactBurstKind,
    power: f32,
) -> [Option<(CombatSample, f32, f32)>; 2] {
    let weight = (power / 2.2).clamp(0.2, 1.0);
    match kind {
        ImpactBurstKind::Ballistic => [Some((CombatSample::Hit, 1.15, 0.26 + weight * 0.14)), None],
        ImpactBurstKind::Energy => [Some((CombatSample::Hit, 1.7, 0.3 + weight * 0.12)), None],
        ImpactBurstKind::Electric => [Some((CombatSample::Hit, 2.1, 0.3 + weight * 0.12)), None],
        ImpactBurstKind::Fire => [Some((CombatSample::Blast, 1.5, 0.22 + weight * 0.12)), None],
        ImpactBurstKind::Explosive => [Some((CombatSample::Blast, 1.0, 0.4 + weight * 0.2)), None],
        ImpactBurstKind::Heavy => [
            Some((CombatSample::Hit, 0.7, 0.4 + weight * 0.2)),
            Some((CombatSample::Blast, 0.85, 0.24 + weight * 0.16)),
        ],
        ImpactBurstKind::Siege => [
            Some((CombatSample::Blast, 0.62, 0.62 + weight * 0.24)),
            Some((CombatSample::Hit, 0.5, 0.4)),
        ],
    }
}

#[cfg(feature = "audio")]
pub(crate) fn play_combat_audio(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Option<Res<AssetServer>>,
    options: Option<Res<MenuOptionsState>>,
    mut mixer: ResMut<CombatAudioMixer>,
    voices: Query<(), With<CombatSfx>>,
) {
    let started = mixer.mix(time.delta_secs(), voices.iter().count());
    let Some(asset_server) = asset_server else {
        return;
    };
    let master = options.as_ref().map_or(1.0, |o| o.master_volume);
    let sfx = options.as_ref().map_or(1.0, |o| o.sfx_volume);
    for request in started {
        commands.spawn((
            AudioPlayer::new(asset_server.load(request.sample.audio_path())),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(request.volume * master * sfx))
                .with_speed(request.speed),
            CombatSfx,
            MatchScopedEntity,
        ));
    }
}

#[cfg(not(feature = "audio"))]
pub(crate) fn play_combat_audio(time: Res<Time>, mut mixer: ResMut<CombatAudioMixer>) {
    mixer.mix(time.delta_secs(), 0);
}

// ---------------------------------------------------------------------------
// Turning simulation effect events into feel
// ---------------------------------------------------------------------------

type FogRevealers<'w, 's> = Query<
    'w,
    's,
    (
        &'static Team,
        &'static Transform,
        &'static VisionRadius,
        &'static VisibilityState,
    ),
>;

/// Whether the viewing player can currently see `position`. Feel effects for
/// fights hidden in the fog would leak information (and noise) to the player.
fn position_revealed(
    position: Vec3,
    visible_player: Option<&VisiblePlayer>,
    relations: Option<&TeamRelations>,
    revealers: &FogRevealers,
) -> bool {
    let (Some(visible_player), Some(relations)) = (visible_player, relations) else {
        return true;
    };
    if visible_player.all_players_visible() {
        return true;
    }
    revealers.iter().any(|(team, transform, radius, state)| {
        state.visible
            && relations.are_allied(visible_player.team, *team)
            && xz_distance(position, transform.translation) <= radius.0 + FOG_COMPENSATION
    })
}

/// Reacts to the effect entities the simulation spawned this frame: big ones
/// get a shockwave, secondary pops, camera shake and a layered blast; every
/// visible hit gets its weapon-class sound.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_combat_feel(
    mut commands: Commands,
    camera: Res<RtsCamera>,
    mut shake: ResMut<CameraShake>,
    mut mixer: ResMut<CombatAudioMixer>,
    visible_player: Option<Res<VisiblePlayer>>,
    relations: Option<Res<TeamRelations>>,
    revealers: FogRevealers,
    new_bursts: Query<(&Transform, &ImpactBurst), Added<ImpactBurst>>,
    new_destructions: Query<(&Transform, &StructureDestructionVfx), Added<StructureDestructionVfx>>,
    new_wrecks: Query<&Transform, Added<CombatWreckage>>,
    new_shots: Query<(&ShotPulse, &ShotWeaponClass), Added<ShotWeaponClass>>,
    new_deaths: Query<(&Transform, &UnitDeathVfx), Added<UnitDeathVfx>>,
    asset_server: Option<Res<AssetServer>>,
    options: Option<Res<MenuOptionsState>>,
) {
    let revealed = |position: Vec3| {
        position_revealed(
            position,
            visible_player.as_deref(),
            relations.as_deref(),
            &revealers,
        )
    };
    let shake_falloff = |position: Vec3| {
        focus_falloff(
            xz_distance(camera.focus, position),
            CAMERA_SHAKE_NEAR_DISTANCE,
            CAMERA_SHAKE_FAR_DISTANCE,
        )
    };

    // Muzzle flash for every weapon shot the player can see. Derived from the
    // tracer itself, so online clients get it from the replicated pulse.
    for (pulse, weapon) in &new_shots {
        let (from, to) = (pulse.from - Vec3::Y * 0.6, pulse.to - Vec3::Y * 0.6);
        if revealed(from) {
            spawn_muzzle_flash(&mut commands, from, to, weapon.kind);
        }
    }

    // Unit deaths replayed from the host: stage the same wreck and fireball
    // the host spawned directly. The wreck then triggers its blast sound below
    // on the next frame, exactly like a locally simulated death.
    if let Some(asset_server) = asset_server.as_deref() {
        for (transform, death) in &new_deaths {
            if death.replica && revealed(transform.translation) {
                spawn_death_presentation(
                    &mut commands,
                    asset_server,
                    transform.translation,
                    death.radius,
                    false,
                );
            }
        }
    }

    for (transform, burst) in &new_bursts {
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        shake.kick(impact_shake_strength(burst) * shake_falloff(position));
        let loudness = audio_falloff(&camera, position);
        let jitter = pitch_jitter(position);
        if is_strike_burst(burst) {
            spawn_strike_explosion(&mut commands, position, burst.radius);
            mixer.request(CombatSample::Blast, 0.5 * jitter, loudness);
            mixer.request(CombatSample::Blast, 0.8 * jitter, 0.7 * loudness);
            mixer.request(CombatSample::Hit, 0.45, 0.5 * loudness);
            continue;
        }
        for (sample, speed, volume) in impact_sound_layers(burst.kind, burst.power)
            .into_iter()
            .flatten()
        {
            mixer.request(sample, speed * jitter, volume * loudness);
        }
        let ring = match burst.kind {
            ImpactBurstKind::Siege => Some((2.4, LinearRgba::new(1.3, 0.9, 0.5, 1.0))),
            ImpactBurstKind::Explosive if burst.power >= 1.0 => {
                Some((1.7, LinearRgba::new(1.2, 0.62, 0.22, 1.0)))
            }
            ImpactBurstKind::Heavy if burst.power >= 1.2 => {
                Some((1.5, LinearRgba::new(1.15, 0.85, 0.5, 1.0)))
            }
            _ => None,
        };
        if let Some((reach, color)) = ring {
            spawn_shockwave_ring(
                &mut commands,
                position,
                burst.radius * 0.3,
                burst.radius * reach,
                0.32,
                color,
            );
        }
    }

    for (transform, effect) in &new_destructions {
        if effect.kind != StructureDestructionVfxKind::ExplosionFireball {
            continue;
        }
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        shake.kick(structure_destruction_shake_strength(effect.radius) * shake_falloff(position));
        spawn_shockwave_ring(
            &mut commands,
            position,
            effect.radius * 0.4,
            effect.radius * 2.4,
            0.5,
            LinearRgba::new(1.25, 0.7, 0.28, 1.0),
        );
        spawn_secondary_explosions(&mut commands, position, effect.radius, 3);
        let loudness = audio_falloff(&camera, position);
        mixer.request(CombatSample::Blast, 0.58, 0.95 * loudness);
        mixer.request(CombatSample::Hit, 0.5, 0.45 * loudness);
    }

    for transform in &new_wrecks {
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        mixer.request(
            CombatSample::Blast,
            0.95 * pitch_jitter(position),
            0.6 * audio_falloff(&camera, position),
        );
    }

    // Shake switched off in Options: discard this frame's kicks as well.
    if options.is_some_and(|options| !options.screen_shake) {
        shake.clear();
    }
}

/// The full support-power detonation: white-hot core, rolling fireball, a
/// wide shockwave, flung wreckage, a scorched crater and cook-off pops.
fn spawn_strike_explosion(commands: &mut Commands, position: Vec3, radius: f32) {
    spawn_combat_flash(
        commands,
        position + Vec3::Y * 0.3,
        radius * 0.25,
        radius * 0.55,
        0.22,
        LinearRgba::new(1.6, 1.45, 1.1, 1.0),
    );
    spawn_combat_flash_with_alpha(
        commands,
        position + Vec3::Y * 0.2,
        radius * 0.25,
        (radius * 0.8).min(3.2),
        0.6,
        LinearRgba::new(1.15, 0.5, 0.14, 1.0),
        0.6,
    );
    spawn_shockwave_ring(
        commands,
        position,
        radius * 0.35,
        radius * 1.9,
        0.55,
        LinearRgba::new(1.3, 0.85, 0.45, 1.0),
    );
    spawn_debris_burst(commands, position, 10, 4.2, DebrisPalette::Metal);
    spawn_debris_burst(commands, position, 12, 5.4, DebrisPalette::Ember);
    spawn_scorch_mark(commands, position, (radius * 0.8).clamp(0.8, 3.2));
    spawn_secondary_explosions(commands, position, radius, 3);
}

// ---------------------------------------------------------------------------
// Unit hit flash
// ---------------------------------------------------------------------------

/// Remembers a unit's last seen health so a drop can flash it. Presentation
/// bookkeeping only; never read by the simulation, saves or snapshots.
#[derive(Component)]
pub(crate) struct HitFlashTracker {
    pub(crate) last_health: f32,
    pub(crate) cooldown: f32,
}

/// A unit under sustained fire flashes at most this often.
pub(crate) const HIT_FLASH_INTERVAL_SECONDS: f32 = 0.2;

/// A building under sustained fire flashes at most this often.
pub(crate) const STRUCTURE_HIT_FLASH_INTERVAL_SECONDS: f32 = 0.35;

/// Whether a health change from `previous` to `current` should flash now.
pub(crate) fn hit_flash_due(previous: f32, current: f32, cooldown: f32) -> bool {
    cooldown <= 0.0 && current > 0.0 && current < previous - 0.01
}

/// Washes a unit white for a few frames whenever it loses health. Derived from
/// [`Health`] alone, so it works identically for direct hits, splash, support
/// powers, and for units that only exist as online replicas.
type HitFlashUnit<'a> = (
    Entity,
    &'a Health,
    &'a Transform,
    &'a Selectable,
    Option<&'a VisibilityState>,
    Option<&'a mut HitFlashTracker>,
    Has<Structure>,
);

pub(crate) fn flash_damaged_units(
    mut commands: Commands,
    time: Res<Time>,
    mut units: Query<HitFlashUnit, Or<(With<Unit>, With<Structure>)>>,
) {
    let dt = time.delta_secs();
    for (entity, health, transform, selectable, visibility, tracker, is_structure) in &mut units {
        let Some(mut tracker) = tracker else {
            commands.entity(entity).try_insert(HitFlashTracker {
                last_health: health.current,
                cooldown: 0.0,
            });
            continue;
        };
        tracker.cooldown = (tracker.cooldown - dt).max(0.0);
        let due = hit_flash_due(tracker.last_health, health.current, tracker.cooldown);
        tracker.last_health = health.current;
        if !due || visibility.is_some_and(|visibility| !visibility.visible) {
            continue;
        }
        if is_structure {
            // Buildings are big and get hit in volleys: a softer, slower,
            // warmer wash over the hull instead of a full white-out.
            tracker.cooldown = STRUCTURE_HIT_FLASH_INTERVAL_SECONDS;
            let radius = selectable.radius.clamp(0.8, 2.6);
            spawn_combat_flash_with_alpha(
                &mut commands,
                transform.translation + Vec3::Y * (radius * 0.1),
                radius * 0.95,
                radius * 1.05,
                0.12,
                LinearRgba::new(1.5, 1.3, 1.0, 1.0),
                0.3,
            );
            continue;
        }
        tracker.cooldown = HIT_FLASH_INTERVAL_SECONDS;
        let radius = selectable.radius.clamp(0.3, 1.2);
        spawn_combat_flash_with_alpha(
            &mut commands,
            // `spawn_combat_flash` lifts by 0.4 * end_scale; aim for mid-body.
            transform.translation + Vec3::Y * (radius * 0.15),
            radius * 0.7,
            radius * 0.82,
            0.09,
            LinearRgba::new(1.5, 1.45, 1.35, 1.0),
            0.5,
        );
    }
}

// ---------------------------------------------------------------------------
// Match-end cleanup
// ---------------------------------------------------------------------------

/// Removes in-flight feel effects once a match resolves. Their update systems
/// stop with the match, so anything left would hang frozen over the result
/// scene (same reasoning as the `CombatFlash` cleanup in
/// `clear_battle_hud_on_match_end`). Wreckage and scorch marks are part of
/// the battlefield and stay.
type InFlightFeelEffect = Or<(
    With<ShockwaveRing>,
    With<SecondaryExplosion>,
    With<DebrisChunk>,
)>;

pub(crate) fn clear_combat_feel_on_match_end(
    mut commands: Commands,
    match_state: Res<MatchState>,
    mut shake: ResMut<CameraShake>,
    mut mixer: ResMut<CombatAudioMixer>,
    transients: Query<Entity, InFlightFeelEffect>,
) {
    if match_state.is_running() {
        return;
    }
    for entity in &transients {
        commands.entity(entity).try_despawn();
    }
    shake.clear();
    mixer.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst(kind: ImpactBurstKind, radius: f32, power: f32) -> ImpactBurst {
        ImpactBurst {
            remaining: 0.3,
            total: 0.3,
            radius,
            power,
            team: Team::Player(0),
            kind,
        }
    }

    #[test]
    fn shake_falls_off_with_distance_from_camera_focus() {
        assert_eq!(focus_falloff(0.0, 5.0, 24.0), 1.0);
        assert_eq!(focus_falloff(5.0, 5.0, 24.0), 1.0);
        let mid = focus_falloff(14.5, 5.0, 24.0);
        assert!((mid - 0.5).abs() < 1e-4, "midpoint falloff was {mid}");
        assert_eq!(focus_falloff(24.0, 5.0, 24.0), 0.0);
        assert_eq!(focus_falloff(500.0, 5.0, 24.0), 0.0);
    }

    #[test]
    fn shake_kicks_do_not_stack_past_the_strongest() {
        let mut shake = CameraShake::default();
        shake.kick(0.4);
        shake.kick(0.3);
        assert_eq!(shake.trauma, 0.4);
    }

    #[test]
    fn shake_offset_never_exceeds_the_cap() {
        let mut shake = CameraShake::default();
        shake.kick(5.0);
        assert_eq!(shake.trauma, 1.0);
        for step in 0..600 {
            shake.clock = step as f32 * 0.0137;
            let offset = shake.offset();
            assert!(offset.x.abs() <= CAMERA_SHAKE_MAX_OFFSET + 1e-5);
            assert!(offset.y.abs() <= CAMERA_SHAKE_MAX_OFFSET + 1e-5);
        }
    }

    #[test]
    fn sustained_bombardment_exhausts_the_shake_budget() {
        let mut shake = CameraShake::default();
        let mut accepted = Vec::new();
        // Ten full-strength strikes, one every 100 ms.
        for _ in 0..10 {
            let before = shake.budget;
            shake.kick(0.9);
            accepted.push(before - shake.budget);
            shake.tick(0.1);
        }
        assert!((accepted[0] - 0.9).abs() < 1e-5);
        let later: f32 = accepted[2..].iter().sum();
        assert!(
            later < 0.9,
            "eight follow-up strikes spent {later} budget; rate limit is not holding"
        );
    }

    #[test]
    fn shake_decays_to_rest_and_budget_recovers() {
        let mut shake = CameraShake::default();
        shake.kick(1.0);
        for _ in 0..120 {
            shake.tick(1.0 / 30.0);
        }
        assert_eq!(shake.trauma, 0.0);
        assert_eq!(shake.offset(), Vec2::ZERO);
        assert_eq!(shake.budget, CAMERA_SHAKE_BUDGET_MAX);
    }

    #[test]
    fn only_big_impacts_shake_the_camera() {
        assert_eq!(
            impact_shake_strength(&burst(ImpactBurstKind::Ballistic, 0.6, 2.2)),
            0.0
        );
        assert_eq!(
            impact_shake_strength(&burst(ImpactBurstKind::Energy, 0.6, 2.2)),
            0.0
        );
        assert_eq!(
            impact_shake_strength(&burst(ImpactBurstKind::Heavy, 0.8, 1.2)),
            0.0
        );
        assert!(impact_shake_strength(&burst(ImpactBurstKind::Heavy, 0.8, 2.0)) > 0.0);
        assert!(impact_shake_strength(&burst(ImpactBurstKind::Siege, 0.9, 1.0)) > 0.0);
        let strike = burst(ImpactBurstKind::Siege, 3.0, 2.2);
        assert!(is_strike_burst(&strike));
        assert!(
            impact_shake_strength(&strike)
                > impact_shake_strength(&burst(ImpactBurstKind::Siege, 1.45, 2.2))
        );
    }

    #[test]
    fn ordinary_weapon_bursts_are_never_mistaken_for_strikes() {
        // `spawn_impact_burst` clamps its radius to 1.45.
        assert!(!is_strike_burst(&burst(ImpactBurstKind::Siege, 1.45, 2.2)));
    }

    #[test]
    fn mixer_starts_the_loudest_sounds_and_caps_the_frame() {
        let mut mixer = CombatAudioMixer::default();
        for volume in [0.2, 0.9, 0.5, 0.7, 0.3] {
            mixer.request(CombatSample::Blast, 1.0, volume);
        }
        let started = mixer.mix(1.0 / 60.0, 0);
        let volumes: Vec<f32> = started.iter().map(|request| request.volume).collect();
        assert_eq!(volumes, vec![0.9, 0.7, 0.5]);
        assert!(mixer.pending.is_empty(), "dropped sounds must not queue up");
    }

    #[test]
    fn mixer_spaces_out_light_hits() {
        let mut mixer = CombatAudioMixer::default();
        mixer.request(CombatSample::Hit, 1.0, 0.5);
        mixer.request(CombatSample::Hit, 1.0, 0.4);
        assert_eq!(mixer.mix(1.0 / 60.0, 0).len(), 1);
        mixer.request(CombatSample::Hit, 1.0, 0.5);
        assert!(mixer.mix(1.0 / 60.0, 0).is_empty());
        mixer.request(CombatSample::Hit, 1.0, 0.5);
        assert_eq!(mixer.mix(COMBAT_AUDIO_HIT_INTERVAL_SECONDS, 0).len(), 1);
    }

    #[test]
    fn mixer_respects_the_voice_cap_and_ignores_inaudible_requests() {
        let mut mixer = CombatAudioMixer::default();
        mixer.request(CombatSample::Blast, 1.0, 0.8);
        assert!(mixer.mix(0.016, COMBAT_AUDIO_MAX_VOICES).is_empty());
        mixer.request(CombatSample::Blast, 1.0, 0.01);
        assert!(mixer.pending.is_empty());
    }

    #[test]
    fn weapon_classes_sound_different() {
        let kinds = [
            ImpactBurstKind::Ballistic,
            ImpactBurstKind::Explosive,
            ImpactBurstKind::Energy,
            ImpactBurstKind::Electric,
            ImpactBurstKind::Fire,
            ImpactBurstKind::Heavy,
            ImpactBurstKind::Siege,
        ];
        for (index, a) in kinds.iter().enumerate() {
            for b in &kinds[index + 1..] {
                assert_ne!(
                    impact_sound_layers(*a, 1.0),
                    impact_sound_layers(*b, 1.0),
                    "{a:?} and {b:?} share a sound"
                );
            }
        }
    }

    #[test]
    fn combat_samples_exist_on_disk() {
        for sample in [CombatSample::Hit, CombatSample::Blast] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(sample.audio_path());
            assert!(path.is_file(), "missing combat sample {}", path.display());
        }
    }

    #[test]
    fn every_weapon_class_draws_its_own_tracer() {
        let (from, to) = (Vec3::new(0.0, 0.6, 0.0), Vec3::new(6.0, 0.6, 2.0));
        let kinds = [
            ImpactBurstKind::Ballistic,
            ImpactBurstKind::Explosive,
            ImpactBurstKind::Energy,
            ImpactBurstKind::Electric,
            ImpactBurstKind::Fire,
            ImpactBurstKind::Heavy,
            ImpactBurstKind::Siege,
        ];
        let shapes: Vec<_> = kinds
            .iter()
            .map(|kind| tracer_segments(*kind, from, to, 0.5))
            .collect();
        for (index, shape) in shapes.iter().enumerate() {
            assert!(!shape.is_empty(), "{:?} draws nothing", kinds[index]);
            for (start, end, color) in shape {
                assert!(start.is_finite() && end.is_finite());
                assert!((0.0..=1.0).contains(&color.alpha));
                // Nothing may be drawn far outside the muzzle-target span.
                let reach = from.distance(to) + 1.0;
                assert!(start.distance(from) <= reach && end.distance(from) <= reach);
            }
            for other in &shapes[index + 1..] {
                assert_ne!(shape, other, "two weapon classes share a tracer");
            }
        }
        // Beams are steady; bullets travel.
        let beam = |life| tracer_segments(ImpactBurstKind::Energy, from, to, life)[2];
        assert_eq!((beam(0.9).0, beam(0.9).1), (beam(0.2).0, beam(0.2).1));
        let dash = |life| tracer_segments(ImpactBurstKind::Ballistic, from, to, life)[0].1;
        assert!(dash(0.9).distance(from) < dash(0.5).distance(from));
        assert!(tracer_segments(ImpactBurstKind::Siege, from, from, 0.5).is_empty());
    }

    #[test]
    fn hit_flash_needs_real_damage_a_live_unit_and_a_rested_cooldown() {
        assert!(hit_flash_due(100.0, 90.0, 0.0));
        assert!(!hit_flash_due(100.0, 90.0, 0.1), "cooldown must gate");
        assert!(!hit_flash_due(100.0, 100.0, 0.0), "no damage, no flash");
        assert!(!hit_flash_due(90.0, 100.0, 0.0), "healing must not flash");
        assert!(
            !hit_flash_due(10.0, 0.0, 0.0),
            "the death blast covers kills"
        );
    }
}
