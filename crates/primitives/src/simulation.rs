//! Simulation (validation) primitives

use ethers::{
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, H256, U256},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Time ineterval before user operation expires (in seconds)
pub const EXPIRATION_TIMESTAMP_DIFF: u64 = 30;

lazy_static! {
    pub static ref CREATE2_OPCODE: String = "CREATE2".into();
    pub static ref RETURN_OPCODE: String = "RETURN".into();
    pub static ref REVERT_OPCODE: String = "REVERT".into();
    pub static ref CREATE_OPCODE: String = "CREATE".into();
    pub static ref VALIDATE_PAYMASTER_USER_OP_FUNCTION: String = "validatePaymasterUserOp".into();
    pub static ref FORBIDDEN_OPCODES: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("GASPRICE".into());
        set.insert("GASLIMIT".into());
        set.insert("DIFFICULTY".into());
        set.insert("TIMESTAMP".into());
        set.insert("BASEFEE".into());
        set.insert("BLOCKHASH".into());
        set.insert("NUMBER".into());
        set.insert("SELFBALANCE".into());
        set.insert("BALANCE".into());
        set.insert("ORIGIN".into());
        set.insert("GAS".into());
        set.insert("CREATE".into());
        set.insert("COINBASE".into());
        set.insert("SELFDESTRUCT".into());
        set.insert("RANDOM".into());
        set.insert("PREVRANDAO".into());
        set
    };
}

/// Validaton config (you can override some validation values).
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    pub min_stake: Option<U256>,
    pub min_unstake_delay: Option<U256>,
    pub topic: Option<String>,
    pub ignore_prev: bool,
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
#[derive(Debug, Default, Clone)]
pub struct StorageMap {
    pub root_hashes: HashMap<Address, H256>,
    pub slots: HashMap<Address, HashMap<String, String>>,
}
