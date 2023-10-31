use std::io::{self, Read, Write};

use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::{AsyncReadExt, AsyncWriteExt, FutureExt};
use libp2p::swarm::Stream;
use libp2p::{core::UpgradeInfo, InboundUpgrade};
use ssz_rs::Deserialize;
use tracing::trace;

use super::models::{
    BoundError, GetMetaData, GoodbyeReason, Ping, PooledUserOpHashesReq, PooledUserOpsByHashReq,
    Status,
};
use super::protocol::{Protocol, SUPPORTED_PROTOCOL};
use super::{
    models::{Request, Response},
    protocol::ProtocolId,
};
/// Max request size in bytes
const REQUEST_SIZE_MAXIMUM: u64 = 1024 * 1024;

pub struct InboundContainer {
    pub(crate) request_sender: oneshot::Sender<Request>,
    pub(crate) response_receiver: oneshot::Receiver<Response>,
}

impl UpgradeInfo for InboundContainer {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        SUPPORTED_PROTOCOL.clone()
    }
}

impl InboundUpgrade<Stream> for InboundContainer {
    type Error = BoundError;
    type Output = bool;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;
    fn upgrade_inbound(self, mut socket: Stream, info: Self::Info) -> Self::Future {
        async move {
            let mut data = Vec::new();
            let socket_mut = &mut socket;
            socket_mut
                .take(REQUEST_SIZE_MAXIMUM)
                .read_to_end(&mut data)
                .await?;
            trace!("Inbound Received {} bytes", data.len());
            // MetaData request would send empty content.
            // https://github.com/eth-infinitism/bundler-spec/blob/main/p2p-specs/p2p-interface.md#getmetadata
            if data.is_empty() && !matches!(info.message_name, Protocol::MetaData) {
                Err(BoundError::IoError(io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "No data received",
                )))
            } else {
                let mut decompressed_data = vec![];
                snap::read::FrameDecoder::new(data.as_slice())
                    .read_to_end(&mut decompressed_data)
                    .map_err(BoundError::IoError)?;
                let request = match info.message_name {
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
                match self.request_sender.send(request) {
                    Ok(()) => {}
                    // It should not happen. There must be something wrong with the codes.
                    Err(_) => panic!(
                        "Expect request receiver to be alive i.e. protocol handler to be alive.",
                    ),
                }
                if let Ok(response) = self.response_receiver.await {
                    let ssz_encoded = response.serialize()?;
                    let mut wtr = snap::write::FrameEncoder::new(vec![]);
                    wtr.write_all(&ssz_encoded)?;
                    let compressed_data = wtr
                        .into_inner()
                        .map_err(|e| BoundError::IoError(e.into_error()))?;
                    socket_mut.write_all(&compressed_data).await?;

                    socket_mut.close().await?;
                    // Response was sent. Indicate to handler to emit a `ResponseSent` event.
                    Ok(true)
                } else {
                    socket_mut.close().await?;
                    Ok(false)
                }
            }
        }
        .boxed()
    }
}
