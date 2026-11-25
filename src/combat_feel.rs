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

use bevy::ecs::system::SystemParam;
use bevy::post_process::effect_stack::{ChromaticAberration, Vignette};

use crate::*;

// ---------------------------------------------------------------------------
// Dynamic post-process impacts (chromatic aberration + damage/HQ vignette)
// and combat hit/kill-streak state
// ---------------------------------------------------------------------------

/// Subtle baseline chromatic aberration during active matches (keeps edges crisp).
pub(crate) const POST_PROCESS_BASE_CHROMA: f32 = 0.0015;

/// Upper cap for peak impact chromatic aberration pulse.
pub(crate) const POST_PROCESS_MAX_CHROMA: f32 = 0.036;

/// Chromatic pulse decay rate (~150ms snap back to baseline).
pub(crate) const POST_PROCESS_CHROMA_DECAY_PER_SEC: f32 = 0.22;

/// Subtle baseline vignette framing for the live RTS camera.
pub(crate) const POST_PROCESS_BASE_VIGNETTE_INTENSITY: f32 = 0.18;

/// Damage-alert vignette decay rate.
pub(crate) const POST_PROCESS_VIGNETTE_DECAY_PER_SEC: f32 = 2.4;

/// Player Command Center HP ratio below which a gentle crimson heartbeat vignette activates.
pub(crate) const HQ_ALERT_HEALTH_THRESHOLD: f32 = 0.35;

/// Time window before the combat hit/kill streak decays back to zero.
pub(crate) const COMBAT_STREAK_DECAY_SECONDS: f32 = 3.0;

/// Duration of the HUD badge pop-scale animation on each streak increment.
pub(crate) const COMBAT_STREAK_POP_SECONDS: f32 = 0.16;

/// Minimum streak count before the right-side streak HUD badge reveals itself.
pub(crate) const COMBAT_STREAK_HUD_THRESHOLD: u32 = 3;

/// Contact audio pitch escalation per streak hit (+2.5%), capped at 8 steps (+20%).
pub(crate) const COMBAT_STREAK_PITCH_STEP: f32 = 0.025;
pub(crate) const COMBAT_STREAK_PITCH_MAX_STEPS: u32 = 8;

/// Duration (75ms) of the real unit mesh emissive hit-flash material swap.
pub(crate) const UNIT_MESH_HIT_FLASH_SECONDS: f32 = 0.075;

pub(crate) fn default_match_chromatic_aberration() -> ChromaticAberration {
    ChromaticAberration {
        color_lut: None,
        intensity: POST_PROCESS_BASE_CHROMA,
        max_samples: 4,
    }
}

pub(crate) fn default_match_vignette() -> Vignette {
    Vignette {
        intensity: POST_PROCESS_BASE_VIGNETTE_INTENSITY,
        radius: 0.92,
        smoothness: 2.8,
        roundness: 0.92,
        color: Color::srgb(0.03, 0.035, 0.05),
        edge_compensation: 0.86,
        center: Vec2::new(0.5, 0.5),
    }
}

/// Presentation-only state driving dynamic `ChromaticAberration` impact pulses
/// and crimson `Vignette` damage/HQ-alert pulses on `MainCamera`.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct PostProcessImpactState {
    pub(crate) chroma_pulse: f32,
    pub(crate) damage_vignette: f32,
}

impl PostProcessImpactState {
    /// Pulses chromatic aberration on heavy blasts, support strikes, and structure
    /// destructions (`strength` is already distance-attenuated in `0..=1`).
    pub(crate) fn spike_chroma(&mut self, strength: f32) {
        if strength >= 0.08 {
            let bonus = (0.008 + (strength - 0.08) * 0.03).clamp(0.008, 0.034);
            self.chroma_pulse = self.chroma_pulse.max(bonus);
        }
    }

    /// Pulses the crimson damage/alert vignette (`strength` in `0..=1`).
    pub(crate) fn spike_damage_vignette(&mut self, strength: f32) {
        let strength = strength.clamp(0.0, 1.0);
        if strength > 0.0 {
            self.damage_vignette = self.damage_vignette.max(strength);
        }
    }

    pub(crate) fn tick(&mut self, delta_secs: f32) {
        let dt = delta_secs.max(0.0);
        self.chroma_pulse = (self.chroma_pulse - POST_PROCESS_CHROMA_DECAY_PER_SEC * dt).max(0.0);
        self.damage_vignette =
            (self.damage_vignette - POST_PROCESS_VIGNETTE_DECAY_PER_SEC * dt).max(0.0);
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

pub(crate) fn impact_chroma_intensity(chroma_pulse: f32) -> f32 {
    (POST_PROCESS_BASE_CHROMA + chroma_pulse.max(0.0))
        .clamp(POST_PROCESS_BASE_CHROMA, POST_PROCESS_MAX_CHROMA)
}

/// Maps the player's highest surviving HQ health fraction into a `0..=1` low-HP
/// urgency factor once below [`HQ_ALERT_HEALTH_THRESHOLD`] (35%).
pub(crate) fn hq_low_health_ratio(best_hq_fraction: Option<f32>) -> f32 {
    let Some(frac) = best_hq_fraction else {
        return 0.0;
    };
    let frac = frac.clamp(0.0, 1.0);
    if frac < HQ_ALERT_HEALTH_THRESHOLD {
        ((HQ_ALERT_HEALTH_THRESHOLD - frac) / HQ_ALERT_HEALTH_THRESHOLD).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Computes live `(intensity, color)` for the camera `Vignette`, blending a
/// subtle dark framing baseline with crimson damage flashes and a gentle
/// low-HQ heartbeat pulse.
pub(crate) fn damage_vignette_params(
    damage_flash: f32,
    hq_low_ratio: f32,
    elapsed_secs: f32,
) -> (f32, Color) {
    let base_r = 0.03_f32;
    let base_g = 0.035_f32;
    let base_b = 0.05_f32;
    let hq_low = hq_low_ratio.clamp(0.0, 1.0);
    let heartbeat = if hq_low > 0.0 {
        hq_low * (0.45 + 0.55 * (elapsed_secs * 5.2).sin().abs())
    } else {
        0.0
    };
    let crimson = (damage_flash.clamp(0.0, 1.0) * 0.85 + heartbeat * 0.65).clamp(0.0, 1.0);
    let intensity = POST_PROCESS_BASE_VIGNETTE_INTENSITY + crimson * 0.44;
    let color = Color::srgb(
        base_r.lerp(0.58, crimson),
        base_g.lerp(0.02, crimson),
        base_b.lerp(0.03, crimson),
    );
    (intensity, color)
}

/// Presentation-only combo/kill-streak tracker driving pitch-escalating contact
/// audio and the auto-hiding bilingual right-side streak HUD badge.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct CombatStreakState {
    pub(crate) streak_hits: u32,
    pub(crate) streak_timer: f32,
    pub(crate) pop_secs: f32,
}

impl CombatStreakState {
    pub(crate) fn register_hit(&mut self) -> u32 {
        self.streak_hits = self.streak_hits.saturating_add(1);
        self.streak_timer = COMBAT_STREAK_DECAY_SECONDS;
        self.pop_secs = COMBAT_STREAK_POP_SECONDS;
        self.streak_hits
    }

    pub(crate) fn tick(&mut self, delta_secs: f32) {
        let dt = delta_secs.max(0.0);
        if self.streak_timer > 0.0 {
            self.streak_timer = (self.streak_timer - dt).max(0.0);
            if self.streak_timer <= 0.0 {
                self.streak_hits = 0;
            }
        }
        self.pop_secs = (self.pop_secs - dt).max(0.0);
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Slight pitch escalation (+2.5% per hit, capped at +20%) on consecutive
/// visible hostile impacts/kills: `1.0 + min(streak - 1, 8) * 0.025`.
pub(crate) fn streak_pitch_multiplier(streak_hits: u32) -> f32 {
    1.0 + (streak_hits
        .saturating_sub(1)
        .min(COMBAT_STREAK_PITCH_MAX_STEPS) as f32)
        * COMBAT_STREAK_PITCH_STEP
}

/// Marker for the right-side auto-hiding bilingual hit/kill-streak HUD badge root.
#[derive(Component)]
pub(crate) struct CombatStreakHudRoot;

/// Marker for the text inside [`CombatStreakHudRoot`].
#[derive(Component)]
pub(crate) struct CombatStreakHudText;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CombatStreakBadgeStyle {
    pub(crate) visible: bool,
    pub(crate) label: String,
    pub(crate) text_color: Color,
    pub(crate) border_color: Color,
    pub(crate) background_color: Color,
    pub(crate) scale: f32,
}

pub(crate) fn combat_streak_badge_style(
    streak_hits: u32,
    pop_secs: f32,
    language: Language,
) -> CombatStreakBadgeStyle {
    let visible = streak_hits >= COMBAT_STREAK_HUD_THRESHOLD;
    let scale = if pop_secs > 0.0 {
        1.0 + (pop_secs / COMBAT_STREAK_POP_SECONDS).clamp(0.0, 1.0) * 0.26
    } else {
        1.0
    };
    let label = if visible {
        match language {
            Language::Zh => format!("连击 x{streak_hits}"),
            Language::En => format!("STREAK x{streak_hits}"),
        }
    } else {
        String::new()
    };
    // Color tiers: 3+ amber, 10+ orange, 20+ blazing gold.
    let (text_color, border_color, background_color) = if streak_hits >= 20 {
        (
            Color::srgb(1.0, 0.94, 0.48),
            Color::srgba(1.0, 0.82, 0.22, 0.98),
            Color::srgba(0.14, 0.09, 0.015, 0.92),
        )
    } else if streak_hits >= 10 {
        (
            Color::srgb(1.0, 0.58, 0.18),
            Color::srgba(1.0, 0.46, 0.12, 0.94),
            Color::srgba(0.11, 0.04, 0.015, 0.88),
        )
    } else {
        (
            Color::srgb(1.0, 0.84, 0.36),
            Color::srgba(0.95, 0.72, 0.22, 0.88),
            Color::srgba(0.08, 0.05, 0.02, 0.86),
        )
    };
    CombatStreakBadgeStyle {
        visible,
        label,
        text_color,
        border_color,
        background_color,
        scale,
    }
}

/// Spawns the sleek right-side bilingual hit/kill-streak HUD badge (hidden until 3+ hits).
pub(crate) fn setup_combat_streak_hud(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Name::new("Combat Streak Badge"),
            CombatStreakHudRoot,
            GlobalZIndex(19),
            Visibility::Hidden,
            UiTransform::IDENTITY,
            Node {
                position_type: PositionType::Absolute,
                right: px(14),
                top: px(OBJECTIVE_TRACKER_TOP_PX + 76.0),
                padding: UiRect::axes(px(12.0), px(6.0)),
                border: UiRect::all(px(1.5)),
                border_radius: BorderRadius::all(px(5.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.05, 0.02, 0.86)),
            BorderColor::all(Color::srgba(0.95, 0.72, 0.22, 0.88)),
            MatchScopedEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                CombatStreakHudText,
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.36)),
            ));
        });
}

#[allow(clippy::type_complexity)]
pub(crate) fn update_post_process_impacts(
    mut commands: Commands,
    time: Res<Time>,
    options: Option<Res<MenuOptionsState>>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut post_process: ResMut<PostProcessImpactState>,
    mut streak: ResMut<CombatStreakState>,
    structures: Query<(&Team, &Structure, &Health), Without<UnderConstruction>>,
    mut cameras: Query<
        (
            Entity,
            Option<&mut ChromaticAberration>,
            Option<&mut Vignette>,
        ),
        With<MainCamera>,
    >,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    post_process.tick(dt);
    streak.tick(dt);

    if options.is_some_and(|options| !options.screen_shake) {
        post_process.chroma_pulse = 0.0;
    }

    let best_hq_fraction = if visible_player
        .as_deref()
        .is_some_and(|vp| vp.is_spectator())
    {
        None
    } else {
        let player_team = visible_player
            .as_deref()
            .map_or(Team::Player(0), |vp| vp.team);
        let mut best: Option<f32> = None;
        for (team, structure, health) in &structures {
            if *team == player_team
                && structure.id == "CommandCenter"
                && health.max > 0.0
                && health.current > 0.0
            {
                let frac = (health.current / health.max).clamp(0.0, 1.0);
                best = Some(best.map_or(frac, |prev| prev.max(frac)));
            }
        }
        best
    };

    let hq_low = hq_low_health_ratio(best_hq_fraction);
    let chroma_val = impact_chroma_intensity(post_process.chroma_pulse);
    let (vig_intensity, vig_color) =
        damage_vignette_params(post_process.damage_vignette, hq_low, elapsed);

    for (cam_entity, chroma, vignette) in &mut cameras {
        if let Some(mut chroma) = chroma {
            chroma.intensity = chroma_val;
        } else {
            let mut next = default_match_chromatic_aberration();
            next.intensity = chroma_val;
            commands.entity(cam_entity).try_insert(next);
        }
        if let Some(mut vignette) = vignette {
            vignette.intensity = vig_intensity;
            vignette.color = vig_color;
        } else {
            let mut next = default_match_vignette();
            next.intensity = vig_intensity;
            next.color = vig_color;
            commands.entity(cam_entity).try_insert(next);
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn update_combat_streak_hud(
    locale: Option<Res<Locale>>,
    streak: Res<CombatStreakState>,
    mut root_q: Query<
        (
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut UiTransform,
        ),
        With<CombatStreakHudRoot>,
    >,
    mut text_q: Query<(&mut Text, &mut TextColor), With<CombatStreakHudText>>,
) {
    let language = locale.as_deref().map_or_else(current_language, |l| l.0);
    let style = combat_streak_badge_style(streak.streak_hits, streak.pop_secs, language);
    for (mut visibility, mut bg, mut border, mut ui_tf) in &mut root_q {
        let wanted = if style.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        *bg = BackgroundColor(style.background_color);
        *border = BorderColor::all(style.border_color);
        ui_tf.scale = Vec2::splat(style.scale);
    }
    if style.visible {
        for (mut text, mut tc) in &mut text_q {
            if text.0 != style.label {
                text.0 = style.label.clone();
            }
            tc.0 = style.text_color;
        }
    }
}

// ---------------------------------------------------------------------------
// Camera shake
// ---------------------------------------------------------------------------

/// Largest camera displacement, in world units, at full trauma. Small on
/// purpose: the camera sits 6-11 units up, so this is roughly 1-2% of the view.
pub(crate) const CAMERA_SHAKE_MAX_OFFSET: f32 = 0.16;

/// Largest forward camera zoom punch, in world units, on massive blasts.
pub(crate) const CAMERA_ZOOM_PUNCH_MAX: f32 = 0.42;

/// Duration (85ms) of the white-hot additive impact contact core.
pub(crate) const CONTACT_HIT_CORE_SECONDS: f32 = 0.085;

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
    /// Instantaneous forward zoom punch displacement (0..=CAMERA_ZOOM_PUNCH_MAX).
    pub(crate) zoom_punch: f32,
    /// Spring velocity for critically-damped zoom punch recovery.
    pub(crate) zoom_punch_velocity: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            budget: CAMERA_SHAKE_BUDGET_MAX,
            clock: 0.0,
            zoom_punch: 0.0,
            zoom_punch_velocity: 0.0,
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

    /// Requests a zero-drift forward impact zoom punch (`strength` in `0..=1`,
    /// already distance-attenuated). Snaps in fast and springs back cleanly.
    pub(crate) fn kick_zoom_punch(&mut self, strength: f32) {
        let strength = strength.clamp(0.0, 1.0);
        if strength < CAMERA_SHAKE_MIN_KICK {
            return;
        }
        let target_impulse = CAMERA_ZOOM_PUNCH_MAX * strength;
        self.zoom_punch = self.zoom_punch.max(target_impulse * 0.65);
        self.zoom_punch_velocity = self.zoom_punch_velocity.max(target_impulse * 9.0);
    }

    pub(crate) fn tick(&mut self, delta_secs: f32) {
        let dt = delta_secs.clamp(0.0, 0.1);
        self.clock += dt;
        self.trauma = (self.trauma - CAMERA_SHAKE_DECAY_PER_SECOND * dt).max(0.0);
        self.budget =
            (self.budget + CAMERA_SHAKE_BUDGET_REGEN_PER_SECOND * dt).min(CAMERA_SHAKE_BUDGET_MAX);

        // Critically-damped spring back to exact 0.0 for camera zoom punch.
        if self.zoom_punch.abs() > 1e-4 || self.zoom_punch_velocity.abs() > 1e-4 {
            let omega = 18.0_f32;
            let f = 1.0 + omega * dt;
            let inv_f = 1.0 / (f * f);
            let next_vel =
                (self.zoom_punch_velocity - self.zoom_punch * (omega * omega * dt)) * inv_f;
            let next_pos = (self.zoom_punch
                + (self.zoom_punch_velocity + self.zoom_punch * omega) * dt)
                * (-omega * dt).exp();
            self.zoom_punch = next_pos.clamp(-CAMERA_ZOOM_PUNCH_MAX * 0.15, CAMERA_ZOOM_PUNCH_MAX);
            self.zoom_punch_velocity = next_vel;
            if self.zoom_punch.abs() < 1e-4 && self.zoom_punch_velocity.abs() < 1e-3 {
                self.zoom_punch = 0.0;
                self.zoom_punch_velocity = 0.0;
            }
        }
    }

    /// Damped 3-harmonic camera-plane displacement (x = right, y = up) for the
    /// current trauma. Squared trauma keeps small kicks subtle and big ones punchy.
    pub(crate) fn offset(&self) -> Vec2 {
        let amplitude = CAMERA_SHAKE_MAX_OFFSET * self.trauma * self.trauma;
        if amplitude <= 0.0 {
            return Vec2::ZERO;
        }
        let t = self.clock;
        Vec2::new(
            (t * 67.0).sin() * 0.55
                + (t * 113.0 + 1.3).sin() * 0.30
                + (t * 179.0 + 2.7).sin() * 0.15,
            (t * 79.0 + 0.7).sin() * 0.55
                + (t * 127.0 + 2.1).sin() * 0.30
                + (t * 193.0 + 0.4).sin() * 0.15,
        ) * amplitude
    }

    /// Forward-axis impact zoom punch offset (world units, zero-drift).
    pub(crate) fn zoom_punch_offset(&self) -> f32 {
        self.zoom_punch.max(0.0)
    }

    /// Subtle micro-roll (radians) coupled to trauma for crisp blast percussion.
    pub(crate) fn roll_offset(&self) -> f32 {
        if self.trauma <= 0.0 {
            return 0.0;
        }
        let amp = 0.011 * self.trauma * self.trauma;
        ((self.clock * 59.0 + 0.5).sin() * 0.65 + (self.clock * 107.0 + 1.9).sin() * 0.35) * amp
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

/// Forward camera zoom punch strength for an impact burst (0 for rifle fire).
pub(crate) fn impact_zoom_punch_strength(burst: &ImpactBurst) -> f32 {
    if is_strike_burst(burst) {
        return 0.95;
    }
    match burst.kind {
        ImpactBurstKind::Siege => 0.42,
        ImpactBurstKind::Explosive | ImpactBurstKind::Heavy if burst.power >= 1.5 => 0.22,
        ImpactBurstKind::Electric if burst.power >= 1.6 => 0.2,
        _ => 0.0,
    }
}

pub(crate) fn structure_destruction_shake_strength(radius: f32) -> f32 {
    (0.38 + radius * 0.1).clamp(0.4, 0.7)
}

/// Decays the shake + zoom punch and nudges the match camera. `bevy_rts_camera`
/// rewrites the camera transform from its own state every frame, so the offset
/// never accumulates and never feeds back into camera focus, picking state or
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
    let zoom = shake.zoom_punch_offset();
    let roll = shake.roll_offset();
    if offset == Vec2::ZERO && zoom == 0.0 && roll == 0.0 {
        return;
    }
    for mut transform in &mut cameras {
        let displacement =
            transform.right() * offset.x + transform.up() * offset.y + *transform.forward() * zoom;
        transform.translation += displacement;
        if roll != 0.0 {
            transform.rotate_local_z(roll);
        }
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

pub(crate) fn spawn_additive_shock_ring(
    commands: &mut Commands,
    position: Vec3,
    start_radius: f32,
    end_radius: f32,
    ttl: f32,
    color: LinearRgba,
) {
    spawn_shockwave_ring(commands, position, start_radius, end_radius, ttl, color);
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
                    .add(Annulus::new(0.86, 1.0).mesh().resolution(48));
                if let Some(mut cache) = world.get_resource_mut::<ShockwaveRingMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let peak_alpha = 0.72;
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(color.red, color.green, color.blue, peak_alpha),
                emissive: color * 1.4,
                alpha_mode: AlphaMode::Add,
                unlit: true,
                cull_mode: None,
                ..default()
            });
        world.spawn((
            Name::new("Shockwave ring"),
            bevy::light::NotShadowCaster,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(position.x, 0.075, position.z))
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
            materials.remove(&ring.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        // Ease-out: the wave bursts out fast and coasts to a stop.
        let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
        let radius = ring.start_radius + (ring.end_radius - ring.start_radius) * eased;
        transform.scale = Vec3::splat(radius);
        if let Some(mut material) = materials.get_mut(&ring.material) {
            let fade = (1.0 - progress) * (1.0 - progress);
            material.base_color.set_alpha(ring.peak_alpha * fade);
            let srgb = material.base_color.to_linear();
            material.emissive = LinearRgba::new(
                srgb.red * fade * 1.4,
                srgb.green * fade * 1.4,
                srgb.blue * fade * 1.4,
                1.0,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AAA Additive Impact VFX: Razor Sparks, 85ms Contact Core, Crit Cross-Glint,
// Tapered Weapon Arc Smears, and Tread/Skid Dust Puffs
// ---------------------------------------------------------------------------

/// Shared elongated razor spark mesh cache.
#[derive(Resource, Default)]
pub(crate) struct RazorSparkMesh(pub(crate) Option<Handle<Mesh>>);

/// Shared crescent arc smear mesh cache.
#[derive(Resource, Default)]
pub(crate) struct WeaponArcSmearMesh(pub(crate) Option<Handle<Mesh>>);

/// Shared crossed-blade crit glint mesh cache.
#[derive(Resource, Default)]
pub(crate) struct CrossGlintMesh(pub(crate) Option<Handle<Mesh>>);

/// Shared faceted dust puff mesh cache.
#[derive(Resource, Default)]
pub(crate) struct TreadDustPuffMesh(pub(crate) Option<Handle<Mesh>>);

/// Directional razor-thin additive impact spark that stretches along velocity.
#[derive(Component)]
pub(crate) struct ImpactRazorSpark {
    pub(crate) velocity: Vec3,
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) base_length: f32,
    pub(crate) base_color: LinearRgba,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Blazing 85ms white-hot additive contact hit-flash core.
#[derive(Component)]
pub(crate) struct ContactHitCore {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) start_scale: f32,
    pub(crate) peak_scale: f32,
    pub(crate) base_color: LinearRgba,
    pub(crate) material: Handle<StandardMaterial>,
}

/// 4-point star / crossed additive glint on heavy, crit, or finisher hits.
#[derive(Component)]
pub(crate) struct ImpactCrossGlint {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) peak_scale: f32,
    pub(crate) spin_rate: f32,
    pub(crate) base_color: LinearRgba,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Tapered crescent weapon muzzle/slash arc smear (`AlphaMode::Add`).
#[derive(Component)]
pub(crate) struct WeaponArcSmear {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) start_scale: f32,
    pub(crate) end_scale: f32,
    pub(crate) sweep_rate: f32,
    pub(crate) base_color: LinearRgba,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Faceted tread / braking skid / landing dust puff.
#[derive(Component)]
pub(crate) struct TreadDustPuff {
    pub(crate) velocity: Vec3,
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) start_scale: f32,
    pub(crate) end_scale: f32,
    pub(crate) peak_alpha: f32,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Builds a double-sided tapered crescent arc ribbon in the XZ plane (centered
/// around `-Z`, sweeping `arc_radians`), razor-thin at both tips and thickest
/// at mid-arc so muzzle/slash sweeps read crisply from RTS camera angles.
pub(crate) fn build_tapered_arc_smear_mesh(
    inner_radius: f32,
    outer_radius: f32,
    arc_radians: f32,
    segments: usize,
) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};

    let segs = segments.max(4);
    let inner_r = inner_radius.max(0.05);
    let outer_r = outer_radius.max(inner_r + 0.05);
    let half_arc = arc_radians.clamp(0.2, std::f32::consts::TAU * 0.9) * 0.5;
    let mid_r = (inner_r + outer_r) * 0.5;
    let half_band = (outer_r - inner_r) * 0.5;

    let mut positions = Vec::with_capacity((segs + 1) * 2);
    let mut normals = Vec::with_capacity((segs + 1) * 2);
    let mut uvs = Vec::with_capacity((segs + 1) * 2);
    let mut indices = Vec::with_capacity(segs * 6);

    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let angle = -half_arc + t * (2.0 * half_arc);
        // Taper smoothly to razor tips at t = 0 and t = 1.
        let taper = (t * std::f32::consts::PI).sin().powf(0.85);
        let r_in = mid_r - half_band * taper;
        let r_out = mid_r + half_band * taper;
        // Centered around -Z (forward).
        let dir = Vec3::new(angle.sin(), 0.0, -angle.cos());
        positions.push((dir * r_in).to_array());
        positions.push((dir * r_out).to_array());
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([t, 0.0]);
        uvs.push([t, 1.0]);
    }

    for i in 0..segs {
        let base = (i * 2) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Builds a crisp 4-point star cross-glint (two orthogonal diamond blades in XZ +
/// one vertical accent blade) for heavy/crit/finisher impacts.
pub(crate) fn build_cross_glint_mesh(radius: f32, thickness: f32) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};

    let r = radius.max(0.1);
    let w = thickness.clamp(0.01, r * 0.4);
    let mut positions = Vec::with_capacity(18);
    let mut normals = Vec::with_capacity(18);
    let mut uvs = Vec::with_capacity(18);
    let mut indices = Vec::with_capacity(24);

    let mut push_diamond = |a: Vec3, b: Vec3, c: Vec3, d: Vec3, n: Vec3| {
        let base = positions.len() as u32;
        for (p, uv) in [
            (a, [0.5, 0.0]),
            (b, [1.0, 0.5]),
            (c, [0.5, 1.0]),
            (d, [0.0, 0.5]),
        ] {
            positions.push(p.to_array());
            normals.push(n.to_array());
            uvs.push(uv);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    // Blade along X in XZ plane
    push_diamond(
        Vec3::new(-r, 0.0, 0.0),
        Vec3::new(0.0, 0.0, w),
        Vec3::new(r, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -w),
        Vec3::Y,
    );
    // Blade along Z in XZ plane
    push_diamond(
        Vec3::new(0.0, 0.0, -r),
        Vec3::new(w, 0.0, 0.0),
        Vec3::new(0.0, 0.0, r),
        Vec3::new(-w, 0.0, 0.0),
        Vec3::Y,
    );
    // Vertical spike blade along Y
    push_diamond(
        Vec3::new(0.0, -r * 0.75, 0.0),
        Vec3::new(w, 0.0, 0.0),
        Vec3::new(0.0, r * 0.95, 0.0),
        Vec3::new(-w, 0.0, 0.0),
        Vec3::Z,
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Builds a lightweight faceted octahedron-like dust puff mesh for tread/skid clouds.
pub(crate) fn build_dust_puff_mesh(radius: f32) -> Mesh {
    let r = radius.max(0.05);
    Sphere::new(r)
        .mesh()
        .ico(1)
        .unwrap_or_else(|_| Sphere::new(r).mesh().build())
}

/// Hard caps on concurrent transient impact VFX so WebGPU / SwiftShader never
/// stalls on shader permutations or per-frame material churn during big fights.
const MAX_ACTIVE_RAZOR_SPARKS: usize = 48;
const MAX_ACTIVE_CONTACT_CORES: usize = 16;
const MAX_ACTIVE_CROSS_GLINTS: usize = 12;
const MAX_ACTIVE_ARC_SMEARS: usize = 16;
const MAX_ACTIVE_TREAD_DUST: usize = 24;

/// Spawns `count` directional additive razor hit-sparks fanning away from an
/// impact along `reflection_dir` (strictly `AlphaMode::Add`, `unlit: true`,
/// `NotShadowCaster`).
pub(crate) fn spawn_razor_hit_sparks(
    commands: &mut Commands,
    position: Vec3,
    reflection_dir: Vec3,
    count: u32,
    speed: f32,
    color: LinearRgba,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
        {
            return;
        }
        let active = world.query::<&ImpactRazorSpark>().iter(world).count();
        let allowed = MAX_ACTIVE_RAZOR_SPARKS.saturating_sub(active);
        if allowed == 0 {
            return;
        }
        let spawn_count = (count as usize).min(allowed) as u32;
        let mesh = match world
            .get_resource::<RazorSparkMesh>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(Cuboid::new(0.026, 0.026, 0.36));
                if let Some(mut cache) = world.get_resource_mut::<RazorSparkMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let seed = ((position.x.to_bits() as u64) << 32) ^ (position.z.to_bits() as u64);
        let bias = Vec3::new(reflection_dir.x, 0.0, reflection_dir.z).normalize_or_zero();
        for index in 0..spawn_count {
            let h = |salt: u32| vfx_hash01(seed, index.wrapping_mul(37).wrapping_add(salt));
            let angle = h(1) * std::f32::consts::TAU;
            let spread = Vec3::new(angle.cos(), 0.25 + h(2) * 0.9, angle.sin()).normalize();
            let dir = (spread + bias * 0.85).normalize_or(Vec3::Y);
            let pace = speed * (0.65 + h(3) * 0.85);
            let velocity = dir * pace;
            let ttl = 0.14 + h(4) * 0.14;
            let base_length = 0.75 + h(5) * 0.85;
            let spark_color = LinearRgba::new(
                (color.red * 1.5 + 0.35).min(3.5),
                (color.green * 1.4 + 0.25).min(3.5),
                (color.blue * 1.3 + 0.15).min(3.5),
                1.0,
            );
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: Color::srgba(
                        spark_color.red.min(1.0),
                        spark_color.green.min(1.0),
                        spark_color.blue.min(1.0),
                        0.95,
                    ),
                    emissive: spark_color,
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    ..default()
                });
            let rot = Quat::from_rotation_arc(Vec3::Z, dir);
            world.spawn((
                Name::new("Razor hit spark"),
                bevy::light::NotShadowCaster,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position + Vec3::Y * 0.32)
                    .with_rotation(rot)
                    .with_scale(Vec3::new(1.0, 1.0, base_length)),
                ImpactRazorSpark {
                    velocity,
                    age: 0.0,
                    ttl,
                    base_length,
                    base_color: spark_color,
                    material,
                },
                MatchScopedEntity,
            ));
        }
    });
}

pub(crate) fn update_razor_hit_sparks(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut sparks: Query<(Entity, &mut ImpactRazorSpark, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut spark, mut transform) in &mut sparks {
        spark.age += dt;
        if spark.age >= spark.ttl {
            materials.remove(&spark.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let life = (1.0 - spark.age / spark.ttl).clamp(0.0, 1.0);
        spark.velocity.y -= 12.5 * dt;
        spark.velocity *= (1.0 - 2.2 * dt).max(0.5);
        transform.translation += spark.velocity * dt;
        if transform.translation.y < 0.04 {
            transform.translation.y = 0.04;
            spark.velocity.y = spark.velocity.y.abs() * 0.35;
        }
        let speed = spark.velocity.length();
        if speed > 0.05 {
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, spark.velocity / speed);
        }
        let stretch = (spark.base_length * (0.35 + life * 0.95)).max(0.15);
        transform.scale = Vec3::new(0.45 + 0.55 * life, 0.45 + 0.55 * life, stretch);
        if let Some(mut mat) = materials.get_mut(&spark.material) {
            let fade = life * life;
            mat.base_color.set_alpha(0.95 * fade);
            mat.emissive = LinearRgba::new(
                spark.base_color.red * fade,
                spark.base_color.green * fade,
                spark.base_color.blue * fade,
                1.0,
            );
        }
    }
}

/// Spawns a blazing **85ms** additive white-hot contact hit-flash core at the
/// impact point (`AlphaMode::Add`, `unlit: true`, `NotShadowCaster`).
pub(crate) fn spawn_contact_hit_core(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    tint: LinearRgba,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
            || world.query::<&ContactHitCore>().iter(world).count() >= MAX_ACTIVE_CONTACT_CORES
        {
            return;
        }
        let mesh = match world
            .get_resource::<CombatFlashMesh>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world.resource_mut::<Assets<Mesh>>().add(
                    Sphere::new(1.0)
                        .mesh()
                        .ico(2)
                        .unwrap_or_else(|_| Sphere::new(1.0).mesh().build()),
                );
                if let Some(mut cache) = world.get_resource_mut::<CombatFlashMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let core_color = LinearRgba::new(
            (1.9 + tint.red * 0.5).min(3.2),
            (1.8 + tint.green * 0.5).min(3.2),
            (1.65 + tint.blue * 0.5).min(3.2),
            1.0,
        );
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.98, 0.94, 0.98),
                emissive: core_color,
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            });
        let peak_scale = radius.clamp(0.18, 1.35);
        world.spawn((
            Name::new("Contact hit core"),
            bevy::light::NotShadowCaster,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position + Vec3::Y * 0.34)
                .with_scale(Vec3::splat(peak_scale * 0.45)),
            ContactHitCore {
                age: 0.0,
                ttl: CONTACT_HIT_CORE_SECONDS,
                start_scale: peak_scale * 0.45,
                peak_scale,
                base_color: core_color,
                material,
            },
            MatchScopedEntity,
        ));
    });
}

pub(crate) fn update_contact_hit_cores(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut cores: Query<(Entity, &mut ContactHitCore, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut core, mut transform) in &mut cores {
        core.age += dt;
        if core.age >= core.ttl {
            materials.remove(&core.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let p = (core.age / core.ttl).clamp(0.0, 1.0);
        // Fast pop in the first 35% of 85ms, then crisp shrink-burnout.
        let scale = if p < 0.35 {
            let t = p / 0.35;
            core.start_scale + (core.peak_scale - core.start_scale) * (1.0 - (1.0 - t).powi(2))
        } else {
            let t = (p - 0.35) / 0.65;
            core.peak_scale * (1.0 - 0.65 * t)
        };
        transform.scale = Vec3::splat(scale.max(0.02));
        if let Some(mut mat) = materials.get_mut(&core.material) {
            let life = (1.0 - p) * (1.0 - p);
            mat.base_color.set_alpha(0.98 * life);
            mat.emissive = LinearRgba::new(
                core.base_color.red * life,
                core.base_color.green * life,
                core.base_color.blue * life,
                1.0,
            );
        }
    }
}

/// Spawns a 4-point crit/heavy additive cross-glint (`AlphaMode::Add`,
/// `unlit: true`, `cull_mode: None`, `NotShadowCaster`).
pub(crate) fn spawn_impact_cross_glint(
    commands: &mut Commands,
    position: Vec3,
    scale: f32,
    color: LinearRgba,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
            || world.query::<&ImpactCrossGlint>().iter(world).count() >= MAX_ACTIVE_CROSS_GLINTS
        {
            return;
        }
        let mesh = match world
            .get_resource::<CrossGlintMesh>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(build_cross_glint_mesh(1.0, 0.14));
                if let Some(mut cache) = world.get_resource_mut::<CrossGlintMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let hot = LinearRgba::new(
            (color.red * 1.8 + 0.4).min(3.6),
            (color.green * 1.7 + 0.35).min(3.6),
            (color.blue * 1.6 + 0.3).min(3.6),
            1.0,
        );
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(
                    hot.red.min(1.0),
                    hot.green.min(1.0),
                    hot.blue.min(1.0),
                    0.95,
                ),
                emissive: hot,
                alpha_mode: AlphaMode::Add,
                unlit: true,
                cull_mode: None,
                ..default()
            });
        let seed = ((position.x.to_bits() as u64) << 32) ^ (position.z.to_bits() as u64);
        let yaw = vfx_hash01(seed, 59) * std::f32::consts::FRAC_PI_2;
        world.spawn((
            Name::new("Impact cross glint"),
            bevy::light::NotShadowCaster,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position + Vec3::Y * 0.42)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::splat(scale * 0.3)),
            ImpactCrossGlint {
                age: 0.0,
                ttl: 0.16,
                peak_scale: scale.clamp(0.4, 2.6),
                spin_rate: 2.8,
                base_color: hot,
                material,
            },
            MatchScopedEntity,
        ));
    });
}

pub(crate) fn update_impact_cross_glints(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut glints: Query<(Entity, &mut ImpactCrossGlint, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut glint, mut transform) in &mut glints {
        glint.age += dt;
        if glint.age >= glint.ttl {
            materials.remove(&glint.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let p = (glint.age / glint.ttl).clamp(0.0, 1.0);
        let snap = if p < 0.25 {
            p / 0.25
        } else {
            1.0 - ((p - 0.25) / 0.75).powi(2)
        };
        transform.scale = Vec3::splat((glint.peak_scale * snap).max(0.03));
        transform.rotate_y(glint.spin_rate * dt);
        if let Some(mut mat) = materials.get_mut(&glint.material) {
            let fade = (1.0 - p) * (1.0 - p);
            mat.base_color.set_alpha(0.95 * fade);
            mat.emissive = LinearRgba::new(
                glint.base_color.red * fade,
                glint.base_color.green * fade,
                glint.base_color.blue * fade,
                1.0,
            );
        }
    }
}

/// Spawns a tapered additive weapon muzzle / slash arc smear oriented from
/// `from` toward `to` (`AlphaMode::Add`, `unlit: true`, `NotShadowCaster`, `NotShadowReceiver`).
pub(crate) fn spawn_weapon_arc_smear(
    commands: &mut Commands,
    from: Vec3,
    to: Vec3,
    kind: ImpactBurstKind,
) {
    let aim = Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalize_or(Vec3::NEG_Z);
    let yaw = f32::atan2(-aim.x, -aim.z);
    let (start_scale, end_scale, ttl, sweep_rate, color) = match kind {
        ImpactBurstKind::Siege => (0.55, 1.45, 0.16, 2.2, LinearRgba::new(1.6, 1.05, 0.42, 1.0)),
        ImpactBurstKind::Heavy => (0.42, 1.05, 0.13, 2.6, LinearRgba::new(1.5, 1.15, 0.58, 1.0)),
        ImpactBurstKind::Explosive => {
            (0.38, 0.92, 0.14, 2.0, LinearRgba::new(1.4, 0.72, 0.24, 1.0))
        }
        ImpactBurstKind::Energy => (0.36, 0.88, 0.13, 3.8, LinearRgba::new(0.45, 1.45, 1.8, 1.0)),
        ImpactBurstKind::Electric => (0.4, 0.96, 0.12, 4.4, LinearRgba::new(0.65, 1.1, 2.0, 1.0)),
        ImpactBurstKind::Fire => (
            0.48,
            1.25,
            0.19,
            3.2,
            LinearRgba::new(1.65, 0.55, 0.12, 1.0),
        ),
        ImpactBurstKind::Ballistic => {
            (0.24, 0.56, 0.09, 3.4, LinearRgba::new(1.3, 1.12, 0.65, 1.0))
        }
    };
    let origin = from + aim * 0.36 + Vec3::Y * 0.56;
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
            || world.query::<&WeaponArcSmear>().iter(world).count() >= MAX_ACTIVE_ARC_SMEARS
        {
            return;
        }
        let mesh = match world
            .get_resource::<WeaponArcSmearMesh>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(build_tapered_arc_smear_mesh(0.28, 0.68, 1.75, 14));
                if let Some(mut cache) = world.get_resource_mut::<WeaponArcSmearMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(
                    color.red.min(1.0),
                    color.green.min(1.0),
                    color.blue.min(1.0),
                    0.88,
                ),
                emissive: color,
                alpha_mode: AlphaMode::Add,
                unlit: true,
                cull_mode: None,
                ..default()
            });
        world.spawn((
            Name::new("Weapon arc smear"),
            bevy::light::NotShadowCaster,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(origin)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::splat(start_scale)),
            WeaponArcSmear {
                age: 0.0,
                ttl,
                start_scale,
                end_scale,
                sweep_rate,
                base_color: color,
                material,
            },
            MatchScopedEntity,
        ));
    });
}

pub(crate) fn update_weapon_arc_smears(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut smears: Query<(Entity, &mut WeaponArcSmear, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut smear, mut transform) in &mut smears {
        smear.age += dt;
        if smear.age >= smear.ttl {
            materials.remove(&smear.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let p = (smear.age / smear.ttl).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - p).powi(3);
        let s = smear.start_scale + (smear.end_scale - smear.start_scale) * ease;
        transform.scale = Vec3::new(s, 1.0, s);
        transform.rotate_y(smear.sweep_rate * dt);
        if let Some(mut mat) = materials.get_mut(&smear.material) {
            let fade = (1.0 - p) * (1.0 - p);
            mat.base_color.set_alpha(0.88 * fade);
            mat.emissive = LinearRgba::new(
                smear.base_color.red * fade,
                smear.base_color.green * fade,
                smear.base_color.blue * fade,
                1.0,
            );
        }
    }
}

/// Spawns a pair of faceted tread / skid / landing dust puffs kicking outward
/// using the shared unlit `AlphaMode::Add` pipeline (no extra blend/shadow variants).
pub(crate) fn spawn_tread_dust_puff(
    commands: &mut Commands,
    position: Vec3,
    forward: Vec3,
    scale: f32,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
            || world.query::<&TreadDustPuff>().iter(world).count() >= MAX_ACTIVE_TREAD_DUST
        {
            return;
        }
        let mesh = match world
            .get_resource::<TreadDustPuffMesh>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world
                    .resource_mut::<Assets<Mesh>>()
                    .add(build_dust_puff_mesh(1.0));
                if let Some(mut cache) = world.get_resource_mut::<TreadDustPuffMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        let fwd = Vec3::new(forward.x, 0.0, forward.z).normalize_or(Vec3::NEG_Z);
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        for sign in [-1.0_f32, 1.0] {
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: Color::srgba(0.62, 0.52, 0.38, 0.24),
                    emissive: LinearRgba::new(0.22, 0.18, 0.12, 1.0),
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    ..default()
                });
            let offset = side * (0.24 * scale * sign) - fwd * (0.18 * scale) + Vec3::Y * 0.1;
            world.spawn((
                Name::new("Tread dust puff"),
                bevy::light::NotShadowCaster,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position + offset).with_scale(Vec3::splat(0.1 * scale)),
                TreadDustPuff {
                    velocity: side * (0.55 * sign) - fwd * 0.4 + Vec3::Y * 0.45,
                    age: 0.0,
                    ttl: 0.55,
                    start_scale: 0.1 * scale,
                    end_scale: 0.38 * scale,
                    peak_alpha: 0.24,
                    material,
                },
                MatchScopedEntity,
            ));
        }
    });
}

pub(crate) fn update_tread_dust_puffs(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puffs: Query<(Entity, &mut TreadDustPuff, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut puff, mut transform) in &mut puffs {
        puff.age += dt;
        if puff.age >= puff.ttl {
            materials.remove(&puff.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let p = (puff.age / puff.ttl).clamp(0.0, 1.0);
        let swell = 1.0 - (1.0 - p) * (1.0 - p);
        let s = puff.start_scale + (puff.end_scale - puff.start_scale) * swell;
        transform.scale = Vec3::splat(s);
        transform.translation += puff.velocity * (1.0 - 0.65 * p) * dt;
        if let Some(mut mat) = materials.get_mut(&puff.material) {
            let fade = (1.0 - p).powf(1.4);
            mat.base_color.set_alpha(puff.peak_alpha * fade);
            mat.emissive = LinearRgba::new(0.22 * fade, 0.18 * fade, 0.12 * fade, 1.0);
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
// Smoke
// ---------------------------------------------------------------------------

/// A dark puff that rises, swells and thins out after something burns.
#[derive(Component)]
pub(crate) struct BlastSmoke {
    remaining: f32,
    total: f32,
    delay: f32,
    start_scale: f32,
    end_scale: f32,
    rise: f32,
    drift: Vec3,
    peak_alpha: f32,
    material: Handle<StandardMaterial>,
}

/// A rolling fireball: a cluster of overlapping additive lobes that swell and
/// burn out at different rates, so a blast reads as boiling fire instead of
/// one flat disc. Offsets come from a position hash, never an RNG.
pub(crate) fn spawn_fireball_cluster(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    lobes: u32,
) {
    for index in 0..lobes {
        let seed = position.x * 7.411 + position.z * 3.173 + index as f32 * 19.37;
        let hash = |offset: f32| ((seed + offset).sin() * 43_758.547).fract().abs();
        let angle = hash(1.0) * std::f32::consts::TAU;
        let reach = radius * (0.25 + hash(2.0) * 0.5);
        let offset = Vec3::new(
            angle.cos() * reach,
            radius * hash(3.0) * 0.45,
            angle.sin() * reach,
        );
        let size = radius * (0.32 + hash(4.0) * 0.3);
        // Inner lobes are hotter and die first; outer ones linger darker.
        let heat = 1.0 - reach / radius;
        let color = LinearRgba::new(1.5, 0.55 + heat * 0.8, 0.12 + heat * 0.5, 1.0);
        spawn_combat_flash(
            commands,
            position + offset,
            size * 0.35,
            size,
            0.32 + hash(5.0) * 0.3 + (1.0 - heat) * 0.15,
            color,
        );
    }
}

/// A loose column of `count` smoke puffs over `position`. Placement comes
/// from a position hash, never an RNG, so it is identical on every peer.
pub(crate) fn spawn_smoke_plume(commands: &mut Commands, position: Vec3, count: u32, scale: f32) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Assets<Mesh>>()
            || !world.contains_resource::<Assets<StandardMaterial>>()
        {
            return;
        }
        // Shares the unit sphere the combat flashes already cache.
        let mesh = match world
            .get_resource::<CombatFlashMesh>()
            .and_then(|mesh| mesh.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle = world.resource_mut::<Assets<Mesh>>().add(
                    Sphere::new(1.0)
                        .mesh()
                        .ico(2)
                        .unwrap_or_else(|_| Sphere::new(1.0).mesh().build()),
                );
                if let Some(mut cache) = world.get_resource_mut::<CombatFlashMesh>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };
        for index in 0..count {
            let seed = position.x * 12.9898 + position.z * 78.233 + index as f32 * 37.719;
            let hash = |offset: f32| ((seed + offset).sin() * 43_758.547).fract().abs();
            let offset = Vec3::new(hash(1.0) - 0.5, 0.0, hash(2.0) - 0.5) * scale * 1.1;
            let shade = 0.2 + hash(3.0) * 0.12;
            let peak_alpha = 0.26 + hash(4.0) * 0.12;
            let lifetime = 1.3 + hash(5.0) * 0.9;
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: Color::srgba(shade, shade * 0.95, shade * 0.9, 0.0),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                });
            world.spawn((
                Name::new("Smoke puff"),
                bevy::light::NotShadowCaster,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position + offset + Vec3::Y * (0.25 * scale))
                    .with_scale(Vec3::ZERO),
                BlastSmoke {
                    remaining: lifetime,
                    total: lifetime,
                    delay: 0.05 + index as f32 * 0.07,
                    start_scale: scale * (0.3 + hash(6.0) * 0.15),
                    end_scale: scale * (0.7 + hash(7.0) * 0.35),
                    rise: 0.7 + hash(8.0) * 0.7,
                    drift: Vec3::new(0.35, 0.0, 0.2) * (0.5 + hash(9.0)),
                    peak_alpha,
                    material,
                },
                MatchScopedEntity,
            ));
        }
    });
}

pub(crate) fn update_blast_smoke(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puffs: Query<(Entity, &mut Transform, &mut BlastSmoke)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut transform, mut puff) in &mut puffs {
        if puff.delay > 0.0 {
            puff.delay -= dt;
            continue;
        }
        puff.remaining -= dt;
        if puff.remaining <= 0.0 {
            materials.remove(&puff.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let age = 1.0 - (puff.remaining / puff.total).clamp(0.0, 1.0);
        // Swell fast, then keep creeping; fade in quickly and out slowly.
        let swell = 1.0 - (1.0 - age) * (1.0 - age);
        transform.scale =
            Vec3::splat(puff.start_scale + (puff.end_scale - puff.start_scale) * swell);
        transform.translation += (Vec3::Y * puff.rise * (1.0 - age * 0.6) + puff.drift) * dt;
        let alpha = puff.peak_alpha * (age * 8.0).min(1.0) * (1.0 - age).powf(1.5);
        if let Some(mut material) = materials.get_mut(&puff.material) {
            material.base_color.set_alpha(alpha);
        }
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

/// Bundled presentation-only outputs for [`emit_combat_feel`] (keeps the system
/// comfortably below Bevy's 16-parameter limit).
#[derive(SystemParam)]
pub(crate) struct CombatFeelOutputs<'w> {
    shake: ResMut<'w, CameraShake>,
    mixer: ResMut<'w, CombatAudioMixer>,
    post_process: ResMut<'w, PostProcessImpactState>,
    streak: ResMut<'w, CombatStreakState>,
}

fn is_allied_to_viewer(
    team: Team,
    visible_player: Option<&VisiblePlayer>,
    relations: Option<&TeamRelations>,
) -> bool {
    let viewer = visible_player.map_or(Team::Player(0), |vp| vp.team);
    relations.map_or(team == viewer, |rel| rel.are_allied(viewer, team))
}

fn is_hostile_to_viewer(
    team: Team,
    visible_player: Option<&VisiblePlayer>,
    relations: Option<&TeamRelations>,
) -> bool {
    let viewer = visible_player.map_or(Team::Player(0), |vp| vp.team);
    relations.map_or(team != viewer && team != Team::Neutral, |rel| {
        rel.are_enemies(viewer, team)
    })
}

fn is_local_player_team(team: Team, visible_player: Option<&VisiblePlayer>) -> bool {
    match visible_player {
        Some(vp) if vp.is_spectator() => false,
        Some(vp) => vp.team == team,
        None => team == Team::Player(0),
    }
}

/// Reacts to the effect entities the simulation spawned this frame: big ones
/// get a shockwave, secondary pops, camera shake, chromatic pulse and a layered
/// blast; every visible hit gets its weapon-class sound with combo pitch scaling.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_combat_feel(
    mut commands: Commands,
    camera: Res<RtsCamera>,
    mut outputs: CombatFeelOutputs,
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

    // Muzzle flash + tapered weapon arc smear for every visible shot.
    // Derived from the tracer itself, so online clients get it identically.
    for (pulse, weapon) in &new_shots {
        let (from, to) = (pulse.from - Vec3::Y * 0.6, pulse.to - Vec3::Y * 0.6);
        if revealed(from) {
            spawn_muzzle_flash(&mut commands, from, to, weapon.kind);
            spawn_weapon_arc_smear(&mut commands, from, to, weapon.kind);
        }
    }

    // Unit deaths: stage replica wreck/fireball if replicated, AND emit the
    // visceral unit-kill finisher shock-ring + cross-glint + razor sparks +
    // subtle camera zoom punch when visible.
    for (transform, death) in &new_deaths {
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        if is_hostile_to_viewer(death.team, visible_player.as_deref(), relations.as_deref()) {
            outputs.streak.register_hit();
        } else if is_local_player_team(death.team, visible_player.as_deref()) {
            outputs.post_process.spike_damage_vignette(0.42);
        }
        if death.replica
            && let Some(asset_server) = asset_server.as_deref()
        {
            spawn_death_presentation(&mut commands, asset_server, position, death.radius, false);
        }
        let r = death.radius.clamp(0.45, 2.2);
        spawn_additive_shock_ring(
            &mut commands,
            position,
            r * 0.3,
            r * 2.15,
            0.34,
            LinearRgba::new(1.55, 0.85, 0.28, 1.0),
        );
        spawn_contact_hit_core(
            &mut commands,
            position,
            r * 0.65,
            LinearRgba::new(1.6, 1.15, 0.5, 1.0),
        );
        spawn_impact_cross_glint(
            &mut commands,
            position,
            r * 1.35,
            LinearRgba::new(1.65, 1.05, 0.38, 1.0),
        );
        spawn_razor_hit_sparks(
            &mut commands,
            position,
            Vec3::Y,
            8,
            5.2,
            LinearRgba::new(1.55, 0.88, 0.3, 1.0),
        );
        outputs
            .shake
            .kick_zoom_punch(0.24 * shake_falloff(position));
    }

    for (transform, burst) in &new_bursts {
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        let allied_hit =
            is_allied_to_viewer(burst.team, visible_player.as_deref(), relations.as_deref());
        let streak_pitch = if allied_hit {
            let count = outputs.streak.register_hit();
            streak_pitch_multiplier(count)
        } else {
            1.0
        };
        let falloff = shake_falloff(position);
        let shake_strength = impact_shake_strength(burst) * falloff;
        outputs.shake.kick(shake_strength);
        outputs
            .shake
            .kick_zoom_punch(impact_zoom_punch_strength(burst) * falloff);
        outputs.post_process.spike_chroma(shake_strength);
        let loudness = audio_falloff(&camera, position);
        let jitter = pitch_jitter(position);
        let flash_tint = impact_flash_color(burst.kind);
        if is_strike_burst(burst) {
            spawn_strike_explosion(&mut commands, position, burst.radius);
            spawn_contact_hit_core(&mut commands, position, burst.radius * 0.48, flash_tint);
            spawn_impact_cross_glint(&mut commands, position, burst.radius * 0.85, flash_tint);
            spawn_razor_hit_sparks(&mut commands, position, Vec3::Y, 14, 7.2, flash_tint);
            outputs.mixer.request(
                CombatSample::Blast,
                (0.5 * jitter * streak_pitch).clamp(0.4, 2.6),
                loudness,
            );
            outputs.mixer.request(
                CombatSample::Blast,
                (0.8 * jitter * streak_pitch).clamp(0.4, 2.6),
                0.7 * loudness,
            );
            outputs.mixer.request(
                CombatSample::Hit,
                (0.45 * streak_pitch).clamp(0.4, 2.6),
                0.5 * loudness,
            );
            continue;
        }
        // Crisp 85ms contact hit-flash core + directional razor sparks on every visible hit.
        let core_radius = (0.18 + burst.power * 0.14).clamp(0.2, 0.72);
        spawn_contact_hit_core(&mut commands, position, core_radius, flash_tint);
        let spark_count = match burst.kind {
            ImpactBurstKind::Siege => 10,
            ImpactBurstKind::Heavy | ImpactBurstKind::Explosive | ImpactBurstKind::Electric => 7,
            ImpactBurstKind::Energy | ImpactBurstKind::Fire => 5,
            ImpactBurstKind::Ballistic => 3,
        };
        let spark_speed = 3.4 + burst.power * 1.4;
        spawn_razor_hit_sparks(
            &mut commands,
            position,
            Vec3::Y,
            spark_count,
            spark_speed,
            flash_tint,
        );
        // Crit / heavy / electric / siege cross-glint accent.
        if matches!(
            burst.kind,
            ImpactBurstKind::Siege | ImpactBurstKind::Heavy | ImpactBurstKind::Electric
        ) || burst.power >= 1.25
        {
            spawn_impact_cross_glint(
                &mut commands,
                position,
                (0.42 + burst.power * 0.32).clamp(0.45, 1.55),
                flash_tint,
            );
        }
        for (sample, speed, volume) in impact_sound_layers(burst.kind, burst.power)
            .into_iter()
            .flatten()
        {
            outputs.mixer.request(
                sample,
                (speed * jitter * streak_pitch).clamp(0.4, 2.6),
                volume * loudness,
            );
        }
        let ring = match burst.kind {
            ImpactBurstKind::Siege => Some((2.4, LinearRgba::new(1.3, 0.9, 0.5, 1.0))),
            ImpactBurstKind::Explosive if burst.power >= 1.0 => {
                Some((1.7, LinearRgba::new(1.2, 0.62, 0.22, 1.0)))
            }
            ImpactBurstKind::Heavy if burst.power >= 1.2 => {
                Some((1.5, LinearRgba::new(1.15, 0.85, 0.5, 1.0)))
            }
            ImpactBurstKind::Electric if burst.power >= 1.0 => {
                Some((1.65, LinearRgba::new(0.55, 0.95, 1.65, 1.0)))
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
        let streak_pitch =
            if is_hostile_to_viewer(effect.team, visible_player.as_deref(), relations.as_deref()) {
                let count = outputs.streak.register_hit();
                streak_pitch_multiplier(count)
            } else {
                if is_local_player_team(effect.team, visible_player.as_deref()) {
                    outputs.post_process.spike_damage_vignette(0.85);
                }
                1.0
            };
        let falloff = shake_falloff(position);
        let shake_strength = structure_destruction_shake_strength(effect.radius) * falloff;
        outputs.shake.kick(shake_strength);
        outputs.shake.kick_zoom_punch(0.68 * falloff);
        outputs.post_process.spike_chroma(shake_strength);
        let blast_color = LinearRgba::new(1.45, 0.8, 0.3, 1.0);
        spawn_shockwave_ring(
            &mut commands,
            position,
            effect.radius * 0.4,
            effect.radius * 2.4,
            0.5,
            blast_color,
        );
        spawn_contact_hit_core(&mut commands, position, effect.radius * 0.55, blast_color);
        spawn_impact_cross_glint(&mut commands, position, effect.radius * 1.1, blast_color);
        spawn_razor_hit_sparks(&mut commands, position, Vec3::Y, 12, 6.4, blast_color);
        spawn_secondary_explosions(&mut commands, position, effect.radius, 3);
        let loudness = audio_falloff(&camera, position);
        outputs.mixer.request(
            CombatSample::Blast,
            (0.58 * streak_pitch).clamp(0.4, 2.6),
            0.95 * loudness,
        );
        outputs.mixer.request(
            CombatSample::Hit,
            (0.5 * streak_pitch).clamp(0.4, 2.6),
            0.45 * loudness,
        );
    }

    for transform in &new_wrecks {
        let position = transform.translation;
        if !revealed(position) {
            continue;
        }
        outputs.mixer.request(
            CombatSample::Blast,
            0.95 * pitch_jitter(position),
            0.6 * audio_falloff(&camera, position),
        );
    }

    // Shake switched off in Options: discard this frame's kicks and chromatic pulse.
    if options.is_some_and(|options| !options.screen_shake) {
        outputs.shake.clear();
        outputs.post_process.chroma_pulse = 0.0;
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
    spawn_fireball_cluster(commands, position, (radius * 0.7).min(3.0), 9);
    spawn_smoke_plume(commands, position, 7, (radius * 0.45).clamp(0.8, 2.4));
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

/// Cached warm-white emissive opaque material shared across all 75ms unit mesh
/// hit-flashes. Reusing one `AlphaMode::Opaque` asset avoids per-hit material
/// churn and extra WebGPU pipeline permutations while keeping original imported
/// materials strictly immutable.
#[derive(Resource, Default)]
pub(crate) struct UnitHitFlashMaterial(pub(crate) Option<Handle<StandardMaterial>>);

/// Temporary 75ms emissive hit-flash applied to a struck unit's descendant mesh
/// materials (skipping health bars, decals, selection rings, and faction markers)
/// and restored cleanly to the exact original `Handle<StandardMaterial>`.
#[derive(Component)]
pub(crate) struct UnitMeshHitFlash {
    pub(crate) remaining: f32,
    pub(crate) original: Handle<StandardMaterial>,
}

fn is_hit_flash_decoration_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("health")
        || lower.contains("selection")
        || lower.contains("decal")
        || lower.contains("ring")
        || lower.contains("shadow")
        || lower.contains("aura")
        || lower.contains("marker")
}

/// Queues a 75ms opaque warm-white emissive material swap on a visible damaged
/// unit's body meshes, preserving the exact original handle across rapid re-hits.
pub(crate) fn queue_unit_mesh_hit_flash(commands: &mut Commands, target: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get_entity(target).is_err()
            || !world.contains_resource::<Assets<StandardMaterial>>()
        {
            return;
        }
        let flash_mat = match world
            .get_resource::<UnitHitFlashMaterial>()
            .and_then(|cache| cache.0.clone())
        {
            Some(handle) => handle,
            None => {
                let handle =
                    world
                        .resource_mut::<Assets<StandardMaterial>>()
                        .add(StandardMaterial {
                            base_color: Color::srgb(1.0, 0.97, 0.90),
                            emissive: LinearRgba::new(3.6, 3.25, 2.55, 1.0),
                            alpha_mode: AlphaMode::Opaque,
                            perceptual_roughness: 0.35,
                            ..default()
                        });
                if let Some(mut cache) = world.get_resource_mut::<UnitHitFlashMaterial>() {
                    cache.0 = Some(handle.clone());
                }
                handle
            }
        };

        // Walk up to 5 levels of descendants, skipping ground markers, health bars,
        // decals, selection rings, and non-opaque overlay/VFX meshes.
        let mut stack = vec![(target, 0_u8)];
        let mut mesh_entities = Vec::new();
        while let Some((node, depth)) = stack.pop() {
            if world.get::<ContextualFactionMarker>(node).is_some() {
                continue;
            }
            if world
                .get::<Name>(node)
                .is_some_and(|name| is_hit_flash_decoration_name(name.as_str()))
            {
                continue;
            }
            if let Some(mat) = world.get::<MeshMaterial3d<StandardMaterial>>(node) {
                let already_flashing = world.get::<UnitMeshHitFlash>(node).is_some();
                let eligible = already_flashing
                    || world
                        .resource::<Assets<StandardMaterial>>()
                        .get(&mat.0)
                        .is_none_or(|asset| {
                            matches!(asset.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))
                        });
                if eligible {
                    mesh_entities.push(node);
                }
            }
            if depth < 5
                && let Some(children) = world.get::<Children>(node)
            {
                for child in children.iter() {
                    stack.push((child, depth + 1));
                }
            }
        }

        for mesh_entity in mesh_entities {
            // Preserve the true original material if already mid-flash!
            let original = if let Some(existing) = world.get::<UnitMeshHitFlash>(mesh_entity) {
                existing.original.clone()
            } else if let Some(mat) = world.get::<MeshMaterial3d<StandardMaterial>>(mesh_entity) {
                mat.0.clone()
            } else {
                continue;
            };
            if let Ok(mut em) = world.get_entity_mut(mesh_entity) {
                em.insert((
                    MeshMaterial3d(flash_mat.clone()),
                    UnitMeshHitFlash {
                        remaining: UNIT_MESH_HIT_FLASH_SECONDS,
                        original,
                    },
                ));
            }
        }
    });
}

pub(crate) fn update_unit_mesh_hit_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(
        Entity,
        &mut UnitMeshHitFlash,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut flash, mut mat) in &mut flashes {
        flash.remaining -= dt;
        if flash.remaining <= 0.0 {
            mat.0 = flash.original.clone();
            commands.entity(entity).try_remove::<UnitMeshHitFlash>();
        }
    }
}

/// Washes a unit white for a few frames whenever it loses health, applies the
/// 75ms emissive mesh hit-flash on visible units, and triggers a crimson damage
/// vignette pulse when the local player's units or structures take heavy damage.
/// Derived from [`Health`] alone, so it works identically for direct hits,
/// splash, support powers, and online replicas.
type HitFlashUnit<'a> = (
    Entity,
    &'a Team,
    &'a Health,
    &'a Transform,
    &'a Selectable,
    Option<&'a VisibilityState>,
    Option<&'a mut HitFlashTracker>,
    Option<&'a Structure>,
);

type HitFlashTarget = Or<(With<Unit>, With<Structure>)>;

pub(crate) fn flash_damaged_units(
    mut commands: Commands,
    time: Res<Time>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut post_process: Option<ResMut<PostProcessImpactState>>,
    mut units: Query<HitFlashUnit, HitFlashTarget>,
) {
    let dt = time.delta_secs();
    for (entity, team, health, transform, selectable, visibility, tracker, structure) in &mut units
    {
        let Some(mut tracker) = tracker else {
            commands.entity(entity).try_insert(HitFlashTracker {
                last_health: health.current,
                cooldown: 0.0,
            });
            continue;
        };
        tracker.cooldown = (tracker.cooldown - dt).max(0.0);
        let previous = tracker.last_health;
        let damage_drop = (previous - health.current).max(0.0);
        let due = hit_flash_due(previous, health.current, tracker.cooldown);
        tracker.last_health = health.current;

        // Crimson vignette alert when the player's own structures or units take heavy damage.
        if damage_drop > 0.01
            && is_local_player_team(*team, visible_player.as_deref())
            && let Some(post_process) = post_process.as_deref_mut()
        {
            let drop_frac = if health.max > 0.0 {
                (damage_drop / health.max).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if let Some(structure) = structure {
                if structure.id == "CommandCenter" {
                    post_process.spike_damage_vignette((0.75 + drop_frac * 1.5).clamp(0.75, 1.0));
                } else if damage_drop >= 10.0 || drop_frac >= 0.05 {
                    post_process.spike_damage_vignette((0.5 + drop_frac * 1.5).clamp(0.5, 0.9));
                }
            } else if damage_drop >= 15.0 || drop_frac >= 0.20 {
                post_process.spike_damage_vignette((0.45 + drop_frac).clamp(0.45, 0.85));
            }
        }

        if !due || visibility.is_some_and(|visibility| !visibility.visible) {
            continue;
        }
        if structure.is_some() {
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
        queue_unit_mesh_hit_flash(&mut commands, entity);
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
    With<BlastSmoke>,
    With<ImpactRazorSpark>,
    With<ContactHitCore>,
    With<ImpactCrossGlint>,
    With<WeaponArcSmear>,
    With<TreadDustPuff>,
)>;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn clear_combat_feel_on_match_end(
    mut commands: Commands,
    match_state: Res<MatchState>,
    mut outputs: CombatFeelOutputs,
    transients: Query<Entity, InFlightFeelEffect>,
    mut mesh_flashes: Query<(
        Entity,
        &UnitMeshHitFlash,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut cameras: Query<(Option<&mut ChromaticAberration>, Option<&mut Vignette>), With<MainCamera>>,
    mut streak_hud: Query<(&mut Visibility, &mut UiTransform), With<CombatStreakHudRoot>>,
) {
    if match_state.is_running() {
        return;
    }
    for entity in &transients {
        commands.entity(entity).try_despawn();
    }
    for (entity, flash, mut mat) in &mut mesh_flashes {
        mat.0 = flash.original.clone();
        commands.entity(entity).try_remove::<UnitMeshHitFlash>();
    }
    outputs.shake.clear();
    outputs.mixer.clear();
    outputs.post_process.clear();
    outputs.streak.clear();

    let base_chroma = impact_chroma_intensity(0.0);
    let (base_vig_intensity, base_vig_color) = damage_vignette_params(0.0, 0.0, 0.0);
    for (chroma, vignette) in &mut cameras {
        if let Some(mut chroma) = chroma {
            chroma.intensity = base_chroma;
        }
        if let Some(mut vignette) = vignette {
            vignette.intensity = base_vig_intensity;
            vignette.color = base_vig_color;
        }
    }
    for (mut visibility, mut ui_tf) in &mut streak_hud {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        ui_tf.scale = Vec2::ONE;
    }
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

    #[test]
    fn camera_zoom_punch_snaps_in_and_returns_to_exact_zero() {
        let mut shake = CameraShake::default();
        assert_eq!(shake.zoom_punch_offset(), 0.0);
        shake.kick_zoom_punch(0.9);
        assert!(
            shake.zoom_punch_offset() > 0.1,
            "zoom punch must kick forward immediately"
        );
        assert!(shake.zoom_punch_offset() <= CAMERA_ZOOM_PUNCH_MAX + 1e-5);
        for _ in 0..90 {
            shake.tick(1.0 / 60.0);
            assert!(shake.zoom_punch_offset() <= CAMERA_ZOOM_PUNCH_MAX + 1e-5);
        }
        assert_eq!(
            shake.zoom_punch_offset(),
            0.0,
            "zoom punch must return cleanly with zero drift"
        );
    }

    #[test]
    fn tapered_arc_smear_and_cross_glint_and_dust_meshes_are_well_formed() {
        let smear = build_tapered_arc_smear_mesh(0.28, 0.68, 1.75, 12);
        let smear_pos = smear.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len();
        let smear_norm = smear.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().len();
        let smear_uv = smear.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().len();
        assert_eq!(smear_pos, (12 + 1) * 2);
        assert_eq!(smear_pos, smear_norm);
        assert_eq!(smear_pos, smear_uv);
        let smear_idx = smear.indices().expect("smear indices");
        assert_eq!(smear_idx.len(), 12 * 6);
        assert!(smear_idx.iter().all(|i| i < smear_pos));

        let glint = build_cross_glint_mesh(1.0, 0.14);
        let glint_pos = glint.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len();
        assert_eq!(glint_pos, 12);
        assert_eq!(glint.indices().expect("glint indices").len(), 18);

        let dust = build_dust_puff_mesh(0.5);
        assert!(dust.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len() >= 12);
        assert!(dust.indices().expect("dust indices").len() >= 36);
    }

    #[test]
    fn ballistic_damage_number_curves_overshoot_arc_and_snap_fade() {
        let start = damage_number_pop_scale(0.0);
        let peak = damage_number_pop_scale(0.14);
        let mid = damage_number_pop_scale(0.5);
        let end = damage_number_pop_scale(1.0);
        assert!(start < 0.7 && peak > 1.35 && (mid - 1.0).abs() < 1e-3 && end < mid);

        assert_eq!(damage_number_alpha(0.0), 1.0);
        assert_eq!(damage_number_alpha(0.65), 1.0);
        assert!(damage_number_alpha(0.85) < 0.8);
        assert_eq!(damage_number_alpha(1.0), 0.0);

        let vel = Vec3::new(0.8, 2.8, -0.5);
        let apex = damage_number_ballistic_offset(vel, 0.45);
        let late = damage_number_ballistic_offset(vel, 0.8);
        assert!(apex.y > 0.5, "number must pop upward");
        assert!(
            late.y < apex.y,
            "gravity must pull number down on ballistic arc"
        );
        assert!(late.x.abs() > 0.3, "number must fan out laterally");
    }

    #[test]
    fn streak_pitch_escalates_by_two_and_a_half_percent_and_caps_at_twenty_percent() {
        assert!((streak_pitch_multiplier(0) - 1.0).abs() < 1e-6);
        assert!((streak_pitch_multiplier(1) - 1.0).abs() < 1e-6);
        assert!((streak_pitch_multiplier(2) - 1.025).abs() < 1e-6);
        assert!((streak_pitch_multiplier(5) - 1.10).abs() < 1e-6);
        assert!((streak_pitch_multiplier(9) - 1.20).abs() < 1e-6);
        assert!((streak_pitch_multiplier(25) - 1.20).abs() < 1e-6);

        let mut streak = CombatStreakState::default();
        assert_eq!(streak.register_hit(), 1);
        assert_eq!(streak.register_hit(), 2);
        assert!((streak.streak_timer - COMBAT_STREAK_DECAY_SECONDS).abs() < 1e-6);
        assert!((streak.pop_secs - COMBAT_STREAK_POP_SECONDS).abs() < 1e-6);

        streak.tick(1.5);
        assert_eq!(streak.streak_hits, 2);
        assert_eq!(streak.pop_secs, 0.0);

        streak.tick(2.0);
        assert_eq!(streak.streak_hits, 0);
        assert_eq!(streak.streak_timer, 0.0);
    }

    #[test]
    fn post_process_chromatic_and_vignette_curves_pulse_and_decay_cleanly() {
        let mut state = PostProcessImpactState::default();
        // Tiny kicks below threshold do not pulse chromatic aberration.
        state.spike_chroma(0.05);
        assert_eq!(state.chroma_pulse, 0.0);
        assert_eq!(
            impact_chroma_intensity(state.chroma_pulse),
            POST_PROCESS_BASE_CHROMA
        );

        // Heavy blast pulses chroma proportionally and stays within cap.
        state.spike_chroma(0.9);
        let peak = impact_chroma_intensity(state.chroma_pulse);
        assert!(peak > POST_PROCESS_BASE_CHROMA + 0.015);
        assert!(peak <= POST_PROCESS_MAX_CHROMA);

        state.spike_damage_vignette(0.8);
        let (base_vig, base_col) = damage_vignette_params(0.0, 0.0, 0.0);
        let (hurt_vig, hurt_col) = damage_vignette_params(state.damage_vignette, 0.0, 0.0);
        assert!(hurt_vig > base_vig + 0.2);
        assert!(hurt_col.to_srgba().red > base_col.to_srgba().red + 0.3);

        // HQ below 35% HP triggers gentle heartbeat pulse.
        assert_eq!(hq_low_health_ratio(None), 0.0);
        assert_eq!(hq_low_health_ratio(Some(0.35)), 0.0);
        assert_eq!(hq_low_health_ratio(Some(0.8)), 0.0);
        let low = hq_low_health_ratio(Some(0.175));
        assert!((low - 0.5).abs() < 1e-5);
        let (beat_a, _) = damage_vignette_params(0.0, low, 0.0);
        let (beat_b, _) = damage_vignette_params(0.0, low, 0.30);
        assert!(beat_a > base_vig);
        assert!(
            beat_b > beat_a,
            "low-HQ vignette must pulse like a heartbeat"
        );

        // Decays back to exact baseline.
        for _ in 0..30 {
            state.tick(0.05);
        }
        assert_eq!(state.chroma_pulse, 0.0);
        assert_eq!(state.damage_vignette, 0.0);
        assert_eq!(
            impact_chroma_intensity(state.chroma_pulse),
            POST_PROCESS_BASE_CHROMA
        );
    }

    #[test]
    fn combat_streak_hud_badge_hides_below_three_and_escalates_tiers_bilingually() {
        let hidden = combat_streak_badge_style(2, 0.1, Language::En);
        assert!(!hidden.visible);

        let amber_en = combat_streak_badge_style(3, COMBAT_STREAK_POP_SECONDS, Language::En);
        assert!(amber_en.visible);
        assert_eq!(amber_en.label, "STREAK x3");
        assert!(amber_en.scale > 1.2);

        let amber_zh = combat_streak_badge_style(5, 0.0, Language::Zh);
        assert!(amber_zh.visible);
        assert_eq!(amber_zh.label, "连击 x5");
        assert_eq!(amber_zh.scale, 1.0);

        let orange = combat_streak_badge_style(12, 0.0, Language::En);
        assert_eq!(orange.label, "STREAK x12");
        assert_ne!(orange.border_color, amber_en.border_color);

        let gold = combat_streak_badge_style(24, 0.0, Language::Zh);
        assert_eq!(gold.label, "连击 x24");
        assert_ne!(gold.border_color, orange.border_color);
        assert_ne!(gold.border_color, amber_en.border_color);
    }

    #[test]
    fn unit_mesh_hit_flash_swaps_cached_opaque_emissive_skips_decorations_and_restores() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<UnitHitFlashMaterial>()
            .add_systems(Update, update_unit_mesh_hit_flashes);

        let body_original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.4, 0.8),
                alpha_mode: AlphaMode::Opaque,
                ..default()
            });
        let marker_mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let ring_mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let glow_mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                alpha_mode: AlphaMode::Add,
                ..default()
            });

        let unit = app.world_mut().spawn(Transform::default()).id();
        let body_mesh = app
            .world_mut()
            .spawn((
                Name::new("Chassis"),
                MeshMaterial3d(body_original.clone()),
                ChildOf(unit),
            ))
            .id();
        let faction_marker = app
            .world_mut()
            .spawn((
                ContextualFactionMarker,
                MeshMaterial3d(marker_mat.clone()),
                ChildOf(unit),
            ))
            .id();
        let selection_ring = app
            .world_mut()
            .spawn((
                Name::new("Selection Ring"),
                MeshMaterial3d(ring_mat.clone()),
                ChildOf(unit),
            ))
            .id();
        let additive_vfx = app
            .world_mut()
            .spawn((
                Name::new("Thruster Glow"),
                MeshMaterial3d(glow_mat.clone()),
                ChildOf(unit),
            ))
            .id();

        {
            let mut cmds = app.world_mut().commands();
            queue_unit_mesh_hit_flash(&mut cmds, unit);
        }
        app.world_mut().flush();
        app.update();

        let flashed_handle = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(body_mesh)
            .unwrap()
            .0
            .clone();
        assert_ne!(flashed_handle, body_original);
        let flash_asset = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&flashed_handle)
            .expect("cached hit-flash material exists");
        assert_eq!(flash_asset.alpha_mode, AlphaMode::Opaque);
        assert!(flash_asset.emissive.red > 2.0);
        // Decorations and additive meshes untouched!
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(faction_marker)
                .unwrap()
                .0,
            marker_mat
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(selection_ring)
                .unwrap()
                .0,
            ring_mat
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(additive_vfx)
                .unwrap()
                .0,
            glow_mat
        );
        // Original imported material remains completely immutable!
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&body_original)
                .unwrap()
                .base_color,
            Color::srgb(0.2, 0.4, 0.8)
        );

        // Rapid re-hit mid-flash preserves true original handle and reuses cached flash material.
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(0.04));
        app.update();
        {
            let mut cmds = app.world_mut().commands();
            queue_unit_mesh_hit_flash(&mut cmds, unit);
        }
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .get::<UnitMeshHitFlash>(body_mesh)
                .unwrap()
                .original,
            body_original
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(body_mesh)
                .unwrap()
                .0,
            flashed_handle,
            "second hit must reuse the single cached hit-flash material handle"
        );

        // Still flashing after 40ms, then restores exact original handle after 75ms.
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(0.04));
        app.update();
        assert!(app.world().get::<UnitMeshHitFlash>(body_mesh).is_some());

        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(0.04));
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(body_mesh)
                .unwrap()
                .0,
            body_original
        );
        assert!(app.world().get::<UnitMeshHitFlash>(body_mesh).is_none());
    }
}
