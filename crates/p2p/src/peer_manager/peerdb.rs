use std::collections::HashMap;

use libp2p::PeerId;

pub enum ConnectionStatus {
    Connected,
    Disconnected,
}
pub struct PeerInfo {
    connection_status: ConnectionStatus,
}

#[derive(Default)]
pub struct PeerDB {
    peers: HashMap<PeerId, PeerInfo>,
}

impl PeerDB {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn new_connected(&mut self, peer_id: PeerId) {
        self.peers
            .entry(peer_id)
            .and_modify(|info| info.connection_status = ConnectionStatus::Connected)
            .or_insert(PeerInfo {
                connection_status: ConnectionStatus::Connected,
            });
    }

    pub fn disconnect(&mut self, peer_id: PeerId) {
        self.peers
            .entry(peer_id)
            .and_modify(|info| info.connection_status = ConnectionStatus::Disconnected);
    }
}
