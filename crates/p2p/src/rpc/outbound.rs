use super::{
    methods::{
        GoodbyeReason, MetaDataRequest, Ping, PooledUserOpHashesRequest,
        PooledUserOpsByHashRequest, Status,
    },
    protocol::{InboundRequest, Protocol, ProtocolId},
};
use futures::future::{ready, Ready};
use libp2p::{core::UpgradeInfo, OutboundUpgrade, Stream};

#[derive(Debug, Clone, PartialEq)]
pub enum OutboundRequest {
    Status(Status),
    Goodbye(GoodbyeReason),
    Ping(Ping),
    MetaData(MetaDataRequest),
    PooledUserOpHashes(PooledUserOpHashesRequest),
    PooledUserOpsByHash(PooledUserOpsByHashRequest),
}

impl PartialEq<InboundRequest> for OutboundRequest {
    fn eq(&self, _other: &InboundRequest) -> bool {
        matches!(self, _other)
    }
}

/// The outbound upgrade for the request-response protocol.
pub struct OutboundProtocolUpgrade(pub OutboundRequest);

impl UpgradeInfo for OutboundProtocolUpgrade {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        match self.0 {
            OutboundRequest::Status(_) => vec![ProtocolId::new(Protocol::Status)],
            OutboundRequest::Goodbye(_) => vec![ProtocolId::new(Protocol::Goodbye)],
            OutboundRequest::Ping(_) => vec![ProtocolId::new(Protocol::Ping)],
            OutboundRequest::MetaData(_) => vec![ProtocolId::new(Protocol::MetaData)],
            OutboundRequest::PooledUserOpHashes(_) => {
                vec![ProtocolId::new(Protocol::PooledUserOpHashes)]
            }
            OutboundRequest::PooledUserOpsByHash(_) => {
                vec![ProtocolId::new(Protocol::PooledUserOpsByHash)]
            }
        }
    }
}

impl OutboundUpgrade<Stream> for OutboundProtocolUpgrade {
    type Error = ();
    type Output = (Stream, Self::Info);
    type Future = Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        ready(Ok((socket, info)))
    }
}
