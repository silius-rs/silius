//! Mempool/related primitives

use crate::{sanity::SanityCheckError, simulation::SimulationCheckError};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, EnumVariantNames};

/// Verification modes for user operation mempool
#[derive(Clone, Copy, Debug, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Mode {
    Standard,
    Unsafe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    Sanity(SanityCheckError),
    Simulation(SimulationCheckError),
}

impl From<SanityCheckError> for ValidationError {
    fn from(err: SanityCheckError) -> Self {
        Self::Sanity(err)
    }
}

impl From<SimulationCheckError> for ValidationError {
    fn from(err: SimulationCheckError) -> Self {
        Self::Simulation(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AddError {
    Verification(ValidationError),
    MempoolError { message: String },
}

impl From<ValidationError> for AddError {
    fn from(err: ValidationError) -> Self {
        Self::Verification(err)
    }
}
