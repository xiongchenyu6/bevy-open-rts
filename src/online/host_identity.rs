use super::*;

impl OnlineSession {
    pub(super) fn hello_for_connected_peer(
        &mut self,
        peer: PeerId,
    ) -> Option<OnlineReliableMessage> {
        // Mesh guests are not the authority established by our Welcome.
        if self.is_host || (self.local_player_id.is_some() && self.host_peer != Some(peer)) {
            return None;
        }
        self.host_peer = Some(peer);
        self.hello_retry_elapsed = 0.0;
        Some(OnlineReliableMessage::Hello {
            protocol: RTS_ONLINE_PROTOCOL,
            session_key: self.session_key.clone(),
            player_name: self.player_name.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcomed_guest_keeps_host_when_six_more_mesh_peers_connect() {
        let host = PeerId("00000000-0000-0000-0000-000000000001".parse().unwrap());
        let mut session = OnlineSession::default();
        let mut setup = MatchSetupSettings::default();
        let mut next = NextState::<AppScreen>::default();
        assert!(session.hello_for_connected_peer(host).is_some());
        let lobby = OnlineLobbySnapshot::new("ABC123".into(), "Host".into());
        accept_online_welcome(
            &mut session,
            host,
            2,
            1,
            lobby.clone(),
            None,
            &mut setup,
            &mut next,
        );
        for id in 3..=8 {
            let peer = PeerId(format!("00000000-0000-0000-0000-{id:012}").parse().unwrap());
            assert!(session.hello_for_connected_peer(peer).is_none());
            accept_online_welcome(
                &mut session,
                peer,
                id,
                0,
                lobby.clone(),
                None,
                &mut setup,
                &mut next,
            );
            assert_eq!(session.host_peer, Some(host));
            assert_eq!(session.local_player_id, Some(2));
            assert_eq!(session.assigned_slot, Some(1));
        }
        // A reconnect to the established authority still performs its handshake.
        assert!(session.hello_for_connected_peer(host).is_some());
        accept_online_welcome(&mut session, host, 2, 1, lobby, None, &mut setup, &mut next);
        assert_eq!(session.host_peer, Some(host));
        session.reset_connection();
        assert!(session.host_peer.is_none());
        assert!(session.local_player_id.is_none());
    }

    #[test]
    fn initial_guest_can_try_later_host_candidate_but_host_never_sends_hello() {
        let a = PeerId("00000000-0000-0000-0000-000000000001".parse().unwrap());
        let b = PeerId("00000000-0000-0000-0000-000000000002".parse().unwrap());
        let mut session = OnlineSession::default();
        assert!(session.hello_for_connected_peer(a).is_some());
        assert!(session.hello_for_connected_peer(b).is_some());
        assert_eq!(session.host_peer, Some(b));
        session.is_host = true;
        assert!(session.hello_for_connected_peer(a).is_none());
    }
}
