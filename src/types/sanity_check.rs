use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
};
use jsonrpsee::types::{error::ErrorCode, ErrorObject};

const SANITY_CHECK_ERROR_CODE: i32 = -32602;

pub type SanityCheckError = ErrorObject<'static>;

#[derive(Debug)]
pub enum BadUserOperationError<M: Middleware> {
    SenderOrInitCode {
        sender: Address,
        init_code: Bytes,
    },
    FactoryVerification {
        init_code: Bytes,
    },
    HighVerificationGasLimit {
        verification_gas_limit: U256,
        max_verification_gas: U256,
    },
    LowPreVerificationGas {
        pre_verification_gas: U256,
        calculated_pre_verification_gas: U256,
    },
    PaymasterVerification {
        paymaster_and_data: Bytes,
    },
    LowCallGasLimit {
        call_gas_limit: U256,
        call_gas_estimation: U256,
    },
    LowMaxFeePerGas {
        max_fee_per_gas: U256,
        max_fee_per_gas_estimated: U256,
    },
    HighMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        max_fee_per_gas: U256,
    },
    LowMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        min_priority_fee_per_gas: U256,
    },
    SenderVerification {
        sender: Address,
    },
    Middleware(M::Error),
}

impl<M: Middleware> From<BadUserOperationError<M>> for SanityCheckError {
    fn from(error: BadUserOperationError<M>) -> Self {
        match error {
            BadUserOperationError::SenderOrInitCode { sender, init_code } => {
                SanityCheckError::owned(
                    SANITY_CHECK_ERROR_CODE,
                    format!(
                        "Either the sender {sender} is an existing contract, or the initCode {init_code} is not empty (but not both)",
                    ),
                    None::<bool>,
                )
            },
            BadUserOperationError::FactoryVerification { init_code } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!("Init code {init_code} is not valid (factory check)",),
                None::<bool>,
            ),
            BadUserOperationError::HighVerificationGasLimit {
                verification_gas_limit,
                max_verification_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Verification gas limit {verification_gas_limit} is higher than max verification gas {max_verification_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowPreVerificationGas {
                pre_verification_gas,
                calculated_pre_verification_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Pre-verification gas {pre_verification_gas} is lower than calculated pre-verification gas {calculated_pre_verification_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::PaymasterVerification { paymaster_and_data } => {
                SanityCheckError::owned(
                    SANITY_CHECK_ERROR_CODE,
                    format!(
                        "Paymaster and data {paymaster_and_data} is invalid (paymaster check)",
                    ),
                    None::<bool>,
                )
            },
            BadUserOperationError::LowCallGasLimit {
                call_gas_limit,
                call_gas_estimation,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Call gas limit {call_gas_limit} is lower than call gas estimation {call_gas_estimation}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowMaxFeePerGas {
                max_fee_per_gas,
                max_fee_per_gas_estimated,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max fee per gas {max_fee_per_gas} is lower than estimated max fee per gas {max_fee_per_gas_estimated}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                max_fee_per_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is higher than max fee per gas {max_fee_per_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                min_priority_fee_per_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is lower than min priority fee per gas {min_priority_fee_per_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::SenderVerification { sender } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!("Sender {sender} is invalid (sender check)",),
                None::<bool>,
            ),
            BadUserOperationError::Middleware(_) => SanityCheckError::from(ErrorCode::InternalError),
        }
    }
}
