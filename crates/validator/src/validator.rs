use crate::sanity_checks::SanityCheck;
use crate::simulation_checks::SimulationCheck;
use crate::tracing_check::TracingCheck;

pub struct Validator {
    pub sanity_checks: Vec<Box<dyn SanityCheck + Send + Sync>>,
    pub simulation_checks: Vec<Box<dyn SimulationCheck + Send + Sync>>,
    pub tracing_checks: Vec<Box<dyn TracingCheck + Send + Sync>>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn standard() -> Self {
        Self {
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn _unsafe() -> Self {
        Self {
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }
}
