use crate::{
    discovery::{self, Discovery},
    peer_manager::{PeerManager, PeerManagerEvent},
    rpc::{self, RPC},
    types::pubsub::SnappyTransform,
};
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
#[behaviour(to_swarm = "Event", event_process = false)]
pub struct Behaviour {
    /// Peer manager
    pub peer_manager: PeerManager,
    /// Request/Response protocol
    pub rpc: RPC,
    /// Discovery protocol
    pub discovery: Discovery,
    /// Gossipsub protocol
    pub gossipsub: Gossipsub,
}
