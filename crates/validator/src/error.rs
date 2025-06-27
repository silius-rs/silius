use crate::{
    sanity_checks::error::SanityCheckError, simulation_checks::error::SimulationCheckError,
    tracing_check::error::TracingCheckError,
};

pub enum ValidationError {
    SanityError(SanityCheckError),
    SimulationError(SimulationCheckError),
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
