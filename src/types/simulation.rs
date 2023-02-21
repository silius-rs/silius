use ethers::providers::Middleware;
use jsonrpsee::types::{error::ErrorCode, ErrorObject};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

const SIMULATE_VALIDATION_ERROR_CODE: i32 = -32500;
const OPCODE_VALIDATION_ERROR_CODE: i32 = -32502;

pub type SimulationError = ErrorObject<'static>;

lazy_static! {
    // 0 - factory, 1 - sender/account, 2 - paymaster
    // opcode NUMBER is marker between levels
    // https://github.com/eth-infinitism/account-abstraction/blob/develop/contracts/core/EntryPoint.sol#L514
    pub static ref LEVEL_TO_ENTITY: HashMap<usize, &'static str> = {
        let mut map = HashMap::new();
        map.insert(0, "factory");
        map.insert(1, "account");
        map.insert(2, "paymaster");
        map
    };

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
        set
    };
}

#[derive(Debug)]
pub enum SimulateValidationError<M: Middleware> {
    UserOperationRejected { message: String },
    OpcodeValidation { entity: String, opcode: String },
    Middleware(M::Error),
}

impl<M: Middleware> From<SimulateValidationError<M>> for SimulationError {
    fn from(error: SimulateValidationError<M>) -> Self {
        match error {
            SimulateValidationError::UserOperationRejected { message } => {
                SimulationError::owned(SIMULATE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::OpcodeValidation { entity, opcode } => SimulationError::owned(
                OPCODE_VALIDATION_ERROR_CODE,
                format!("{entity} uses opcode {opcode}"),
                None::<bool>,
            ),
            SimulateValidationError::Middleware(_) => {
                SimulationError::from(ErrorCode::InternalError)
            }
        }
    }
}
