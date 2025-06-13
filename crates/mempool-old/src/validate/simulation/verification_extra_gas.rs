use crate::{
    validate::{SimulationCheck, SimulationHelper},
    SimulationError,
};
use silius_contracts::entry_point::SimulateValidationResult;
use silius_primitives::{constants::validation::simulation::MIN_EXTRA_GAS, UserOperation};

#[derive(Clone)]
pub struct VerificationExtraGas;

impl SimulationCheck for VerificationExtraGas {
    /// The method implementation validates the needed extra gas.
    ///
    /// # Arguments
    /// `uo` - Not used in this check
    /// `helper` - The [SimulationHelper]
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationHelper,
    ) -> Result<(), SimulationError> {
        let pre_op_gas = match helper.simulate_validation_result {
            SimulateValidationResult::ValidationResult(res) => res.return_info.0,
            SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.0,
        };

        let extra_gas = uo.verification_gas_limit - (pre_op_gas - uo.pre_verification_gas);

        // If account is not deployed, check against MIN_EXTRA_GAS, else MIN_EXTRA_GAS / 2
        let extra_gas_check =
            if uo.init_code.is_empty() { MIN_EXTRA_GAS / 2 } else { MIN_EXTRA_GAS };

        if extra_gas.as_u64() < extra_gas_check {
            return Err(SimulationError::Validation {
                inner: format!(
                    "Verification gas should have extra ${extra_gas_check} gas (has ${extra_gas})"
                ),
            });
        }

        Ok(())
    }
}
