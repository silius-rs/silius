use silius_primitives::UserOperation;
use ssz_rs::{Bitvector, List, Serialize, Vector};
use std::io;

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Status(Status),
    GoodbyeReason(GoodbyeReason),
    Ping(Ping),
    GetMetaData(GetMetaData),
    PooledUserOpHashesReq(PooledUserOpHashesReq),
    PooledUserOpsByHashReq(PooledUserOpsByHashReq),
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct Status {
    supported_mempool: List<[u8; 32], 1024>,
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
    data: u64,
}

impl Ping {
    pub fn new(data: u64) -> Self {
        Self { data }
    }
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct Pong {
    data: u64,
}

impl Pong {
    pub fn new(data: u64) -> Self {
        Self { data }
    }
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpHashesReq {
    mempool: [u8; 32],
    offset: u64,
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpsByHashReq {
    hashes: List<Vector<u8, 32>, 1024>,
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct MetaData {
    seq_number: u64,
    mempool_nets: Bitvector<32>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GetMetaData;

impl ssz_rs::Serializable for GetMetaData {
    fn is_variable_size() -> bool {
        false
    }
    fn size_hint() -> usize {
        0
    }
}

impl ssz_rs::Serialize for GetMetaData {
    fn serialize(&self, _buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
        Ok(0)
    }
}

impl ssz_rs::Deserialize for GetMetaData {
    fn deserialize(_encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError>
    where
        Self: Sized,
    {
        Ok(GetMetaData)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Status(Status),
    GoodbyeReason(GoodbyeReason),
    Pong(Pong),
    MetaData(MetaData),
    PooledUserOpHashes(PooledUserOpHashes),
    PooledUserOpsByHash(PooledUserOpsByHash),
}

impl Response {
    pub fn serialize(self) -> Result<Vec<u8>, ssz_rs::SerializeError> {
        let mut buffer = Vec::new();
        match self {
            Response::Status(status) => status.serialize(&mut buffer),
            Response::GoodbyeReason(reason) => reason.serialize(&mut buffer),
            Response::Pong(pong) => pong.serialize(&mut buffer),
            Response::MetaData(metadata) => metadata.serialize(&mut buffer),
            Response::PooledUserOpHashes(pooled_user_op_hashes) => {
                pooled_user_op_hashes.serialize(&mut buffer)
            }
            Response::PooledUserOpsByHash(pooled_user_ops_by_hash) => {
                pooled_user_ops_by_hash.serialize(&mut buffer)
            }
        }?;
        Ok(buffer)
    }
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpHashes {
    more_flag: u64,
    hashes: List<[u8; 32], 1024>,
}

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct PooledUserOpsByHash {
    hashes: List<UserOperation, 1024>,
}

#[derive(Clone, Default)]
pub struct SSZSnappyCodec<Req, Res> {
    phantom: std::marker::PhantomData<(Req, Res)>,
}

#[derive(Debug)]
pub enum BoundError {
    IoError(io::Error),
    SSZError(snap::Error),
    DeserializeError(ssz_rs::DeserializeError),
    SerializeError(ssz_rs::SerializeError),
}

impl From<io::Error> for BoundError {
    fn from(value: io::Error) -> Self {
        BoundError::IoError(value)
    }
}

impl From<snap::Error> for BoundError {
    fn from(value: snap::Error) -> Self {
        BoundError::SSZError(value)
    }
}

impl From<ssz_rs::DeserializeError> for BoundError {
    fn from(value: ssz_rs::DeserializeError) -> Self {
        BoundError::DeserializeError(value)
    }
}

impl From<ssz_rs::SerializeError> for BoundError {
    fn from(value: ssz_rs::SerializeError) -> Self {
        BoundError::SerializeError(value)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(pub(crate) u64);

#[cfg(test)]
mod tests {
    use super::Ping;
    use ssz_rs::Serialize;
    use std::io::Write;

    #[test]
    fn serilize() {
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
