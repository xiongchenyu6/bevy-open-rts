//! Isolated audio QA: real loaders/playback; no simulated game-event acceptance.
use crate::*;
use bevy::audio::{AudioPlugin, AudioSinkPlayback, Decodable};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[cfg(feature = "audio")]
pub fn capture_voice_audio(output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err("audio QA output already exists".into());
    }
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        AudioPlugin::default(),
    ));
    app.init_resource::<AudioFeedback>();
    app.insert_resource(MenuOptionsState::default());
    app.finish();
    app.cleanup();
    let events = [
        UnitVoiceEvent::Hello,
        UnitVoiceEvent::Ack1,
        UnitVoiceEvent::Ack2,
        UnitVoiceEvent::Training,
        UnitVoiceEvent::UnitReady,
        UnitVoiceEvent::ConstructionComplete,
        UnitVoiceEvent::NotEnoughResources,
        UnitVoiceEvent::SupportPowerReady,
        UnitVoiceEvent::SupportPowerFired,
        UnitVoiceEvent::EnemySupportPowerFired,
        UnitVoiceEvent::EnemySuperweaponReady,
        UnitVoiceEvent::EnemySuperweaponLaunched,
        UnitVoiceEvent::Victory,
        UnitVoiceEvent::Defeat,
        UnitVoiceEvent::BaseUnderAttack,
        UnitVoiceEvent::UnitUnderAttack,
        UnitVoiceEvent::UnitLost,
    ];
    let mut rows = Vec::new();
    let mut device_unavailable = false;
    let persist = |rows: &Vec<serde_json::Value>, finished: bool| -> Result<(), String> {
        let loader_pass = rows.len() == 34 && rows.iter().all(|r| r["loader_success"] == true);
        let playback_pass = loader_pass
            && rows
                .iter()
                .all(|r| r["natural_completed"] == true && r["sink_seen"] == true);
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"Audio QA only; no game event or human listening acceptance",
                "events":rows,"finished":finished,"loader_pass":loader_pass,
                "playback_pass":playback_pass,"human_listening_accepted":false
            }))
            .unwrap(),
        )
        .map_err(|e| e.to_string())
    };
    for language in [Language::En, Language::Zh] {
        app.insert_resource(Locale(language));
        for event in events {
            let path = event.audio_path(language);
            let handle: Handle<AudioSource> = app.world().resource::<AssetServer>().load(path);
            let deadline = Instant::now() + Duration::from_secs(15);
            while app
                .world()
                .resource::<Assets<AudioSource>>()
                .get(&handle)
                .is_none()
            {
                if Instant::now() > deadline {
                    rows.push(serde_json::json!({"event":format!("{event:?}"),"language":format!("{language:?}"),"path":path,"loader_success":false,"error":"loader timeout"}));
                    persist(&rows, false)?;
                    return Err(format!("audio loader timeout: {path}"));
                }
                app.update();
                std::thread::sleep(Duration::from_millis(10));
            }
            let samples = app
                .world()
                .resource::<Assets<AudioSource>>()
                .get(&handle)
                .unwrap()
                .decoder()
                .count();
            if samples == 0 {
                rows.push(serde_json::json!({"event":format!("{event:?}"),"language":format!("{language:?}"),"path":path,"loader_success":false,"error":"empty decoder"}));
                persist(&rows, false)?;
                return Err(format!("empty decoded source: {path}"));
            }
            app.world_mut()
                .resource_mut::<AudioFeedback>()
                .pending_voice = Some(event);
            app.world_mut()
                .run_system_once(play_pending_audio_feedback)
                .map_err(|e| e.to_string())?;
            let (entity, volume) = app
                .world_mut()
                .query::<(Entity, &PlaybackSettings)>()
                .iter(app.world())
                .next()
                .map(|(e, p)| (e, p.volume))
                .ok_or("missing playback settings")?;
            if volume != Volume::Linear(0.92) {
                return Err(format!("wrong voice gain: {volume:?}"));
            }
            let started = Instant::now();
            let mut sink_seen = false;
            let mut natural_completed = false;
            let mut timed_out = false;
            let mut sink_volume = None;
            loop {
                app.update();
                if let Some(sink) = app.world().get::<AudioSink>(entity) {
                    sink_seen = true;
                    sink_volume = Some(format!("{:?}", sink.volume()));
                }
                if app.world().get_entity(entity).is_err() {
                    natural_completed = sink_seen;
                    break;
                }
                if !sink_seen
                    && (device_unavailable || started.elapsed() >= Duration::from_millis(500))
                {
                    device_unavailable = true;
                    break;
                }
                if started.elapsed() >= Duration::from_secs(12) {
                    timed_out = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let forced_cleanup = app.world().get_entity(entity).is_ok();
            if forced_cleanup {
                app.world_mut().despawn(entity);
            }
            rows.push(serde_json::json!({"event":format!("{event:?}"),"language":format!("{language:?}"),"path":path,
                "loader_success":true,"decoded_samples":samples,"volume":0.92,"sink_volume":sink_volume,
                "sink_seen":sink_seen,"natural_completed":natural_completed,"timed_out":timed_out,
                "forced_cleanup":forced_cleanup,"elapsed_seconds":started.elapsed().as_secs_f64(),
                "scope":if !sink_seen {"loader-only: no output sink observed"} else {"loader and actual playback"}}));
            persist(&rows, false)?;
            if timed_out {
                return Err(format!("actual playback timeout: {path}"));
            }
        }
    }
    persist(&rows, true)?;
    Ok(())
}
#[cfg(not(feature = "audio"))]
pub fn capture_voice_audio(_: &Path) -> Result<(), String> {
    Err("audio feature required".into())
}
