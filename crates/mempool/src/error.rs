#[cfg(feature = "mdbx")]
use crate::DatabaseError;
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};
use silius_contracts::EntryPointError;
use silius_primitives::UserOperationHash;
use thiserror::Error;

pub type MempoolResult<T> = Result<T, MempoolError>;

/// A trait for additional errors that can be thrown by the transaction pool.
pub trait MempoolUserOperationError: std::error::Error + Send + Sync {
    fn is_bad_user_operation(&self) -> bool;
}

// Needed for `#[error(transparent)]`
impl std::error::Error for Box<dyn MempoolUserOperationError> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        (**self).source()
    }
}

/// Mempool error
#[derive(Debug, Error, Serialize, Deserialize)]
#[error("{kind}")]
pub struct MempoolError {
    /// The user operation hash that caused the error
    pub hash: UserOperationHash,
    /// The error kind
    pub kind: MempoolErrorKind,
}

/// Mempool error kind
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum MempoolErrorKind {
    /// User operation rejected because validation failed
    #[error(transparent)]
    InvalidUserOperation(#[from] InvalidMempoolUserOperationError),
    /// Provider error
    #[error("provider error: {inner}")]
    Provider {
        /// The inner error message
        inner: String,
    },
    /// Database error
    #[cfg(feature = "mdbx")]
    #[error(transparent)]
    Database(DatabaseError),
    /// Any other error
    #[error("other error: {inner}")]
    Other {
        /// The inner error message
        inner: String,
    },
}

impl From<ReputationError> for MempoolErrorKind {
    fn from(err: ReputationError) -> Self {
        MempoolErrorKind::InvalidUserOperation(InvalidMempoolUserOperationError::Reputation(err))
    }
}

impl From<SanityError> for MempoolErrorKind {
    fn from(err: SanityError) -> Self {
        MempoolErrorKind::InvalidUserOperation(InvalidMempoolUserOperationError::Sanity(err))
    }
}

impl From<SimulationError> for MempoolErrorKind {
    fn from(err: SimulationError) -> Self {
        MempoolErrorKind::InvalidUserOperation(InvalidMempoolUserOperationError::Simulation(err))
    }
}

#[cfg(feature = "mdbx")]
impl From<reth_db::Error> for MempoolErrorKind {
    fn from(e: reth_db::Error) -> Self {
        Self::Database(e.into())
    }
}

/// Error when validating user operation failed
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum InvalidMempoolUserOperationError {
    /// User operation rejected because of the reputation of the entities
    #[error(transparent)]
    Reputation(#[from] ReputationError),
    /// User operation rejected because sanity check failed
    #[error(transparent)]
    Sanity(#[from] SanityError),
    /// User operation rejected because simulation check failed
    #[error(transparent)]
    Simulation(#[from] SimulationError),
}

/// Error related to reputation of the entities
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ReputationError {
    /// Entity is banned
    #[error("{entity} {address:?} is banned")]
    BannedEntity { entity: String, address: Address },
    /// Entity is throttled
    #[error("{entity} {address:?} is throttled")]
    ThrottledEntity { entity: String, address: Address },
    /// Stake of the entity is too low
    #[error("{entity} {address:?} stake {stake} is too low {min_stake}")]
    StakeTooLow { entity: String, address: Address, stake: U256, min_stake: U256 },
    /// Unstake delay of the entity is too low
    #[error("{entity} {address:?} unstake delay {unstake_delay} is too low {min_unstake_delay}")]
    UnstakeDelayTooLow {
        address: Address,
        entity: String,
        unstake_delay: U256,
        min_unstake_delay: U256,
    },
    /// Entity is unstaked
    #[error("{entity} {address:?} is unstaked")]
    UnstakedEntity { entity: String, address: Address },
    /// Database error
    #[cfg(feature = "mdbx")]
    #[error(transparent)]
    Database(DatabaseError),
}

#[cfg(feature = "mdbx")]
impl From<reth_db::Error> for ReputationError {
    fn from(e: reth_db::Error) -> Self {
        Self::Database(e.into())
    }
}

/// Error when sanity check fails
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SanityError {
    /// Verification gas limit is too high
    #[error("verificationGasLimit too high: expected at most {verification_gas_limit_expected}")]
    VerificationGasLimitTooHigh {
        verification_gas_limit: U256,
        verification_gas_limit_expected: U256,
    },
    /// Pre verification gas is too low
    #[error("preVerificationGas too low: expected at least {pre_verification_gas_expected}")]
    PreVerificationGasTooLow { pre_verification_gas: U256, pre_verification_gas_expected: U256 },
    /// Call gas limit is too low
    #[error("callGasLimit too low: expected at least {call_gas_limit_expected}")]
    CallGasLimitTooLow { call_gas_limit: U256, call_gas_limit_expected: U256 },
    /// Max fee per gas is too low (lower than current base fee per gas)
    #[error("maxFeePerGas too low: expected at least {base_fee_per_gas}")]
    MaxFeePerGasTooLow { max_fee_per_gas: U256, base_fee_per_gas: U256 },
    /// Max priority fee per gas is too high (higher than max fee per gas)
    #[error("maxPriorityFeePerGas too high: expected at most {max_fee_per_gas}")]
    MaxPriorityFeePerGasTooHigh { max_priority_fee_per_gas: U256, max_fee_per_gas: U256 },
    /// Max priority fee per gas is too low (lower than this bundler accepts)
    #[error("maxPriorityFeePerGas too low: expected at least {max_priority_fee_per_gas_expected}")]
    MaxPriorityFeePerGasTooLow {
        max_priority_fee_per_gas: U256,
        max_priority_fee_per_gas_expected: U256,
    },
    /// Paymaster validation failed
    #[error("{inner}")]
    Paymaster { inner: String },
    /// Sender validation failed
    #[error("{inner}")]
    Sender { inner: String },
    /// Entity role validation
    #[error("A {entity} at {address:?} in this user operation is used as a {entity_other} entity in another useroperation currently in mempool")]
    EntityRoles { entity: String, address: Address, entity_other: String },
    /// Reputation error
    #[error(transparent)]
    Reputation(ReputationError),
    /// Provider error
    #[error("provider error: {inner}")]
    Provider {
        /// The inner error message
        inner: String,
    },
    /// Database error
    #[cfg(feature = "mdbx")]
    #[error(transparent)]
    Database(DatabaseError),
    /// Any other error
    #[error("other error: {inner}")]
    Other {
        /// The inner error message
        inner: String,
    },
}

impl From<ReputationError> for SanityError {
    fn from(err: ReputationError) -> Self {
        SanityError::Reputation(err)
    }
}

impl From<EntryPointError> for SanityError {
    fn from(err: EntryPointError) -> Self {
        match err {
            EntryPointError::Provider { inner } => SanityError::Provider { inner },
            _ => SanityError::Other { inner: err.to_string() },
        }
    }
}

/// Error when simulation fails
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SimulationError {
    /// Signature verification failed
    #[error("Invalid userop signature or paymaster signature")]
    Signature,
    /// User operation timestamp invalid
    #[error("{inner}")]
    Timestamp { inner: String },
    /// Error during user operation validation
    #[error("{inner}")]
    Validation { inner: String },
    /// Error during user operation execution
    #[error("{inner}")]
    Execution { inner: String },
    /// Opcode error
    #[error("{entity} uses banned opcode: {opcode}")]
    Opcode { entity: String, opcode: String },
    /// Storage access error
    #[error("Storage access validation failed for slot: {slot}")]
    StorageAccess { slot: String },
    /// Unstaked entity did something it shouldn't
    #[error("A unstaked {entity} at {address:?}: {inner}")]
    Unstaked { entity: String, address: Address, inner: String },
    /// Errors related to calls
    #[error("Illegal call into {inner}")]
    CallStack { inner: String },
    /// Codes hashes changed between the first and the second simulations
    #[error("Code hashes changed between the first and the second simulations")]
    CodeHashes,
    /// User operation out of gas
    #[error("User operation out of gas")]
    OutOfGas,
    /// Reputation error
    #[error(transparent)]
    Reputation(ReputationError),
    /// Provider error
    #[error("provider error: {inner}")]
    Provider {
        /// The inner error message
        inner: String,
    },
    /// Database error
    #[cfg(feature = "mdbx")]
    #[error(transparent)]
    Database(DatabaseError),
    /// Any other error
    #[error("other error: {inner}")]
    Other {
        /// The inner error message
        inner: String,
    },
}

impl From<ReputationError> for SimulationError {
    fn from(err: ReputationError) -> Self {
        SimulationError::Reputation(err)
    }
}

impl From<EntryPointError> for SimulationError {
    fn from(err: EntryPointError) -> Self {
        match err {
            EntryPointError::FailedOp(op) => SimulationError::Execution { inner: op.to_string() },
            EntryPointError::Provider { inner } => SimulationError::Provider { inner },
            _ => SimulationError::Other { inner: err.to_string() },
        }
    }
}
