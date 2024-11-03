use super::{
    methods::{RPCResponse, RequestId},
    outbound::{OutboundProtocolUpgrade, OutboundRequest},
    protocol::{InboundProtocolUpgrade, InboundRequest},
};
use crate::rpc::codec::ssz_snappy::{SSZSnappyInboundCodec, SSZSnappyOutboundCodec};
use futures::{
    channel::oneshot::{self, Receiver, Sender},
    future::BoxFuture,
    stream::FuturesUnordered,
    AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt, TryFutureExt,
};
use libp2p::swarm::{
    handler::{ConnectionEvent, FullyNegotiatedInbound, FullyNegotiatedOutbound},
    ConnectionHandler, ConnectionHandlerEvent, ConnectionId, StreamUpgradeError, SubstreamProtocol,
};
use silius_primitives::constants::p2p::{REQUEST_SIZE_MAXIMUM, RESPONSE_SIZE_MAXIMUM};
use std::{
    collections::VecDeque,
    io,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};
use tokio_util::{
    bytes::BytesMut,
    codec::{Decoder, Encoder},
};
use tracing::{trace, warn};

/// Information about the inbound connection.
pub struct InboundInfo {
    /// ID of the request.
    req_id: RequestId,
    /// Sender of the request.
    sender: Sender<InboundRequest>,
    /// Receiver of the response.
    receiver: Receiver<RPCResponse>,
}

/// Information about the outbound connection.
#[derive(Debug, Clone)]
pub struct OutboundInfo {
    /// ID of the request.
    pub req_id: RequestId,
    /// Request.
    pub request: OutboundRequest,
}

/// Request container.
#[derive(Debug)]
pub struct RequestContainer {
    pub req_id: RequestId,
    pub request: InboundRequest,
    pub sender: Sender<RPCResponse>,
}

/// Response container.
#[derive(Debug)]
pub struct ResponseContainer {
    pub req_id: RequestId,
    pub response: RPCResponse,
}

/// Events from the handler.
#[derive(Debug)]
pub enum HandlerEvent {
    Request(RequestContainer),
    Response(ResponseContainer),
    ResponseSent(RequestId),
    ResponseOmission(RequestId),
    InboundTimeout(RequestId),
    OutboundTimeout(RequestId),
    InboundError { req_id: RequestId, err: Error },
    OutboundError { req_id: RequestId, err: Error },
    DialUpgradeTimeout(RequestId),
    OutboundUnsuportedProtocol(RequestId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BoundTypeId {
    Inbound(RequestId),
    Outbound(RequestId),
}

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    SSZError(snap::Error),
    DeserializeError(ssz_rs::DeserializeError),
    SerializeError(ssz_rs::SerializeError),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<snap::Error> for Error {
    fn from(value: snap::Error) -> Self {
        Error::SSZError(value)
    }
}

impl From<ssz_rs::DeserializeError> for Error {
    fn from(value: ssz_rs::DeserializeError) -> Self {
        Error::DeserializeError(value)
    }
}

impl From<ssz_rs::SerializeError> for Error {
    fn from(value: ssz_rs::SerializeError) -> Self {
        Error::SerializeError(value)
    }
}

/// A handler for inbound and outbound substreams.
pub struct RPCHandler {
    /// The connection id the handler holds.
    _connection_id: ConnectionId,
    /// The ID of the next inbound request.
    inbound_req_id: Arc<AtomicU64>,
    /// The timeout for inbound and outbound substreams (i.e. request
    /// and response processing).
    substream_timeout: Duration,
    /// Queue of events to emit in `poll()`.
    pending_events: VecDeque<HandlerEvent>,
    /// Outbound upgrades waiting to be emitted as an `OutboundSubstreamRequest`.
    outbound: VecDeque<OutboundInfo>,
    /// Inbound upgrades waiting for the incoming request.
    inbound: FuturesUnordered<BoxFuture<'static, Result<RequestContainer, oneshot::Canceled>>>,
    /// Worker streams.
    worker_streams: futures_bounded::FuturesMap<BoundTypeId, Result<HandlerEvent, Error>>,
}

impl RPCHandler {
    pub fn new(
        inbound_req_id: Arc<AtomicU64>,
        connection_id: ConnectionId,
        substream_timeout: Duration,
    ) -> Self {
        Self {
            _connection_id: connection_id,
            inbound_req_id,
            substream_timeout,
            pending_events: VecDeque::new(),
            outbound: VecDeque::new(),
            inbound: FuturesUnordered::new(),
            worker_streams: futures_bounded::FuturesMap::new(substream_timeout, 100),
        }
    }

    /// Handle a fully negotiated inbound substream.
    fn on_fully_negotiated_inbound(
        &mut self,
        FullyNegotiatedInbound { protocol, info }: FullyNegotiatedInbound<
            <Self as ConnectionHandler>::InboundProtocol,
            <Self as ConnectionHandler>::InboundOpenInfo,
        >,
    ) {
        let (mut socket, protocol_id) = protocol;
        let InboundInfo { req_id, sender, receiver } = info;
        let recv = async move {
            let mut data = Vec::new();
            let socket = &mut socket;
            socket.take(REQUEST_SIZE_MAXIMUM).read_to_end(&mut data).await?;

            trace!("Received {:?} bytes", data.len());

            let mut bytes = BytesMut::new();
            bytes.extend_from_slice(&data);

            let mut codec = SSZSnappyInboundCodec::new(protocol_id);
            let request = codec.decode(&mut bytes)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Failed to decode request")
            })?;

            match sender.send(request) {
                Ok(()) => {}
                // It should not happen. There must be something wrong with the codes.
                Err(_) => {
                    panic!(
                        "Expect request receiver to be alive, i.e. protocol handler to be alive.",
                    )
                }
            }

            if let Ok(response) = receiver.await {
                bytes.clear();
                codec.encode(response, &mut bytes)?;

                socket.write_all(&bytes).await?;
                socket.close().await?;

                // Response was sent. Indicate to handler to emit a `ResponseSent` event.
                Ok(HandlerEvent::ResponseSent(req_id))
            } else {
                socket.close().await?;

                Ok(HandlerEvent::ResponseOmission(req_id))
            }
        };

        if self.worker_streams.try_push(BoundTypeId::Inbound(req_id), recv.boxed()).is_err() {
            warn!("Dropping inbound stream because we are at capacity")
        }
    }

    /// Handle a fully negotiated outbound substream.
    fn on_fully_negotiated_outbound(
        &mut self,
        FullyNegotiatedOutbound { protocol, info }: FullyNegotiatedOutbound<
            <Self as ConnectionHandler>::OutboundProtocol,
            <Self as ConnectionHandler>::OutboundOpenInfo,
        >,
    ) {
        let (mut socket, protocol_id) = protocol;
        let OutboundInfo { req_id, request } = info;
        let send = async move {
            let mut bytes = BytesMut::new();

            let mut codec = SSZSnappyOutboundCodec::new(protocol_id);
            codec.encode(request, &mut bytes)?;

            socket.write_all(&bytes).await?;
            socket.close().await?;

            let mut data = Vec::new();
            socket.take(RESPONSE_SIZE_MAXIMUM).read_to_end(&mut data).await?;

            bytes.clear();
            bytes.extend_from_slice(&data);

            trace!("Received {:?} bytes", bytes.len());

            let response = codec.decode(&mut bytes)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Failed to decode response")
            })?;

            Ok(HandlerEvent::Response(ResponseContainer { req_id, response }))
        };

        if self.worker_streams.try_push(BoundTypeId::Outbound(req_id), send.boxed()).is_err() {
            warn!("Dropping inbound stream because we are at capacity")
        }
    }
}

impl ConnectionHandler for RPCHandler {
    type FromBehaviour = OutboundInfo;
    type ToBehaviour = HandlerEvent;
    type InboundOpenInfo = InboundInfo;
    type InboundProtocol = InboundProtocolUpgrade;
    type OutboundOpenInfo = OutboundInfo;
    type OutboundProtocol = OutboundProtocolUpgrade;

    fn connection_keep_alive(&self) -> bool {
        true
    }

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        // A channel for notifying the handler when the inbound
        // upgrade received the request.
        let (req_sender, req_receiver) = oneshot::channel();

        // A channel for notifying the inbound upgrade when the
        // response is sent.
        let (resp_sender, resp_receiver) = oneshot::channel();

        let req_id = RequestId(self.inbound_req_id.fetch_add(1, Ordering::Relaxed));

        let inbound_info = InboundInfo { req_id, sender: req_sender, receiver: resp_receiver };

        self.inbound.push(
            req_receiver
                .map_ok(move |req| RequestContainer { req_id, request: req, sender: resp_sender })
                .boxed(),
        );

        SubstreamProtocol::new(InboundProtocolUpgrade, inbound_info)
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
            ConnectionEvent::FullyNegotiatedInbound(fully_negotiated_inbound) => {
                self.on_fully_negotiated_inbound(fully_negotiated_inbound)
            }
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                self.on_fully_negotiated_outbound(fully_negotiated_outbound)
            }
            ConnectionEvent::DialUpgradeError(error) => match error.error {
                StreamUpgradeError::Timeout => self
                    .pending_events
                    .push_back(HandlerEvent::DialUpgradeTimeout(error.info.req_id)),
                StreamUpgradeError::NegotiationFailed => self
                    .pending_events
                    .push_back(HandlerEvent::OutboundUnsuportedProtocol(error.info.req_id)),
                dial_error => {
                    warn!("Outbound stream with {:?} failed with {dial_error:?}", error.info)
                }
            },
            _ => {}
        }
    }

    #[allow(deprecated)]
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>,
    > {
        match self.worker_streams.poll_unpin(cx) {
            Poll::Ready((_, Ok(Ok(event)))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
            }
            Poll::Ready((BoundTypeId::Inbound(id), Err(futures_bounded::Timeout { .. }))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::InboundTimeout(id),
                ));
            }
            Poll::Ready((BoundTypeId::Outbound(id), Err(futures_bounded::Timeout { .. }))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::OutboundTimeout(id),
                ));
            }
            Poll::Ready((BoundTypeId::Inbound(req_id), Ok(Err(err)))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::InboundError { req_id, err },
                ));
            }
            Poll::Ready((BoundTypeId::Outbound(req_id), Ok(Err(err)))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::OutboundError { req_id, err },
                ));
            }
            Poll::Pending => {}
        }

        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(event));
        };

        while let Poll::Ready(Some(result)) = self.inbound.poll_next_unpin(cx) {
            match result {
                Ok(RequestContainer { req_id, request, sender }) => {
                    return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(
                        HandlerEvent::Request(RequestContainer { req_id, request, sender }),
                    ));
                }
                Err(oneshot::Canceled) => {}
            }
        }

        if let Some(request) = self.outbound.pop_front() {
            return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(
                    OutboundProtocolUpgrade(request.clone().request),
                    request,
                )
                .with_timeout(self.substream_timeout),
            });
        };

        Poll::Pending
    }
}
