use thiserror::Error;

#[derive(Error, Debug)]
pub enum SanityCheckError {
    #[error("{0}")]
    Check(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for SanityCheckError {
    fn from(err: anyhow::Error) -> Self {
        SanityCheckError::Other(err.to_string())
    }
}
