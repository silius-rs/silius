//! Sanity check (validation) primitives

use crate::reputation::ReputationError;
use ethers::{
    providers::MiddlewareError,
    types::{Address, Bytes, U256},
};
use serde::{Deserialize, Serialize};

/// Error object for sanity check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SanityCheckError {
    SenderOrInitCode { sender: Address, init_code: Bytes },
    FactoryVerification { init_code: Bytes },
    HighVerificationGasLimit { verification_gas_limit: U256, max_verification_gas: U256 },
    LowPreVerificationGas { pre_verification_gas: U256, pre_verification_gas_expected: U256 },
    PaymasterVerification { paymaster_and_data: Bytes },
    LowCallGasLimit { call_gas_limit: U256, call_gas_limit_expected: U256 },
    LowMaxFeePerGas { max_fee_per_gas: U256, base_fee_per_gas: U256 },
    HighMaxPriorityFeePerGas { max_priority_fee_per_gas: U256, max_fee_per_gas: U256 },
    LowMaxPriorityFeePerGas { max_priority_fee_per_gas: U256, min_priority_fee_per_gas: U256 },
    SenderVerification { sender: Address, message: String },
    EntityVerification { entity: String, address: Address, message: String },
    Reputation(ReputationError),
    Validation { message: String },
    MiddlewareError { message: String },
    UnknownError { message: String },
}

impl From<ReputationError> for SanityCheckError {
    fn from(err: ReputationError) -> Self {
        SanityCheckError::Reputation(err)
    }
}

impl<M: MiddlewareError> From<M> for SanityCheckError {
    fn from(err: M) -> Self {
        SanityCheckError::MiddlewareError { message: err.to_string() }
    }
}
