use super::{
    methods::{
        GoodbyeReason, MetaDataRequest, Ping, PooledUserOpHashesRequest,
        PooledUserOpsByHashRequest, Status,
    },
    outbound::OutboundRequest,
};
use futures::future::{ready, Ready};
use lazy_static::lazy_static;
use libp2p::{core::UpgradeInfo, InboundUpgrade, Stream};
use silius_primitives::constants::p2p::PROTOCOL_PREFIX;
use std::fmt::Display;

lazy_static! {
    pub static ref SUPPORTED_PROTOCOLS: Vec<ProtocolId> = vec![
        ProtocolId::new(Protocol::Status),
        ProtocolId::new(Protocol::Goodbye),
        ProtocolId::new(Protocol::Ping),
        ProtocolId::new(Protocol::MetaData),
        ProtocolId::new(Protocol::PooledUserOpHashes),
        ProtocolId::new(Protocol::PooledUserOpsByHash),
    ];
}

#[derive(Clone, Debug, Copy)]
pub enum Protocol {
    Status,
    Goodbye,
    Ping,
    MetaData,
    PooledUserOpHashes,
    PooledUserOpsByHash,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            Protocol::Status => "status",
            Protocol::Goodbye => "goodbye",
            Protocol::Ping => "ping",
            Protocol::MetaData => "metadata",
            Protocol::PooledUserOpHashes => "pooled_user_op_hashes",
            Protocol::PooledUserOpsByHash => "pooled_user_ops_by_hash",
        };
        f.write_str(result)
    }
}

#[derive(Clone, Debug)]
pub struct ProtocolId {
    /// The protocol name.
    pub protocol: Protocol,

    /// The version of the RPC.
    pub version: Version,

    /// The encoding of the RPC.
    pub encoding: Encoding,

    /// The protocol id.
    protocol_id: String,
}

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        &self.protocol_id
    }
}

impl ProtocolId {
    pub fn new(protocol: Protocol) -> Self {
        let protocol_id =
            format!("{PROTOCOL_PREFIX}/{protocol}/{}/{}", Version::V1, Encoding::SSZSnappy);
        Self { protocol, version: Version::V1, encoding: Encoding::SSZSnappy, protocol_id }
    }
}

#[derive(Clone, Debug)]
pub enum Version {
    V1,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("1")
    }
}

#[derive(Clone, Debug)]
pub enum Encoding {
    SSZSnappy,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ssz_snappy")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboundRequest {
    Status(Status),
    Goodbye(GoodbyeReason),
    Ping(Ping),
    MetaData(MetaDataRequest),
    PooledUserOpHashes(PooledUserOpHashesRequest),
    PooledUserOpsByHash(PooledUserOpsByHashRequest),
}

impl PartialEq<OutboundRequest> for InboundRequest {
    fn eq(&self, _other: &OutboundRequest) -> bool {
        matches!(self, _other)
    }
}

/// The inbound upgrade for the request-response protocol.
pub struct InboundProtocolUpgrade;

impl UpgradeInfo for InboundProtocolUpgrade {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        SUPPORTED_PROTOCOLS.clone()
    }
}

impl InboundUpgrade<Stream> for InboundProtocolUpgrade {
    type Error = ();
    type Output = (Stream, Self::Info);
    type Future = Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        ready(Ok((socket, info)))
    }
}

#[cfg(test)]
mod tests {
    use super::SUPPORTED_PROTOCOLS;

    #[test]
    fn test_protocol() {
        let protocols = SUPPORTED_PROTOCOLS
            .iter()
            .map(|protocol_id| protocol_id.protocol_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            protocols,
            vec![
                "/account_abstraction/req/status/1/ssz_snappy",
                "/account_abstraction/req/goodbye/1/ssz_snappy",
                "/account_abstraction/req/ping/1/ssz_snappy",
                "/account_abstraction/req/metadata/1/ssz_snappy",
                "/account_abstraction/req/pooled_user_op_hashes/1/ssz_snappy",
                "/account_abstraction/req/pooled_user_ops_by_hash/1/ssz_snappy"
            ]
        )
    }
}
