use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    consts::entities::LEVEL_TO_ENTITY, reputation::ReputationEntry,
    simulation::SimulationCheckError, UserOperation,
};

pub struct Gas;

#[async_trait::async_trait]
impl<M: Middleware, P, R> SimulationTraceCheck<M, P, R> for Gas
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
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
        helper: &mut SimulationTraceHelper<M, P, R>,
    ) -> Result<(), SimulationCheckError> {
        for (i, _) in LEVEL_TO_ENTITY.iter().enumerate() {
            if let Some(l) = helper.js_trace.number_levels.get(i) {
                if l.oog.unwrap_or(false) {
                    return Err(SimulationCheckError::OutOfGas {});
                }
            }
        }
        Ok(())
    }
}
