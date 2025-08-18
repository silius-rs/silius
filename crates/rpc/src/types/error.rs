use std::{error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use silius_mempool::error::MempoolError;
use silius_validator::{error::ValidationError, tracing_checks::error::TracingCheckError};

use crate::types::codes::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, INVALID_SIGNATURE, METHOD_NOT_FOUND,
    OPCODE_STORAGE_VALIDATION, PARSE_ERROR,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorData {
    pub code: i32,
    pub message: String,
    pub data: Value,
}

impl ErrorData {
    pub fn new(code: i32, message: &str) -> Self {
        Self {
            code,
            message: String::from(message),
            data: Value::Null,
        }
    }

    pub fn std(code: i32) -> Self {
        match code {
            PARSE_ERROR => ErrorData::new(PARSE_ERROR, "Parse error"),
            INVALID_REQUEST => ErrorData::new(INVALID_REQUEST, "Invalid Request"),
            METHOD_NOT_FOUND => ErrorData::new(METHOD_NOT_FOUND, "Method not found"),
            INVALID_PARAMS => ErrorData::new(INVALID_PARAMS, "Invalid params"),
            INTERNAL_ERROR => ErrorData::new(INTERNAL_ERROR, "Internal error"),
            _ => panic!("Undefined pre-defined error codes"),
        }
    }

    pub fn dump(&self) -> String {
        serde_json::to_string(self).expect("Should never failed")
    }
}

impl error::Error for ErrorData {}

impl fmt::Display for ErrorData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.code, self.message, self.data)
    }
}

impl From<MempoolError> for ErrorData {
    fn from(error: MempoolError) -> Self {
        match error {
            MempoolError::Validation(e) => match e {
                ValidationError::Signature(e) => ErrorData::new(INVALID_SIGNATURE, &e),
                ValidationError::TracingError(e) => match e {
                    TracingCheckError::BannedOpcode(_)
                    | TracingCheckError::OutOfGas(_)
                    | TracingCheckError::UndeployedContractAccess(_, _, _)
                    | TracingCheckError::IllegalPrecompileAccess(_)
                    | TracingCheckError::ForbiddenSlotAccess(_, _, _, _, _)
                    | TracingCheckError::UnstakedEntitySlotAccess(_, _, _) => {
                        ErrorData::new(OPCODE_STORAGE_VALIDATION, &e.to_string())
                    }
                    _ => ErrorData::new(INTERNAL_ERROR, &e.to_string()), // TODO: expand errors
                },
                _ => ErrorData::new(INTERNAL_ERROR, &e.to_string()), // TODO: expand errors
            },
            MempoolError::Chain(e) => ErrorData::new(INTERNAL_ERROR, &e.to_string()),
            MempoolError::Database(e) => ErrorData::new(INTERNAL_ERROR, &e.to_string()),
        }
    }
}
