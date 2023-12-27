use crate::codes::{
    BANNED_OR_THROTTLED_ENTITY, EXECUTION, OPCODE, SANITY, SIGNATURE, STAKE_TOO_LOW, TIMESTAMP,
    VALIDATION,
};
use jsonrpsee::types::{
    error::{ErrorCode, INTERNAL_ERROR_CODE},
    ErrorObject, ErrorObjectOwned,
};
use silius_mempool::{
    InvalidMempoolUserOperationError, MempoolError, MempoolErrorKind, ReputationError, SanityError,
    SimulationError,
};

/// A wrapper for the [ErrorObjectOwned](ErrorObjectOwned) type.
pub struct JsonRpcError(pub ErrorObjectOwned);

impl From<JsonRpcError> for ErrorObjectOwned {
    /// Convert a [JsonRpcError](JsonRpcError) to a [ErrorObjectOwned](ErrorObjectOwned).
    fn from(err: JsonRpcError) -> Self {
        err.0
    }
}

impl From<ErrorObjectOwned> for JsonRpcError {
    /// Convert a [ErrorObjectOwned](ErrorObjectOwned) to a [JsonRpcError](JsonRpcError).
    fn from(err: ErrorObjectOwned) -> Self {
        JsonRpcError(err)
    }
}

impl From<MempoolError> for JsonRpcError {
    /// Convert a [MempoolError](MempoolError) to a [JsonRpcError](JsonRpcError).
    fn from(err: MempoolError) -> Self {
        match err.kind {
            MempoolErrorKind::InvalidUserOperation(err) => match err {
                InvalidMempoolUserOperationError::Sanity(err) => err.into(),
                InvalidMempoolUserOperationError::Simulation(err) => err.into(),
                InvalidMempoolUserOperationError::Reputation(err) => err.into(),
            },
            _ => ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<bool>).into(),
        }
    }
}

impl From<ReputationError> for JsonRpcError {
    /// Convert a [ReputationError](ReputationError) to a [JsonRpcError](JsonRpcError).
    fn from(err: ReputationError) -> Self {
        JsonRpcError(match err {
            ReputationError::BannedEntity { entity: _, address: _ } => {
                ErrorObject::owned(BANNED_OR_THROTTLED_ENTITY, err.to_string(), None::<bool>)
            }
            ReputationError::ThrottledEntity { entity: _, address: _ } => {
                ErrorObject::owned(BANNED_OR_THROTTLED_ENTITY, err.to_string(), None::<bool>)
            }
            ReputationError::StakeTooLow { entity: _, address: _, stake: _, min_stake: _ } => {
                ErrorObject::owned(STAKE_TOO_LOW, err.to_string(), None::<bool>)
            }
            ReputationError::UnstakeDelayTooLow {
                entity: _,
                address: _,
                unstake_delay: _,
                min_unstake_delay: _,
            } => ErrorObject::owned(STAKE_TOO_LOW, err.to_string(), None::<bool>),
            ReputationError::UnstakedEntity { entity: _, address: _ } => {
                ErrorObject::owned(STAKE_TOO_LOW, err.to_string(), None::<bool>)
            }
            _ => ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<bool>),
        })
    }
}

impl From<SanityError> for JsonRpcError {
    /// Convert a [SanityError](SanityError) to a [JsonRpcError](JsonRpcError).
    fn from(err: SanityError) -> Self {
        JsonRpcError(match err {
            SanityError::VerificationGasLimitTooHigh {
                verification_gas_limit: _,
                verification_gas_limit_expected: _,
            } => ErrorObject::owned(SANITY, err.to_string(), None::<bool>),
            SanityError::PreVerificationGasTooLow {
                pre_verification_gas: _,
                pre_verification_gas_expected: _,
            } => ErrorObject::owned(SANITY, err.to_string(), None::<bool>),
            SanityError::CallGasLimitTooLow { call_gas_limit: _, call_gas_limit_expected: _ } => {
                ErrorObject::owned(SANITY, err.to_string(), None::<bool>)
            }
            SanityError::MaxFeePerGasTooLow { max_fee_per_gas: _, base_fee_per_gas: _ } => {
                ErrorObject::owned(SANITY, err.to_string(), None::<bool>)
            }
            SanityError::MaxPriorityFeePerGasTooHigh {
                max_priority_fee_per_gas: _,
                max_fee_per_gas: _,
            } => ErrorObject::owned(SANITY, err.to_string(), None::<bool>),
            SanityError::MaxPriorityFeePerGasTooLow {
                max_priority_fee_per_gas: _,
                max_priority_fee_per_gas_expected: _,
            } => ErrorObject::owned(SANITY, err.to_string(), None::<bool>),
            SanityError::Paymaster { inner: _ } => {
                ErrorObject::owned(SANITY, err.to_string(), None::<bool>)
            }
            SanityError::Sender { inner: _ } => {
                ErrorObject::owned(SANITY, err.to_string(), None::<bool>)
            }
            SanityError::EntityRoles { entity: _, address: _, entity_other: _ } => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SanityError::Reputation(err) => JsonRpcError::from(err).0,
            _ => ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<bool>),
        })
    }
}

impl From<SimulationError> for JsonRpcError {
    /// Convert a [SimulationError](SimulationError) to a [JsonRpcError](JsonRpcError).
    fn from(err: SimulationError) -> Self {
        JsonRpcError(match err {
            SimulationError::Signature => {
                ErrorObject::owned(SIGNATURE, err.to_string(), None::<bool>)
            }
            SimulationError::Timestamp { inner: _ } => {
                ErrorObject::owned(TIMESTAMP, err.to_string(), None::<bool>)
            }
            SimulationError::Validation { inner: _ } => {
                ErrorObject::owned(VALIDATION, err.to_string(), None::<bool>)
            }
            SimulationError::Execution { inner: _ } => {
                ErrorObject::owned(EXECUTION, err.to_string(), None::<bool>)
            }
            SimulationError::Opcode { entity: _, opcode: _ } => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::StorageAccess { slot: _ } => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::Unstaked { entity: _, address: _, inner: _ } => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::CallStack { inner: _ } => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::CodeHashes {} => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::OutOfGas {} => {
                ErrorObject::owned(OPCODE, err.to_string(), None::<bool>)
            }
            SimulationError::Reputation(err) => JsonRpcError::from(err).0,
            _ => ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<bool>),
        })
    }
}

impl From<tonic::Status> for JsonRpcError {
    /// Convert a tonic status to a [JsonRpcError](JsonRpcError).
    fn from(s: tonic::Status) -> Self {
        JsonRpcError(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!("gRPC error: {}", s.message()),
            None::<bool>,
        ))
    }
}

impl From<serde_json::Error> for JsonRpcError {
    /// Convert a [serde_json error](serde_json::Error) to a [JsonRpcError](JsonRpcError).
    fn from(err: serde_json::Error) -> Self {
        JsonRpcError(ErrorObject::owned(
            ErrorCode::ParseError.code(),
            format!("JSON serializing error: {err}"),
            None::<bool>,
        ))
    }
}
