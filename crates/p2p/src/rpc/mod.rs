pub mod codec;
pub mod handler;
pub mod methods;
pub mod outbound;
pub mod protocol;

use self::{
    handler::{Error, HandlerEvent, OutboundInfo, RPCHandler, RequestContainer, ResponseContainer},
    methods::{RPCResponse, RequestId},
    outbound::OutboundRequest,
    protocol::InboundRequest,
};
use futures::channel::oneshot::Sender;
use libp2p::{
    swarm::{
        dial_opts::DialOpts, ConnectionClosed, ConnectionDenied, ConnectionId, DialFailure,
        FromSwarm, NetworkBehaviour, NotifyHandler, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    },
    PeerId,
};
use silius_primitives::constants::p2p::RESP_TIMEOUT;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{atomic::AtomicU64, Arc},
    task::Poll,
    time::Duration,
};

/// The event of the request-response protocol.
#[derive(Debug)]
pub enum RPCEvent {
    Request {
        peer_id: PeerId,
        request: InboundRequest,
        req_id: RequestId,
        sender: Sender<RPCResponse>,
    },
    Response {
        peer_id: PeerId,
        req_id: RequestId,
        response: RPCResponse,
    },
    InboundFailure {
        peer_id: PeerId,
        req_id: RequestId,
        err: InboundFailure,
    },
    OutboundFailure {
        peer_id: PeerId,
        req_id: RequestId,
        err: OutboundFailure,
    },
    ResponseSent {
        peer_id: PeerId,
        req_id: RequestId,
    },
    UpgradeFailure {
        peer_id: PeerId,
        req_id: RequestId,
    },
}

#[derive(Debug)]
pub enum InboundFailure {
    /// The inbound request timed out, either while reading the
    /// incoming request or before a response is sent, e.g. if
    /// [`libp2p::request_response::Behaviour`] is not called in a
    /// timely manner.
    Timeout,
    /// The connection closed before a response could be send.
    ConnectionClosed,
    /// The local peer supports none of the protocols requested
    /// by the remote.
    UnsupportedProtocols,
    /// The local peer failed to respond to an inbound request
    /// due to the [`libp2p::request_response::ResponseChannel`] being dropped instead of
    /// being passed to [`libp2p::request_response::Behaviour::send_response`].
    ResponseOmission,
    /// Error happended while handling the inbound
    Error(Error),
}

#[derive(Debug)]
pub enum OutboundFailure {
    /// A dialing attempt failed.
    DialFailure,
    /// The request timed out before a response was received.
    ///
    /// It is not known whether the request may have been
    /// received (and processed) by the remote peer.
    Timeout,
    /// The connection closed before a response was received.
    ///
    /// It is not known whether the request may have been
    /// received (and processed) by the remote peer.
    ConnectionClosed,
    /// The remote supports none of the requested protocols.
    UnsupportedProtocols,
    /// Error happended while handling the outbound
    Error(Error),
}

/// A connection with inbound and outbound request id.
struct Connection {
    id: ConnectionId,
    /// Pending inbound responses for previously sent requests on this
    /// connection.
    pending_inbound_responses: HashSet<RequestId>,
    /// Pending outbound responses where corresponding inbound requests have
    /// been received on this connection and emitted via `poll` but have not yet
    /// been answered.
    pending_outbound_responses: HashSet<RequestId>,
}

impl Connection {
    fn new(id: ConnectionId) -> Self {
        Self {
            id,
            pending_inbound_responses: Default::default(),
            pending_outbound_responses: Default::default(),
        }
    }
}

pub struct RPC {
    /// Timeout for response.
    resp_timeout: Duration,
    /// The next (local) request ID.
    next_request_id: RequestId,
    /// The next (inbound) request ID.
    next_inbound_id: Arc<AtomicU64>,
    /// The next (outbound) request ID
    /// pending events to return from `Poll`
    pending_events: VecDeque<ToSwarm<RPCEvent, OutboundInfo>>,
    /// The set of connected peers and their connections.
    connected: HashMap<PeerId, Vec<Connection>>,
    /// The set of pending outbound requests for each peer.
    pending_outbound_requests: HashMap<PeerId, Vec<OutboundInfo>>,
}

impl Default for RPC {
    fn default() -> Self {
        Self::new()
    }
}

impl RPC {
    pub fn new() -> Self {
        Self {
            resp_timeout: Duration::from_secs(RESP_TIMEOUT),
            next_request_id: RequestId(1),
            next_inbound_id: Arc::new(AtomicU64::new(1)),
            pending_events: VecDeque::new(),
            connected: HashMap::new(),
            pending_outbound_requests: HashMap::new(),
        }
    }

    /// Returns the next request ID.
    fn next_request_id(&mut self) -> RequestId {
        let request_id = self.next_request_id;
        self.next_request_id.0 += 1;
        request_id
    }

    /// Try to send a request to the given peer.
    fn try_send_request(&mut self, peer: &PeerId, request: OutboundInfo) -> Option<OutboundInfo> {
        if let Some(connections) = self.connected.get_mut(peer) {
            if connections.is_empty() {
                return Some(request);
            }
            let id = (request.req_id.0 as usize) % connections.len();
            let conn = &mut connections[id];
            conn.pending_inbound_responses.insert(request.req_id);
            self.pending_events.push_back(ToSwarm::NotifyHandler {
                peer_id: *peer,
                handler: NotifyHandler::One(conn.id),
                event: request,
            });
            None
        } else {
            Some(request)
        }
    }

    /// Send a request to the given peer.
    pub fn send_request(&mut self, peer: &PeerId, request: OutboundRequest) -> RequestId {
        let req_id = self.next_request_id();
        let request = OutboundInfo { req_id, request };

        if let Some(request) = self.try_send_request(peer, request) {
            self.pending_events.push_back(ToSwarm::Dial { opts: DialOpts::peer_id(*peer).build() });
            self.pending_outbound_requests.entry(*peer).or_default().push(request);
        }

        req_id
    }

    /// Send a response to the given peer.
    pub fn send_response(
        &mut self,
        sender: Sender<RPCResponse>,
        response: RPCResponse,
    ) -> Result<(), RPCResponse> {
        sender.send(response)
    }

    /// Returns a mutable reference to the connection in `self.connected`
    /// corresponding to the given [`PeerId`] and [`ConnectionId`].
    fn get_connection_mut(
        &mut self,
        peer: &PeerId,
        connection: ConnectionId,
    ) -> Option<&mut Connection> {
        self.connected
            .get_mut(peer)
            .and_then(|connections| connections.iter_mut().find(|c| c.id == connection))
    }

    /// Remove pending outbound response for the given peer and connection.
    ///
    /// Returns `true` if the provided connection to the given peer is still
    /// alive and the [`RequestId`] was previously present and is now removed.
    /// Returns `false` otherwise.
    fn remove_pending_outbound_response(
        &mut self,
        peer: &PeerId,
        connection: ConnectionId,
        request: RequestId,
    ) -> bool {
        self.get_connection_mut(peer, connection)
            .map(|c| c.pending_outbound_responses.remove(&request))
            .unwrap_or(false)
    }

    /// Remove pending inbound response for the given peer and connection.
    ///
    /// Returns `true` if the provided connection to the given peer is still
    /// alive and the [`RequestId`] was previously present and is now removed.
    /// Returns `false` otherwise.
    fn remove_pending_inbound_response(
        &mut self,
        peer: &PeerId,
        connection: ConnectionId,
        request: &RequestId,
    ) -> bool {
        self.get_connection_mut(peer, connection)
            .map(|c| c.pending_inbound_responses.remove(request))
            .unwrap_or(false)
    }

    fn on_connection_closed(
        &mut self,
        ConnectionClosed { peer_id, connection_id, remaining_established, .. }: ConnectionClosed,
    ) {
        let connections = self
            .connected
            .get_mut(&peer_id)
            .expect("Expected some established connection to peer before closing.");

        let connection = connections
            .iter()
            .position(|c| c.id == connection_id)
            .map(|p: usize| connections.remove(p))
            .expect("Expected connection to be established before closing.");

        debug_assert_eq!(connections.is_empty(), remaining_established == 0);
        if connections.is_empty() {
            self.connected.remove(&peer_id);
        }

        for req_id in connection.pending_outbound_responses {
            self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::InboundFailure {
                peer_id,
                req_id,
                err: InboundFailure::ConnectionClosed,
            }));
        }

        for req_id in connection.pending_inbound_responses {
            self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::OutboundFailure {
                peer_id,
                req_id,
                err: OutboundFailure::ConnectionClosed,
            }));
        }
    }

    fn on_dial_failure(&mut self, DialFailure { peer_id, .. }: DialFailure) {
        if let Some(peer_id) = peer_id {
            // If there are pending outgoing requests when a dial failure occurs,
            // it is implied that we are not connected to the peer, since pending
            // outgoing requests are drained when a connection is established and
            // only created when a peer is not connected when a request is made.
            // Thus these requests must be considered failed, even if there is
            // another, concurrent dialing attempt ongoing.
            if let Some(pending) = self.pending_outbound_requests.remove(&peer_id) {
                for request in pending {
                    self.pending_events.push_back(ToSwarm::GenerateEvent(
                        RPCEvent::OutboundFailure {
                            peer_id,
                            req_id: request.req_id,
                            err: OutboundFailure::DialFailure,
                        },
                    ));
                }
            }
        }
    }
}

impl NetworkBehaviour for RPC {
    type ConnectionHandler = RPCHandler;
    type ToSwarm = RPCEvent;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        _peer: libp2p::PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(RPCHandler::new(self.next_inbound_id.clone(), connection_id, self.resp_timeout))
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        _peer: libp2p::PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(RPCHandler::new(self.next_inbound_id.clone(), connection_id, self.resp_timeout))
    }

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        Ok(())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _maybe_peer: Option<libp2p::PeerId>,
        _addresses: &[libp2p::Multiaddr],
        _effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<libp2p::Multiaddr>, ConnectionDenied> {
        Ok(vec![])
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match event {
            HandlerEvent::Request(RequestContainer { req_id, request, sender }) => {
                let message = RPCEvent::Request { req_id, peer_id, request, sender };
                self.pending_events.push_back(ToSwarm::GenerateEvent(message));

                match self.get_connection_mut(&peer_id, connection_id) {
                    Some(connection) => {
                        connection.pending_outbound_responses.insert(req_id);
                    }
                    // Connection closed after `Event::Request` has been emitted.
                    None => {
                        self.pending_events.push_back(ToSwarm::GenerateEvent(
                            RPCEvent::InboundFailure {
                                peer_id,
                                req_id,
                                err: InboundFailure::ConnectionClosed,
                            },
                        ));
                    }
                }
            }
            HandlerEvent::Response(ResponseContainer { req_id, response }) => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &req_id);

                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::Response {
                    peer_id,
                    req_id,
                    response,
                }));
            }
            HandlerEvent::InboundTimeout(req_id) => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &req_id);
                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::InboundFailure {
                    peer_id,
                    req_id,
                    err: InboundFailure::Timeout,
                }))
            }
            HandlerEvent::InboundError { req_id, err } => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &req_id);
                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::InboundFailure {
                    peer_id,
                    req_id,
                    err: InboundFailure::Error(err),
                }))
            }
            HandlerEvent::OutboundError { req_id, err } => {
                self.remove_pending_outbound_response(&peer_id, connection_id, req_id);
                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::OutboundFailure {
                    peer_id,
                    req_id,
                    err: OutboundFailure::Error(err),
                }))
            }
            HandlerEvent::DialUpgradeTimeout(_) => {}
            HandlerEvent::ResponseSent(req_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, req_id);

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(RPCEvent::ResponseSent { peer_id, req_id }));
            }
            HandlerEvent::ResponseOmission(req_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, req_id);

                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::InboundFailure {
                    peer_id,
                    req_id,
                    err: InboundFailure::ResponseOmission,
                }));
            }
            HandlerEvent::OutboundTimeout(req_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, req_id);

                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::OutboundFailure {
                    peer_id,
                    req_id,
                    err: OutboundFailure::Timeout,
                }));
            }
            HandlerEvent::OutboundUnsuportedProtocol(req_id) => {
                let removed =
                    self.remove_pending_inbound_response(&peer_id, connection_id, &req_id);
                debug_assert!(removed, "Expect req_id to be pending before failing to connect.",);

                self.pending_events.push_back(ToSwarm::GenerateEvent(RPCEvent::OutboundFailure {
                    peer_id,
                    req_id,
                    err: OutboundFailure::UnsupportedProtocols,
                }));
            }
        }
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        match event {
            FromSwarm::ConnectionEstablished(connection_established) => {
                self.connected
                    .entry(connection_established.peer_id)
                    .or_default()
                    .push(Connection::new(connection_established.connection_id));

                if connection_established.other_established == 0 {
                    if let Some(pending) =
                        self.pending_outbound_requests.remove(&connection_established.peer_id)
                    {
                        for request in pending {
                            let request =
                                self.try_send_request(&connection_established.peer_id, request);
                            assert!(request.is_none());
                        }
                    }
                }
            }
            FromSwarm::ConnectionClosed(connection_closed) => {
                self.on_connection_closed(connection_closed)
            }
            FromSwarm::DialFailure(dial_failure) => self.on_dial_failure(dial_failure),
            _ => {}
        }
    }

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Some(ev) = self.pending_events.pop_front() {
            return Poll::Ready(ev);
        }
        Poll::Pending
    }
}
