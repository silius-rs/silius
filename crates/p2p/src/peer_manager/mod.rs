pub mod peerdb;

use self::peerdb::PeerDB;
use delay_map::HashSetDelay;
use futures::StreamExt;
use libp2p::{
    swarm::{dummy::ConnectionHandler, NetworkBehaviour, ToSwarm},
    PeerId,
};
use std::{collections::VecDeque, task::Poll, time::Duration};
use tracing::error;

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
    pub fn new(ping_interval: Duration, target_peers: usize) -> Self {
        Self {
            ping_peers: HashSetDelay::new(ping_interval),
            _target_peers: target_peers,
            peer_db: Default::default(),
            events: Default::default(),
        }
    }
}

impl NetworkBehaviour for PeerManager {
    type ConnectionHandler = ConnectionHandler;
    type ToSwarm = PeerManagerEvent;
    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        if let libp2p::swarm::FromSwarm::ConnectionClosed(close_info) = event {
            self.peer_db.disconnect(close_info.peer_id);
            self.events.push_back(PeerManagerEvent::PeerDisconnected(close_info.peer_id));
        }
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        _event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.peer_db.new_connected(peer);
        self.ping_peers.insert(peer);
        self.events.push_back(PeerManagerEvent::PeerConnectedIncoming(peer));
        Ok(ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.peer_db.new_connected(peer);
        self.ping_peers.insert(peer);
        self.events.push_back(PeerManagerEvent::PeerConnectedOutgoing(peer));
        Ok(ConnectionHandler)
    }

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<(), libp2p::swarm::ConnectionDenied> {
        Ok(())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _maybe_peer: Option<PeerId>,
        _addresses: &[libp2p::Multiaddr],
        _effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<libp2p::Multiaddr>, libp2p::swarm::ConnectionDenied> {
        Ok(vec![])
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>> {
        loop {
            match self.ping_peers.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(peer))) => {
                    self.events.push_back(PeerManagerEvent::Ping(peer));
                    self.ping_peers.insert(peer);
                }
                Poll::Ready(Some(Err(e))) => {
                    error!("Failed to check ping peer with {e:?}")
                }
                _ => break,
            };
        }

        match self.events.pop_front() {
            Some(event) => Poll::Ready(ToSwarm::GenerateEvent(event)),
            _ => Poll::Pending,
        }
    }
}
