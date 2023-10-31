use super::{
    protocol::{Protocol, ProtocolId, SUPPORTED_PROTOCOL},
    Request,
};
use futures::future::{ready, Ready};
use libp2p::{core::UpgradeInfo, InboundUpgrade, OutboundUpgrade, Stream};

/// The inbound upgrade for the request protocol.
pub struct InboundReqUpgrade;

impl UpgradeInfo for InboundReqUpgrade {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        SUPPORTED_PROTOCOL.clone()
    }
}
impl InboundUpgrade<Stream> for InboundReqUpgrade {
    type Error = ();
    type Output = (Stream, Self::Info);
    type Future = Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_inbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        ready(Ok((socket, info)))
    }
}

/// The outbound upgrade for the request protocol.
pub struct OutboundRepUpgrade(pub Request);

impl UpgradeInfo for OutboundRepUpgrade {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        match self.0 {
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

impl OutboundUpgrade<Stream> for OutboundRepUpgrade {
    type Error = ();
    type Output = (Stream, Self::Info);
    type Future = Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_outbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        ready(Ok((socket, info)))
    }
}
