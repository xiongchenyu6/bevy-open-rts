//! Campaign completion is a player profile, separate from a single match save.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CampaignProfileData {
    version: u32,
    training_complete: bool,
    completed: Vec<String>,
}
impl Default for CampaignProfileData {
    fn default() -> Self {
        Self {
            version: 1,
            training_complete: false,
            completed: Vec::new(),
        }
    }
}
#[derive(Resource, Default)]
pub(crate) struct CampaignProgress {
    data: CampaignProfileData,
    path: Option<std::path::PathBuf>,
    storage_error: Option<String>,
    pub(crate) notice: Option<String>,
}
impl CampaignProgress {
    pub(crate) fn completed(&self, index: usize) -> bool {
        mission_by_index(index).is_some_and(|m| self.data.completed.iter().any(|id| id == m.id))
    }
    pub(crate) fn unlocked(&self, index: usize) -> bool {
        index < CAMPAIGN_MISSIONS.len() && (index == 0 || self.completed(index - 1))
    }
    pub(crate) fn training_complete(&self) -> bool {
        self.data.training_complete
    }
    pub(crate) fn load_for_player() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Ok(directory) = user_data_directory() else {
                return Self {
                    storage_error: Some("Campaign progress storage is unavailable".into()),
                    notice: Some("Campaign progress storage is unavailable".into()),
                    ..default()
                };
            };
            Self::load_from_path(directory.join("campaign.ron"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut profile = Self::default();
            if let Some(text) = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item("open-rts-campaign").ok().flatten())
            {
                match ron::from_str::<CampaignProfileData>(&text) {
                    Ok(data) if data.version == 1 => profile.data = data,
                    _ => profile.notice = Some("Campaign progress could not be read".into()),
                }
            }
            profile
        }
    }
    /// Path-based loading keeps capture sandboxes and I/O tests away from the
    /// real player directory and process-wide environment variables.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load_from_path(path: impl Into<std::path::PathBuf>) -> Self {
        let path = path.into();
        let mut profile = Self {
            path: Some(path.clone()),
            ..default()
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                let data = std::str::from_utf8(&bytes)
                    .ok()
                    .and_then(|text| ron::from_str::<CampaignProfileData>(text).ok());
                if let Some(data) = data.filter(|data| data.version == 1) {
                    profile.data = data;
                } else {
                    match Self::keep_recovery_copy(&path, &bytes) {
                        Ok(()) => {
                            profile.notice = Some(
                                "Campaign progress could not be read; a recovery copy was kept"
                                    .into(),
                            )
                        }
                        Err(error) => {
                            let message = format!(
                                "Campaign progress could not be read; recovery copy failed and the original will not be overwritten: {error}"
                            );
                            profile.storage_error = Some(message.clone());
                            profile.notice = Some(message);
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                let message = format!(
                    "Campaign progress could not be read; the original will not be overwritten: {error}"
                );
                profile.storage_error = Some(message.clone());
                profile.notice = Some(message);
            }
        }
        profile
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn keep_recovery_copy(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
        let mut suffix = 0usize;
        loop {
            let recovery = path.with_extension(if suffix == 0 {
                "ron.corrupt".to_string()
            } else {
                format!("ron.corrupt.{suffix}")
            });
            match std::fs::read(&recovery) {
                Ok(existing) if existing == bytes => return Ok(()),
                Ok(_) => {
                    suffix += 1;
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
            std::fs::copy(path, &recovery).map_err(|error| error.to_string())?;
            if std::fs::read(&recovery).map_err(|error| error.to_string())? != bytes {
                return Err("Recovery copy does not match the original profile".into());
            }
            return Ok(());
        }
    }

    fn persist(&mut self) {
        if let Some(error) = &self.storage_error {
            self.notice = Some(format!("Progress could not be saved: {error}"));
            return;
        }
        let result = ron::ser::to_string(&self.data)
            .map_err(|e| e.to_string())
            .and_then(|text| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let Some(path) = &self.path else {
                        return Ok(());
                    };
                    let parent = path.parent().ok_or("Campaign directory is unavailable")?;
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
                    std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
                    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let storage = web_sys::window()
                        .and_then(|w| w.local_storage().ok().flatten())
                        .ok_or("Campaign storage is unavailable")?;
                    storage
                        .set_item("open-rts-campaign", &text)
                        .map_err(|_| "Campaign progress could not be saved".to_string())
                }
            });
        match result {
            Err(error) => self.notice = Some(format!("Progress could not be saved: {error}")),
            Ok(()) => self.notice = None,
        }
    }
    fn record_completion(&mut self, mission_index: Option<usize>, training: bool) {
        let mission = mission_index.and_then(mission_by_index);
        if mission.is_none() && !training {
            return;
        }
        if let Some(mission) = mission
            && !self.data.completed.iter().any(|id| id == mission.id)
        {
            self.data.completed.push(mission.id.into());
        }
        self.data.training_complete |= training;
        // A fresh victory retries a prior failed save even when completion is
        // already present in memory. IDs remain unique; one write carries both.
        self.persist();
    }
}

pub(crate) fn record_campaign_completion(
    state: Res<MatchState>,
    active: Res<ActiveMission>,
    tutorial: Res<TutorialState>,
    mut profile: ResMut<CampaignProgress>,
    mut recorded_victory: Local<bool>,
) {
    if state.phase == MatchPhase::Running {
        *recorded_victory = false;
        return;
    }
    if state.phase != MatchPhase::HumanVictory || *recorded_victory {
        return;
    }
    *recorded_victory = true;
    profile.record_completion(active.0, tutorial.stage == TutorialStage::Complete);
}

#[cfg(test)]
mod tests;
