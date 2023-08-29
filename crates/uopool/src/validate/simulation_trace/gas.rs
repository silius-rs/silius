use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    reputation::ReputationEntry, simulation::SimulationCheckError, UserOperation,
};

pub struct Gas;

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SimulationTraceCheck<M, P, R, E> for Gas
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    /// The [check_user_operation] method implementation that checks if the user operation runs out of gas
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M, P, R, E>,
    ) -> Result<(), SimulationCheckError> {
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            if call_info.oog.unwrap_or(false) {
                return Err(SimulationCheckError::OutOfGas {});
            }
        }

        Ok(())
    }
}
