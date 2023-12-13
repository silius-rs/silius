use crate::config::Config;
use crate::discovery::{self, Discovery};
use crate::gossipsub::{create_gossisub, Gossipsub};
use crate::peer_manager::{PeerManager, PeerManagerEvent};
use crate::request_response;
use discv5::enr::CombinedKey;
use discv5::Enr;
use libp2p::gossipsub;
use libp2p::swarm::NetworkBehaviour;
use std::time::Duration;

/// The behaviour of the p2p network.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event", event_process = false)]
pub struct Behaviour {
    /// Gossipsub protocol
    pub gossipsub: Gossipsub,
    /// Request/Response protocol
    pub reqrep: request_response::Behaviour,
    /// Discovery protocol
    pub discv5: Discovery,
    /// Peer manager
    pub peer_manager: PeerManager,
}

impl Behaviour {
    pub fn new(
        enr: Enr,
        key: CombinedKey,
        config: Config,
        p2p_mempool_id: Vec<String>,
        ping_interval: Duration,
        target_peers: usize,
    ) -> eyre::Result<Self> {
        let gossipsub = create_gossisub(p2p_mempool_id).map_err(|e| eyre::anyhow!(e))?;
        let reqrep = request_response::Behaviour::new(Default::default());
        let discovery = Discovery::new(enr, key, config)?;
        let peer_manager = PeerManager::new(ping_interval, target_peers);

        Ok(Self {
            gossipsub,
            reqrep,
            discv5: discovery,
            peer_manager,
        })
    }
}

impl From<gossipsub::Event> for Event {
    fn from(value: gossipsub::Event) -> Self {
        Event::GossipSub(Box::new(value))
    }
}

impl From<request_response::Event> for Event {
    fn from(value: request_response::Event) -> Self {
        Event::Reqrep(value)
    }
}

impl From<discovery::DiscoveredPeers> for Event {
    fn from(value: discovery::DiscoveredPeers) -> Self {
        Event::Discovery(value)
    }
}

impl From<PeerManagerEvent> for Event {
    fn from(value: PeerManagerEvent) -> Self {
        Event::PeerManager(value)
    }
}

/// Events emitted by the p2p network.
#[derive(Debug)]
pub enum Event {
    /// Gossipsub protocol event
    GossipSub(Box<gossipsub::Event>),
    /// Request/Response protocol event
    Reqrep(request_response::Event),
    /// Discovery protocol event
    Discovery(discovery::DiscoveredPeers),
    /// Peer manager event
    PeerManager(PeerManagerEvent),
}
