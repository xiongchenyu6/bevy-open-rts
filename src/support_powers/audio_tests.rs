use super::*;

fn cooldown_app(team: Team, allied: bool, already_charged: bool) -> App {
    let mut app = App::new();
    let mut relations = TeamRelations::default();
    relations.set_allied(Team::Player(0), Team::Player(1), allied);
    let mut cooldowns = SupportCooldowns::default();
    let index = cooldowns.ensure_team(team).unwrap();
    if already_charged {
        cooldowns.initial_charge_started[index] = [true; SupportPowerKind::ALL.len()];
        cooldowns.remaining[index] = [10.0; SupportPowerKind::ALL.len()];
        cooldowns.remaining[index][SupportPowerKind::StrategicMissile.idx()] = 0.5;
    }
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs(1));
    app.insert_resource(time)
        .insert_resource(relations)
        .insert_resource(VisiblePlayer::per_player(Team::Player(0)))
        .insert_resource(Economies {
            players: vec![TeamEconomy::new(32, 16), TeamEconomy::new(32, 16)],
        })
        .insert_resource(cooldowns)
        .init_resource::<AudioFeedback>()
        .init_resource::<BattleLog>()
        .add_systems(Update, update_support_cooldowns);
    app.world_mut().spawn((
        Structure {
            id: "WeatherControlSpire",
        },
        team,
        Transform::default(),
    ));
    app
}

#[test]
fn actual_ready_cooldown_uses_live_alliances_and_preserves_local_enemy_events() {
    let mut app = cooldown_app(Team::Player(1), true, true);
    app.update();
    assert!(
        app.world()
            .resource::<SupportCooldowns>()
            .ready(Team::Player(1), SupportPowerKind::StrategicMissile)
    );
    assert_eq!(app.world().resource::<AudioFeedback>().pending_voice, None);
    assert_eq!(app.world().resource::<AudioFeedback>().pending_sound, None);
    assert!(app.world().resource::<BattleLog>().entries.is_empty());
    // The same actor is now hostile: the next natural cooldown expiry uses the new relation.
    app.world_mut().resource_mut::<TeamRelations>().set_allied(
        Team::Player(0),
        Team::Player(1),
        false,
    );
    app.world_mut().resource_mut::<SupportCooldowns>().set(
        Team::Player(1),
        SupportPowerKind::StrategicMissile,
        0.5,
    );
    app.update();
    assert_eq!(
        app.world().resource::<AudioFeedback>().pending_voice,
        Some(UnitVoiceEvent::EnemySuperweaponReady)
    );
    assert_eq!(
        app.world().resource::<AudioFeedback>().pending_sound,
        Some(SoundEffectKind::SuperweaponWarning)
    );
    assert_eq!(app.world().resource::<BattleLog>().entries.len(), 1);
    let mut own = cooldown_app(Team::Player(0), false, true);
    own.update();
    assert_eq!(
        own.world().resource::<AudioFeedback>().pending_voice,
        Some(UnitVoiceEvent::SupportPowerReady)
    );
    assert_eq!(
        own.world().resource::<AudioFeedback>().pending_sound,
        Some(SoundEffectKind::SupportPowerReady)
    );
    assert_eq!(own.world().resource::<BattleLog>().entries.len(), 1);
}

#[test]
fn actual_initial_charge_never_warns_for_allies_and_still_warns_for_enemies() {
    let mut ally = cooldown_app(Team::Player(1), true, false);
    ally.update();
    assert!(
        ally.world()
            .resource::<SupportCooldowns>()
            .remaining_for(Team::Player(1), SupportPowerKind::StrategicMissile)
            > 0.0
    );
    assert_eq!(ally.world().resource::<AudioFeedback>().pending_sound, None);
    assert!(ally.world().resource::<BattleLog>().entries.is_empty());
    let mut enemy = cooldown_app(Team::Player(1), false, false);
    enemy.update();
    assert_eq!(
        enemy.world().resource::<AudioFeedback>().pending_sound,
        Some(SoundEffectKind::SuperweaponWarning)
    );
    assert_eq!(enemy.world().resource::<BattleLog>().entries.len(), 2);
    let mut own = cooldown_app(Team::Player(0), false, false);
    own.update();
    assert_eq!(own.world().resource::<AudioFeedback>().pending_sound, None);
    assert_eq!(own.world().resource::<BattleLog>().entries.len(), 2);
}

#[test]
fn every_fired_power_distinguishes_player_ally_enemy_and_neutral() {
    let mut relations = TeamRelations::default();
    relations.set_allied(Team::Player(0), Team::Player(1), true);
    for power in SupportPowerKind::ALL {
        for actor in [Team::Player(1), Team::Neutral] {
            let mut feedback = AudioFeedback {
                pending_voice: Some(UnitVoiceEvent::Hello),
                pending_sound: Some(SoundEffectKind::Select),
                ..default()
            };
            record_support_power_audio_feedback(
                &mut feedback,
                actor,
                Team::Player(0),
                power,
                &relations,
            );
            assert_eq!(
                feedback.pending_voice,
                Some(UnitVoiceEvent::Hello),
                "allied/neutral firing must not overwrite another cue"
            );
            assert_eq!(feedback.pending_sound, Some(SoundEffectKind::Select));
        }
        let mut own = AudioFeedback::default();
        record_support_power_audio_feedback(
            &mut own,
            Team::Player(0),
            Team::Player(0),
            power,
            &relations,
        );
        assert_eq!(own.pending_voice, Some(UnitVoiceEvent::SupportPowerFired));
        assert_eq!(own.pending_sound, Some(SoundEffectKind::SupportPowerFire));
        let mut enemy = AudioFeedback::default();
        record_support_power_audio_feedback(
            &mut enemy,
            Team::Player(2),
            Team::Player(0),
            power,
            &relations,
        );
        assert_eq!(
            enemy.pending_voice,
            Some(if power.is_superweapon() {
                UnitVoiceEvent::EnemySuperweaponLaunched
            } else {
                UnitVoiceEvent::EnemySupportPowerFired
            })
        );
        assert_eq!(
            enemy.pending_sound,
            Some(if power.is_superweapon() {
                SoundEffectKind::SuperweaponWarning
            } else {
                SoundEffectKind::SupportPowerFire
            })
        );
    }
}
