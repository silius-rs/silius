use crate::validate::{SimulationTraceCheck, SimulationTraceHelper};
use ethers::providers::Middleware;
use silius_primitives::{
    consts::entities::LEVEL_TO_ENTITY,
    simulation::{SimulationCheckError, CREATE2_OPCODE},
    UserOperation,
};

pub struct ExternalContracts;

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for ExternalContracts {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M>,
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
            }
        }

        Ok(())
    }
}
