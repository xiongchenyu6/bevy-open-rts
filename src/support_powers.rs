//! Support powers (F1–F9): kinds/cooldowns, activation + targeting, the HUD
//! panel with its buttons/tooltips, and the AI's power usage.

use bevy::prelude::*;

use crate::*;

pub(crate) const SUPPORT_POWER_BUTTON_SIZE_PX: f32 = 64.0;

pub(crate) const SUPPORT_POWER_BUTTON_GAP_PX: f32 = 5.0;

pub(crate) const SUPPORT_POWER_PANEL_PADDING_PX: f32 = 5.0;

pub(crate) const SUPPORT_POWER_PANEL_TOP_PX: f32 = 8.0;

pub(crate) const SUPPORT_POWER_PANEL_RIGHT_PX: f32 = 12.0;

pub(crate) const SUPPORT_POWER_PANEL_WIDTH_PX: f32 = SUPPORT_POWER_PANEL_PADDING_PX * 2.0
    + SUPPORT_POWER_BUTTON_SIZE_PX * 9.0
    + SUPPORT_POWER_BUTTON_GAP_PX * 8.0;

pub(crate) const SUPPORT_POWER_PANEL_HEIGHT_PX: f32 =
    SUPPORT_POWER_PANEL_PADDING_PX * 2.0 + SUPPORT_POWER_BUTTON_SIZE_PX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SupportPowerKind {
    RadarSweep,
    OrbitalStrike,
    EmpPulse,
    ChronoRelay,
    ShieldOverdrive,
    NaniteRepairSwarm,
    WeatherStorm,
    StrategicMissile,
    Paradrop,
}

impl SupportPowerKind {
    pub(crate) const ALL: [Self; 9] = [
        Self::RadarSweep,
        Self::OrbitalStrike,
        Self::EmpPulse,
        Self::ChronoRelay,
        Self::ShieldOverdrive,
        Self::NaniteRepairSwarm,
        Self::WeatherStorm,
        Self::StrategicMissile,
        Self::Paradrop,
    ];

    pub(crate) fn idx(self) -> usize {
        match self {
            Self::RadarSweep => 0,
            Self::OrbitalStrike => 1,
            Self::EmpPulse => 2,
            Self::ChronoRelay => 3,
            Self::ShieldOverdrive => 4,
            Self::NaniteRepairSwarm => 5,
            Self::WeatherStorm => 6,
            Self::StrategicMissile => 7,
            Self::Paradrop => 8,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::RadarSweep => t("雷达扫描", "Radar Sweep"),
            Self::OrbitalStrike => t("轨道打击", "Orbital Strike"),
            Self::EmpPulse => t("EMP脉冲", "EMP Pulse"),
            Self::ChronoRelay => t("时光回响", "Chrono Relay"),
            Self::ShieldOverdrive => t("护盾超载", "Shield Overdrive"),
            Self::NaniteRepairSwarm => t("纳米修复", "Nanite Repair Swarm"),
            Self::WeatherStorm => t("气象风暴", "Weather Storm"),
            Self::StrategicMissile => t("战略导弹", "Strategic Missile"),
            Self::Paradrop => t("空投", "Paradrop"),
        }
    }

    pub(crate) fn is_superweapon(self) -> bool {
        matches!(self, Self::WeatherStorm | Self::StrategicMissile)
    }

    pub(crate) fn hotkey(self) -> KeyCode {
        match self {
            Self::RadarSweep => KeyCode::F1,
            Self::OrbitalStrike => KeyCode::F2,
            Self::EmpPulse => KeyCode::F3,
            Self::ChronoRelay => KeyCode::F4,
            Self::ShieldOverdrive => KeyCode::F5,
            Self::NaniteRepairSwarm => KeyCode::F6,
            Self::WeatherStorm => KeyCode::F7,
            Self::StrategicMissile => KeyCode::F8,
            Self::Paradrop => KeyCode::F9,
        }
    }

    pub(crate) fn hotkey_label(self) -> &'static str {
        match self {
            Self::RadarSweep => "F1",
            Self::OrbitalStrike => "F2",
            Self::EmpPulse => "F3",
            Self::ChronoRelay => "F4",
            Self::ShieldOverdrive => "F5",
            Self::NaniteRepairSwarm => "F6",
            Self::WeatherStorm => "F7",
            Self::StrategicMissile => "F8",
            Self::Paradrop => "F9",
        }
    }

    pub(crate) fn icon_path(self) -> &'static str {
        match self {
            Self::RadarSweep => "ui/icons/RadarSweep.png",
            Self::OrbitalStrike => "ui/icons/OrbitalStrike.png",
            Self::EmpPulse => "ui/icons/EmpPulse.png",
            Self::ChronoRelay => "ui/icons/ChronoRelay.png",
            Self::ShieldOverdrive => "ui/icons/ShieldOverdrive.png",
            Self::NaniteRepairSwarm => "ui/icons/NaniteRepairSwarm.png",
            Self::WeatherStorm => "ui/icons/WeatherStorm.png",
            Self::StrategicMissile => "ui/icons/StrategicMissile.png",
            Self::Paradrop => "ui/icons/Paradrop.png",
        }
    }

    pub(crate) fn definition(self) -> SupportPowerDef {
        match self {
            Self::RadarSweep => SupportPowerDef {
                requirements: &["RadarUplink"],
                cooldown: 18.0,
                radius: 12.0,
                duration: 8.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::OrbitalStrike => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 45.0,
                radius: 3.4,
                duration: 0.0,
                impact_delay: 0.7,
                requires_power: true,
                damage: 8.0,
                damage_scale: 1.2,
                initial_cooldown: 0.0,
                healing: 0.0,
                unit_paths: &[],
            },
            Self::EmpPulse => SupportPowerDef {
                requirements: &["RoboticsBay"],
                cooldown: 36.0,
                radius: 4.8,
                duration: 5.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::ChronoRelay => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 38.0,
                radius: 4.6,
                duration: 7.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.75,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::ShieldOverdrive => SupportPowerDef {
                requirements: &["TechLab"],
                cooldown: 55.0,
                radius: 4.8,
                duration: 8.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 0.25,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::NaniteRepairSwarm => SupportPowerDef {
                requirements: &["RoboticsBay"],
                cooldown: 42.0,
                radius: 5.2,
                duration: 0.0,
                impact_delay: 0.0,
                requires_power: true,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 10.0,
                initial_cooldown: 0.0,
                unit_paths: &[],
            },
            Self::WeatherStorm => SupportPowerDef {
                requirements: &["WeatherControlSpire"],
                cooldown: 90.0,
                radius: 6.4,
                duration: 0.0,
                impact_delay: 1.8,
                requires_power: true,
                damage: 12.0,
                damage_scale: 1.6,
                healing: 0.0,
                initial_cooldown: 90.0,
                unit_paths: &[],
            },
            Self::StrategicMissile => SupportPowerDef {
                requirements: &["WeatherControlSpire"],
                cooldown: 105.0,
                radius: 4.8,
                duration: 0.0,
                impact_delay: 1.4,
                requires_power: true,
                damage: 20.0,
                damage_scale: 2.0,
                healing: 0.0,
                initial_cooldown: 105.0,
                unit_paths: &[],
            },
            Self::Paradrop => SupportPowerDef {
                requirements: &["TechAirport"],
                cooldown: 52.0,
                radius: 2.4,
                duration: 0.0,
                impact_delay: 1.1,
                requires_power: false,
                damage: 0.0,
                damage_scale: 1.0,
                healing: 0.0,
                initial_cooldown: 0.0,
                unit_paths: &["LightRifleInfantry", "LightRifleInfantry", "RocketInfantry"],
            },
        }
    }
}

#[derive(Component)]
pub(crate) struct SupportWarning {
    pub(crate) remaining: f32,
    pub(crate) radius: f32,
    pub(crate) color: Color,
}

#[derive(Component)]
pub(crate) struct TemporarySupportReveal {
    pub(crate) remaining: f32,
    pub(crate) radius: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct SupportPowerTargetSnapshot {
    pub(crate) entity: Entity,
    pub(crate) team: Team,
    pub(crate) position: Vec3,
    pub(crate) health: Health,
    pub(crate) mobile: bool,
}

#[derive(Resource)]
pub(crate) struct SupportCooldowns {
    pub(crate) remaining: Vec<[f32; SupportPowerKind::ALL.len()]>,
    pub(crate) initial_charge_started: Vec<[bool; SupportPowerKind::ALL.len()]>,
}

impl SupportCooldowns {
    pub(crate) fn ensure_team(&mut self, team: Team) -> Option<usize> {
        let index = team.economy_index()?;
        if self.remaining.len() <= index {
            self.remaining
                .resize(index + 1, [0.0; SupportPowerKind::ALL.len()]);
            self.initial_charge_started
                .resize(index + 1, [false; SupportPowerKind::ALL.len()]);
        }
        Some(index)
    }

    pub(crate) fn ready(&self, team: Team, power: SupportPowerKind) -> bool {
        self.remaining_for(team, power) <= 0.0
    }

    pub(crate) fn remaining_for(&self, team: Team, power: SupportPowerKind) -> f32 {
        team.economy_index()
            .and_then(|index| self.remaining.get(index))
            .map_or(0.0, |remaining| remaining[power.idx()])
    }

    pub(crate) fn set(&mut self, team: Team, power: SupportPowerKind, base: f32) {
        if let Some(index) = self.ensure_team(team) {
            self.remaining[index][power.idx()] = base;
        }
    }
}

impl Default for SupportCooldowns {
    fn default() -> Self {
        Self {
            remaining: Vec::new(),
            initial_charge_started: Vec::new(),
        }
    }
}

#[derive(Component)]
pub(crate) struct SupportPowersPanel;

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerButton {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerCooldownLabel {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SupportPowerHotkeyLabel {
    pub(crate) kind: SupportPowerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SupportPowerButtonSpec {
    pub(crate) kind: SupportPowerKind,
    pub(crate) icon_path: &'static str,
    pub(crate) hotkey_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SupportPowerButtonState {
    pub(crate) enabled: bool,
    pub(crate) unlocked: bool,
    pub(crate) active: bool,
    pub(crate) low_power: bool,
    pub(crate) cooldown_seconds: Option<u32>,
    pub(crate) badge_text: String,
}

pub(crate) fn support_power_button_specs() -> Vec<SupportPowerButtonSpec> {
    SupportPowerKind::ALL
        .into_iter()
        .map(|kind| SupportPowerButtonSpec {
            kind,
            icon_path: kind.icon_path(),
            hotkey_label: kind.hotkey_label(),
        })
        .collect()
}

pub(crate) const HUMAN_SUPPORT_RATE_MULTIPLIER: f32 = 1.15;

pub(crate) fn faction_support_rate_multiplier(faction: Option<SkirmishFaction>) -> f32 {
    match faction {
        Some(SkirmishFaction::Alliance) => HUMAN_SUPPORT_RATE_MULTIPLIER,
        Some(SkirmishFaction::Demon | SkirmishFaction::Chaos) | None => 1.0,
    }
}

pub(crate) fn setup_support_cooldowns(mut support_cooldowns: ResMut<SupportCooldowns>) {
    *support_cooldowns = SupportCooldowns::default();
}

pub(crate) fn update_support_cooldowns(
    time: Res<Time>,
    economies: Res<Economies>,
    active_teams: Option<Res<ActiveTeams>>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut support_cooldowns: ResMut<SupportCooldowns>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    structures: Query<StructurePrereqItem<'_>>,
) {
    let delta = time.delta_secs();
    let player_team = visible_player_team(visible_player.as_deref());
    let team_count = active_teams
        .as_deref()
        .map(|active| active.0.len())
        .unwrap_or(economies.players.len());
    for team in player_teams(team_count) {
        if !team_is_active(team, active_teams.as_deref()) {
            continue;
        }
        let Some(team_index) = support_cooldowns.ensure_team(team) else {
            continue;
        };
        for power in SupportPowerKind::ALL {
            let idx = power.idx();
            let def = power.definition();
            let requirements_met = support_requirements_met(team, def.requirements, &structures);
            if !requirements_met {
                support_cooldowns.initial_charge_started[team_index][idx] = false;
            } else if def.initial_cooldown > 0.0
                && !support_cooldowns.initial_charge_started[team_index][idx]
                && support_cooldowns.remaining[team_index][idx] <= 0.0
            {
                support_cooldowns.initial_charge_started[team_index][idx] = true;
                support_cooldowns.remaining[team_index][idx] = def.initial_cooldown;
                record_support_power_charging_feedback(
                    &mut audio_feedback,
                    &mut battle_log,
                    team,
                    player_team,
                    power,
                    def.initial_cooldown,
                );
                continue;
            }
            let before = support_cooldowns.remaining[team_index][idx];
            support_cooldowns.remaining[team_index][idx] = (before - delta).max(0.0);
            let became_ready = before > 0.0 && support_cooldowns.remaining[team_index][idx] == 0.0;
            if became_ready
                && support_power_available_for_audio(team, power, &economies, &structures)
            {
                record_support_power_ready_audio_feedback(
                    &mut audio_feedback,
                    team,
                    player_team,
                    power,
                );
                record_support_power_ready_battle_log(&mut battle_log, team, player_team, power);
            }
        }
    }
}

pub(crate) fn record_support_power_audio_feedback(
    feedback: &mut AudioFeedback,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerFire);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::SupportPowerFired);
    } else if power.is_superweapon() {
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySuperweaponLaunched);
    } else {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerFire);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySupportPowerFired);
    }
}

pub(crate) fn record_support_power_charging_feedback(
    feedback: &mut AudioFeedback,
    battle_log: &mut BattleLog,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
    charge_seconds: f32,
) {
    if !power.is_superweapon() {
        return;
    }
    let charge_seconds = charge_seconds.ceil() as i32;
    if team == player_team {
        push_battle_log(
            battle_log,
            format!(
                "{}: {} {charge_seconds}s",
                t("超级武器充能", "Superweapon charging"),
                power.label()
            ),
            None,
        );
    } else {
        push_battle_log(
            battle_log,
            format!(
                "{}: {} {charge_seconds}s",
                t("敌方超级武器充能", "Enemy superweapon charging"),
                power.label()
            ),
            None,
        );
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
    }
}

pub(crate) fn record_support_power_ready_battle_log(
    battle_log: &mut BattleLog,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        push_battle_log(
            battle_log,
            format!("{}: {}", t("支援就绪", "Support ready"), power.label()),
            None,
        );
    } else if power.is_superweapon() {
        push_battle_log(
            battle_log,
            format!(
                "{}: {}",
                t("敌方超级武器就绪", "Enemy superweapon ready"),
                power.label()
            ),
            None,
        );
    }
}

pub(crate) fn support_power_button(kind: SupportPowerKind) -> impl Bundle {
    (
        Button,
        SupportPowerButton { kind },
        Node {
            display: Display::None,
            position_type: PositionType::Relative,
            width: px(SUPPORT_POWER_BUTTON_SIZE_PX),
            height: px(SUPPORT_POWER_BUTTON_SIZE_PX),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(px(0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.32, 0.42, 0.46)),
        BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.9)),
        Visibility::Hidden,
    )
}

pub(crate) fn disarm_support_power_on_left_click(
    command_mode: &mut CommandMode,
    mouse: &ButtonInput<MouseButton>,
    cursor_over_hud: bool,
) -> bool {
    if mouse.just_pressed(MouseButton::Left)
        && command_mode.support_power.is_some()
        && !cursor_over_hud
    {
        command_mode.support_power = None;
        return true;
    }
    false
}

pub(crate) fn support_power_target_snapshots(
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Vec<SupportPowerTargetSnapshot> {
    selectable_q
        .iter()
        .filter_map(
            |(
                entity,
                transform,
                _selectable,
                target_team,
                _visibility,
                _resource_node,
                _supply_crate,
                health,
                unit,
                structure,
                _under_construction,
            )| {
                let health = health?;
                (unit.is_some() || structure.is_some()).then_some(SupportPowerTargetSnapshot {
                    entity,
                    team: *target_team,
                    position: transform.translation,
                    health: *health,
                    mobile: unit.is_some_and(|unit| unit.speed > 0.0),
                })
            },
        )
        .collect::<Vec<_>>()
}

pub(crate) fn activate_support_power(
    mut commands: &mut Commands,
    target: Vec3,
    power: SupportPowerKind,
    team: Team,
    player_team: Team,
    economies: &Economies,
    support_cooldowns: &mut SupportCooldowns,
    battle_log: &mut BattleLog,
    relations: &TeamRelations,
    structures: &Query<StructurePrereqItem<'_>>,
    targets: &[SupportPowerTargetSnapshot],
) -> bool {
    let def = power.definition();
    if !support_cooldowns.ready(team, power) {
        return false;
    }
    if def.requires_power && economies.get(team).low_power() {
        return false;
    }
    if !support_requirements_met(team, def.requirements, structures) {
        return false;
    }

    match power {
        SupportPowerKind::RadarSweep => {
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.01),
                TemporarySupportReveal {
                    remaining: def.duration,
                    radius: def.radius,
                },
                team,
                VisibilityState { visible: true },
                VisionRadius(def.radius),
                MatchScopedEntity,
            ));
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                SupportWarning {
                    remaining: def.duration,
                    radius: def.radius,
                    color: Color::srgba(0.32, 0.88, 0.42, 0.45),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::OrbitalStrike => {
            let delay = def.impact_delay;
            let warning_color = Color::srgba(1.0, 0.72, 0.22, 0.55);
            if delay <= 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: 0.0,
                        radius: def.radius,
                        damage: def.damage,
                        impact_scale: def.damage_scale,
                        team,
                    },
                    SupportWarning {
                        remaining: 0.15,
                        radius: def.radius * 0.55,
                        color: warning_color,
                    },
                    MatchScopedEntity,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: delay,
                        radius: def.radius,
                        damage: def.damage,
                        impact_scale: def.damage_scale,
                        team,
                    },
                    SupportWarning {
                        remaining: delay,
                        radius: def.radius,
                        color: warning_color,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::EmpPulse => {
            for target_snapshot in targets {
                if target_snapshot.health.current <= 0.0 {
                    continue;
                }
                if !target_snapshot.mobile {
                    continue;
                }
                if relations.are_enemies(team, target_snapshot.team)
                    && xz_distance(target_snapshot.position, target) <= def.radius
                {
                    queue_apply_emp_disabled(&mut commands, target_snapshot.entity, def.duration);
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                SupportWarning {
                    remaining: 0.75,
                    radius: def.radius,
                    color: Color::srgba(0.8, 0.45, 1.0, 0.55),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::ChronoRelay => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                {
                    continue;
                }
                if !target_snapshot.mobile {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    queue_apply_chrono_relay(
                        &mut commands,
                        target_snapshot.entity,
                        def.duration,
                        def.damage_scale,
                    );
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.45),
                SupportWarning {
                    remaining: 0.2,
                    radius: def.radius,
                    color: Color::srgba(0.36, 0.93, 0.98, 0.45),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::ShieldOverdrive => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    queue_apply_support_shield(
                        &mut commands,
                        target_snapshot.entity,
                        def.duration,
                        def.damage_scale,
                    );
                }
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.45),
                SupportWarning {
                    remaining: 0.2,
                    radius: def.radius,
                    color: Color::srgba(0.6, 0.85, 0.55, 0.42),
                },
                MatchScopedEntity,
            ));
        }
        SupportPowerKind::NaniteRepairSwarm => {
            for target_snapshot in targets {
                if !relations.are_allied(team, target_snapshot.team)
                    || target_snapshot.health.current <= 0.0
                    || target_snapshot.health.max <= 0.0
                {
                    continue;
                }
                if xz_distance(target_snapshot.position, target) <= def.radius {
                    let healed_health = (target_snapshot.health.current + def.healing)
                        .min(target_snapshot.health.max);
                    if healed_health <= target_snapshot.health.current {
                        continue;
                    }
                    commands.entity(target_snapshot.entity).try_insert(Health {
                        current: healed_health,
                        max: target_snapshot.health.max,
                    });
                    commands.spawn((
                        ShotPulse {
                            from: target_snapshot.position + Vec3::new(0.0, 0.45, 0.0),
                            to: target_snapshot.position + Vec3::new(0.0, 0.12, 0.0),
                            ttl: 0.14,
                            team,
                        },
                        MatchScopedEntity,
                    ));
                }
            }
        }
        SupportPowerKind::WeatherStorm => {
            let secondary_radius = def.radius * 0.55;
            let impact_points = [
                (Vec3::ZERO, def.radius),
                (Vec3::new(1.6, 0.0, -1.2), secondary_radius),
                (Vec3::new(-1.4, 0.0, 1.1), secondary_radius),
            ];
            if def.impact_delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: def.impact_delay,
                        radius: def.radius,
                        color: Color::srgba(0.4, 0.9, 1.0, 0.5),
                    },
                    MatchScopedEntity,
                ));
            }
            for (idx, (offset, radius)) in impact_points.into_iter().enumerate() {
                commands.spawn((
                    Transform::from_translation(target + offset + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: def.impact_delay,
                        radius,
                        damage: if idx == 0 { def.damage } else { 0.0 },
                        impact_scale: def.damage_scale,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::StrategicMissile => {
            let secondary_radius = def.radius * 0.45;
            let impact_points = [
                (Vec3::ZERO, def.radius),
                (Vec3::new(0.9, 0.0, 0.9), secondary_radius),
                (Vec3::new(-0.8, 0.0, -0.7), secondary_radius),
            ];
            if def.impact_delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: def.impact_delay,
                        radius: def.radius,
                        color: Color::srgba(1.0, 0.16, 0.08, 0.58),
                    },
                    MatchScopedEntity,
                ));
                commands.spawn((
                    ShotPulse {
                        from: target + Vec3::new(0.0, 8.5, 0.0),
                        to: target + Vec3::new(0.0, 0.4, 0.0),
                        ttl: def.impact_delay,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
            for (idx, (offset, radius)) in impact_points.into_iter().enumerate() {
                commands.spawn((
                    Transform::from_translation(target + offset + Vec3::Y * 0.03),
                    PendingOrbitalStrike {
                        remaining: def.impact_delay,
                        radius,
                        damage: if idx == 0 { def.damage } else { 0.0 },
                        impact_scale: def.damage_scale,
                        team,
                    },
                    MatchScopedEntity,
                ));
            }
        }
        SupportPowerKind::Paradrop => {
            let delay = def.impact_delay;
            if delay > 0.0 {
                commands.spawn((
                    Transform::from_translation(target + Vec3::Y * 0.04),
                    SupportWarning {
                        remaining: delay,
                        radius: def.radius,
                        color: Color::srgba(0.25, 0.8, 1.0, 0.48),
                    },
                    MatchScopedEntity,
                ));
            }
            commands.spawn((
                Transform::from_translation(target + Vec3::Y * 0.03),
                PendingParadrop {
                    remaining: delay,
                    team,
                    target,
                    unit_paths: def.unit_paths,
                },
                MatchScopedEntity,
            ));
        }
    }
    let (message, ping_kind) = if team == player_team {
        (
            format!("{}: {}", t("支援已使用", "Support used"), power.label()),
            BattleEventPingKind::SupportPower,
        )
    } else if matches!(
        power,
        SupportPowerKind::StrategicMissile | SupportPowerKind::WeatherStorm
    ) {
        (
            format!(
                "{}: {}",
                t("敌方超级武器", "Enemy superweapon"),
                power.label()
            ),
            BattleEventPingKind::EnemySuperweapon,
        )
    } else {
        (
            format!("{}: {}", t("敌方支援", "Enemy support"), power.label()),
            BattleEventPingKind::EnemySupportPower,
        )
    };
    push_battle_log_with_kind(battle_log, message, Some(target), ping_kind);
    support_cooldowns.set(team, power, def.cooldown);
    true
}

pub(crate) fn support_hotkey_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::AltLeft)
        || keyboard.pressed(KeyCode::AltRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight)
        || keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
}

pub(crate) fn player_support_power_available(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    support_cooldowns: &SupportCooldowns,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    support_cooldowns.ready(team, power)
        && (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

pub(crate) fn support_power_unlocked(
    team: Team,
    power: SupportPowerKind,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    support_requirements_met(team, power.definition().requirements, structures)
}

pub(crate) fn visible_support_power_count(
    team: Team,
    structures: &Query<StructurePrereqItem<'_>>,
) -> usize {
    SupportPowerKind::ALL
        .into_iter()
        .filter(|power| support_power_unlocked(team, *power, structures))
        .count()
}

pub(crate) fn support_power_button_state(
    power: SupportPowerKind,
    unlocked: bool,
    low_power: bool,
    cooldown_remaining: f32,
    active: bool,
) -> SupportPowerButtonState {
    let cooldown_seconds = (cooldown_remaining > 0.0).then_some(cooldown_remaining.ceil() as u32);
    let enabled = unlocked
        && (!power.definition().requires_power || !low_power)
        && cooldown_seconds.is_none();
    let badge_text = if unlocked {
        cooldown_seconds.map_or_else(String::new, |seconds| seconds.to_string())
    } else {
        String::new()
    };
    SupportPowerButtonState {
        enabled,
        unlocked,
        active: active && enabled,
        low_power,
        cooldown_seconds,
        badge_text,
    }
}

pub(crate) fn support_power_missing_requirement_labels(
    team: Team,
    requirements: &[&'static str],
    structures: &Query<StructurePrereqItem<'_>>,
) -> Vec<String> {
    requirements
        .iter()
        .filter(|requirement| !team_has_constructed_structure(team, requirement, structures))
        .map(|requirement| localized_compact_entity_label(requirement))
        .collect()
}

pub(crate) fn support_power_requirement_text(requirements: &[&'static str]) -> String {
    if requirements.is_empty() {
        return t("无", "None").to_string();
    }
    requirements
        .iter()
        .map(|requirement| localized_compact_entity_label(requirement))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn support_power_tooltip(
    power: SupportPowerKind,
    state: &SupportPowerButtonState,
    missing_requirements: &[String],
) -> String {
    let def = power.definition();
    let mut lines = vec![
        format!("{}  {}", power.hotkey_label(), power.label()),
        format!("{}: {:.0}s", t("冷却", "Cooldown"), def.cooldown),
        format!(
            "{}: {}",
            t("需求", "Requires"),
            support_power_requirement_text(def.requirements)
        ),
        format!("{}: {:.1}", t("半径", "Radius"), def.radius),
    ];
    if def.damage > 0.0 {
        lines.push(format!("{}: {:.0}", t("伤害", "Damage"), def.damage));
    }
    if def.healing > 0.0 {
        lines.push(format!("{}: {:.0}", t("治疗", "Healing"), def.healing));
    }
    if def.duration > 0.0 {
        lines.push(format!("{}: {:.0}s", t("持续", "Duration"), def.duration));
    }
    if def.impact_delay > 0.0 {
        lines.push(format!(
            "{}: {:.1}s",
            t("落点延迟", "Impact Delay"),
            def.impact_delay
        ));
    }
    if !missing_requirements.is_empty() {
        lines.push(format!(
            "{}: {}",
            t("缺少科技", "Missing tech"),
            missing_requirements.join(", ")
        ));
    } else if state.low_power && def.requires_power {
        lines.push(t("低电力: 支援离线", "Low power: support offline").to_string());
    } else if let Some(seconds) = state.cooldown_seconds {
        lines.push(format!("{}: {seconds}s", t("冷却中", "Cooling down")));
    } else if state.active {
        lines.push(t("选择目标位置", "Choose a target position").to_string());
    } else if state.enabled {
        lines.push(t("就绪: 点击后选择目标", "Ready: click then choose a target").to_string());
    } else {
        lines.push(t("不可用", "Unavailable").to_string());
    }
    lines.join("\n")
}

pub(crate) fn support_power_button_colors(
    state: &SupportPowerButtonState,
    interaction: Interaction,
) -> (Color, Color) {
    if state.active {
        return (
            Color::srgba(0.18, 0.14, 0.04, 0.97),
            Color::srgb(0.96, 0.72, 0.24),
        );
    }
    if !state.unlocked {
        return (
            Color::srgba(0.045, 0.052, 0.056, 0.72),
            Color::srgb(0.58, 0.34, 0.18),
        );
    }
    if !state.enabled {
        return (
            Color::srgba(0.045, 0.052, 0.06, 0.74),
            Color::srgb(0.22, 0.28, 0.31),
        );
    }
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.14, 0.24, 0.28, 0.97),
            Color::srgb(0.42, 0.72, 0.76),
        ),
        Interaction::Hovered => (
            Color::srgba(0.08, 0.12, 0.14, 0.94),
            Color::srgb(0.34, 0.58, 0.60),
        ),
        Interaction::None => (
            Color::srgba(0.035, 0.045, 0.055, 0.9),
            Color::srgb(0.32, 0.42, 0.46),
        ),
    }
}

pub(crate) fn support_power_badge_color(state: &SupportPowerButtonState) -> TextColor {
    if !state.unlocked {
        TextColor(Color::srgb(1.0, 0.56, 0.24))
    } else if state.cooldown_seconds.is_some() {
        TextColor(Color::srgb(0.98, 0.84, 0.42))
    } else if state.low_power {
        TextColor(Color::srgb(1.0, 0.42, 0.32))
    } else {
        TextColor(Color::srgb(0.98, 0.84, 0.42))
    }
}

pub(crate) fn support_power_hotkey_color(state: &SupportPowerButtonState) -> TextColor {
    if state.enabled || state.active {
        TextColor(Color::srgb(0.78, 0.96, 0.92))
    } else {
        TextColor(Color::srgba(0.56, 0.68, 0.68, 0.78))
    }
}

pub(crate) fn refresh_support_power_panel(
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    mut command_mode: ResMut<CommandMode>,
    mut panel_state: ResMut<SupportPowerPanelState>,
    structures: Query<StructurePrereqItem<'_>>,
    mut panel_q: Query<
        &mut Visibility,
        (
            With<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerCooldownLabel>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut buttons: Query<
        (
            &SupportPowerButton,
            &Interaction,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerCooldownLabel>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut cooldown_labels: Query<
        (&SupportPowerCooldownLabel, &mut Text, &mut TextColor),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerHotkeyLabel>,
        ),
    >,
    mut hotkey_labels: Query<
        (&SupportPowerHotkeyLabel, &mut TextColor),
        (
            Without<SupportPowersPanel>,
            Without<SupportPowerButton>,
            Without<SupportPowerCooldownLabel>,
        ),
    >,
) {
    let Some(team) = controlled_player_team(Some(&*visible_player)) else {
        panel_state.visible_count = 0;
        command_mode.support_power = None;
        for mut visibility in &mut panel_q {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    if command_mode
        .support_power
        .is_some_and(|power| !support_power_unlocked(team, power, &structures))
    {
        command_mode.support_power = None;
    }

    let low_power = economies.get(team).low_power();
    let mut visible_count = 0usize;
    for (button, interaction, mut node, mut button_visibility, mut background, mut border) in
        &mut buttons
    {
        let unlocked = support_power_unlocked(team, button.kind, &structures);
        if unlocked {
            visible_count += 1;
            node.display = Display::Flex;
            *button_visibility = Visibility::Inherited;
        } else {
            node.display = Display::None;
            *button_visibility = Visibility::Hidden;
        }
        let state = support_power_button_state(
            button.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, button.kind),
            command_mode.support_power == Some(button.kind),
        );
        let (bg, border_color) = support_power_button_colors(&state, *interaction);
        *background = BackgroundColor(bg);
        *border = BorderColor::all(border_color);
    }
    panel_state.visible_count = visible_count;
    debug_assert_eq!(
        visible_count,
        visible_support_power_count(team, &structures)
    );
    for mut visibility in &mut panel_q {
        *visibility = if visible_count > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (label, mut text, mut text_color) in &mut cooldown_labels {
        let unlocked = support_power_unlocked(team, label.kind, &structures);
        let state = support_power_button_state(
            label.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, label.kind),
            command_mode.support_power == Some(label.kind),
        );
        **text = state.badge_text.clone();
        *text_color = support_power_badge_color(&state);
    }
    for (label, mut text_color) in &mut hotkey_labels {
        let unlocked = support_power_unlocked(team, label.kind, &structures);
        let state = support_power_button_state(
            label.kind,
            unlocked,
            low_power,
            support_cooldowns.remaining_for(team, label.kind),
            command_mode.support_power == Some(label.kind),
        );
        *text_color = support_power_hotkey_color(&state);
    }
}

pub(crate) fn support_power_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    support_cooldowns: Res<SupportCooldowns>,
    structures: Query<StructurePrereqItem<'_>>,
    mut command_mode: ResMut<CommandMode>,
    buttons: Query<(&Interaction, &SupportPowerButton)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(team) = controlled_player_team(Some(&*visible_player)) else {
        command_mode.support_power = None;
        return;
    };
    let Some((_, button)) = buttons
        .iter()
        .filter(|(interaction, _)| **interaction == Interaction::Pressed)
        .min_by_key(|(_, button)| button.kind.idx())
    else {
        return;
    };
    if player_support_power_available(
        team,
        button.kind,
        &economies,
        &support_cooldowns,
        &structures,
    ) {
        toggle_support_power_mode(&mut command_mode, button.kind);
    } else if command_mode.support_power == Some(button.kind) {
        command_mode.support_power = None;
    }
}

pub(crate) fn toggle_support_power_mode(
    command_mode: &mut CommandMode,
    power: SupportPowerKind,
) -> bool {
    let enabled = command_mode.support_power != Some(power);
    clear_targeting_modes(command_mode);
    if enabled {
        command_mode.support_power = Some(power);
    }
    enabled
}

pub(crate) fn support_requirements_met(
    team: Team,
    required: &[&'static str],
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    if required.is_empty() {
        return true;
    }
    for requirement in required {
        if !structures
            .iter()
            .any(|(structure, structure_team, _, under_construction)| {
                structure_is_constructed(under_construction)
                    && structure_team == &team
                    && structure.id == *requirement
            })
        {
            return false;
        }
    }
    true
}

pub(crate) fn try_activate_ai_support_power(
    team: Team,
    player_team: Team,
    commands: &mut Commands,
    economies: &Economies,
    support_cooldowns: &mut SupportCooldowns,
    battle_log: &mut BattleLog,
    audio_feedback: &mut AudioFeedback,
    relations: &TeamRelations,
    structures: &Query<StructurePrereqItem<'_>>,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structure_targets: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> bool {
    for power in AI_SUPPORT_POWER_PRIORITY {
        if !ai_support_power_available(team, power, economies, support_cooldowns, structures) {
            continue;
        }
        let Some(target) =
            ai_support_power_target(team, power, relations, units, structure_targets)
        else {
            continue;
        };
        let support_targets = ai_support_power_targets(units, structure_targets);
        if activate_support_power(
            commands,
            target,
            power,
            team,
            player_team,
            economies,
            support_cooldowns,
            battle_log,
            relations,
            structures,
            &support_targets,
        ) {
            record_support_power_audio_feedback(audio_feedback, team, player_team, power);
            return true;
        }
    }
    false
}

pub(crate) fn support_power_available_for_audio(
    team: Team,
    power: SupportPowerKind,
    economies: &Economies,
    structures: &Query<StructurePrereqItem<'_>>,
) -> bool {
    let def = power.definition();
    (!def.requires_power || !economies.get(team).low_power())
        && support_requirements_met(team, def.requirements, structures)
}

pub(crate) fn any_enemy_support_target_position(
    team: Team,
    relations: &TeamRelations,
    units: &Query<(Entity, &Team, &Transform, &Selectable, &Health, &Unit), With<Unit>>,
    structures: &Query<CaptureStructureTargetItem<'_>, With<Structure>>,
) -> Option<Vec3> {
    units
        .iter()
        .find_map(|(_, unit_team, transform, _, health, _)| {
            (relations.are_enemies(team, *unit_team) && health.current > 0.0)
                .then_some(transform.translation)
        })
        .or_else(|| {
            structures.iter().find_map(
                |(_, _, structure_team, transform, health, under_construction)| {
                    (relations.are_enemies(team, *structure_team)
                        && health.current > 0.0
                        && structure_is_constructed(under_construction))
                    .then_some(transform.translation)
                },
            )
        })
}

/// Set for one frame when a left-click just fired an armed support power, so
/// the selection system swallows that click instead of also box-selecting.
#[derive(Resource, Default)]
pub(crate) struct SupportFireClickGuard(pub(crate) bool);

/// Fires the armed support power at the cursor on LEFT-click — the genre
/// convention (left = confirm target, right = cancel). This used to be
/// right-click-to-fire with left-click silently disarming, which read as
/// "F1 does nothing" the moment anyone clicked their target normally.
pub(crate) fn fire_support_power_on_left_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    hud_zones: Res<HudHitZones>,
    visible_player: Res<VisiblePlayer>,
    economies: Res<Economies>,
    structures: Query<StructurePrereqItem<'_>>,
    selectable_q: Query<SelectableOrderTargetItem<'_>>,
    mut order_resources: OrderResources,
    mut guard: ResMut<SupportFireClickGuard>,
) {
    guard.0 = false;
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(power) = order_resources.command_mode.support_power else {
        return;
    };
    let Some(visible_team) = controlled_player_team(Some(&*visible_player)) else {
        return;
    };
    let Ok(window) = window_q.single() else {
        return;
    };
    if window
        .cursor_position()
        .is_none_or(|cursor| cursor_blocks_world_order_controls(cursor, &hud_zones))
    {
        return;
    }
    let Some(raw_point) = pointer_ground(window, &camera_q, &order_resources.terrain) else {
        return;
    };
    let Some(point) = validated_terrain_target_in_bounds(raw_point, *order_resources.map_bounds)
    else {
        return;
    };
    let support_targets = support_power_target_snapshots(&selectable_q);
    if activate_support_power(
        &mut commands,
        point,
        power,
        visible_team,
        visible_team,
        &economies,
        &mut order_resources.support_cooldowns,
        &mut order_resources.battle_log,
        &order_resources.relations,
        &structures,
        &support_targets,
    ) {
        record_support_power_audio_feedback(
            &mut order_resources.audio_feedback,
            visible_team,
            visible_team,
            power,
        );
    }
    order_resources.command_mode.support_power = None;
    guard.0 = true;
}
