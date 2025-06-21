use silius_storage::error::DatabaseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
}
