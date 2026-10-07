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
                    notice: Some("Campaign progress storage is unavailable".into()),
                    ..default()
                };
            };
            let path = directory.join("campaign.ron");
            let mut profile = Self {
                path: Some(path.clone()),
                ..default()
            };
            match std::fs::read_to_string(&path) {
                Ok(text) => match ron::from_str::<CampaignProfileData>(&text) {
                    Ok(data) if data.version == 1 => {
                        profile.data = data;
                    }
                    _ => {
                        let _ = std::fs::copy(&path, path.with_extension("ron.corrupt"));
                        profile.notice = Some(
                            "Campaign progress could not be read; a recovery copy was kept".into(),
                        );
                    }
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => profile.notice = Some("Campaign progress could not be read".into()),
            }
            profile
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
    fn persist(&mut self) {
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
        if let Err(error) = result {
            self.notice = Some(format!("Progress could not be saved: {error}"));
        }
    }
    fn record_mission(&mut self, index: usize) {
        let Some(mission) = mission_by_index(index) else {
            return;
        };
        if self.completed(index) {
            return;
        }
        self.data.completed.push(mission.id.into());
        self.persist();
    }
}

pub(crate) fn record_campaign_completion(
    state: Res<MatchState>,
    active: Res<ActiveMission>,
    tutorial: Res<TutorialState>,
    mut profile: ResMut<CampaignProgress>,
) {
    if state.phase != MatchPhase::HumanVictory {
        return;
    }
    if let Some(index) = active.0 {
        profile.record_mission(index);
    }
    if tutorial.stage == TutorialStage::Complete && !profile.data.training_complete {
        profile.data.training_complete = true;
        profile.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completing_a_mission_unlocks_the_next_and_roundtrips() {
        let mut progress = CampaignProgress::default();
        assert!(progress.unlocked(0));
        assert!(!progress.unlocked(1));
        progress.record_mission(0);
        assert!(progress.unlocked(1));
        assert!(!progress.unlocked(2));
        let data = ron::ser::to_string(&progress.data).unwrap();
        let restored: CampaignProfileData = ron::from_str(&data).unwrap();
        assert_eq!(restored.completed, vec![CAMPAIGN_MISSIONS[0].id]);
        progress.record_mission(0);
        assert_eq!(progress.data.completed.len(), 1);
    }
}
