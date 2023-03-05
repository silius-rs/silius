use ethers::{
    providers::Middleware,
    types::{Address, GethTrace, U256},
};

use crate::{
    contracts::{tracer::JsTracerFrame, EntryPointErr, SimulateValidationResult},
    types::{
        simulation::{SimulateValidationError, CREATE2_OPCODE, FORBIDDEN_OPCODES, LEVEL_TO_ENTITY},
        user_operation::UserOperation,
    },
    uopool::mempool_id,
};

use super::UoPoolService;

impl<M: Middleware + 'static> UoPoolService<M>
where
    EntryPointErr<M>: From<<M as Middleware>::Error>,
{
    async fn simulate_validation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<SimulateValidationResult, SimulateValidationError<M>> {
        let mempool_id = mempool_id(entry_point, &self.chain_id);

        if let Some(entry_point) = self.entry_points.get(&mempool_id) {
            return match entry_point
                .simulate_validation(user_operation.clone())
                .await
            {
                Ok(simulate_validation_result) => Ok(simulate_validation_result),
                Err(entry_point_error) => match entry_point_error {
                    EntryPointErr::MiddlewareErr(middleware_error) => {
                        Err(SimulateValidationError::Middleware(middleware_error))
                    }
                    EntryPointErr::FailedOp(failed_op) => {
                        Err(SimulateValidationError::UserOperationRejected {
                            message: format!("{failed_op}"),
                        })
                    }
                    _ => Err(SimulateValidationError::UserOperationRejected {
                        message: "unknown error".to_string(),
                    }),
                },
            };
        }

        Err(SimulateValidationError::UserOperationRejected {
            message: "invalid entry point".to_string(),
        })
    }

    async fn simulate_validation_trace(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<GethTrace, SimulateValidationError<M>> {
        let mempool_id = mempool_id(entry_point, &self.chain_id);

        if let Some(entry_point) = self.entry_points.get(&mempool_id) {
            return match entry_point
                .simulate_validation_trace(user_operation.clone())
                .await
            {
                Ok(geth_trace) => Ok(geth_trace),
                Err(entry_point_error) => match entry_point_error {
                    EntryPointErr::MiddlewareErr(middleware_error) => {
                        Err(SimulateValidationError::Middleware(middleware_error))
                    }
                    EntryPointErr::FailedOp(failed_op) => {
                        Err(SimulateValidationError::UserOperationRejected {
                            message: format!("{failed_op}"),
                        })
                    }
                    _ => Err(SimulateValidationError::UserOperationRejected {
                        message: "unknown error".to_string(),
                    }),
                },
            };
        }

        Err(SimulateValidationError::UserOperationRejected {
            message: "invalid entry point".to_string(),
        })
    }

    async fn forbidden_opcodes(
        &self,
        simulate_validation_result: &SimulateValidationResult,
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError<M>> {
        let mut stake_info: Vec<(U256, U256)> = vec![];

        match simulate_validation_result {
            SimulateValidationResult::ValidationResult(validation_result) => {
                stake_info.push(validation_result.factory_info);
                stake_info.push(validation_result.sender_info);
                stake_info.push(validation_result.paymaster_info);
            }
            SimulateValidationResult::ValidationResultWithAggregation(
                validation_result_with_aggregation,
            ) => {
                stake_info.push(validation_result_with_aggregation.factory_info);
                stake_info.push(validation_result_with_aggregation.sender_info);
                stake_info.push(validation_result_with_aggregation.paymaster_info);
            }
        }

        for (index, _) in stake_info.iter().enumerate() {
            for opcode in trace.number_levels[index].opcodes.keys() {
                if FORBIDDEN_OPCODES.contains(opcode) {
                    return Err(SimulateValidationError::OpcodeValidation {
                        entity: LEVEL_TO_ENTITY[&index].to_string(),
                        opcode: opcode.clone(),
                    });
                }
            }

            if let Some(count) = trace.number_levels[index].opcodes.get(&*CREATE2_OPCODE) {
                if LEVEL_TO_ENTITY[&index] == "factory" && *count == 1 {
                    continue;
                }
                return Err(SimulateValidationError::OpcodeValidation {
                    entity: LEVEL_TO_ENTITY[&index].to_string(),
                    opcode: CREATE2_OPCODE.to_string(),
                });
            }
        }

        Ok(())
    }

    pub async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<(), SimulateValidationError<M>> {
        let simulate_validation_result = self
            .simulate_validation(user_operation, entry_point)
            .await?;

        let geth_trace = self
            .simulate_validation_trace(user_operation, entry_point)
            .await?;

        let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace).map_err(|error| {
            SimulateValidationError::UserOperationRejected {
                message: error.to_string(),
            }
        })?;

        // may not invokes any forbidden opcodes
        self.forbidden_opcodes(&simulate_validation_result, &js_trace)
            .await?;

        Ok(())
    }
}
