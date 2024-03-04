use crate::{
    mempool::Mempool,
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation, SimulationError,
};
use ethers::providers::Middleware;
use silius_primitives::UserOperation;

#[derive(Clone)]
pub struct Gas;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for Gas {
    /// The method implementation that checks if the user operation runs out
    /// of gas
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        // [OP-020] - revert on "out of gas" is forbidden as it can "leak" the gas limit or the
        // current call stack depth
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            if call_info.oog.unwrap_or(false) {
                return Err(SimulationError::OutOfGas {});
            }
        }

        Ok(())
    }
}
