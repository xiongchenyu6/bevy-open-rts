//! Player preferences are separate from match saves and campaign progress.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    version: u32,
    options: MenuOptionsState,
}

fn decode(text: &str) -> Result<MenuOptionsState, String> {
    let saved: Preferences = ron::from_str(text).map_err(|e| e.to_string())?;
    let o = saved.options;
    if saved.version != 1
        || [
            o.master_volume,
            o.music_volume,
            o.sfx_volume,
            o.voice_volume,
        ]
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || !o.camera_tilt.is_finite()
        || !(CAMERA_TILT_MIN..=CAMERA_TILT_MAX).contains(&o.camera_tilt)
        || !o.camera_pan_speed.is_finite()
        || !(CAMERA_PAN_SPEED_MIN..=CAMERA_PAN_SPEED_MAX).contains(&o.camera_pan_speed)
    {
        return Err("Invalid or unsupported player preferences".into());
    }
    Ok(o)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_path(path: &std::path::Path) -> Result<Option<MenuOptionsState>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => decode(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_path(path: &std::path::Path, options: MenuOptionsState) -> Result<(), String> {
    // Preserve malformed/incompatible existing files instead of replacing them.
    load_path(path)?;
    let text = ron::ser::to_string(&Preferences {
        version: 1,
        options,
    })
    .map_err(|e| e.to_string())?;
    decode(&text)?;
    std::fs::create_dir_all(path.parent().ok_or("No preferences directory")?)
        .map_err(|e| e.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}

fn load() -> Result<Option<MenuOptionsState>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_path(&user_data_directory()?.join("preferences.ron"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or("Player preferences storage is unavailable")?;
        storage
            .get_item("open-rts-preferences")
            .map_err(|e| format!("{e:?}"))?
            .map(|text| decode(&text))
            .transpose()
    }
}

fn save(options: MenuOptionsState) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        save_path(&user_data_directory()?.join("preferences.ron"), options)
    }
    #[cfg(target_arch = "wasm32")]
    {
        load()?;
        let text = ron::ser::to_string(&Preferences {
            version: 1,
            options,
        })
        .map_err(|e| e.to_string())?;
        decode(&text)?;
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or("Player preferences storage is unavailable")?
            .set_item("open-rts-preferences", &text)
            .map_err(|e| format!("{e:?}"))
    }
}

pub(crate) fn restore_player_preferences(
    mut options: ResMut<MenuOptionsState>,
    mut locale: ResMut<Locale>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    match load() {
        Ok(Some(saved)) => {
            *options = saved;
            locale.0 = saved.language;
            set_current_language(saved.language);
            if let Ok(mut window) = windows.single_mut() {
                set_window_fullscreen(&mut window, saved.fullscreen);
            }
        }
        Ok(None) => {
            if let Ok(window) = windows.single_mut() {
                options.fullscreen = window_is_fullscreen(&window);
            }
        }
        Err(error) => warn!("Could not restore player preferences: {error}"),
    }
}

pub(crate) fn persist_player_preferences(
    options: Res<MenuOptionsState>,
    locale: Res<Locale>,
    mut previous: Local<Option<MenuOptionsState>>,
) {
    let mut current = *options;
    current.language = locale.0;
    if previous.is_some_and(|old| old != current) {
        if let Err(error) = save(current) {
            warn!("Could not save player preferences: {error}");
        }
    }
    *previous = Some(current);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn muted_channels_and_camera_preferences_survive_fresh_reload() {
        let dir = std::env::temp_dir().join(format!("rts-preferences-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fresh-reload.ron");
        let saved = MenuOptionsState {
            fullscreen: false,
            music_volume: 0.0,
            sfx_volume: 0.0,
            voice_volume: 0.0,
            camera_edge_pan: false,
            language: Language::Zh,
            ..default()
        };
        save_path(&path, saved).unwrap();
        assert_eq!(load_path(&path).unwrap().unwrap(), saved);
        let changed = MenuOptionsState {
            voice_volume: 0.25,
            fullscreen: true,
            ..saved
        };
        save_path(&path, changed).unwrap();
        assert_eq!(load_path(&path).unwrap().unwrap(), changed);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_preferences_are_rejected_and_existing_bytes_preserved() {
        let dir =
            std::env::temp_dir().join(format!("rts-preferences-invalid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("preferences.ron");
        for contents in [
            "not ron".to_string(),
            ron::ser::to_string(&Preferences {
                version: 2,
                options: MenuOptionsState::default(),
            })
            .unwrap(),
            ron::ser::to_string(&Preferences {
                version: 1,
                options: MenuOptionsState {
                    voice_volume: 2.0,
                    ..default()
                },
            })
            .unwrap(),
        ] {
            std::fs::write(&path, &contents).unwrap();
            assert!(load_path(&path).is_err());
            assert!(save_path(&path, MenuOptionsState::default()).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
        }
        std::fs::remove_file(path).unwrap();
    }
}
