use crate::{
    validate::{utils::unpack_account_validation_data, SimulationCheck, SimulationHelper},
    SimulationError,
};
use silius_primitives::{entrypoint::SimulateValidationResult, UserOperation};

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
            SimulateValidationResult::ValidationResult(res) => {
                let validation_data =
                    unpack_account_validation_data(res.return_info.account_validation_data);
                validation_data.sig_authorizer.is_zero()
            }
        };

        if !sig_check {
            return Err(SimulationError::Signature {});
        }

        Ok(())
    }
}
