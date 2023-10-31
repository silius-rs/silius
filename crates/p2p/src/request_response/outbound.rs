use std::io::{Read, Write};

use futures::{future::BoxFuture, AsyncReadExt, AsyncWriteExt, FutureExt};
use libp2p::{core::UpgradeInfo, OutboundUpgrade, Stream};
use ssz_rs::{Deserialize, Serialize};
use tracing::{debug, trace};

use super::{
    models::{
        BoundError, GoodbyeReason, MetaData, Pong, PooledUserOpHashes, PooledUserOpsByHash,
        Request, RequestId, Response, Status,
    },
    protocol::{Protocol, ProtocolId},
};

/// Max response size in bytes
const RESPONSE_SIZE_MAXIMUM: u64 = 10 * 1024 * 1024;

#[derive(Debug)]
pub struct OutboundContainer {
    pub request: Request,
    pub request_id: RequestId,
}

impl UpgradeInfo for OutboundContainer {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        debug!("protocol_info {:?}", self.request);
        match self.request {
            Request::Status(_) => vec![ProtocolId::new(Protocol::Status)],
            Request::GoodbyeReason(_) => vec![ProtocolId::new(Protocol::Goodbye)],
            Request::Ping(_) => vec![ProtocolId::new(Protocol::Ping)],
            Request::GetMetaData(_) => vec![ProtocolId::new(Protocol::MetaData)],
            Request::PooledUserOpHashesReq(_) => {
                vec![ProtocolId::new(Protocol::PooledUserOpHashes)]
            }
            Request::PooledUserOpsByHashReq(_) => {
                vec![ProtocolId::new(Protocol::PooledUserOpsByHash)]
            }
        }
    }
}

impl OutboundUpgrade<Stream> for OutboundContainer {
    type Error = BoundError;
    type Output = Response;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;
    fn upgrade_outbound(self, mut socket: Stream, info: Self::Info) -> Self::Future {
        async move {
            trace!("Outbound {:?}!!!", self.request);
            let mut buffer = Vec::new();
            let socket_mut = &mut socket;
            match self.request {
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
            let response = match info.message_name {
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
            Ok(response)
        }
        .boxed()
    }
}
