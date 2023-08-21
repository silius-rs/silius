use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    consts::entities::LEVEL_TO_ENTITY,
    reputation::ReputationEntry,
    simulation::{SimulationCheckError, CREATE2_OPCODE},
    UserOperation,
};

pub struct ExternalContracts;

#[async_trait::async_trait]
impl<M: Middleware, P, R> SimulationTraceCheck<M, P, R> for ExternalContracts
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M, P, R>,
    ) -> Result<(), SimulationCheckError> {
        for (i, _) in LEVEL_TO_ENTITY.iter().enumerate() {
            if let Some(l) = helper.js_trace.number_levels.get(i) {
                for (addr, size) in l.contract_size.iter() {
                    if *addr != uo.sender
                        && size.contract_size <= 2
                        && size.opcode != CREATE2_OPCODE.to_string()
                    {
                        return Err(SimulationCheckError::Opcode {
                            entity: LEVEL_TO_ENTITY[i].to_string(),
                            opcode: size.opcode.clone(),
                        });
                    }
                }

                // TODO: uncomment when bundler spec tests updated
                // for (addr, info) in l.ext_code_access_info.iter() {
                //     if *addr == helper.entry_point.address() {
                //         return Err(SimulationCheckError::Opcode {
                //             entity: LEVEL_TO_ENTITY[i].to_string(),
                //             opcode: info.clone(),
                //         });
                //     }
                // }
            }
        }

        Ok(())
    }
}
