use super::*;
#[test]
fn completing_a_mission_unlocks_the_next_and_roundtrips() {
    let mut progress = CampaignProgress::default();
    assert!(progress.unlocked(0));
    assert!(!progress.unlocked(1));
    progress.record_completion(Some(0), false);
    assert!(progress.unlocked(1));
    assert!(!progress.unlocked(2));
    let data = ron::ser::to_string(&progress.data).unwrap();
    let restored: CampaignProfileData = ron::from_str(&data).unwrap();
    assert_eq!(restored.completed, vec![CAMPAIGN_MISSIONS[0].id]);
    progress.record_completion(Some(0), false);
    assert_eq!(progress.data.completed.len(), 1);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod filesystem_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("open-rts-profile-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn profile_path(&self) -> PathBuf {
            self.0.join("nested/campaign.ron")
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn a_new_victory_retries_failed_mission_and_training_progress_without_frame_retries() {
        for training in [false, true] {
            let directory = TestDirectory::new();
            let path = directory.profile_path();
            let existing = ron::to_string(&CampaignProfileData::default()).unwrap();
            write(&path, existing.as_bytes());
            let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::create_dir(&temporary).unwrap();
            let mut app = App::new();
            app.insert_resource(MatchState::default());
            app.insert_resource(ActiveMission(if training { None } else { Some(0) }));
            app.insert_resource(TutorialState {
                stage: if training {
                    TutorialStage::Complete
                } else {
                    TutorialStage::SelectWorker
                },
                ..default()
            });
            app.insert_resource(CampaignProgress::load_from_path(path.clone()));
            app.add_systems(Update, record_campaign_completion);
            app.update();
            app.world_mut().resource_mut::<MatchState>().phase = MatchPhase::HumanVictory;
            app.update();
            assert!(
                app.world()
                    .resource::<CampaignProgress>()
                    .notice
                    .as_deref()
                    .unwrap()
                    .contains("Progress could not be saved")
            );
            assert_eq!(std::fs::read(&path).unwrap(), existing.as_bytes());

            // Fix storage while the same victory remains visible. Repeated
            // victory frames must not trigger additional persistence attempts.
            std::fs::remove_dir(&temporary).unwrap();
            for _ in 0..5 {
                app.update();
            }
            assert_eq!(std::fs::read(&path).unwrap(), existing.as_bytes());
            assert!(app.world().resource::<CampaignProgress>().notice.is_some());

            // Playing and winning again retries the exact same completion,
            // even though its flag/ID was already recorded in memory.
            app.world_mut().resource_mut::<MatchState>().phase = MatchPhase::Running;
            app.update();
            app.world_mut().resource_mut::<MatchState>().phase = MatchPhase::HumanVictory;
            app.update();
            let profile = CampaignProgress::load_from_path(path.clone());
            assert!(app.world().resource::<CampaignProgress>().notice.is_none());
            if training {
                assert!(profile.training_complete());
                assert!(profile.data.completed.is_empty());
            } else {
                assert!(profile.completed(0));
                assert_eq!(profile.data.completed.len(), 1);
            }
            let saved = std::fs::read(&path).unwrap();
            // Also prove that successful victory frames avoid writing. A new
            // blocked temporary path would expose an accidental retry as error.
            std::fs::create_dir(&temporary).unwrap();
            for _ in 0..5 {
                app.update();
            }
            assert!(app.world().resource::<CampaignProgress>().notice.is_none());
            assert_eq!(std::fs::read(&path).unwrap(), saved);
        }
    }

    #[test]
    fn mission_and_training_completion_survive_filesystem_reload() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        let mut progress = CampaignProgress::load_from_path(path.clone());
        assert!(progress.notice.is_none());
        progress.record_completion(Some(0), false);
        progress.data.training_complete = true;
        progress.persist();
        assert!(progress.notice.is_none());
        assert!(path.is_file());
        let restored = CampaignProgress::load_from_path(path);
        assert!(restored.completed(0));
        assert!(restored.unlocked(1));
        assert!(restored.training_complete());
        assert!(restored.notice.is_none());
    }

    #[test]
    fn corrupt_profile_is_preserved_before_writing_fresh_progress() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        let corrupt = b"not valid campaign data";
        write(&path, corrupt);
        let mut progress = CampaignProgress::load_from_path(path.clone());
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("a recovery copy was kept")
        );
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        assert_eq!(
            std::fs::read(path.with_extension("ron.corrupt")).unwrap(),
            corrupt
        );
        progress.record_completion(Some(0), false);
        assert!(progress.notice.is_none());
        assert!(CampaignProgress::load_from_path(path.clone()).completed(0));
        assert_eq!(
            std::fs::read(path.with_extension("ron.corrupt")).unwrap(),
            corrupt
        );
    }

    #[test]
    fn invalid_utf8_and_unsupported_profiles_get_exact_recovery_copies() {
        for bytes in [
            vec![0xff, 0xfe, 0x80],
            b"(version:2,training_complete:true,completed:[])".to_vec(),
        ] {
            let directory = TestDirectory::new();
            let path = directory.profile_path();
            write(&path, &bytes);
            let progress = CampaignProgress::load_from_path(path.clone());
            assert!(
                progress
                    .notice
                    .as_deref()
                    .unwrap()
                    .contains("a recovery copy was kept")
            );
            assert_eq!(
                std::fs::read(path.with_extension("ron.corrupt")).unwrap(),
                bytes
            );
            assert!(!progress.training_complete());
        }
    }

    #[test]
    fn recovery_never_overwrites_an_earlier_different_copy() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        write(&path, b"new damaged profile");
        write(
            &path.with_extension("ron.corrupt"),
            b"older damaged profile",
        );
        let progress = CampaignProgress::load_from_path(path.clone());
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("a recovery copy was kept")
        );
        assert_eq!(
            std::fs::read(path.with_extension("ron.corrupt")).unwrap(),
            b"older damaged profile"
        );
        assert_eq!(
            std::fs::read(path.with_extension("ron.corrupt.1")).unwrap(),
            b"new damaged profile"
        );
    }

    #[test]
    fn failed_recovery_blocks_writes_and_never_claims_a_copy_was_kept() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        let corrupt = b"unreadable profile";
        write(&path, corrupt);
        // A directory at the recovery destination fails on every platform,
        // including test processes privileged enough to ignore mode bits.
        std::fs::create_dir(path.with_extension("ron.corrupt")).unwrap();
        let mut progress = CampaignProgress::load_from_path(path.clone());
        let notice = progress.notice.as_deref().unwrap();
        assert!(notice.contains("recovery copy failed"));
        assert!(!notice.contains("a recovery copy was kept"));
        progress.record_completion(Some(0), false);
        progress.data.training_complete = true;
        progress.persist();
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("Progress could not be saved")
        );
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        assert!(
            !path
                .with_extension(format!("{}.tmp", std::process::id()))
                .exists()
        );
    }

    #[cfg(unix)]
    #[test]
    fn failed_copy_operation_blocks_overwrites_with_truthful_notice() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        let corrupt = b"damaged data requiring a backup";
        write(&path, corrupt);
        // Reading this dangling link reports NotFound, but copying through it
        // fails because its destination parent is absent. Exercise copy's Err.
        let recovery = path.with_extension("ron.corrupt");
        std::os::unix::fs::symlink(directory.0.join("missing-parent/recovery.ron"), &recovery)
            .unwrap();
        let mut progress = CampaignProgress::load_from_path(path.clone());
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("recovery copy failed")
        );
        assert!(
            !progress
                .notice
                .as_deref()
                .unwrap()
                .contains("a recovery copy was kept")
        );
        progress.record_completion(Some(0), false);
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        assert!(!directory.0.join("missing-parent").exists());
    }

    #[test]
    fn failed_temporary_write_preserves_saved_profile_and_reports_failure() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        let existing = ron::to_string(&CampaignProfileData::default()).unwrap();
        write(&path, existing.as_bytes());
        let mut progress = CampaignProgress::load_from_path(path.clone());
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::create_dir(&temporary).unwrap();
        progress.record_completion(Some(0), false);
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("Progress could not be saved")
        );
        assert_eq!(std::fs::read(&path).unwrap(), existing.as_bytes());
        assert!(!CampaignProgress::load_from_path(path.clone()).completed(0));
        std::fs::remove_dir(&temporary).unwrap();
        progress.persist();
        assert!(
            progress.notice.is_none(),
            "successful retry clears the stale failure notice"
        );
        assert!(CampaignProgress::load_from_path(path).completed(0));
    }

    #[test]
    fn unreadable_profile_path_blocks_overwriting_existing_directory() {
        let directory = TestDirectory::new();
        let path = directory.profile_path();
        std::fs::create_dir_all(&path).unwrap();
        let sentinel = path.join("keep.txt");
        std::fs::write(&sentinel, b"existing data").unwrap();
        let mut progress = CampaignProgress::load_from_path(path.clone());
        assert!(
            progress
                .notice
                .as_deref()
                .unwrap()
                .contains("original will not be overwritten")
        );
        progress.record_completion(Some(0), false);
        assert!(path.is_dir());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"existing data");
    }
}
