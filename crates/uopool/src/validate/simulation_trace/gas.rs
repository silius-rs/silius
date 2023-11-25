use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{simulation::SimulationCheckError, UserOperation};

#[derive(Clone)]
pub struct Gas;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for Gas {
    /// The [check_user_operation] method implementation that checks if the user operation runs out of gas
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        // [OP-020] - revert on "out of gas" is forbidden as it can "leak" the gas limit or the current call stack depth
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            if call_info.oog.unwrap_or(false) {
                return Err(SimulationCheckError::OutOfGas {});
            }
        }

        Ok(())
    }
}
