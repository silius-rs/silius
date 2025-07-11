use thiserror::Error;

use crate::{
    sanity_checks::error::SanityCheckError, simulation_checks::error::SimulationCheckError,
    tracing_check::error::TracingCheckError,
};

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("{0}")]
    SanityError(SanityCheckError),
    #[error("{0}")]
    SimulationError(SimulationCheckError),
    #[error("{0}")]
    TracingError(TracingCheckError),
}

impl From<SanityCheckError> for ValidationError {
    fn from(error: SanityCheckError) -> Self {
        ValidationError::SanityError(error)
    }
}

impl From<SimulationCheckError> for ValidationError {
    fn from(error: SimulationCheckError) -> Self {
        ValidationError::SimulationError(error)
    }
}

impl From<TracingCheckError> for ValidationError {
    fn from(error: TracingCheckError) -> Self {
        ValidationError::TracingError(error)
    }
}
