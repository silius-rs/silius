use silius_chain::error::ChainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TracingCheckError {
    #[error("{0}")]
    Check(String),

    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for TracingCheckError {
    fn from(err: anyhow::Error) -> Self {
        TracingCheckError::Other(err.to_string())
    }
}
