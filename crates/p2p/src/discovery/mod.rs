pub mod enr;
pub mod enr_ext;

use self::enr_ext::{peer_id_to_node_id, EnrExt};
use crate::{config::Config, types::globals::NetworkGlobals};
use discv5::{
    enr::{CombinedKey, NodeId},
    Discv5, Enr, Event,
};
use futures::{stream::FuturesUnordered, Future, FutureExt, StreamExt, TryFutureExt};
use libp2p::{
    core::Endpoint,
    swarm::{
        dummy::ConnectionHandler, ConnectionDenied, ConnectionId, DialError, DialFailure,
        FromSwarm, NetworkBehaviour, THandler, THandlerOutEvent, ToSwarm,
    },
    Multiaddr, PeerId,
};
use lru::LruCache;
use std::{num::NonZeroUsize, pin::Pin, sync::Arc, task::Poll};
use tokio::sync::mpsc;
use tracing::{debug, warn};

type QueryResult = Result<Vec<Enr>, discv5::QueryError>;

#[derive(Debug)]
pub struct DiscoveredPeers {
    pub peers: Vec<Enr>,
}

pub enum EventStream {
    /// Awaiting an event stream to be generated. This is required due to the poll nature of
    /// `Discovery`
    Awaiting(Pin<Box<dyn Future<Output = Result<mpsc::Receiver<Event>, discv5::Error>> + Send>>),
    /// The future has completed.
    Present(mpsc::Receiver<Event>),
    // The future has failed or discv5 has been disabled. There are no events from discv5.
    InActive,
}

pub struct Discovery {
    /// Core discv5 service.
    discovery: Discv5,

    /// Network globals.
    _network_globals: Arc<NetworkGlobals>,

    /// Active discovery queries.
    active_queries: FuturesUnordered<Pin<Box<dyn Future<Output = QueryResult> + Send>>>,

    /// A cache of discovered ENRs.
    cached_enrs: LruCache<PeerId, Enr>,

    /// The event stream of discv5.
    event_stream: EventStream,
}

impl Discovery {
    pub async fn new(
        key: CombinedKey,
        config: Config,
        network_globals: Arc<NetworkGlobals>,
    ) -> eyre::Result<Self> {
        let enr = network_globals.local_enr();

        let mut discovery: Discv5<_> =
            Discv5::new(enr, key, config.discv5_config).map_err(|e| eyre::anyhow!(e))?;

        // adding bootnodes
        for bootnode in config.bootnodes {
            if bootnode.peer_id() == network_globals.peer_id() {
                continue;
            }

            let _ = discovery.add_enr(bootnode);
        }

        // start the discv5 service
        discovery.start().map_err(|e| eyre::format_err!(e.to_string())).await?;
        let event_stream = EventStream::Awaiting(Box::pin(discovery.event_stream()));

        Ok(Self {
            discovery,
            _network_globals: network_globals,
            active_queries: Default::default(),
            cached_enrs: LruCache::new(NonZeroUsize::new(50).expect("50 is a valid value")),
            event_stream,
        })
    }

    /// Return the nodes local ENR.
    pub fn local_enr(&self) -> Enr {
        self.discovery.local_enr()
    }

    /// Adds an ENR to the discovery.
    pub fn add_enr(&mut self, enr: Enr) -> eyre::Result<()> {
        self.discovery.add_enr(enr).map_err(|e| eyre::eyre!(e.to_string()))
    }

    /// Return cached ENRs.
    pub fn cached_enrs(&self) -> impl Iterator<Item = &Enr> {
        self.cached_enrs.iter().map(|(_, enr)| enr)
    }

    /// Remove cached ENR.
    pub fn remove_enr(&mut self, peer_id: &PeerId) {
        self.cached_enrs.pop(peer_id);
    }

    /// Discovers peers on the network.
    pub fn discover_peers(&mut self, target_peers: usize) {
        debug!("Starting a peer discovery request target_peers {target_peers:}");

        // Generate a random target node id.
        let random_node = NodeId::random();
        let predicate: Box<dyn Fn(&Enr) -> bool + Send> =
            Box::new(move |enr: &Enr| enr.tcp4().is_some() || enr.tcp6().is_some());

        // Build the future
        let query_future = self.discovery.find_node_predicate(random_node, predicate, target_peers);

        self.active_queries.push(Box::pin(query_future));
    }

    pub fn disconnect_peer(&mut self, peer_id: &PeerId) {
        if let Ok(node_id) = peer_id_to_node_id(peer_id) {
            self.discovery.disconnect_node(&node_id);
        }
        self.cached_enrs.pop(peer_id);
    }

    pub fn on_dial_failure(&mut self, peer_id: Option<PeerId>, error: &DialError) {
        if let Some(peer_id) = peer_id {
            match error {
                DialError::LocalPeerId { .. } |
                DialError::Denied { .. } |
                DialError::NoAddresses |
                DialError::Transport(_) |
                DialError::WrongPeerId { .. } => {
                    self.disconnect_peer(&peer_id);
                }
                DialError::DialPeerConditionFalse(_) | DialError::Aborted => {}
            }
        }
    }
}

impl NetworkBehaviour for Discovery {
    type ConnectionHandler = ConnectionHandler;
    type ToSwarm = DiscoveredPeers;

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<libp2p::swarm::ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>> {
        while let Poll::Ready(Some(query_result)) = self.active_queries.poll_next_unpin(cx) {
            match query_result {
                Ok(peers) => {
                    for enr in peers.iter() {
                        self.cached_enrs.put(enr.peer_id(), enr.clone());
                    }
                    return Poll::Ready(ToSwarm::GenerateEvent(DiscoveredPeers { peers }));
                }
                Err(e) => warn!("Discovery query failed: {:?}", e),
            }
        }

        match self.event_stream {
            EventStream::Awaiting(ref mut fut) => {
                if let Poll::Ready(event_stream) = fut.poll_unpin(cx) {
                    match event_stream {
                        Ok(stream) => self.event_stream = EventStream::Present(stream),
                        Err(err) => {
                            warn!("Discovery event stream failed: {:?}", err);
                            self.event_stream = EventStream::InActive;
                        }
                    }
                }
            }
            EventStream::Present(ref mut stream) => {
                while let Poll::Ready(Some(event)) = stream.poll_recv(cx) {
                    match event {
                        Event::Discovered(_) => {}
                        Event::EnrAdded { .. } |
                        Event::NodeInserted { .. } |
                        Event::SessionEstablished(_, _) |
                        Event::SocketUpdated(_) |
                        Event::TalkRequest(_) => {}
                    }
                }
            }
            EventStream::InActive => {}
        };

        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        if let FromSwarm::DialFailure(DialFailure { peer_id, error, .. }) = event {
            self.on_dial_failure(peer_id, error)
        }
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: PeerId,
        _connection_id: ConnectionId,
        _event: THandlerOutEvent<Self>,
    ) {
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(ConnectionHandler)
    }

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        Ok(())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _maybe_peer: Option<PeerId>,
        _addresses: &[Multiaddr],
        _effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        Ok(Vec::new())
    }
}
