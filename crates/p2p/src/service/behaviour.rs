use crate::{
    config::Config,
    discovery::{self, Discovery},
    peer_manager::{PeerManager, PeerManagerEvent},
    rpc::{self, RPC},
    types::pubsub::{create_gossipsub, SnappyTransform},
};
use discv5::{enr::CombinedKey, Enr};
use libp2p::{
    gossipsub::{self, WhitelistSubscriptionFilter},
    swarm::NetworkBehaviour,
};

pub type Gossipsub = gossipsub::Behaviour<SnappyTransform, WhitelistSubscriptionFilter>;

/// Events emitted by the p2p network.
#[derive(Debug)]
pub enum Event {
    /// Gossipsub protocol event
    GossipSub(Box<gossipsub::Event>),
    /// Request-response protocol event
    RPC(rpc::RPCEvent),
    /// Discovery protocol event
    Discovery(discovery::DiscoveredPeers),
    /// Peer manager event
    PeerManager(PeerManagerEvent),
}

impl From<gossipsub::Event> for Event {
    fn from(value: gossipsub::Event) -> Self {
        Event::GossipSub(Box::new(value))
    }
}

impl From<rpc::RPCEvent> for Event {
    fn from(value: rpc::RPCEvent) -> Self {
        Event::RPC(value)
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

/// The behaviour of the p2p network.
#[derive(NetworkBehaviour)]
pub struct Behaviour {
    /// Hard limit of connections.
    pub connection_limits: libp2p::connection_limits::Behaviour,
    /// Peer manager
    pub peer_manager: PeerManager,
    /// Request/Response protocol
    pub rpc: RPC,
    /// Discovery protocol
    pub discovery: Discovery,
    /// Gossipsub protocol
    pub gossipsub: Gossipsub,
}

impl Behaviour {
    pub fn new(
        enr: Enr,
        key: CombinedKey,
        config: Config,
        mempool_ids: Vec<String>,
    ) -> eyre::Result<Self> {
        let gossipsub = create_gossipsub(mempool_ids).map_err(|e| eyre::anyhow!(e))?;
        let rpc = RPC::new();
        let discovery = Discovery::new(enr, key, config.clone())?;
        let peer_manager = PeerManager::new(config);

        Ok(Self { gossipsub, rpc, discovery, peer_manager })
    }
}
