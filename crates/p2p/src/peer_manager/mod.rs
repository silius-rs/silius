pub mod network_behaviour;
pub mod peer;
pub mod peerdb;

use self::peer::peer_info::ConnectionDirection;
use crate::{
    discovery::enr_ext::EnrExt,
    rpc::methods::{GoodbyeReason, MetaData},
    types::globals::NetworkGlobals,
};
use delay_map::HashSetDelay;
use discv5::Enr;
use libp2p::{Multiaddr, PeerId};
use silius_primitives::constants::p2p::{
    HEARTBEAT_INTERVAL, PING_INTERVAL_INBOUND, PING_INTERVAL_OUTBOUND, TARGET_PEERS,
};
use std::{collections::VecDeque, net::IpAddr, sync::Arc, time::Duration};
use tracing::debug;

/// The events that the `PeerManager` outputs (requests).
#[derive(Debug)]
pub enum PeerManagerEvent {
    /// A peer has dialed us.
    PeerConnectedIncoming(PeerId),
    /// A peer has been dialed.
    PeerConnectedOutgoing(PeerId),
    /// A peer has disconnected.
    PeerDisconnected(PeerId),
    /// Sends a ping to a peer.
    Ping(PeerId),
    /// Request metadata fro a peer.
    MetaData(PeerId),
    /// Request the behaviour to discover more peers and the amount of peers to discover.
    DiscoverPeers(usize),
    /// Discconnecting from peer.
    DisconnectPeer(PeerId, GoodbyeReason),
}

enum ConnectingType {
    Dialing,
    IngoingConnected { multiaddr: Multiaddr },
    OutgoingConnected { multiaddr: Multiaddr },
}

pub struct PeerManager {
    /// Accessing `PeerDB` through  network globals.
    network_globals: Arc<NetworkGlobals>,
    /// Events that the `PeerManager` outputs.
    events: VecDeque<PeerManagerEvent>,
    /// List of inbound peers we need to ping.
    inbound_ping_peers: HashSetDelay<PeerId>,
    /// List of outbound peers we need to ping.
    outbound_ping_peers: HashSetDelay<PeerId>,
    /// the target peers we want to connect,
    target_peers: usize,
    /// Peers needs to be dialed.
    peers_to_dial: Vec<Enr>,
    /// The list of whitelisted ENRs.
    peers_whitelist: Vec<Enr>,
    /// The list of whitelisted IPs.
    ips_whitelist: Vec<IpAddr>,
    /// The heartbeat interval for peer management.
    heartbeat: tokio::time::Interval,
}

impl PeerManager {
    pub fn new(
        network_globals: Arc<NetworkGlobals>,
        peers_whitelist: Vec<Enr>,
        ips_whitelist: Vec<IpAddr>,
    ) -> Self {
        Self {
            network_globals,
            events: Default::default(),
            inbound_ping_peers: HashSetDelay::new(Duration::from_secs(PING_INTERVAL_INBOUND)),
            outbound_ping_peers: HashSetDelay::new(Duration::from_secs(PING_INTERVAL_OUTBOUND)),
            target_peers: TARGET_PEERS,
            peers_to_dial: Vec::new(),
            peers_whitelist,
            ips_whitelist,
            heartbeat: tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL)),
        }
    }

    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.network_globals.peers.read().is_connected(peer_id)
    }

    pub fn dial_peer(&mut self, enr: Enr) -> bool {
        if self.network_globals.peers.read().should_dial(&enr.peer_id()) {
            self.peers_to_dial.push(enr);
            true
        } else {
            false
        }
    }

    pub fn maintain_peer_count(&mut self, dialing_peers: usize) {
        if self.network_globals.connected_or_dialing_peers() <
            self.target_peers.saturating_sub(dialing_peers) &&
            dialing_peers != 0
        {
            self.events.push_back(PeerManagerEvent::DiscoverPeers(dialing_peers));
        }
    }

    pub fn peers_discovered(&mut self, results: Vec<Enr>) {
        let mut to_dial_peers = 0;

        for enr in results {
            if !self.peers_to_dial.contains(&enr) {
                let peer_id = enr.peer_id();
                if self.dial_peer(enr) {
                    debug! {"Dialing discovered peer: {}", peer_id};
                    to_dial_peers += 1;
                }
            }
        }

        self.maintain_peer_count(to_dial_peers);
    }

    /// Ping request has been received.
    pub fn ping_request(&mut self, peer_id: &PeerId, seq: u64) {
        if let Some(peer_info) = self.network_globals.peers.read().peer_info(peer_id) {
            match peer_info.connection_direction() {
                Some(ConnectionDirection::Incoming) => {
                    self.inbound_ping_peers.insert(*peer_id);
                }
                Some(ConnectionDirection::Outgoing) => {
                    self.outbound_ping_peers.insert(*peer_id);
                }
                None => {}
            }

            if let Some(metadata) = &peer_info.metadata() {
                if metadata.seq_number < seq {
                    self.events.push_back(PeerManagerEvent::MetaData(*peer_id));
                }
            } else {
                self.events.push_back(PeerManagerEvent::MetaData(*peer_id));
            }
        }
    }

    /// The peer has responded with a pong.
    pub fn pong_response(&mut self, peer_id: &PeerId, seq: u64) {
        if let Some(peer_info) = self.network_globals.peers.read().peer_info(peer_id) {
            if let Some(metadata) = &peer_info.metadata() {
                if metadata.seq_number < seq {
                    self.events.push_back(PeerManagerEvent::MetaData(*peer_id));
                }
            } else {
                self.events.push_back(PeerManagerEvent::MetaData(*peer_id));
            }
        }
    }

    /// The peer has responded with metadata.
    pub fn metadata_response(&mut self, peer_id: &PeerId, metadata: MetaData) {
        if let Some(peer_info) = self.network_globals.peers.write().peer_info_mut(peer_id) {
            peer_info.set_metadata(metadata);
        }
    }

    fn inject_connect_ingoing(
        &mut self,
        peer_id: &PeerId,
        multiaddr: Multiaddr,
        enr: Option<Enr>,
    ) -> bool {
        self.inject_peer_connection(peer_id, ConnectingType::IngoingConnected { multiaddr }, enr)
    }

    fn inject_connect_outgoing(
        &mut self,
        peer_id: &PeerId,
        multiaddr: Multiaddr,
        enr: Option<Enr>,
    ) -> bool {
        self.inject_peer_connection(peer_id, ConnectingType::OutgoingConnected { multiaddr }, enr)
    }

    fn inject_disconnect(&mut self, peer_id: &PeerId) {
        self.network_globals.peers.write().inject_disconnect(peer_id);

        self.inbound_ping_peers.remove(peer_id);
        self.outbound_ping_peers.remove(peer_id);
    }

    fn inject_peer_connection(
        &mut self,
        peer_id: &PeerId,
        connection: ConnectingType,
        enr: Option<Enr>,
    ) -> bool {
        let mut peer_db = self.network_globals.peers.write();

        match connection {
            ConnectingType::Dialing => {
                peer_db.dialing_peer(peer_id, enr);
                return true;
            }
            ConnectingType::IngoingConnected { multiaddr } => {
                peer_db.connect_ingoing(peer_id, multiaddr, enr);
                self.inbound_ping_peers.insert(*peer_id);
            }
            ConnectingType::OutgoingConnected { multiaddr } => {
                peer_db.connect_outgoing(peer_id, multiaddr, enr);
                self.outbound_ping_peers.insert(*peer_id);
            }
        }

        true
    }

    fn _disconnect_peer(&mut self, peer_id: PeerId, reason: GoodbyeReason) {
        self.events.push_back(PeerManagerEvent::DisconnectPeer(peer_id, reason));
        self.network_globals.peers.write().notify_disconnecting(&peer_id);
    }

    fn heartbeat(&mut self) {
        // TODO: optionally run discovery
    }
}
