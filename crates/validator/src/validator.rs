use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::config::ValidatorConfig;
use crate::error::ValidationError;
use crate::sanity_checks::SanityCheck;
use crate::simulation_checks::SimulationCheck;
use crate::tracing_check::TracingCheck;

pub struct Validator<P: Provider> {
    pub config: ValidatorConfig,
    pub sanity_checks: Vec<Box<dyn SanityCheck<P> + Send + Sync>>,
    pub simulation_checks: Vec<Box<dyn SimulationCheck<P> + Send + Sync>>,
    pub tracing_checks: Vec<Box<dyn TracingCheck<P> + Send + Sync>>,
}

impl<P: Provider> Validator<P> {
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn standard(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn _unsafe(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
        db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), ValidationError> {
        for sanity_check in &self.sanity_checks {
            sanity_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        for simulation_check in &self.simulation_checks {
            simulation_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        for tracing_check in &self.tracing_checks {
            tracing_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        Ok(())
    }
}
