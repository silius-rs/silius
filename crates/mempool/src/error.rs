use silius_chain::error::ChainError;
use silius_storage::error::DatabaseError;
use silius_validator::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
}
