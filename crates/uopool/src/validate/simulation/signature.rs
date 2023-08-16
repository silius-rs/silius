use crate::validate::{SimulationCheck, SimulationHelper};
use ethers::providers::Middleware;
use silius_contracts::entry_point::SimulateValidationResult;
use silius_primitives::{simulation::SimulationCheckError, UserOperation};

#[derive(Debug)]
pub struct Signature;

#[async_trait::async_trait]
impl<M: Middleware> SimulationCheck<M> for Signature {
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationHelper<M>,
    ) -> Result<(), SimulationCheckError> {
        let sig_check = match helper.simulate_validation_result {
            SimulateValidationResult::ValidationResult(res) => res.return_info.2,
            SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.2,
        };

        if sig_check {
            return Err(SimulationCheckError::Signature {});
        }

        Ok(())
    }
}
