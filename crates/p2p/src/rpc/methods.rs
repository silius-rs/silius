use silius_primitives::{
    constants::p2p::{MAX_IPFS_CID_LENGTH, MAX_OPS_PER_REQUEST, MAX_SUPPORTED_MEMPOOLS},
    VerifiedUserOperation,
};
use ssz_rs::{List, Serialize, Vector};

/// Metadata of a node/peer.
#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct MetaData {
    /// The sequence number of the metadata (incremente when data updated).
    pub seq_number: u64,
    /// List of all supported mempools (canonical and alt).
    pub supported_mempools: List<Vector<u8, MAX_IPFS_CID_LENGTH>, MAX_SUPPORTED_MEMPOOLS>,
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct Status {
    pub chain_id: u64,
    pub block_hash: [u8; 32],
    pub block_number: u64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum GoodbyeReason {
    #[default]
    ClientShutdown,
    IrrelevantNetwork,
    Error,
    Unknown(u64),
}

impl ssz_rs::Deserialize for GoodbyeReason {
    fn deserialize(reader: &[u8]) -> Result<Self, ssz_rs::DeserializeError> {
        let value = <u64 as ssz_rs::Deserialize>::deserialize(reader)?;
        Ok(value.into())
    }
}

impl ssz_rs::Serialize for GoodbyeReason {
    fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
        let value: u64 = self.clone().into();
        value.serialize(buffer)
    }
}

impl ssz_rs::Serializable for GoodbyeReason {
    fn is_variable_size() -> bool {
        false
    }
    fn size_hint() -> usize {
        <u64 as ssz_rs::Serializable>::size_hint()
    }
}

impl From<u64> for GoodbyeReason {
    fn from(value: u64) -> Self {
        match value {
            1 => GoodbyeReason::ClientShutdown,
            2 => GoodbyeReason::IrrelevantNetwork,
            3 => GoodbyeReason::Error,
            _ => GoodbyeReason::Unknown(value),
        }
    }
}

impl From<GoodbyeReason> for u64 {
    fn from(value: GoodbyeReason) -> Self {
        match value {
            GoodbyeReason::ClientShutdown => 1,
            GoodbyeReason::IrrelevantNetwork => 2,
            GoodbyeReason::Error => 3,
            GoodbyeReason::Unknown(v) => v,
        }
    }
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct Ping {
    pub data: u64,
}

impl Ping {
    pub fn new(data: u64) -> Self {
        Self { data }
    }
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpHashesRequest {
    cursor: [u8; 32],
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpsByHashRequest {
    hashes: List<Vector<u8, 32>, MAX_OPS_PER_REQUEST>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MetaDataRequest;

impl ssz_rs::Serializable for MetaDataRequest {
    fn is_variable_size() -> bool {
        false
    }
    fn size_hint() -> usize {
        0
    }
}

impl ssz_rs::Serialize for MetaDataRequest {
    fn serialize(&self, _buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
        Ok(0)
    }
}

impl ssz_rs::Deserialize for MetaDataRequest {
    fn deserialize(_encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError>
    where
        Self: Sized,
    {
        Ok(MetaDataRequest)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(pub(crate) u64);

#[derive(Debug, Clone, PartialEq)]
pub enum RPCResponse {
    Status(Status),
    Goodbye(GoodbyeReason),
    Pong(Ping),
    MetaData(MetaData),
    PooledUserOpHashes(PooledUserOpHashesResponse),
    PooledUserOpsByHash(PooledUserOpsByHashResponse),
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpHashesResponse {
    hashes: List<[u8; 32], MAX_OPS_PER_REQUEST>,
    next_cursor: [u8; 32],
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpsByHashResponse {
    hashes: List<VerifiedUserOperation, MAX_OPS_PER_REQUEST>,
}

impl RPCResponse {
    pub fn serialize(self) -> Result<Vec<u8>, ssz_rs::SerializeError> {
        let mut buffer = Vec::new();
        match self {
            RPCResponse::Status(status) => status.serialize(&mut buffer),
            RPCResponse::Goodbye(reason) => reason.serialize(&mut buffer),
            RPCResponse::Pong(pong) => pong.serialize(&mut buffer),
            RPCResponse::MetaData(metadata) => metadata.serialize(&mut buffer),
            RPCResponse::PooledUserOpHashes(pooled_user_op_hashes) => {
                pooled_user_op_hashes.serialize(&mut buffer)
            }
            RPCResponse::PooledUserOpsByHash(pooled_user_ops_by_hash) => {
                pooled_user_ops_by_hash.serialize(&mut buffer)
            }
        }?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::Ping;
    use ssz_rs::Serialize;
    use std::io::Write;

    #[test]
    fn serialize() {
        let ping = Ping::new(1);
        let mut buffer = Vec::new();
        ping.serialize(&mut buffer).unwrap();
        let mut wtr = snap::write::FrameEncoder::new(vec![]);
        wtr.write_all(&buffer).unwrap();
        let compress = wtr.into_inner().unwrap();
        let number: u64 = 1;
        let mut buffer2 = Vec::new();
        number.serialize(&mut buffer2).unwrap();
        let mut wtr = snap::write::FrameEncoder::new(vec![]);
        wtr.write_all(&buffer).unwrap();
        let compress2 = wtr.into_inner().unwrap();
        assert_eq!(compress, compress2);
    }
}
