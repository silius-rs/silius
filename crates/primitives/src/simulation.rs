use ethers::{
    prelude::{EthAbiCodec, EthAbiType},
    providers::MiddlewareError,
    types::{Address, H256, U256},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Time ineterval before user operation expires (in seconds)
pub const EXPIRATION_TIMESTAMP_DIFF: u64 = 30;

lazy_static! {
    pub static ref CREATE2_OPCODE: String = "CREATE2".to_string();
    pub static ref RETURN_OPCODE: String = "RETURN".to_string();
    pub static ref REVERT_OPCODE: String = "REVERT".to_string();
    pub static ref CREATE_OPCODE: String = "CREATE".to_string();
    pub static ref PAYMASTER_VALIDATION_FUNCTION: String = "validatePaymasterUserOp".to_string();
    pub static ref FORBIDDEN_OPCODES: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("GASPRICE".to_string());
        set.insert("GASLIMIT".to_string());
        set.insert("DIFFICULTY".to_string());
        set.insert("TIMESTAMP".to_string());
        set.insert("BASEFEE".to_string());
        set.insert("BLOCKHASH".to_string());
        set.insert("NUMBER".to_string());
        set.insert("SELFBALANCE".to_string());
        set.insert("BALANCE".to_string());
        set.insert("ORIGIN".to_string());
        set.insert("GAS".to_string());
        set.insert("CREATE".to_string());
        set.insert("COINBASE".to_string());
        set.insert("SELFDESTRUCT".to_string());
        set.insert("RANDOM".to_string());
        set.insert("PREVRANDAO".to_string());
        set
    };
}

/// Error object for simulation
#[derive(Debug, Serialize, Deserialize)]
pub enum SimulationCheckError {
    Signature {},
    Expiration {
        valid_after: U256,
        valid_until: U256,
        paymaster: Option<Address>,
    },
    Validation {
        message: String,
    },
    Opcode {
        entity: String,
        opcode: String,
    },
    Execution {
        message: String,
    },
    StorageAccess {
        slot: String,
    },
    Unstaked {
        entity: String,
        message: String,
    },
    CallStack {
        message: String,
    },
    CodeHashes {
        message: String,
    },
    OutOfGas {},
    MiddlewareError {
        message: String,
    },
    UnknownError {
        message: String,
    },
}

impl<M: MiddlewareError> From<M> for SimulationCheckError {
    fn from(err: M) -> Self {
        SimulationCheckError::MiddlewareError {
            message: err.to_string(),
        }
    }
}

/// Code hash - hash of the code of the contract
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

/// Storage map
pub type StorageMap = HashMap<Address, HashMap<String, u64>>;
