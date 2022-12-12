use ethers::providers::ProviderError;

use crate::{types::user_operation::UserOperation, uopool::services::uopool::UoPoolService};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BadUserOperationError {}

#[derive(Debug)]
pub enum UserOperationSimulationError {
    Simulation(BadUserOperationError),
    Internal(anyhow::Error),
    Provider(ProviderError),
}

impl From<anyhow::Error> for UserOperationSimulationError {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSimulationError::Internal(e)
    }
}

impl From<ProviderError> for UserOperationSimulationError {
    fn from(e: ProviderError) -> Self {
        UserOperationSimulationError::Provider(e)
    }
}

impl UoPoolService {
    async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSimulationError> {
        Ok(())
    }
}
