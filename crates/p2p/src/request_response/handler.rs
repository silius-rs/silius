use super::{
    models::{Request, RequestId, Response},
    upgrade::{InboundReqUpgrade, OutboundRepUpgrade},
};
use crate::{
    config::Metadata,
    request_response::{
        protocol::Protocol, BoundError, GetMetadata, GoodbyeReason, Ping, Pong, PooledUserOpHashes,
        PooledUserOpHashesReq, PooledUserOpsByHash, PooledUserOpsByHashReq, Status,
    },
};
use futures::{
    channel::oneshot::{self, Receiver, Sender},
    future::BoxFuture,
    stream::FuturesUnordered,
    AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt, TryFutureExt,
};
use libp2p::{
    bytes::BytesMut,
    swarm::{
        handler::{ConnectionEvent, FullyNegotiatedInbound, FullyNegotiatedOutbound},
        ConnectionHandler, ConnectionHandlerEvent, ConnectionId, StreamUpgradeError,
        SubstreamProtocol,
    },
};
use ssz_rs::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{trace, warn};
use unsigned_varint::codec::Uvi;

/// Max request size in bytes
const REQUEST_SIZE_MAXIMUM: u64 = 1024 * 1024;
/// Max response size in bytes
const RESPONSE_SIZE_MAXIMUM: u64 = 10 * 1024 * 1024;

pub struct InboundInfo {
    request_sender: Sender<Request>,
    response_receiver: Receiver<Response>,
    request_id: RequestId,
}

#[derive(Debug, Clone)]
pub struct OutboundInfo {
    pub request: Request,
    pub request_id: RequestId,
}

/// Events emitted by the handler.
#[derive(Debug)]
pub enum HandlerEvent {
    Request { request: Request, request_id: RequestId, response_sender: oneshot::Sender<Response> },
    Response { response: Response, request_id: RequestId },
    ResponseSent(RequestId),
    ResponseOmission(RequestId),
    InboundTimeout(RequestId),
    OutboundTimeout(RequestId),
    InboundError { request_id: RequestId, error: BoundError },
    OutboundError { request_id: RequestId, error: BoundError },
    DialUpgradeTimeout(RequestId),
    OutboundUnsurpportedProtocol(RequestId),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

struct RequestContainer {
    request: Request,
    request_id: RequestId,
    response_sender: oneshot::Sender<Response>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BoundTypeId {
    Inbound(RequestId),
    Outbound(RequestId),
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
    outbound: VecDeque<OutboundInfo>,
    /// Inbound upgrades waiting for the incoming request.
    inbound: FuturesUnordered<BoxFuture<'static, Result<RequestContainer, oneshot::Canceled>>>,
    worker_streams: futures_bounded::FuturesMap<BoundTypeId, Result<HandlerEvent, BoundError>>,
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
            worker_streams: futures_bounded::FuturesMap::new(substream_timeout, 100),
            inbound_request_id,
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
        let InboundInfo { request_sender, response_receiver, request_id } = info;
        let recv = async move {
            let mut data = Vec::new();
            let socket_mut = &mut socket;
            socket_mut.take(REQUEST_SIZE_MAXIMUM).read_to_end(&mut data).await?;

            trace!("Inbound bytes: {:?}", data);
            trace!("Received {:?} bytes", data.len());

            // MetaData request would send empty content.
            // https://github.com/eth-infinitism/bundler-spec/blob/main/p2p-specs/p2p-interface.md#getmetadata
            if data.is_empty() && !matches!(protocol_id.message_name, Protocol::MetaData) {
                Err(BoundError::IoError(io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "No data received",
                )))
            } else {
                let mut bytes = BytesMut::new();
                bytes.extend_from_slice(&data);

                // decode header
                let mut codec: Uvi<usize> = Uvi::default();
                let _ = codec.decode(&mut bytes)?; // TODO: use length to verify size

                // decode payload
                let mut buffer = vec![];
                snap::read::FrameDecoder::<&[u8]>::new(&bytes).read_to_end(&mut buffer)?;

                trace!("Inbound buffer {:?}", buffer);

                let request = match protocol_id.message_name {
                    Protocol::Status => Request::Status(Status::deserialize(&buffer)?),
                    Protocol::Goodbye => {
                        Request::GoodbyeReason(GoodbyeReason::deserialize(&buffer)?)
                    }
                    Protocol::Ping => Request::Ping(Ping::deserialize(&buffer)?),
                    Protocol::MetaData => Request::GetMetadata(GetMetadata::deserialize(&buffer)?),
                    Protocol::PooledUserOpHashes => {
                        Request::PooledUserOpHashesReq(PooledUserOpHashesReq::deserialize(&buffer)?)
                    }
                    Protocol::PooledUserOpsByHash => Request::PooledUserOpsByHashReq(
                        PooledUserOpsByHashReq::deserialize(&buffer)?,
                    ),
                };

                trace!("Inbound {:?}", request);

                match request_sender.send(request) {
                    Ok(()) => {}
                    // It should not happen. There must be something wrong with the codes.
                    Err(_) => panic!(
                        "Expect request receiver to be alive i.e. protocol handler to be alive.",
                    ),
                }

                if let Ok(response) = response_receiver.await {
                    bytes.clear();

                    // response_chunk ::= <result> | <encoding-dependent-header> | <encoded-payload>

                    // encode <result>
                    // TODO: for now let's add 0, but we should handle other cases
                    bytes.extend_from_slice(&[0]);

                    let ssz_bytes = response.serialize()?;

                    // encode <encoding-dependent-header>
                    let mut codec = Uvi::default();
                    codec.encode(ssz_bytes.len(), &mut bytes)?;

                    // encode <encoded-payload>
                    let mut writer = snap::write::FrameEncoder::new(vec![]);
                    writer.write_all(&ssz_bytes)?;
                    let compressed_data =
                        writer.into_inner().map_err(|e| BoundError::IoError(e.into_error()))?;
                    bytes.extend_from_slice(&compressed_data);

                    trace!("Inbound sending {:?}", bytes.to_vec());

                    socket_mut.write_all(&bytes).await?;
                    socket_mut.close().await?;

                    // Response was sent. Indicate to handler to emit a `ResponseSent` event.
                    Ok(HandlerEvent::ResponseSent(request_id))
                } else {
                    socket_mut.close().await?;

                    Ok(HandlerEvent::ResponseOmission(request_id))
                }
            }
        };

        if self.worker_streams.try_push(BoundTypeId::Inbound(request_id), recv.boxed()).is_err() {
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
        let OutboundInfo { request, request_id } = info;
        let send = async move {
            trace!("Outbound {:?}", request);
            let mut buffer = Vec::new();
            let socket_mut = &mut socket;

            match request {
                Request::Status(status) => status.serialize(&mut buffer)?,
                Request::GoodbyeReason(reason) => reason.serialize(&mut buffer)?,
                Request::Ping(ping) => ping.serialize(&mut buffer)?,
                Request::GetMetadata(meta_data) => meta_data.serialize(&mut buffer)?,
                Request::PooledUserOpHashesReq(pooled_user_op_hashes_req) => {
                    pooled_user_op_hashes_req.serialize(&mut buffer)?
                }

                Request::PooledUserOpsByHashReq(pooled_user_ops_by_hash_req) => {
                    pooled_user_ops_by_hash_req.serialize(&mut buffer)?
                }
            };

            trace!("Outbound buffer {:?}", buffer);

            let mut bytes = BytesMut::new();

            // encode header
            let mut codec = Uvi::default();
            codec.encode(buffer.len(), &mut bytes)?;

            // encode payload
            let mut writer = snap::write::FrameEncoder::new(vec![]);
            writer.write_all(&buffer)?;
            let compressed_data =
                writer.into_inner().map_err(|e| BoundError::IoError(e.into_error()))?;
            bytes.extend_from_slice(&compressed_data);

            trace!("Outbound bytes {:?}", bytes.to_vec());
            trace!("Sending {:?} bytes", bytes.len());
            socket_mut.write_all(&bytes).await?;
            socket_mut.close().await?;

            let mut compressed_response = Vec::new();
            socket_mut.take(RESPONSE_SIZE_MAXIMUM).read_to_end(&mut compressed_response).await?;

            trace!("Outbound received {:?}", compressed_response);

            bytes = BytesMut::new();
            bytes.extend_from_slice(&compressed_response);

            // response_chunk ::= <result> | <encoding-dependent-header> | <encoded-payload>

            // TODO: response chunks

            // decode <result>
            let _ = bytes.split_to(1); // TODO: handle result

            // decode <encoding-dependent-header>
            let mut codec: Uvi<usize> = Uvi::default();
            let _ = codec.decode(&mut bytes)?; // TODO: use length to verify size

            // decode <encoded-payload>
            let mut decompressed = vec![];
            snap::read::FrameDecoder::<&[u8]>::new(&bytes).read_to_end(&mut decompressed)?;

            let response = match protocol_id.message_name {
                Protocol::Status => Response::Status(Status::deserialize(&decompressed)?),
                Protocol::Goodbye => {
                    Response::GoodbyeReason(GoodbyeReason::deserialize(&decompressed)?)
                }
                Protocol::Ping => Response::Pong(Pong::deserialize(&decompressed)?),
                Protocol::MetaData => Response::Metadata(Metadata::deserialize(&decompressed)?),
                Protocol::PooledUserOpHashes => {
                    Response::PooledUserOpHashes(PooledUserOpHashes::deserialize(&decompressed)?)
                }
                Protocol::PooledUserOpsByHash => {
                    Response::PooledUserOpsByHash(PooledUserOpsByHash::deserialize(&decompressed)?)
                }
            };

            Ok(HandlerEvent::Response { response, request_id })
        };

        if self.worker_streams.try_push(BoundTypeId::Outbound(request_id), send.boxed()).is_err() {
            warn!("Dropping inbound stream because we are at capacity")
        }
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = OutboundInfo;
    type ToBehaviour = HandlerEvent;
    type InboundOpenInfo = InboundInfo;
    type InboundProtocol = InboundReqUpgrade;
    type OutboundOpenInfo = OutboundInfo;
    type OutboundProtocol = OutboundRepUpgrade;

    fn connection_keep_alive(&self) -> bool {
        true
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

        let inbound_info =
            InboundInfo { request_sender: rq_send, response_receiver: rs_recv, request_id };

        self.inbound.push(
            rq_recv
                .map_ok(move |rq| RequestContainer {
                    request: rq,
                    request_id,
                    response_sender: rs_send,
                })
                .boxed(),
        );

        SubstreamProtocol::new(InboundReqUpgrade, inbound_info)
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
                    .push_back(HandlerEvent::DialUpgradeTimeout(error.info.request_id)),
                StreamUpgradeError::NegotiationFailed => self
                    .pending_events
                    .push_back(HandlerEvent::OutboundUnsurpportedProtocol(error.info.request_id)),
                dial_error => {
                    warn!("outbound stream with {:?} failed with {dial_error:?}", error.info)
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
            Poll::Ready((BoundTypeId::Inbound(request_id), Ok(Err(error)))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::InboundError { request_id, error },
                ));
            }
            Poll::Ready((BoundTypeId::Outbound(request_id), Ok(Err(error)))) => {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    HandlerEvent::OutboundError { request_id, error },
                ));
            }
            Poll::Pending => {}
        }

        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(event));
        };

        while let Poll::Ready(Some(result)) = self.inbound.poll_next_unpin(cx) {
            match result {
                Ok(RequestContainer { request, request_id, response_sender }) => {
                    return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::NotifyBehaviour(
                        HandlerEvent::Request { request, response_sender, request_id },
                    ));
                }
                Err(oneshot::Canceled) => {}
            }
        }

        if let Some(request) = self.outbound.pop_front() {
            return Poll::Ready(libp2p::swarm::ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(
                    OutboundRepUpgrade(request.clone().request),
                    request,
                )
                .with_timeout(self.substream_timeout),
            });
        };

        Poll::Pending
    }
}
