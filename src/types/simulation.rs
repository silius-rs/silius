use ethers::abi::{AbiDecode, AbiEncode};
use ethers::prelude::EthAbiType;
use ethers::{
    prelude::EthAbiCodec,
    types::{Address, Bytes, H256, U256},
};
use jsonrpsee::types::{error::ErrorCode, ErrorObject};
use lazy_static::lazy_static;
use reth_db::table::{Compress, Decompress};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SIMULATE_VALIDATION_ERROR_CODE: i32 = -32500;
const OPCODE_VALIDATION_ERROR_CODE: i32 = -32502;
const SIMULATION_EXECUTION_ERROR_CODE: i32 = -32521;

pub type SimulationError = ErrorObject<'static>;

// https://github.com/eth-infinitism/account-abstraction/blob/develop/contracts/core/EntryPoint.sol#L514
// 0 - factory, 1 - sender/account, 2 - paymaster
// opcode NUMBER is marker between levels
pub const NUMBER_LEVELS: usize = 3;
pub const LEVEL_TO_ENTITY: [&str; NUMBER_LEVELS] = ["factory", "account", "paymaster"];

lazy_static! {
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
    pub static ref CREATE2_OPCODE: String = "CREATE2".to_string();
    pub static ref RETURN_OPCODE: String = "RETURN".to_string();
    pub static ref REVERT_OPCODE: String = "REVERT".to_string();
    pub static ref CREATE_OPCODE: String = "CREATE".to_string();
    pub static ref PAYMASTER_VALIDATION_FUNCTION: String = "validatePaymasterUserOp".to_string();
}

pub struct StakeInfo {
    pub address: Address,
    pub stake: (U256, U256),
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize, EthAbiCodec, EthAbiType)]
pub struct CodeHash {
    pub address: Address,
    pub code_hash: H256,
}

impl Compress for CodeHash {
    type Compressed = Bytes;
    fn compress(self) -> Self::Compressed {
        Bytes::from(self.encode())
    }
}

impl Decompress for CodeHash {
    fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        Self::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
    }
}

#[derive(Debug)]
pub enum SimulateValidationError {
    UserOperationRejected { message: String },
    OpcodeValidation { entity: String, opcode: String },
    UserOperationExecution { message: String },
    StorageAccessValidation { slot: String },
    CallStackValidation { message: String },
    CodeHashesValidation { message: String },
    UnknownError { error: String },
}

impl From<SimulateValidationError> for SimulationError {
    fn from(error: SimulateValidationError) -> Self {
        match error {
            SimulateValidationError::UserOperationRejected { message } => {
                SimulationError::owned(SIMULATE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::OpcodeValidation { entity, opcode } => SimulationError::owned(
                OPCODE_VALIDATION_ERROR_CODE,
                format!("{entity} uses banned opcode: {opcode}"),
                None::<bool>,
            ),
            SimulateValidationError::UserOperationExecution { message } => {
                SimulationError::owned(SIMULATION_EXECUTION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::StorageAccessValidation { slot } => SimulationError::owned(
                OPCODE_VALIDATION_ERROR_CODE,
                format!("Storage access validation failed for slot: {slot}"),
                None::<bool>,
            ),
            SimulateValidationError::CallStackValidation { message } => {
                SimulationError::owned(OPCODE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::CodeHashesValidation { message } => {
                SimulationError::owned(OPCODE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::UnknownError { error } => {
                SimulationError::owned(ErrorCode::InternalError.code(), error, None::<bool>)
            }
        }
    }
}
