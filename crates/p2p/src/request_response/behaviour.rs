use super::{
    handler::{Handler, HandlerEvent, OutboundInfo},
    models::{Request, RequestId, Response},
    BoundError,
};
use futures::channel::oneshot;
use libp2p::{
    swarm::{
        dial_opts::DialOpts, ConnectionClosed, ConnectionDenied, ConnectionId, DialFailure,
        FromSwarm, NetworkBehaviour, NotifyHandler, PollParameters, THandler, THandlerInEvent,
        THandlerOutEvent, ToSwarm,
    },
    PeerId,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{atomic::AtomicU64, Arc},
    task::Poll,
    time::Duration,
};

/// The event of the request-response protocol.
#[derive(Debug)]
pub enum Event {
    Request {
        peer_id: PeerId,
        request: Request,
        request_id: RequestId,
        response_sender: oneshot::Sender<Response>,
    },
    Response {
        peer_id: PeerId,
        request_id: RequestId,
        response: Response,
    },
    OutboundFailure {
        peer_id: PeerId,
        request_id: RequestId,
        error: OutboundFailure,
    },
    InboundFailure {
        peer_id: PeerId,
        request_id: RequestId,
        error: InboundFailure,
    },
    ResponseSent {
        peer_id: PeerId,
        request_id: RequestId,
    },
    UpgradeFailure {
        peer_id: PeerId,
        request_id: RequestId,
    },
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
    /// Error happended while handleing the outbound
    BoundError(BoundError),
}

#[derive(Debug)]
pub enum InboundFailure {
    /// The inbound request timed out, either while reading the
    /// incoming request or before a response is sent, e.g. if
    /// [`Behaviour::send_response`] is not called in a
    /// timely manner.
    Timeout,
    /// The connection closed before a response could be send.
    ConnectionClosed,
    /// The local peer supports none of the protocols requested
    /// by the remote.
    UnsupportedProtocols,
    /// The local peer failed to respond to an inbound request
    /// due to the [`ResponseChannel`] being dropped instead of
    /// being passed to [`Behaviour::send_response`].
    ResponseOmission,
    /// Error happended while handleing the inbound
    BoundError(BoundError),
}
#[derive(Debug)]
pub struct Config {
    pub request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
        }
    }
}
pub struct Behaviour {
    /// Configuration for the request-response protocol.
    config: Config,
    /// The next (local) request ID.
    next_request_id: RequestId,
    /// The next (inbound) request ID.
    next_inbound_id: Arc<AtomicU64>,
    /// The next (outbound) request ID
    /// pending events to return from `Poll`
    pending_events: VecDeque<ToSwarm<Event, OutboundInfo>>,
    /// The set of connected peers and their connections.
    connected: HashMap<PeerId, Vec<Connection>>,
    /// The set of pending outbound requests for each peer.
    pending_outbound_requests: HashMap<PeerId, Vec<OutboundInfo>>,
}

impl Behaviour {
    pub fn new(config: Config) -> Self {
        Self {
            config,
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

    /// Send a request to the given peer.
    pub fn send_request(&mut self, peer: &PeerId, request: Request) -> RequestId {
        let request_id = self.next_request_id();
        let request = OutboundInfo {
            request,
            request_id,
        };

        if let Some(request) = self.try_send_request(peer, request) {
            self.pending_events.push_back(ToSwarm::Dial {
                opts: DialOpts::peer_id(*peer).build(),
            });
            self.pending_outbound_requests
                .entry(*peer)
                .or_default()
                .push(request);
        }

        request_id
    }

    /// Send a response to the given peer.
    pub fn send_response(
        &mut self,
        sender: oneshot::Sender<Response>,
        response: Response,
    ) -> Result<(), Response> {
        sender.send(response)
    }

    /// Try to send a request to the given peer.
    fn try_send_request(&mut self, peer: &PeerId, request: OutboundInfo) -> Option<OutboundInfo> {
        if let Some(connections) = self.connected.get_mut(peer) {
            if connections.is_empty() {
                return Some(request);
            }
            let id = (request.request_id.0 as usize) % connections.len();
            let conn = &mut connections[id];
            conn.pending_inbound_responses.insert(request.request_id);
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

    fn on_connection_closed(
        &mut self,
        ConnectionClosed {
            peer_id,
            connection_id,
            remaining_established,
            ..
        }: ConnectionClosed<<Self as NetworkBehaviour>::ConnectionHandler>,
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

        for request_id in connection.pending_outbound_responses {
            self.pending_events
                .push_back(ToSwarm::GenerateEvent(Event::InboundFailure {
                    peer_id,
                    request_id,
                    error: InboundFailure::ConnectionClosed,
                }));
        }

        for request_id in connection.pending_inbound_responses {
            self.pending_events
                .push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
                    peer_id,
                    request_id,
                    error: OutboundFailure::ConnectionClosed,
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
                    self.pending_events
                        .push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
                            peer_id,
                            request_id: request.request_id,
                            error: OutboundFailure::DialFailure,
                        }));
                }
            }
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = Handler;
    type ToSwarm = Event;
    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        _peer: libp2p::PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(Handler::new(
            connection_id,
            self.config.request_timeout,
            self.next_inbound_id.clone(),
        ))
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        _peer: libp2p::PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(Handler::new(
            connection_id,
            self.config.request_timeout,
            self.next_inbound_id.clone(),
        ))
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
            HandlerEvent::Request {
                request,
                request_id,
                response_sender,
            } => {
                let message = Event::Request {
                    peer_id,
                    request_id,
                    request,
                    response_sender,
                };
                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(message));

                match self.get_connection_mut(&peer_id, connection_id) {
                    Some(connection) => {
                        connection.pending_outbound_responses.insert(request_id);
                    }
                    // Connection closed after `Event::Request` has been emitted.
                    None => {
                        self.pending_events.push_back(ToSwarm::GenerateEvent(
                            Event::InboundFailure {
                                peer_id,
                                request_id,
                                error: InboundFailure::ConnectionClosed,
                            },
                        ));
                    }
                }
            }
            HandlerEvent::Response {
                response,
                request_id,
            } => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &request_id);

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::Response {
                        peer_id,
                        request_id,
                        response,
                    }));
            }
            HandlerEvent::InboundTimeout(request_id) => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &request_id);
                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::InboundFailure {
                        peer_id,
                        request_id,
                        error: InboundFailure::Timeout,
                    }))
            }
            HandlerEvent::InboundError { request_id, error } => {
                self.remove_pending_inbound_response(&peer_id, connection_id, &request_id);
                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::InboundFailure {
                        peer_id,
                        request_id,
                        error: InboundFailure::BoundError(error),
                    }))
            }
            HandlerEvent::OutboundError { request_id, error } => {
                self.remove_pending_outbound_response(&peer_id, connection_id, request_id);
                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
                        peer_id,
                        request_id,
                        error: OutboundFailure::BoundError(error),
                    }))
            }
            HandlerEvent::DialUpgradeTimeout(_) => {}
            HandlerEvent::ResponseSent(request_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, request_id);

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::ResponseSent {
                        peer_id,
                        request_id,
                    }));
            }
            HandlerEvent::ResponseOmission(request_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, request_id);

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::InboundFailure {
                        peer_id,
                        request_id,
                        error: InboundFailure::ResponseOmission,
                    }));
            }
            HandlerEvent::OutboundTimeout(request_id) => {
                self.remove_pending_outbound_response(&peer_id, connection_id, request_id);

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
                        peer_id,
                        request_id,
                        error: OutboundFailure::Timeout,
                    }));
            }
            HandlerEvent::OutboundUnsurpportedProtocol(request_id) => {
                let removed =
                    self.remove_pending_inbound_response(&peer_id, connection_id, &request_id);
                debug_assert!(
                    removed,
                    "Expect request_id to be pending before failing to connect.",
                );

                self.pending_events
                    .push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
                        peer_id,
                        request_id,
                        error: OutboundFailure::UnsupportedProtocols,
                    }));
            }
        }
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(connection_established) => {
                self.connected
                    .entry(connection_established.peer_id)
                    .or_default()
                    .push(Connection::new(connection_established.connection_id));

                if connection_established.other_established == 0 {
                    if let Some(pending) = self
                        .pending_outbound_requests
                        .remove(&connection_established.peer_id)
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
        _params: &mut impl PollParameters,
    ) -> std::task::Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Some(ev) = self.pending_events.pop_front() {
            return Poll::Ready(ev);
        }
        Poll::Pending
    }
}

/// A connection with inbound and outbound request id.
struct Connection {
    id: ConnectionId,
    /// Pending outbound responses where corresponding inbound requests have
    /// been received on this connection and emitted via `poll` but have not yet
    /// been answered.
    pending_outbound_responses: HashSet<RequestId>,
    /// Pending inbound responses for previously sent requests on this
    /// connection.
    pending_inbound_responses: HashSet<RequestId>,
}

impl Connection {
    fn new(id: ConnectionId) -> Self {
        Self {
            id,
            pending_outbound_responses: Default::default(),
            pending_inbound_responses: Default::default(),
        }
    }
}
