use alloy_primitives::Bytes;
use alloy_sol_types::{SolError, SolInterface, sol};
use thiserror::Error;

use crate::entry_point::IEntryPoint::IEntryPointErrors;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    library Errors {
        error Error(string);
    }
}

#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Chain id mismatch: expected {expected}, got {got}")]
    ChainIdMismatch { expected: u64, got: u64 },

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Contract reverted: {0}")]
    Revert(Bytes),

    #[error("{0}")]
    Unknown(String),
}

pub enum RevertReason {
    InvalidSignature,
    InvalidPaymasterSignature,
    False,
    Unknown,
}

pub fn decode_revert_reason(data: Bytes) -> RevertReason {
    if let Ok(error) = Errors::Error::abi_decode(&data) {
        return if error.0.starts_with("AA90") {
            RevertReason::False
        } else {
            RevertReason::Unknown
        };
    }
    match IEntryPointErrors::abi_decode(&data) {
        Ok(decoded_error) => match decoded_error {
            IEntryPointErrors::FailedOp(failed_op) => {
                if failed_op.reason.starts_with("AA24") {
                    RevertReason::InvalidSignature
                } else if failed_op.reason.starts_with("AA34") {
                    RevertReason::InvalidPaymasterSignature
                } else if failed_op.reason.starts_with("AA95") {
                    RevertReason::False
                } else {
                    RevertReason::Unknown
                }
            }
            _ => RevertReason::Unknown,
        },
        Err(_) => RevertReason::Unknown,
    }
}
