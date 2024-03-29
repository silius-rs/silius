use crate::{
    mempool::{UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Mempool, Reputation, SimulationError,
};
use ethers::providers::Middleware;
use silius_contracts::entry_point::SELECTORS_INDICES;
use silius_primitives::{
    constants::validation::entities::{FACTORY, LEVEL_TO_ENTITY},
    simulation::{CREATE2_OPCODE, FORBIDDEN_OPCODES},
    UserOperation,
};

#[derive(Clone)]
pub struct Opcodes;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for Opcodes {
    /// The [check_user_operation] method implementation that checks the use of forbidden opcodes
    ///
    /// # Arguments
    /// `_uo` - Not used
    /// `helper` - The [SimulationTraceHelper]
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            let level = SELECTORS_INDICES.get(call_info.top_level_method_sig.as_ref()).cloned();

            if let Some(l) = level {
                // [OP-011] - block opcodes
                for op in call_info.opcodes.keys() {
                    if FORBIDDEN_OPCODES.contains(op) {
                        return Err(SimulationError::Opcode {
                            entity: LEVEL_TO_ENTITY[l].to_string(),
                            opcode: op.clone(),
                        });
                    }
                }

                // [OP-031] - CREATE2 is allowed exactly once in the deployment phase and must
                // deploy code for the "sender" address
                if let Some(c) = call_info.opcodes.get(&*CREATE2_OPCODE) {
                    if LEVEL_TO_ENTITY[l] == FACTORY && *c == 1 {
                        continue;
                    }
                    return Err(SimulationError::Opcode {
                        entity: LEVEL_TO_ENTITY[l].to_string(),
                        opcode: CREATE2_OPCODE.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}
