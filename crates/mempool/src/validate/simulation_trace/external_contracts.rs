use crate::{
    mempool::Mempool,
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation, SimulationError,
};
use ethers::providers::Middleware;
use silius_contracts::entry_point::SELECTORS_INDICES;
use silius_primitives::{
    constants::validation::entities::LEVEL_TO_ENTITY, simulation::CREATE2_OPCODE, UserOperation,
};

#[derive(Clone)]
pub struct ExternalContracts;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for ExternalContracts {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            let level = SELECTORS_INDICES.get(call_info.top_level_method_sig.as_ref()).cloned();

            if let Some(l) = level {
                // [OP-041] - access to an address without a deployed code is forbidden for EXTCODE*
                // and *CALL opcodes
                for (addr, size) in call_info.contract_size.iter() {
                    if *addr != uo.sender // [OP-042] - exception: access to "sender" address is allowed
                        && size.contract_size <= 2
                        && size.opcode != CREATE2_OPCODE.to_string()
                    {
                        return Err(SimulationError::Opcode {
                            entity: LEVEL_TO_ENTITY[l].into(),
                            opcode: size.opcode.clone(),
                        });
                    }
                }

                for (addr, info) in call_info.ext_code_access_info.iter() {
                    if *addr == helper.entry_point.address() {
                        return Err(SimulationError::Opcode {
                            entity: LEVEL_TO_ENTITY[l].into(),
                            opcode: info.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
