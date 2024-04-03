use super::peer::peer_info::{ConnectionDirection, PeerConnectionStatus, PeerInfo};
use discv5::Enr;
use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use tracing::error;

#[derive(Default)]
pub struct PeerDB {
    peers: HashMap<PeerId, PeerInfo>,
}

impl PeerDB {
    pub fn new(trusted_peers: Vec<PeerId>) -> Self {
        let mut peers = HashMap::new();
        for peer_id in trusted_peers {
            peers.insert(peer_id, Default::default());
        }
        Self { peers }
    }

    pub fn peer_info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }

    pub fn peer_info_mut(&mut self, peer_id: &PeerId) -> Option<&mut PeerInfo> {
        self.peers.get_mut(peer_id)
    }

    pub fn connection_status(&self, peer_id: &PeerId) -> Option<PeerConnectionStatus> {
        self.peer_info(peer_id).map(|info| info.connection_status().clone())
    }

    pub fn is_connected_or_disconnecting(&self, peer_id: &PeerId) -> bool {
        matches!(
            self.connection_status(peer_id),
            Some(PeerConnectionStatus::Connected | PeerConnectionStatus::Disconnecting)
        )
    }

    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        matches!(self.connection_status(peer_id), Some(PeerConnectionStatus::Connected))
    }

    pub fn is_connected_or_dialing(&self, peer_id: &PeerId) -> bool {
        matches!(
            self.connection_status(peer_id),
            Some(PeerConnectionStatus::Connected | PeerConnectionStatus::Dialing)
        )
    }

    pub fn connected_or_dialing_peers(&self) -> impl Iterator<Item = &PeerId> {
        self.peers
            .iter()
            .filter(|(_, info)| info.is_connected() || info.is_dialing())
            .map(|(peer_id, _)| peer_id)
    }

    pub fn should_dial(&self, peer_id: &PeerId) -> bool {
        matches!(
            self.connection_status(peer_id),
            Some(PeerConnectionStatus::Disconnected | PeerConnectionStatus::Unknown) | None
        )
    }

    pub fn update_connection_state(&mut self, peer_id: &PeerId, new_state: NewConnectionState) {
        let info = self.peers.entry(*peer_id).or_default();

        match (info.connection_status().clone(), new_state) {
            (_current_state, NewConnectionState::Connected { enr, direction, seen_address }) => {
                info.set_enr(enr);

                match direction {
                    ConnectionDirection::Incoming => info.connect_ingoing(seen_address),
                    ConnectionDirection::Outgoing => info.connect_outgoing(seen_address),
                }
            }
            (_old_state, NewConnectionState::Dialing { enr }) => {
                info.set_enr(enr);

                if let Err(e) = info.set_dialing_peer() {
                    error!("Error dialing peer: {:?}", e);
                }
            }
            (_old_state, NewConnectionState::Disconnected) => {
                info.set_connection_status(PeerConnectionStatus::Disconnected)
            }
            (_old_state, NewConnectionState::Disconnecting) => {
                info.set_connection_status(PeerConnectionStatus::Disconnecting)
            }
        }
    }

    pub fn inject_disconnect(&mut self, peer_id: &PeerId) {
        self.update_connection_state(peer_id, NewConnectionState::Disconnected)
    }

    pub fn dialing_peer(&mut self, peer_id: &PeerId, enr: Option<Enr>) {
        self.update_connection_state(peer_id, NewConnectionState::Dialing { enr })
    }

    pub fn connect_ingoing(&mut self, peer_id: &PeerId, seen_address: Multiaddr, enr: Option<Enr>) {
        self.update_connection_state(
            peer_id,
            NewConnectionState::Connected {
                enr,
                direction: ConnectionDirection::Incoming,
                seen_address,
            },
        )
    }

    pub fn connect_outgoing(
        &mut self,
        peer_id: &PeerId,
        seen_address: Multiaddr,
        enr: Option<Enr>,
    ) {
        self.update_connection_state(
            peer_id,
            NewConnectionState::Connected {
                enr,
                direction: ConnectionDirection::Outgoing,
                seen_address,
            },
        )
    }

    pub fn notify_disconnecting(&mut self, peer_id: &PeerId) {
        self.update_connection_state(peer_id, NewConnectionState::Disconnecting)
    }
}

#[derive(Debug)]
pub enum NewConnectionState {
    Connected { enr: Option<Enr>, direction: ConnectionDirection, seen_address: Multiaddr },
    Disconnected,
    Disconnecting,
    Dialing { enr: Option<Enr> },
}
