use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_contracts::entry_point::SELECTORS_INDICES;
use silius_primitives::{
    consts::entities::LEVEL_TO_ENTITY,
    reputation::ReputationEntry,
    simulation::{SimulationCheckError, CREATE2_OPCODE},
    UserOperation,
};

pub struct ExternalContracts;

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SimulationTraceCheck<M, P, R, E> for ExternalContracts
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M, P, R, E>,
    ) -> Result<(), SimulationCheckError> {
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            let level = SELECTORS_INDICES
                .get(call_info.top_level_method_sig.as_ref())
                .cloned();

            if let Some(l) = level {
                for (addr, size) in call_info.contract_size.iter() {
                    if *addr != uo.sender
                        && size.contract_size <= 2
                        && size.opcode != CREATE2_OPCODE.to_string()
                    {
                        return Err(SimulationCheckError::Opcode {
                            entity: LEVEL_TO_ENTITY[l].to_string(),
                            opcode: size.opcode.clone(),
                        });
                    }
                }

                for (addr, info) in call_info.ext_code_access_info.iter() {
                    if *addr == helper.entry_point.address() {
                        return Err(SimulationCheckError::Opcode {
                            entity: LEVEL_TO_ENTITY[l].to_string(),
                            opcode: info.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
