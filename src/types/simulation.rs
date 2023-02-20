use std::collections::HashSet;

use ethers::providers::Middleware;
use jsonrpsee::types::{error::ErrorCode, ErrorObject};
use lazy_static::lazy_static;

const SIMULATE_VALIDATION_ERROR_CODE: i32 = -32500;
const OPCODE_VALIDATION_ERROR_CODE: i32 = -32502;

pub type SimulationError = ErrorObject<'static>;

lazy_static! {
    static ref FORBIDDEN_OPCODES: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("GASPRICE");
        set.insert("GASLIMIT");
        set.insert("DIFFICULTY");
        set.insert("TIMESTAMP");
        set.insert("BASEFEE");
        set.insert("BLOCKHASH");
        set.insert("NUMBER");
        set.insert("SELFBALANCE");
        set.insert("BALANCE");
        set.insert("ORIGIN");
        set.insert("GAS");
        set.insert("CREATE");
        set.insert("COINBASE");
        set.insert("SELFDESTRUCT");
        set
    };
}

#[derive(Debug)]
pub enum SimulateValidationError<M: Middleware> {
    UserOperationRejected { message: String },
    OpcodeValidation { opcode: String },
    Middleware(M::Error),
}

impl<M: Middleware> From<SimulateValidationError<M>> for SimulationError {
    fn from(error: SimulateValidationError<M>) -> Self {
        match error {
            SimulateValidationError::UserOperationRejected { message } => {
                SimulationError::owned(SIMULATE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::OpcodeValidation { opcode } => SimulationError::owned(
                OPCODE_VALIDATION_ERROR_CODE,
                format!("opcode {opcode} is not allowed"),
                None::<bool>,
            ),
            SimulateValidationError::Middleware(_) => {
                SimulationError::from(ErrorCode::InternalError)
            }
        }
    }
}
