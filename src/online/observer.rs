//! Opt-in read-only evidence for the ordinary frontend. Never drives the lobby or game.
use super::*;

#[derive(Resource)]
pub(super) struct OnlineObserver {
    pub(super) enabled: bool,
    snapshots: BTreeMap<u64, OnlineWorldSnapshot>,
    last_report: String,
    last_context: serde_json::Value,
    dirty: bool,
}

impl Default for OnlineObserver {
    fn default() -> Self {
        Self {
            enabled: requested(),
            snapshots: BTreeMap::new(),
            last_report: String::new(),
            last_context: serde_json::Value::Null,
            dirty: false,
        }
    }
}

pub(super) fn requested() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("OPEN_BEVY_ONLINE_OBSERVE_STATUS").is_ok_and(|p| !p.trim().is_empty())
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
            .is_some_and(|p| p.get("online_observe").as_deref() == Some("1"))
    }
}

impl OnlineObserver {
    pub(super) fn record(&mut self, snapshot: &OnlineWorldSnapshot) {
        if !self.enabled {
            return;
        }
        self.snapshots.insert(snapshot.tick, snapshot.clone());
        self.dirty = true;
        while self.snapshots.len() > 40 {
            self.snapshots.pop_first();
        }
    }
}

pub(super) fn publish(
    mut observer: ResMut<OnlineObserver>,
    session: Res<OnlineSession>,
    screen: Res<State<AppScreen>>,
    replication: Res<OnlineMatchReplication>,
    match_state: Res<MatchState>,
) {
    if !observer.enabled {
        return;
    }
    let context = serde_json::json!({
        "schema": 1, "observe_only": true, "protocol": RTS_ONLINE_PROTOCOL,
        "app_screen": format!("{:?}", screen.get()),
        "online_phase": format!("{:?}", session.phase),
        "match_phase": format!("{:?}", match_state.phase),
        "room_code": session.room.as_ref().map(|r| r.room_code.to_string()),
        "local_player_id": session.local_player_id, "assigned_slot": session.assigned_slot,
        "is_host": session.is_host, "lobby": session.lobby,
        "last_sent_tick": replication.next_tick,
        "last_applied_tick": replication.last_applied_tick,
    });
    if !observer.dirty && context == observer.last_context {
        return;
    }
    let report = serde_json::json!({
        "context": context,
        "game_clock": match_state.start_time_sec,
        "snapshot_semantics": if session.is_host { "successfully_sent" } else { "validated_received_not_world_equivalence" },
        "snapshots": observer.snapshots.values().collect::<Vec<_>>(),
    });
    let Ok(report) = serde_json::to_string(&report) else {
        return;
    };
    if report == observer.last_report {
        return;
    }
    if write_report(&report).is_ok() {
        observer.last_report = report;
        observer.last_context = context;
        observer.dirty = false;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_report(report: &str) -> Result<(), String> {
    let Ok(path) = std::env::var("OPEN_BEVY_ONLINE_OBSERVE_STATUS") else {
        return Ok(());
    };
    let path = std::path::Path::new(&path);
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, report).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn write_report(report: &str) -> Result<(), String> {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("No browser document")?;
    let element = match doc.get_element_by_id("open-bevy-online-observer") {
        Some(e) => e,
        None => {
            let e = doc
                .create_element("script")
                .map_err(|_| "Could not create observer element")?;
            e.set_id("open-bevy-online-observer");
            e.set_attribute("type", "application/json")
                .map_err(|_| "Could not configure observer element")?;
            doc.body()
                .ok_or("No browser body")?
                .append_child(&e)
                .map_err(|_| "Could not attach observer element")?;
            e
        }
    };
    element.set_text_content(Some(report));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observer_does_not_enter_lobby_create_room_or_submit_commands() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.world_mut().resource_mut::<OnlineObserver>().enabled = true;
        for _ in 0..40 {
            app.update();
        }
        let session = app.world().resource::<OnlineSession>();
        assert_eq!(session.phase, OnlinePhase::Home);
        assert!(session.room.is_none() && session.lobby.is_none());
        let harness = app.world().resource::<OnlineVerificationHarness>();
        assert!(harness.config.is_none());
        assert_eq!(harness.stage, OnlineVerificationStage::Disabled);
        assert!(
            app.world()
                .resource::<OnlineCommandOutbox>()
                .pending
                .is_empty()
        );
        assert_ne!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::OnlineLobby
        );
        assert_ne!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::InMatch
        );
    }
    #[test]
    fn observer_is_off_by_default_and_stores_only_bounded_received_dtos() {
        let mut observer = OnlineObserver::default();
        let mut snapshot: OnlineWorldSnapshot = serde_json::from_value(serde_json::json!({
            "protocol":4,"tick":1,"entities":[],"economies":[],"build_queue":[],
            "support_cooldowns":[],"support_initial_charge_started":[],
            "match_state":{"start_time_sec":0.0,"remaining_teams":0,"remaining_anchors":0,"active_anchor_teams":[],"finished":false},"transient_events":[]
        })).unwrap();
        observer.record(&snapshot);
        assert!(observer.snapshots.is_empty());
        observer.enabled = true;
        for tick in 1..=50 {
            snapshot.tick = tick;
            observer.record(&snapshot);
        }
        assert_eq!(observer.snapshots.len(), 40);
        assert_eq!(observer.snapshots.first_key_value().unwrap().0, &11);
        assert_eq!(observer.snapshots.get(&50), Some(&snapshot));
    }
}
