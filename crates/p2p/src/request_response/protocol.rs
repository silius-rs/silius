use lazy_static::lazy_static;
use silius_primitives::consts::p2p::REQREP_PROTOCOL_PREFIX;
use std::fmt::Display;

lazy_static! {
    pub static ref SUPPORTED_PROTOCOL: Vec<ProtocolId> = vec![
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
            Protocol::PooledUserOpHashes => "pooled_user_ops_hashes",
            Protocol::PooledUserOpsByHash => "pooled_user_ops_by_hash",
        };
        f.write_str(result)
    }
}

#[derive(Clone, Debug)]
pub struct ProtocolId {
    /// The RPC message type/name.
    pub message_name: Protocol,

    /// The version of the RPC.
    pub version: Version,

    /// The encoding of the RPC.
    pub encoding: Encoding,

    protocol_id: String,
}

impl ProtocolId {
    pub fn new(message_name: Protocol) -> Self {
        let protocol_id = format!(
            "{REQREP_PROTOCOL_PREFIX}/{message_name}/{}/{}",
            Version::V1,
            Encoding::SSZSnappy
        );
        Self {
            message_name,
            version: Version::V1,
            encoding: Encoding::SSZSnappy,
            protocol_id,
        }
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

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        &self.protocol_id
    }
}

#[cfg(test)]
mod test {
    use super::SUPPORTED_PROTOCOL;

    #[test]
    fn test_protoco() {
        let protocols = SUPPORTED_PROTOCOL
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
                "/account_abstraction/req/pooled_user_ops_hashes/1/ssz_snappy",
                "/account_abstraction/req/pooled_user_ops_by_hash/1/ssz_snappy"
            ]
        )
    }
}
