use crate::config::Config;
use discv5::{
    enr::{CombinedKey, NodeId},
    ConfigBuilder, Discv5, Enr, Event,
};
use futures::{stream::FuturesUnordered, Future, FutureExt, StreamExt};
use libp2p::swarm::{dummy::ConnectionHandler, NetworkBehaviour};
use std::{collections::HashSet, pin::Pin, task::Poll};
use tokio::sync::mpsc;
use tracing::{debug, warn};

type QueryResult = Result<Vec<Enr>, discv5::QueryError>;

pub struct Discovery {
    /// Core discv5 service.
    pub discovery: Discv5,

    /// Active discovery queries.
    active_queries: FuturesUnordered<Pin<Box<dyn Future<Output = QueryResult> + Send>>>,

    /// A cache of discovered ENRs.
    cached_enrs: HashSet<Enr>,

    /// The event stream of discv5.
    event_stream: EventStream,
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

impl Discovery {
    pub fn new(enr: Enr, key: CombinedKey, config: Config) -> eyre::Result<Self> {
        let config = ConfigBuilder::new(config.to_listen_config()).build();
        let discovery: Discv5<_> = Discv5::new(enr, key, config).map_err(|e| eyre::anyhow!(e))?;

        let event_stream_fut = discovery.event_stream().boxed();
        Ok(Self {
            discovery,
            active_queries: Default::default(),
            cached_enrs: HashSet::new(),
            event_stream: EventStream::Awaiting(event_stream_fut),
        })
    }

    /// Return the nodes local ENR.
    pub fn local_enr(&self) -> Enr {
        self.discovery.local_enr()
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
}

#[derive(Debug)]
pub struct DiscoveredPeers {
    pub peers: Vec<Enr>,
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
                Ok(enrs) => {
                    for enr in enrs.into_iter() {
                        self.cached_enrs.insert(enr);
                    }
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
    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        _event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer: libp2p::PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer: libp2p::PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
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
        _maybe_peer: Option<libp2p::PeerId>,
        _addresses: &[libp2p::Multiaddr],
        _effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<libp2p::Multiaddr>, libp2p::swarm::ConnectionDenied> {
        Ok(Vec::new())
    }
}
