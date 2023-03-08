use std::collections::{HashMap, HashSet};

use ethers::{
    providers::Middleware,
    types::{Address, Bytes, GethTrace},
    utils::keccak256,
};

use crate::{
    contracts::{tracer::JsTracerFrame, EntryPointErr, SimulateValidationResult},
    types::{
        reputation::StakeInfo,
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

    fn extract_stake_info(
        &self,
        user_operation: &UserOperation,
        simulate_validation_result: &SimulateValidationResult,
        stake_info_by_entity: &mut HashMap<usize, StakeInfo>,
    ) {
        let (factory_info, sender_info, paymaster_info) = match simulate_validation_result {
            SimulateValidationResult::ValidationResult(validation_result) => (
                validation_result.factory_info,
                validation_result.sender_info,
                validation_result.paymaster_info,
            ),
            SimulateValidationResult::ValidationResultWithAggregation(
                validation_result_with_aggregation,
            ) => (
                validation_result_with_aggregation.factory_info,
                validation_result_with_aggregation.sender_info,
                validation_result_with_aggregation.paymaster_info,
            ),
        };

        // factory
        stake_info_by_entity.insert(
            0,
            StakeInfo {
                address: if user_operation.init_code.len() >= 20 {
                    Address::from_slice(&user_operation.init_code[0..20])
                } else {
                    Address::zero()
                },
                stake: factory_info.0,
                unstake_delay: factory_info.1,
            },
        );

        // account
        stake_info_by_entity.insert(
            1,
            StakeInfo {
                address: user_operation.sender,
                stake: sender_info.0,
                unstake_delay: sender_info.1,
            },
        );

        // paymaster
        stake_info_by_entity.insert(
            2,
            StakeInfo {
                address: if user_operation.paymaster_and_data.len() >= 20 {
                    Address::from_slice(&user_operation.paymaster_and_data[0..20])
                } else {
                    Address::zero()
                },
                stake: paymaster_info.0,
                unstake_delay: paymaster_info.1,
            },
        );
    }

    fn forbidden_opcodes(
        &self,
        stake_info_by_entity: &HashMap<usize, StakeInfo>,
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError<M>> {
        for index in stake_info_by_entity.keys() {
            if let Some(level) = trace.number_levels.get(*index) {
                for opcode in level.opcodes.keys() {
                    if FORBIDDEN_OPCODES.contains(opcode) {
                        return Err(SimulateValidationError::OpcodeValidation {
                            entity: LEVEL_TO_ENTITY[index].to_string(),
                            opcode: opcode.clone(),
                        });
                    }
                }
            }

            if let Some(level) = trace.number_levels.get(*index) {
                if let Some(count) = level.opcodes.get(&*CREATE2_OPCODE) {
                    if LEVEL_TO_ENTITY[index] == "factory" && *count == 1 {
                        continue;
                    }
                    return Err(SimulateValidationError::OpcodeValidation {
                        entity: LEVEL_TO_ENTITY[index].to_string(),
                        opcode: CREATE2_OPCODE.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    fn parse_slots(
        &self,
        keccak: Vec<Bytes>,
        stake_info_by_entity: &HashMap<usize, StakeInfo>,
        slots: &mut HashMap<Address, HashSet<Bytes>>,
    ) {
        for kecc in keccak {
            for entity in stake_info_by_entity.values() {
                if entity.address.is_zero() {
                    continue;
                }

                let k = keccak256(Bytes::from(kecc.clone()));

                if kecc.starts_with(entity.address.as_bytes()) {
                    slots
                        .entry(entity.address)
                        .or_insert(HashSet::new())
                        .insert(k.into());
                }
            }
        }
    }

    fn storage_access(
        &self,
        stake_info_by_entity: &HashMap<usize, StakeInfo>,
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError<M>> {
        if trace.keccak.is_empty() {
            return Ok(());
        }

        let mut slots = HashMap::new();
        self.parse_slots(trace.keccak.clone(), stake_info_by_entity, &mut slots);

        Ok(())
    }

    pub async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<SimulateValidationResult, SimulateValidationError<M>> {
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

        let mut stake_info_by_entity: HashMap<usize, StakeInfo> = HashMap::new();
        self.extract_stake_info(
            user_operation,
            &simulate_validation_result,
            &mut stake_info_by_entity,
        );

        // may not invokes any forbidden opcodes
        self.forbidden_opcodes(&stake_info_by_entity, &js_trace)?;

        // verify storage access
        self.storage_access(&stake_info_by_entity, &js_trace)?;

        Ok(simulate_validation_result)
    }
}
