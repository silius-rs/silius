use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    consts::entities::{FACTORY, LEVEL_TO_ENTITY},
    reputation::ReputationEntry,
    simulation::{SimulationCheckError, CREATE2_OPCODE, FORBIDDEN_OPCODES},
    UserOperation,
};

pub struct Opcodes;

#[async_trait::async_trait]
impl<M: Middleware, P, R> SimulationTraceCheck<M, P, R> for Opcodes
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    /// The [check_user_operation] method implementation that checks the use of forbidden opcodes
    ///
    /// # Arguments
    /// `_uo` - Not used
    /// `helper` - The [SimulationTraceHelper]
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
                for op in l.opcodes.keys() {
                    if FORBIDDEN_OPCODES.contains(op) {
                        return Err(SimulationCheckError::Opcode {
                            entity: LEVEL_TO_ENTITY[i].to_string(),
                            opcode: op.clone(),
                        });
                    }
                }
            }

            if let Some(l) = helper.js_trace.number_levels.get(i) {
                if let Some(c) = l.opcodes.get(&*CREATE2_OPCODE) {
                    if LEVEL_TO_ENTITY[i] == FACTORY && *c == 1 {
                        continue;
                    }
                    return Err(SimulationCheckError::Opcode {
                        entity: LEVEL_TO_ENTITY[i].to_string(),
                        opcode: CREATE2_OPCODE.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}
