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
pub enum BehaviourEvent {
    /// Gossipsub protocol event
    GossipSub(Box<gossipsub::Event>),
    /// Request-response protocol event
    RPC(rpc::RPCEvent),
    /// Discovery protocol event
    Discovery(discovery::DiscoveredPeers),
    /// Peer manager event
    PeerManager(PeerManagerEvent),
}

impl From<gossipsub::Event> for BehaviourEvent {
    fn from(value: gossipsub::Event) -> Self {
        BehaviourEvent::GossipSub(Box::new(value))
    }
}

impl From<rpc::RPCEvent> for BehaviourEvent {
    fn from(value: rpc::RPCEvent) -> Self {
        BehaviourEvent::RPC(value)
    }
}

impl From<discovery::DiscoveredPeers> for BehaviourEvent {
    fn from(value: discovery::DiscoveredPeers) -> Self {
        BehaviourEvent::Discovery(value)
    }
}

impl From<PeerManagerEvent> for BehaviourEvent {
    fn from(value: PeerManagerEvent) -> Self {
        BehaviourEvent::PeerManager(value)
    }
}

/// The behaviour of the p2p network.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviourEvent", event_process = false)]
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
