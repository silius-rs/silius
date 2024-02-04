pub mod network_behaviour;
pub mod peer;
pub mod peerdb;

use self::peerdb::PeerDB;
use crate::config::Config;
use delay_map::HashSetDelay;
use libp2p::PeerId;
use silius_primitives::constants::p2p::PING_INTERVAL;
use std::{collections::VecDeque, time::Duration};

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
    /// A list of peers that we need to ping.
    ping_peers: HashSetDelay<PeerId>,
    /// the target peers we want to connect
    _target_peers: usize,
    /// Peer database
    peer_db: PeerDB,
    /// Events that the `PeerManager` outputs.
    events: VecDeque<PeerManagerEvent>,
}

impl PeerManager {
    pub fn new(config: Config) -> Self {
        Self {
            ping_peers: HashSetDelay::new(Duration::from_secs(PING_INTERVAL)),
            _target_peers: config.target_peers,
            peer_db: Default::default(),
            events: Default::default(),
        }
    }
}
