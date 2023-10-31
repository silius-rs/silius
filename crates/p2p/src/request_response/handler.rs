use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};

use futures::{
    channel::oneshot, future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt,
    TryFutureExt,
};
use libp2p::swarm::{
    handler::{ConnectionEvent, FullyNegotiatedInbound, FullyNegotiatedOutbound},
    ConnectionHandler, ConnectionId, KeepAlive, StreamUpgradeError, SubstreamProtocol,
};
use tracing::warn;

use super::{
    inbound::InboundContainer,
    models::{Request, RequestId, Response},
    outbound::OutboundContainer,
};
#[derive(Debug)]
pub enum HandlerEvent {
    Request {
        request: Request,
        request_id: RequestId,
        response_sender: oneshot::Sender<Response>,
    },
    Response {
        response: Response,
        request_id: RequestId,
    },
    ResponseSent(RequestId),
    ResponseOmission(RequestId),
    OutboundTimeout(RequestId),
    OutboundUnsurpportedProtocol(RequestId),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

struct RequestContainer {
    request: Request,
    request_id: RequestId,
    response_sender: oneshot::Sender<Response>,
}

/// A handler for inbound and outbound substreams.
pub struct Handler {
    inbound_request_id: Arc<AtomicU64>,
    /// The connection id the handler holds.
    _connection_id: ConnectionId,
    /// The timeout for inbound and outbound substreams (i.e. request
    /// and response processing).
    substream_timeout: Duration,
    /// Queue of events to emit in `poll()`.
    pending_events: VecDeque<HandlerEvent>,
    /// Outbound upgrades waiting to be emitted as an `OutboundSubstreamRequest`.
    outbound: VecDeque<OutboundContainer>,
    /// Inbound upgrades waiting for the incoming request.
    inbound: FuturesUnordered<BoxFuture<'static, Result<RequestContainer, oneshot::Canceled>>>,
}

impl Handler {
    pub fn new(
        connection_id: ConnectionId,
        substream_timeout: Duration,
        inbound_request_id: Arc<AtomicU64>,
    ) -> Self {
        Self {
            _connection_id: connection_id,
            substream_timeout,
            pending_events: VecDeque::new(),
            outbound: VecDeque::new(),
            inbound: FuturesUnordered::new(),
            inbound_request_id,
        }
    }
}

impl ConnectionHandler for Handler {
    type Error = Error;
    type FromBehaviour = OutboundContainer;
    type ToBehaviour = HandlerEvent;
    type InboundOpenInfo = RequestId;
    type InboundProtocol = InboundContainer;
    type OutboundOpenInfo = RequestId;
    type OutboundProtocol = OutboundContainer;
    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        KeepAlive::Yes
    }
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        // A channel for notifying the handler when the inbound
        // upgrade received the request.
        let (rq_send, rq_recv) = oneshot::channel();

        // A channel for notifying the inbound upgrade when the
        // response is sent.
        let (rs_send, rs_recv) = oneshot::channel();

        let request_id = RequestId(self.inbound_request_id.fetch_add(1, Ordering::Relaxed));

        let protocol = InboundContainer {
            request_sender: rq_send,
            response_receiver: rs_recv,
        };

        self.inbound.push(
            rq_recv
                .map_ok(move |rq| RequestContainer {
                    request: rq,
                    request_id,
                    response_sender: rs_send,
                })
                .boxed(),
        );

        SubstreamProtocol::new(protocol, request_id)
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        self.outbound.push_back(event)
    }

    fn on_connection_event(
        &mut self,
        event: libp2p::swarm::handler::ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound { protocol, info }) => {
                if protocol {
                    self.pending_events
                        .push_back(HandlerEvent::ResponseSent(info))
                } else {
                    self.pending_events
                        .push_back(HandlerEvent::ResponseOmission(info))
                }
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol,
                info,
            }) => self.pending_events.push_back(HandlerEvent::Response {
                response: protocol,
                request_id: info,
            }),
            ConnectionEvent::DialUpgradeError(error) => match error.error {
                StreamUpgradeError::Timeout => self
                    .pending_events
                    .push_back(HandlerEvent::OutboundTimeout(error.info)),
                StreamUpgradeError::NegotiationFailed => self
                    .pending_events
                    .push_back(HandlerEvent::OutboundUnsurpportedProtocol(error.info)),
                dial_error => warn!(
                    "outbount stream with {:?} failed with {dial_error:?}",
                    error.info
                ),
            },
            ConnectionEvent::ListenUpgradeError(_)
            | ConnectionEvent::LocalProtocolsChange(_)
            | ConnectionEvent::RemoteProtocolsChange(_)
            | ConnectionEvent::AddressChange(_) => {}
        }
    }

    #[allow(deprecated)]
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
    > {
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(
                event,
            ));
        };

        while let Poll::Ready(Some(result)) = self.inbound.poll_next_unpin(cx) {
            match result {
                Ok(RequestContainer {
                    request,
                    request_id,
                    response_sender,
                }) => {
                    return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(
                        HandlerEvent::Request {
                            request,
                            response_sender,
                            request_id,
                        },
                    ));
                }
                Err(oneshot::Canceled) => {}
            }
        }

        if let Some(request) = self.outbound.pop_front() {
            let request_id = request.request_id;
            return Poll::Ready(
                libp2p::swarm::ConnectionHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(request, request_id)
                        .with_timeout(self.substream_timeout),
                },
            );
        };

        Poll::Pending
    }
}
