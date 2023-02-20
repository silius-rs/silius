use ethers::providers::Middleware;
use jsonrpsee::types::{error::ErrorCode, ErrorObject};

const SIMULATE_VALIDATION_ERROR_CODE: i32 = -32500;

pub type SimulationError = ErrorObject<'static>;

#[derive(Debug)]
pub enum SimulateValidationError<M: Middleware> {
    UserOperationRejected { message: String },
    Middleware(M::Error),
}

impl<M: Middleware> From<SimulateValidationError<M>> for SimulationError {
    fn from(error: SimulateValidationError<M>) -> Self {
        match error {
            SimulateValidationError::UserOperationRejected { message } => {
                SimulationError::owned(SIMULATE_VALIDATION_ERROR_CODE, message, None::<bool>)
            }
            SimulateValidationError::Middleware(_) => {
                SimulationError::from(ErrorCode::InternalError)
            }
        }
    }
}
