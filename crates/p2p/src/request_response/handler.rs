use super::{
    models::{Request, RequestId, Response},
    upgrade::{InboundReqUpgrade, OutboundRepUpgrade},
};
use crate::request_response::{
    protocol::Protocol, BoundError, GetMetaData, GoodbyeReason, MetaData, Ping, Pong,
    PooledUserOpHashes, PooledUserOpHashesReq, PooledUserOpsByHash, PooledUserOpsByHashReq, Status,
};
use futures::{
    channel::oneshot::{self, Receiver, Sender},
    future::BoxFuture,
    stream::FuturesUnordered,
    FutureExt, StreamExt, TryFutureExt,
};
use futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::swarm::ConnectionHandlerEvent;
use libp2p::swarm::{
    handler::{ConnectionEvent, FullyNegotiatedInbound, FullyNegotiatedOutbound},
    ConnectionHandler, ConnectionId, KeepAlive, StreamUpgradeError, SubstreamProtocol,
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
use tracing::{trace, warn};

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
    InboundTimeout(RequestId),
    OutboundTimeout(RequestId),
    InboundError {
        request_id: RequestId,
        error: BoundError,
    },
    OutboundError {
        request_id: RequestId,
        error: BoundError,
    },
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
        let InboundInfo {
            request_sender,
            response_receiver,
            request_id,
        } = info;
        let recv = async move {
            let mut data = Vec::new();
            let socket_mut = &mut socket;
            socket_mut
                .take(REQUEST_SIZE_MAXIMUM)
                .read_to_end(&mut data)
                .await?;
            trace!("Inbound Received {} bytes", data.len());
            // MetaData request would send empty content.
            // https://github.com/eth-infinitism/bundler-spec/blob/main/p2p-specs/p2p-interface.md#getmetadata
            if data.is_empty() && !matches!(protocol_id.message_name, Protocol::MetaData) {
                Err(BoundError::IoError(io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "No data received",
                )))
            } else {
                let mut decompressed_data = vec![];
                snap::read::FrameDecoder::new(data.as_slice())
                    .read_to_end(&mut decompressed_data)
                    .map_err(BoundError::IoError)?;
                let request = match protocol_id.message_name {
                    Protocol::Status => Request::Status(Status::deserialize(&decompressed_data)?),
                    Protocol::Goodbye => {
                        Request::GoodbyeReason(GoodbyeReason::deserialize(&decompressed_data)?)
                    }
                    Protocol::Ping => Request::Ping(Ping::deserialize(&decompressed_data)?),
                    Protocol::MetaData => {
                        Request::GetMetaData(GetMetaData::deserialize(&decompressed_data)?)
                    }
                    Protocol::PooledUserOpHashes => Request::PooledUserOpHashesReq(
                        PooledUserOpHashesReq::deserialize(&decompressed_data)?,
                    ),
                    Protocol::PooledUserOpsByHash => Request::PooledUserOpsByHashReq(
                        PooledUserOpsByHashReq::deserialize(&decompressed_data)?,
                    ),
                };
                match request_sender.send(request) {
                    Ok(()) => {}
                    // It should not happen. There must be something wrong with the codes.
                    Err(_) => panic!(
                        "Expect request receiver to be alive i.e. protocol handler to be alive.",
                    ),
                }
                if let Ok(response) = response_receiver.await {
                    let ssz_encoded = response.serialize()?;
                    let mut wtr = snap::write::FrameEncoder::new(vec![]);
                    wtr.write_all(&ssz_encoded)?;
                    let compressed_data = wtr
                        .into_inner()
                        .map_err(|e| BoundError::IoError(e.into_error()))?;
                    socket_mut.write_all(&compressed_data).await?;

                    socket_mut.close().await?;
                    // Response was sent. Indicate to handler to emit a `ResponseSent` event.
                    Ok(HandlerEvent::ResponseSent(request_id))
                } else {
                    socket_mut.close().await?;
                    Ok(HandlerEvent::ResponseOmission(request_id))
                }
            }
        };

        if self
            .worker_streams
            .try_push(BoundTypeId::Inbound(request_id), recv.boxed())
            .is_err()
        {
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
        let OutboundInfo {
            request,
            request_id,
        } = info;
        let send = async move {
            trace!("Outbound {:?}!!!", request);
            let mut buffer = Vec::new();
            let socket_mut = &mut socket;
            match request {
                Request::Status(status) => status.serialize(&mut buffer)?,
                Request::GoodbyeReason(reason) => reason.serialize(&mut buffer)?,
                Request::Ping(ping) => ping.serialize(&mut buffer)?,
                Request::GetMetaData(meta_data) => meta_data.serialize(&mut buffer)?,
                Request::PooledUserOpHashesReq(pooled_user_op_hashes_req) => {
                    pooled_user_op_hashes_req.serialize(&mut buffer)?
                }

                Request::PooledUserOpsByHashReq(pooled_user_ops_by_hash_req) => {
                    pooled_user_ops_by_hash_req.serialize(&mut buffer)?
                }
            };
            trace!("Outbound buffer {:?}", buffer);
            let mut wtr = snap::write::FrameEncoder::new(vec![]);
            wtr.write_all(&buffer)?;
            let compressed = wtr
                .into_inner()
                .map_err(|e| BoundError::IoError(e.into_error()))?;
            trace!("Sending {:?} bytes", compressed.len());
            socket_mut.write_all(compressed.as_ref()).await?;
            socket_mut.close().await?;
            let mut comressed_response = Vec::new();
            socket_mut
                .take(RESPONSE_SIZE_MAXIMUM)
                .read_to_end(&mut comressed_response)
                .await?;
            trace!("Outbound received {:?}!!!", comressed_response);
            let mut decompressed = vec![];
            snap::read::FrameDecoder::<&[u8]>::new(comressed_response.as_ref())
                .read_to_end(&mut decompressed)?;
            let response = match protocol_id.message_name {
                Protocol::Status => Response::Status(Status::deserialize(&decompressed)?),
                Protocol::Goodbye => {
                    Response::GoodbyeReason(GoodbyeReason::deserialize(&decompressed)?)
                }
                Protocol::Ping => Response::Pong(Pong::deserialize(&decompressed)?),
                Protocol::MetaData => Response::MetaData(MetaData::deserialize(&decompressed)?),
                Protocol::PooledUserOpHashes => {
                    Response::PooledUserOpHashes(PooledUserOpHashes::deserialize(&decompressed)?)
                }
                Protocol::PooledUserOpsByHash => {
                    Response::PooledUserOpsByHash(PooledUserOpsByHash::deserialize(&decompressed)?)
                }
            };

            Ok(HandlerEvent::Response {
                response,
                request_id,
            })
        };

        if self
            .worker_streams
            .try_push(BoundTypeId::Outbound(request_id), send.boxed())
            .is_err()
        {
            warn!("Dropping inbound stream because we are at capacity")
        }
    }
}

impl ConnectionHandler for Handler {
    type Error = Error;
    type FromBehaviour = OutboundInfo;
    type ToBehaviour = HandlerEvent;
    type InboundOpenInfo = InboundInfo;
    type InboundProtocol = InboundReqUpgrade;
    type OutboundOpenInfo = OutboundInfo;
    type OutboundProtocol = OutboundRepUpgrade;
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

        let inbound_info = InboundInfo {
            request_sender: rq_send,
            response_receiver: rs_recv,
            request_id,
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
                StreamUpgradeError::NegotiationFailed => {
                    self.pending_events
                        .push_back(HandlerEvent::OutboundUnsurpportedProtocol(
                            error.info.request_id,
                        ))
                }
                dial_error => warn!(
                    "outbound stream with {:?} failed with {dial_error:?}",
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
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
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
            return Poll::Ready(
                libp2p::swarm::ConnectionHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(
                        OutboundRepUpgrade(request.clone().request),
                        request,
                    )
                    .with_timeout(self.substream_timeout),
                },
            );
        };

        Poll::Pending
    }
}
