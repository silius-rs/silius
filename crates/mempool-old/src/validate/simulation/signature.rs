use crate::{
    validate::{SimulationCheck, SimulationHelper},
    SimulationError,
};
use silius_contracts::entry_point::SimulateValidationResult;
use silius_primitives::UserOperation;

#[derive(Clone)]
pub struct Signature;

impl SimulationCheck for Signature {
    /// The method implementation that validates the signature of the user operation.
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationHelper]
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationHelper,
    ) -> Result<(), SimulationError> {
        let sig_check = match helper.simulate_validation_result {
            SimulateValidationResult::ValidationResult(res) => res.return_info.2,
            SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.2,
        };

        if sig_check {
            return Err(SimulationError::Signature {});
        }

        Ok(())
    }
}
