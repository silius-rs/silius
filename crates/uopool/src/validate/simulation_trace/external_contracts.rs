use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_contracts::entry_point::SELECTORS_INDICES;
use silius_primitives::{
    consts::entities::LEVEL_TO_ENTITY,
    simulation::{SimulationCheckError, CREATE2_OPCODE},
    UserOperation,
};

#[derive(Clone)]
pub struct ExternalContracts;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for ExternalContracts {
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
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
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            let level = SELECTORS_INDICES
                .get(call_info.top_level_method_sig.as_ref())
                .cloned();

            if let Some(l) = level {
                // [OP-041] - access to an address without a deployed code is forbidden for EXTCODE* and *CALL opcodes
                for (addr, size) in call_info.contract_size.iter() {
                    if *addr != uo.sender // [OP-042] - exception: access to "sender" address is allowed
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
