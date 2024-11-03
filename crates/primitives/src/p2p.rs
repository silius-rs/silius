//! P2P primitives

use crate::{
    constants::entry_point, simulation::ValidationConfig, utils::deserialize_stringified_float,
    UserOperation, UserOperationSigned,
};
use alloy_chains::Chain;
use ethers::types::{Address, H160, H256, U256 as EthersU256};
use ssz_rs::{Vector, U256};
use ssz_rs_derive::Serializable;
use std::str::FromStr;

/// Canonical mempool config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MempoolConfig {
    #[serde(deserialize_with = "ethers::types::serde_helpers::deserialize_stringified_numeric")]
    pub chain_id: EthersU256,
    #[serde(rename = "entryPointContract")]
    pub entry_point: Address,
    pub description: String,
    #[serde(rename = "minimumStake")]
    #[serde(deserialize_with = "deserialize_stringified_float")]
    pub min_stake: EthersU256,
    #[serde(skip_serializing, skip_deserializing)]
    pub id: String,
}

impl MempoolConfig {
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn dev() -> MempoolConfig {
        Self {
            chain_id: Chain::dev().id().into(),
            entry_point: H160::from_str(entry_point::ADDRESS).unwrap_or_default(),
            description: "".to_string(),
            min_stake: EthersU256::zero(),
            id: "".to_string(),
        }
    }
}

/// Messages types the network can receive.
#[derive(Debug)]
pub enum NetworkMessage {
    Publish {
        user_operation: UserOperation,
        verified_at_block_hash: EthersU256,
        validation_config: ValidationConfig,
    },
    // Find the next canonical mempool to validate user operation
    FindNewMempool {
        user_operation: UserOperation,
        topic: String, // topic where validation failed
    },
    Validate {
        user_operation: UserOperation,
        validation_config: ValidationConfig,
    },
    NewBlock {
        block_hash: H256,
        block_number: u64,
    },
}

/// P2P message type
#[derive(Clone, Debug, Default, Serializable, PartialEq)]
pub struct VerifiedUserOperation {
    user_operation: UserOperationSigned,
    entry_point: Vector<u8, 20>,
    verified_at_block_hash: U256,
}

impl VerifiedUserOperation {
    pub fn new(
        user_operation: UserOperationSigned,
        entry_point: Address,
        verified_at_block_hash: EthersU256,
    ) -> Self {
        let mut buf: [u8; 32] = [0; 32];
        verified_at_block_hash.to_little_endian(&mut buf);
        let verified_at_block_hash = U256::from_bytes_le(buf);

        Self {
            user_operation,
            entry_point: <Vector<u8, 20>>::try_from(entry_point.as_bytes().to_vec())
                .expect("entrypoint address is valid"),
            verified_at_block_hash,
        }
    }

    pub fn user_operation(self) -> UserOperationSigned {
        self.user_operation
    }

    pub fn entry_point(&self) -> Address {
        Address::from_slice(&self.entry_point)
    }
}
