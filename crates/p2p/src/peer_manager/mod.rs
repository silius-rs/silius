pub mod network_behaviour;
pub mod peer;
pub mod peerdb;

use crate::types::globals::NetworkGlobals;
use delay_map::HashSetDelay;
use libp2p::PeerId;
use silius_primitives::constants::p2p::PING_INTERVAL;
use std::{collections::VecDeque, sync::Arc, time::Duration};

/// The events that the `PeerManager` outputs (requests).
#[derive(Debug)]
pub enum PeerManagerEvent {
    /// A peer has dialed us.
    PeerConnectedIncoming(PeerId),
    /// A peer has been dialed.
    PeerConnectedOutgoing(PeerId),
    /// A peer has disconnected.
    PeerDisconnected(PeerId),
    /// Sends a PING to a peer.
    Ping(PeerId),
    /// Request the behaviour to discover more peers and the amount of peers to discover.
    DiscoverPeers(usize),
}

pub struct PeerManager {
    /// Accessing `PeerDB` through  network globals.
    network_globals: Arc<NetworkGlobals>,
    /// A list of peers that we need to ping.
    ping_peers: HashSetDelay<PeerId>,
    /// the target peers we want to connect,
    _target_peers: usize,
    /// Events that the `PeerManager` outputs.
    events: VecDeque<PeerManagerEvent>,
}

impl PeerManager {
    pub fn new(network_globals: Arc<NetworkGlobals>, target_peers: usize) -> Self {
        Self {
            network_globals,
            ping_peers: HashSetDelay::new(Duration::from_secs(PING_INTERVAL)),
            _target_peers: target_peers,
            events: Default::default(),
        }
    }
}
