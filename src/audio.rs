//! Audio feedback: unit acknowledgment voices, announcer lines and UI/sfx
//! samples, queued via [`AudioFeedback`] and flushed by the play system.
//!
//! Pure move out of lib.rs (module-split Stage 3); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitVoiceEvent {
    Hello,
    Ack1,
    Ack2,
    Training,
    UnitReady,
    ConstructionComplete,
    NotEnoughResources,
    SupportPowerReady,
    SupportPowerFired,
    EnemySupportPowerFired,
    EnemySuperweaponReady,
    EnemySuperweaponLaunched,
    Victory,
    Defeat,
    BaseUnderAttack,
    UnitUnderAttack,
    UnitLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundEffectKind {
    Select,
    Command,
    ProductionStart,
    ProductionReady,
    ConstructionStarted,
    ConstructionCanceled,
    Error,
    LowPower,
    RepairStarted,
    StructureCaptured,
    StructureLost,
    StructureSold,
    SupplyCrate,
    UnitPromoted,
    SupportPowerReady,
    SupportPowerFire,
    SuperweaponWarning,
    WeaponHit,
    Explosion,
}

#[derive(Resource)]
pub(crate) struct AudioFeedback {
    pub(crate) pending_voice: Option<UnitVoiceEvent>,
    pub(crate) pending_sound: Option<SoundEffectKind>,
    pub(crate) last_voice: Option<UnitVoiceEvent>,
    pub(crate) last_sound: Option<SoundEffectKind>,
    pub(crate) last_command_key: Option<&'static str>,
    pub(crate) last_low_power: Option<bool>,
    pub(crate) next_ack_is_first: bool,
}

impl Default for AudioFeedback {
    fn default() -> Self {
        Self {
            pending_voice: None,
            pending_sound: None,
            last_voice: None,
            last_sound: None,
            last_command_key: None,
            last_low_power: None,
            next_ack_is_first: true,
        }
    }
}

#[cfg(feature = "audio")]
pub(crate) fn play_pending_audio_feedback(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut feedback: ResMut<AudioFeedback>,
) {
    if let Some(sound) = feedback.pending_sound.take() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(sound.audio_path())),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(sound.volume())),
            MatchScopedEntity,
        ));
        feedback.last_sound = Some(sound);
    }
    if let Some(voice) = feedback.pending_voice.take() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(voice.audio_path())),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.92)),
            MatchScopedEntity,
        ));
        feedback.last_voice = Some(voice);
    }
}

#[cfg(not(feature = "audio"))]
pub(crate) fn play_pending_audio_feedback(mut feedback: ResMut<AudioFeedback>) {
    if let Some(sound) = feedback.pending_sound.take() {
        feedback.last_sound = Some(sound);
    }
    if let Some(voice) = feedback.pending_voice.take() {
        feedback.last_voice = Some(voice);
    }
}

impl SoundEffectKind {
    #[allow(dead_code)]
    pub(crate) fn audio_path(self) -> &'static str {
        match self {
            Self::Select => "sfx/ui_select.wav",
            Self::Command => "sfx/command_confirm.wav",
            Self::ProductionStart => "sfx/production_start.wav",
            Self::ProductionReady => "sfx/production_ready.wav",
            Self::ConstructionStarted => "sfx/construction_started.wav",
            Self::ConstructionCanceled => "sfx/construction_canceled.wav",
            Self::Error => "sfx/error.wav",
            Self::LowPower => "sfx/low_power_warning.wav",
            Self::RepairStarted => "sfx/repair_started.wav",
            Self::StructureCaptured => "sfx/structure_captured.wav",
            Self::StructureLost => "sfx/structure_lost.wav",
            Self::StructureSold => "sfx/structure_sold.wav",
            Self::SupplyCrate => "sfx/supply_crate.wav",
            Self::UnitPromoted => "sfx/unit_promoted.wav",
            Self::SupportPowerReady => "sfx/support_power_ready.wav",
            Self::SupportPowerFire => "sfx/support_power_fire.wav",
            Self::SuperweaponWarning => "sfx/superweapon_warning.wav",
            Self::WeaponHit => "sfx/weapon_hit.wav",
            Self::Explosion => "sfx/explosion_small.wav",
        }
    }

    #[allow(dead_code)]
    pub(crate) fn volume(self) -> f32 {
        match self {
            Self::Select => 0.72,
            Self::Command => 0.66,
            Self::ProductionStart => 0.5,
            Self::ProductionReady => 0.56,
            Self::ConstructionStarted => 0.56,
            Self::ConstructionCanceled => 0.56,
            Self::Error => 0.63,
            Self::LowPower => 0.7,
            Self::RepairStarted => 0.56,
            Self::StructureCaptured => 0.63,
            Self::StructureLost => 0.72,
            Self::StructureSold => 0.63,
            Self::SupplyCrate => 0.56,
            Self::UnitPromoted => 0.63,
            Self::SupportPowerReady => 0.63,
            Self::SupportPowerFire => 0.7,
            Self::SuperweaponWarning => 0.82,
            Self::WeaponHit => 0.45,
            Self::Explosion => 0.63,
        }
    }
}

impl UnitVoiceEvent {
    #[allow(dead_code)]
    pub(crate) fn audio_path(self) -> &'static str {
        match self {
            Self::Hello => "voice/english/ttsmaker-com-2704-jackson-us/sir.ogg",
            Self::Ack1 => "voice/english/ttsmaker-com-2704-jackson-us/yes_sir.ogg",
            Self::Ack2 => "voice/english/ttsmaker-com-2704-jackson-us/acknowledged.ogg",
            Self::Training => "voice/english/ttsmaker-com-148-alayna-us/training.ogg",
            Self::UnitReady => "voice/english/ttsmaker-com-148-alayna-us/unit_ready.ogg",
            Self::ConstructionComplete => {
                "voice/english/ttsmaker-com-148-alayna-us/construction_complete.ogg"
            }
            Self::NotEnoughResources => {
                "voice/english/ttsmaker-com-148-alayna-us/not_enough_resources.ogg"
            }
            Self::SupportPowerReady => "voice/english/ttsmaker-com-148-alayna-us/unit_ready.ogg",
            Self::SupportPowerFired => {
                "voice/english/ttsmaker-com-2704-jackson-us/acknowledged.ogg"
            }
            Self::EnemySupportPowerFired => {
                "voice/english/ttsmaker-com-148-alayna-us/unit_under_attack.ogg"
            }
            Self::EnemySuperweaponReady | Self::EnemySuperweaponLaunched => {
                "voice/english/ttsmaker-com-148-alayna-us/your_base_is_under_attack.ogg"
            }
            Self::Victory => "voice/english/ttsmaker-com-148-alayna-us/you_are_victorious.ogg",
            Self::Defeat => "voice/english/ttsmaker-com-148-alayna-us/you_have_lost.ogg",
            Self::BaseUnderAttack => {
                "voice/english/ttsmaker-com-148-alayna-us/your_base_is_under_attack.ogg"
            }
            Self::UnitUnderAttack => {
                "voice/english/ttsmaker-com-148-alayna-us/unit_under_attack.ogg"
            }
            Self::UnitLost => "voice/english/ttsmaker-com-148-alayna-us/unit_lost.ogg",
        }
    }
}

pub(crate) fn record_sound_audio_feedback(feedback: &mut AudioFeedback, sound: SoundEffectKind) {
    feedback.pending_sound = Some(sound);
}

pub(crate) fn record_voice_audio_feedback(feedback: &mut AudioFeedback, voice: UnitVoiceEvent) {
    feedback.pending_voice = Some(voice);
}

pub(crate) fn record_low_power_audio_feedback(
    feedback: &mut AudioFeedback,
    is_low_power: bool,
) -> bool {
    let became_low_power = feedback
        .last_low_power
        .is_some_and(|was_low_power| !was_low_power && is_low_power);
    if became_low_power {
        record_sound_audio_feedback(feedback, SoundEffectKind::LowPower);
    }
    feedback.last_low_power = Some(is_low_power);
    became_low_power
}

pub(crate) fn record_support_power_ready_audio_feedback(
    feedback: &mut AudioFeedback,
    team: Team,
    player_team: Team,
    power: SupportPowerKind,
) {
    if team == player_team {
        record_sound_audio_feedback(feedback, SoundEffectKind::SupportPowerReady);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::SupportPowerReady);
    } else if power.is_superweapon() {
        record_sound_audio_feedback(feedback, SoundEffectKind::SuperweaponWarning);
        record_voice_audio_feedback(feedback, UnitVoiceEvent::EnemySuperweaponReady);
    }
}

pub(crate) fn record_selection_audio_feedback(
    feedback: &mut AudioFeedback,
    selected_owned: bool,
    selected_owned_voice_unit: bool,
) {
    if selected_owned {
        feedback.pending_sound = Some(SoundEffectKind::Select);
    }
    if selected_owned_voice_unit {
        feedback.pending_voice = Some(UnitVoiceEvent::Hello);
    }
}

pub(crate) fn record_command_audio_feedback(
    feedback: &mut AudioFeedback,
    has_owned_voice_unit: bool,
    command_key: Option<&'static str>,
) {
    if !has_owned_voice_unit {
        return;
    }
    feedback.pending_sound = Some(SoundEffectKind::Command);
    let event = if feedback.next_ack_is_first {
        UnitVoiceEvent::Ack1
    } else {
        UnitVoiceEvent::Ack2
    };
    feedback.pending_voice = Some(event);
    feedback.next_ack_is_first = !feedback.next_ack_is_first;
    feedback.last_command_key = command_key;
}

pub(crate) fn is_voice_unit(unit: &Unit) -> bool {
    unit.speed > 0.0
}
