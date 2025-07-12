use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Chain id mismatch: expected {expected}, got {got}")]
    ChainIdMismatch { expected: u64, got: u64 },

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("{0}")]
    Unknown(String),
}
