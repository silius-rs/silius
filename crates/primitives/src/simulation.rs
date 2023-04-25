use ethers::{
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, H256},
};
use jsonrpsee::types::ErrorObject;
use serde::{Deserialize, Serialize};

pub type SimulationError = ErrorObject<'static>;

#[derive(
    Debug,
    Default,
    Clone,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EthAbiCodec,
    EthAbiType,
)]
pub struct CodeHash {
    pub address: Address,
    pub hash: H256,
}
