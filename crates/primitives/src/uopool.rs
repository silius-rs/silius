use crate::{sanity_check::SanityCheckError, simulation::SimulationError};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, EnumVariantNames};

/// Verification modes for user operation mempool
#[derive(Clone, Copy, Debug, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Mode {
    Standard,
    Unsafe,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VerificationError {
    SanityCheck(SanityCheckError),
    Simulation(SimulationError),
}

impl From<SanityCheckError> for VerificationError {
    fn from(err: SanityCheckError) -> Self {
        Self::SanityCheck(err)
    }
}

impl From<SimulationError> for VerificationError {
    fn from(err: SimulationError) -> Self {
        Self::Simulation(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AddError {
    Verification(VerificationError),
    MempoolError { message: String },
}

impl From<VerificationError> for AddError {
    fn from(err: VerificationError) -> Self {
        Self::Verification(err)
    }
}
