//! Rebuild failed transport without resetting the established match or identity.
use super::*;

pub(super) fn note_transport_stopped(
    transport: &OnlineTransport,
    session: &OnlineSession,
    generation: u64,
) -> bool {
    generation == transport.generation
        && transport.config.is_some()
        && session.phase != OnlinePhase::Home
}

pub(super) fn request_recovery(transport: &mut OnlineTransport) {
    if transport.config.is_some() && !transport.needs_rebuild {
        transport.recovering = true;
        transport.needs_rebuild = true;
        transport.retry_elapsed = 0.0;
    }
}

pub(super) fn recover_transport(
    time: Res<Time<Real>>,
    mut session: ResMut<OnlineSession>,
    mut transport: ResMut<OnlineTransport>,
    inbox: Res<OnlineAsyncInbox>,
) {
    if session.phase == OnlinePhase::Home
        || (!transport.needs_rebuild && (session.is_host || !transport.recovering))
    {
        return;
    }
    transport.retry_elapsed += time.delta_secs();
    if transport.retry_elapsed < 1.0 || transport.authority_pending {
        return;
    }
    transport.retry_elapsed = 0.0;
    if session.is_host {
        let Some(config) = transport.config.clone() else {
            return;
        };
        // The original host ticket authorizes the replacement host UUID.
        match install_online_transport(&mut transport, &inbox, config) {
            Ok(()) => {
                transport.needs_rebuild = false;
                transport.recovering = false;
            }
            Err(error) => session.set_status(error),
        }
    } else {
        let Some(room) = session.room.as_ref() else {
            return;
        };
        let code = room.room_code.clone();
        let generation = transport.generation;
        let Some((client, _)) = validated_online_request(&mut session) else {
            return;
        };
        transport.authority_pending = true;
        spawn_online_request(inbox.clone(), async move {
            let result = client.room(&code).await.map_err(client_error_text);
            OnlineAsyncResult::RecoveryAuthority { generation, result }
        });
    }
}

fn authoritative_host(session: &OnlineSession, room: &RoomDescriptor) -> Option<PeerId> {
    let original = session.room.as_ref()?;
    if room.game_id != original.game_id
        || room.game_protocol != original.game_protocol
        || room.session_protocol != original.session_protocol
        || room.room_code != original.room_code
        || room.build_id != original.build_id
        || !room.host_connected
    {
        return None;
    }
    Some(PeerId(room.host_peer_id.as_ref()?.parse().ok()?))
}

pub(super) fn accept_authority(
    session: &mut OnlineSession,
    transport: &mut OnlineTransport,
    inbox: &OnlineAsyncInbox,
    generation: u64,
    result: Result<RoomDescriptor, String>,
) {
    if generation != transport.generation
        || !transport.recovering
        || session.phase == OnlinePhase::Home
    {
        return;
    }
    transport.authority_pending = false;
    let room = match result {
        Ok(room) => room,
        Err(error) => {
            session.set_status(error);
            return;
        }
    };
    let Some(host) = authoritative_host(session, &room) else {
        session.set_status(t(
            "等待信令确认房主",
            "Waiting for signaling to confirm the host",
        ));
        return;
    };
    if transport.needs_rebuild {
        let Some(config) = transport.config.clone() else {
            return;
        };
        if let Err(error) = install_online_transport(transport, inbox, config) {
            session.set_status(error);
            return;
        }
        transport.needs_rebuild = false;
    }
    // Refresh the pin while awaiting Welcome, even if the first response
    // still advertised the disconnected host. Keep the new socket alive.
    session.host_peer = Some(host);
    session.room = Some(room);
    session.hello_retry_elapsed = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn peer(id: u64) -> PeerId {
        PeerId(format!("00000000-0000-0000-0000-{id:012}").parse().unwrap())
    }

    fn room(host: Option<PeerId>) -> RoomDescriptor {
        RoomDescriptor {
            game_id: online_game_id(),
            build_id: online_build_id(),
            session_protocol: open_bevy_protocol::SESSION_PROTOCOL_VERSION,
            game_protocol: RTS_ONLINE_PROTOCOL,
            room_code: RoomCode::new("ABC12345").unwrap(),
            visibility: RoomVisibility::Public,
            max_peers: 8,
            peer_count: 2,
            host_connected: host.is_some(),
            host_peer_id: host.map(|peer| peer.0.to_string()),
            created_at_unix_ms: 1,
            expires_at_unix_ms: 100000,
            metadata: Default::default(),
        }
    }

    fn fixture() -> App {
        let mut app = build_game_app(GameAppMode::Headless);
        let descriptor = room(Some(peer(1)));
        let config = TransportConfig::player(
            "ws://127.0.0.1:1/v1/signal",
            &descriptor,
            PlayerName::new("Guest").unwrap(),
            online_build_id(),
            None,
            vec![],
        );
        let mut session = app.world_mut().resource_mut::<OnlineSession>();
        session.phase = OnlinePhase::InMatch;
        session.room = Some(descriptor);
        session.host_peer = Some(peer(1));
        session.local_player_id = Some(2);
        session.assigned_slot = Some(1);
        session.lobby = Some(OnlineLobbySnapshot::new("ABC12345".into(), "Host".into()));
        let mut transport = app.world_mut().resource_mut::<OnlineTransport>();
        transport.config = Some(config);
        transport.generation = 7;
        app.world_mut()
            .resource_mut::<OnlineMatchReplication>()
            .last_applied_tick = 44;
        app
    }

    #[test]
    fn transient_actual_lifecycle_preserves_world_identity_tick_and_explicit_leave_resets() {
        let mut app = fixture();
        let actor = app
            .world_mut()
            .spawn((
                MatchScopedEntity,
                Team::Player(1),
                Transform::from_translation(Vec3::new(3.0, 1.5, 4.0)),
                MoveOrder {
                    target: Vec3::new(8.0, 0.0, 9.0),
                },
            ))
            .id();
        let position = *app.world().get::<Transform>(actor).unwrap();
        let key = app.world().resource::<OnlineSession>().session_key.clone();
        app.world_mut()
            .resource_mut::<OnlineLifecycleControl>()
            .transport_stopped_reason = Some("Transient test transport loss".into());
        app.world_mut()
            .run_system_once(process_online_lifecycle)
            .unwrap();
        let session = app.world().resource::<OnlineSession>();
        assert_eq!(session.phase, OnlinePhase::InMatch);
        assert_eq!(session.session_key, key);
        assert_eq!(session.local_player_id, Some(2));
        assert_eq!(session.assigned_slot, Some(1));
        assert!(session.room.is_some() && session.lobby.is_some());
        assert_eq!(
            app.world()
                .resource::<OnlineMatchReplication>()
                .last_applied_tick,
            44
        );
        assert_eq!(*app.world().get::<Transform>(actor).unwrap(), position);
        assert_eq!(
            app.world().get::<MoveOrder>(actor).unwrap().target,
            Vec3::new(8.0, 0.0, 9.0)
        );
        let transport = app.world().resource::<OnlineTransport>();
        assert!(transport.recovering && transport.needs_rebuild);
        assert!(!note_transport_stopped(transport, session, 6));
        assert!(note_transport_stopped(transport, session, 7));
        app.world_mut()
            .resource_mut::<OnlineLifecycleControl>()
            .local_leave_session = true;
        app.world_mut()
            .run_system_once(process_online_lifecycle)
            .unwrap();
        let transport = app.world().resource::<OnlineTransport>();
        let session = app.world().resource::<OnlineSession>();
        assert_eq!(session.phase, OnlinePhase::Home);
        assert!(transport.config.is_none() && !transport.recovering && !transport.needs_rebuild);
        assert!(!note_transport_stopped(transport, session, 7));
    }

    #[test]
    fn authority_change_requires_trusted_same_room_and_foreign_mesh_welcome_stays_rejected() {
        let mut app = fixture();
        let valid = room(Some(peer(9)));
        assert_eq!(
            authoritative_host(app.world().resource::<OnlineSession>(), &valid),
            Some(peer(9))
        );
        let mut foreign = valid.clone();
        foreign.room_code = RoomCode::new("FOREIGN1").unwrap();
        assert!(authoritative_host(app.world().resource::<OnlineSession>(), &foreign).is_none());
        foreign = valid.clone();
        foreign.build_id = BuildId::new("foreign-build").unwrap();
        assert!(authoritative_host(app.world().resource::<OnlineSession>(), &foreign).is_none());
        assert!(authoritative_host(app.world().resource::<OnlineSession>(), &room(None)).is_none());
        let mut session = app.world_mut().remove_resource::<OnlineSession>().unwrap();
        let mut transport = app
            .world_mut()
            .remove_resource::<OnlineTransport>()
            .unwrap();
        transport.recovering = true;
        transport.needs_rebuild = false;
        let inbox = app.world().resource::<OnlineAsyncInbox>();
        assert!(session.hello_for_connected_peer(peer(8)).is_none());
        accept_authority(&mut session, &mut transport, inbox, 6, Ok(valid.clone()));
        assert_eq!(
            session.host_peer,
            Some(peer(1)),
            "stale request cannot replace host"
        );
        accept_authority(&mut session, &mut transport, inbox, 7, Ok(foreign));
        assert_eq!(
            session.host_peer,
            Some(peer(1)),
            "foreign room cannot replace host"
        );
        accept_authority(&mut session, &mut transport, inbox, 7, Ok(valid));
        assert_eq!(session.host_peer, Some(peer(9)));
        assert_eq!(
            transport.generation, 7,
            "authority refresh must not rebuild socket repeatedly"
        );
        assert!(session.hello_for_connected_peer(peer(8)).is_none());
        assert!(session.hello_for_connected_peer(peer(9)).is_some());
        assert_eq!(session.local_player_id, Some(2));
        assert_eq!(session.assigned_slot, Some(1));
    }

    #[test]
    fn resumed_guest_reuses_same_slot_without_new_admission_or_duplicate_player() {
        let mut lobby = OnlineLobbySnapshot::new("ABC12345".into(), "Host".into());
        let mut runtime = HostLobbyRuntime::new("host-key".into());
        let (id, slot) = runtime
            .admit(
                &mut lobby,
                peer(2),
                "guest-key".into(),
                "Guest".into(),
                true,
            )
            .unwrap();
        runtime.disconnect(&mut lobby, peer(2));
        let next_id = runtime.next_player_id;
        assert_eq!(
            runtime
                .admit(
                    &mut lobby,
                    peer(3),
                    "guest-key".into(),
                    "Guest".into(),
                    false
                )
                .unwrap(),
            (id, slot)
        );
        assert_eq!(runtime.next_player_id, next_id);
        assert_eq!(runtime.peer_players.len(), 1);
        assert_eq!(runtime.player_for_session("guest-key"), Some(id));
        assert!(
            runtime
                .admit(&mut lobby, peer(4), "new-key".into(), "Other".into(), false)
                .is_err()
        );
    }
}
