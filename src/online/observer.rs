//! Opt-in read-only evidence for the ordinary frontend. Never drives the lobby or game.
use super::*;

pub(super) mod ui;
pub(super) mod world;

#[derive(Resource)]
pub(super) struct OnlineObserver {
    pub(super) enabled: bool,
    snapshots: BTreeMap<u64, OnlineWorldSnapshot>,
    applied_worlds: BTreeMap<u64, serde_json::Value>,
    last_report: String,
    last_context: serde_json::Value,
    dirty: bool,
    ui_events: VecDeque<serde_json::Value>,
    last_ui_state: serde_json::Value,
}

impl Default for OnlineObserver {
    fn default() -> Self {
        Self {
            enabled: requested(),
            snapshots: BTreeMap::new(),
            applied_worlds: BTreeMap::new(),
            last_report: String::new(),
            last_context: serde_json::Value::Null,
            dirty: false,
            ui_events: VecDeque::new(),
            last_ui_state: serde_json::Value::Null,
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

pub(super) fn reset_for_match(mut observer: ResMut<OnlineObserver>) {
    observer.snapshots.clear();
    observer.applied_worlds.clear();
    observer.last_report.clear();
    observer.last_context = serde_json::Value::Null;
    observer.dirty = true;
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
        "ui_events": observer.ui_events,
        "applied_worlds": observer.applied_worlds.values().collect::<Vec<_>>(),
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
    use bevy::ecs::system::RunSystemOnce;

    fn actors(world: &mut World) -> Vec<(u64, [f32; 3], [f32; 4], [f32; 3], f32, f32)> {
        let mut query = world.query::<(Entity, &Transform, &Health)>();
        let mut result: Vec<_> = query
            .iter(world)
            .map(|(entity, transform, health)| {
                (
                    entity.to_bits(),
                    transform.translation.to_array(),
                    transform.rotation.to_array(),
                    transform.scale.to_array(),
                    health.current,
                    health.max,
                )
            })
            .collect();
        result.sort_by_key(|a| a.0);
        result
    }

    #[test]
    fn enabled_ui_observer_preserves_actual_match_actors_economy_and_commands() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..8 {
            app.update();
        }
        let before = actors(app.world_mut());
        assert!(!before.is_empty(), "actual normal match actors required");
        let bank = {
            let economies = app.world().resource::<Economies>();
            (
                economies.get(Team::Player(0)).ore,
                economies.get(Team::Player(0)).crystal,
            )
        };
        app.world_mut().resource_mut::<OnlineObserver>().enabled = true;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut().run_system_once(ui::record).unwrap();
        assert_eq!(actors(app.world_mut()), before);
        let economies = app.world().resource::<Economies>();
        assert_eq!(
            (
                economies.get(Team::Player(0)).ore,
                economies.get(Team::Player(0)).crystal
            ),
            bank
        );
        assert!(
            app.world()
                .resource::<OnlineCommandOutbox>()
                .pending
                .is_empty()
        );
        assert!(app.world().resource::<OnlineSession>().room.is_none());
        assert!(
            app.world()
                .resource::<OnlineVerificationHarness>()
                .config
                .is_none()
        );
        assert_eq!(app.world().resource::<OnlineObserver>().ui_events.len(), 1);
    }

    #[test]
    fn ui_observer_is_event_driven_and_keeps_only_forty_actual_input_records() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.world_mut().run_system_once(ui::record).unwrap();
        assert!(
            app.world()
                .resource::<OnlineObserver>()
                .ui_events
                .is_empty()
        );
        app.world_mut().resource_mut::<OnlineObserver>().enabled = true;
        app.world_mut().run_system_once(ui::record).unwrap();
        let initial = app.world().resource::<OnlineObserver>().ui_events.len();
        for _ in 0..10 {
            app.world_mut().run_system_once(ui::record).unwrap();
        }
        assert_eq!(
            app.world().resource::<OnlineObserver>().ui_events.len(),
            initial
        );
        for index in 0..50 {
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Right);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(if index % 2 == 0 {
                    MouseButton::Left
                } else {
                    MouseButton::Right
                });
            app.world_mut().run_system_once(ui::record).unwrap();
        }
        let observer = app.world().resource::<OnlineObserver>();
        assert_eq!(observer.ui_events.len(), 40);
        assert_eq!(
            observer
                .ui_events
                .iter()
                .filter(|e| e["right_pressed"] == true)
                .count(),
            20
        );
        assert_eq!(
            observer
                .ui_events
                .iter()
                .filter(|e| e["left_pressed"] == true)
                .count(),
            20
        );
        assert!(
            observer
                .ui_events
                .iter()
                .all(|event| event["left_pressed"] == true || event["right_pressed"] == true)
        );
    }
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
