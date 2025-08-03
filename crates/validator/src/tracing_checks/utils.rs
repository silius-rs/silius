use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;

use crate::types::ValidationResult;

pub fn extract_validation_result(frame: &Erc7562Frame) -> ValidationResult {
    ValidationResult::default()
}
