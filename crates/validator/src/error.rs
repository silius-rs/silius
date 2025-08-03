use silius_chain::error::ChainError;
use thiserror::Error;

use crate::{sanity_checks::error::SanityCheckError, tracing_checks::error::TracingCheckError};

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid user operation signature or paymaster signature")]
    SignatureError,
    #[error("{0}")]
    NotInTimeRange(String),
    #[error("{0}")]
    SanityError(SanityCheckError),
    #[error("{0}")]
    TracingError(TracingCheckError),
    #[error("{0}")]
    ChainError(ChainError),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<SanityCheckError> for ValidationError {
    fn from(error: SanityCheckError) -> Self {
        ValidationError::SanityError(error)
    }
}

impl From<TracingCheckError> for ValidationError {
    fn from(error: TracingCheckError) -> Self {
        ValidationError::TracingError(error)
    }
}

impl From<ChainError> for ValidationError {
    fn from(error: ChainError) -> Self {
        ValidationError::ChainError(error)
    }
}
