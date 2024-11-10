use super::{ConnectingType, PeerManager, PeerManagerEvent};
use crate::discovery::enr_ext::EnrExt;
use futures::StreamExt;
use libp2p::{
    core::ConnectedPoint,
    swarm::{
        behaviour::ConnectionEstablished,
        dial_opts::{DialOpts, PeerCondition},
        dummy::ConnectionHandler,
        ConnectionClosed, ConnectionDenied, DialFailure, FromSwarm, NetworkBehaviour, ToSwarm,
    },
    PeerId,
};
use std::{net::IpAddr, task::Poll};
use tracing::error;

impl NetworkBehaviour for PeerManager {
    type ConnectionHandler = ConnectionHandler;
    type ToSwarm = PeerManagerEvent;

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        match event {
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                endpoint,
                other_established,
                ..
            }) => self.on_connection_established(peer_id, endpoint, other_established),
            FromSwarm::ConnectionClosed(ConnectionClosed {
                peer_id,
                endpoint,
                remaining_established,
                ..
            }) => self.on_connection_closed(peer_id, endpoint, remaining_established),
            FromSwarm::DialFailure(DialFailure { peer_id, error: _, connection_id: _ }) => {
                self.on_dial_failure(peer_id)
            }
            _ => {}
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
        peer_id: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        // check if whitelist exists and if the peer is in the whitelist
        if !self.peers_whitelist.is_empty() &&
            self.peers_whitelist.iter().filter(|enr| enr.peer_id() == peer_id).count() == 0
        {
            return Err(libp2p::swarm::ConnectionDenied::new("Peer not in the whitelist"));
        }

        Ok(ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer_id: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        // check if whitelist exists and if the peer is in the whitelist
        if !self.peers_whitelist.is_empty() &&
            self.peers_whitelist.iter().filter(|enr| enr.peer_id() == peer_id).count() == 0
        {
            return Err(libp2p::swarm::ConnectionDenied::new("Peer not in the whitelist"));
        }

        Ok(ConnectionHandler)
    }

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: &libp2p::Multiaddr,
        remote_addr: &libp2p::Multiaddr,
    ) -> Result<(), libp2p::swarm::ConnectionDenied> {
        // get the IP address to verify it's whitelisted
        let ip = match remote_addr.iter().next() {
            Some(libp2p::multiaddr::Protocol::Ip6(ip)) => IpAddr::V6(ip),
            Some(libp2p::multiaddr::Protocol::Ip4(ip)) => IpAddr::V4(ip),
            _ => {
                return Err(ConnectionDenied::new(format!(
                    "Connection to peer rejected: invalid multiaddr: {remote_addr}"
                )))
            }
        };

        // check if whitelist exists and if the IP is in the whitelist
        if !self.ips_whitelist.is_empty() &&
            self.ips_whitelist.iter().filter(|&&whitelist_ip| whitelist_ip == ip).count() == 0
        {
            return Err(ConnectionDenied::new(format!(
                "Connection to peer rejected: IP {ip} not in the whitelist"
            )));
        }

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
        while self.heartbeat.poll_tick(cx).is_ready() {
            self.heartbeat();
        }

        loop {
            match self.inbound_ping_peers.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(peer_id))) => {
                    self.inbound_ping_peers.insert(peer_id);
                    self.events.push_back(PeerManagerEvent::Ping(peer_id));
                }
                Poll::Ready(Some(Err(e))) => {
                    error!("Failed to check inbound ping peer with {e:?}");
                }
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        loop {
            match self.outbound_ping_peers.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(peer_id))) => {
                    self.outbound_ping_peers.insert(peer_id);
                    self.events.push_back(PeerManagerEvent::Ping(peer_id));
                }
                Poll::Ready(Some(Err(e))) => {
                    error!("Failed to check inbound ping peer with {e:?}");
                }
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        if !self.events.is_empty() {
            if let Some(event) = self.events.pop_front() {
                return Poll::Ready(ToSwarm::GenerateEvent(event));
            }
        }

        if let Some(enr) = self.peers_to_dial.pop() {
            let peer_id = enr.peer_id();
            self.inject_peer_connection(&peer_id, ConnectingType::Dialing, Some(enr.clone()));
            return Poll::Ready(ToSwarm::Dial {
                opts: DialOpts::peer_id(peer_id)
                    .condition(PeerCondition::Disconnected)
                    .addresses(enr.multiaddr())
                    .build(),
            });
        }

        Poll::Pending
    }
}

impl PeerManager {
    fn on_connection_established(
        &mut self,
        peer_id: PeerId,
        endpoint: &ConnectedPoint,
        other_established: usize,
    ) {
        if other_established == 0 {
            self.events.push_back(PeerManagerEvent::MetaData(peer_id));
        }

        match endpoint {
            ConnectedPoint::Listener { send_back_addr, .. } => {
                self.inject_connect_ingoing(&peer_id, send_back_addr.clone(), None);
                self.events.push_back(PeerManagerEvent::PeerConnectedIncoming(peer_id));
            }
            ConnectedPoint::Dialer { address, .. } => {
                self.inject_connect_outgoing(&peer_id, address.clone(), None);
                self.events.push_back(PeerManagerEvent::PeerConnectedOutgoing(peer_id));
            }
        }
    }

    fn on_connection_closed(
        &mut self,
        peer_id: PeerId,
        _endpoint: &ConnectedPoint,
        remaining_established: usize,
    ) {
        if remaining_established > 0 {
            return;
        }

        if self.network_globals.peers.read().is_connected_or_disconnecting(&peer_id) {
            self.events.push_back(PeerManagerEvent::PeerDisconnected(peer_id));
        }

        self.inject_disconnect(&peer_id);
    }

    fn on_dial_failure(&mut self, peer_id: Option<PeerId>) {
        if let Some(peer_id) = peer_id {
            if !self.network_globals.peers.read().is_connected(&peer_id) {
                self.inject_disconnect(&peer_id);
            }
        }
    }
}
