use crate::validate::{SimulationCheck, SimulationHelper};
use silius_contracts::entry_point::SimulateValidationResult;
use silius_primitives::{simulation::SimulationCheckError, UserOperation};

pub struct Signature;

impl SimulationCheck for Signature {
    /// The [check_user_operation] method implementation that validates the signature of the user operation.
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationHelper](crate::validate::SimulationHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationHelper,
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
