use crate::{
    validate::{SimulationCheck, SimulationHelper},
    SimulationError,
};
use silius_primitives::{
    constants::validation::simulation::MIN_EXTRA_GAS, entrypoint::SimulateValidationResult,
    UserOperation,
};

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
            SimulateValidationResult::ValidationResult(res) => res.return_info.pre_op_gas,
        };
        let extra_gas = uo.verification_gas_limit - (pre_op_gas - uo.pre_verification_gas);

        if extra_gas.as_u64() < MIN_EXTRA_GAS {
            return Err(SimulationError::Validation {
                inner: format!("Verification gas should have extra 2000 gas (has ${extra_gas})"),
            });
        }

        Ok(())
    }
}
