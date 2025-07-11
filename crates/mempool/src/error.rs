use silius_storage::error::DatabaseError;
use silius_validator::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
}
