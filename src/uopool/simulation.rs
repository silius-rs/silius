use super::UoPool;
use crate::{
    contracts::{
        gen::{ValidatePaymasterUserOpReturn, CONTRACTS_FUNCTIONS},
        tracer::{Call, CallEntry, JsTracerFrame},
        EntryPointErr, SimulateValidationResult,
    },
    types::{
        reputation::StakeInfo,
        simulation::{
            CodeHash, SimulateValidationError, CREATE2_OPCODE, CREATE_OPCODE, FORBIDDEN_OPCODES,
            LEVEL_TO_ENTITY, NUMBER_LEVELS, PAYMASTER_VALIDATION_FUNCTION, RETURN_OPCODE,
            REVERT_OPCODE,
        },
        user_operation::UserOperation,
    },
    uopool::utils::equal_code_hashes,
};
use ethers::{
    abi::AbiDecode,
    providers::Middleware,
    types::{Address, Bytes, GethTrace, H256, U256},
    utils::keccak256,
};
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;
use tracing::trace;

#[derive(Debug)]
pub struct SimulationResult {
    pub simulate_validation_result: SimulateValidationResult,
    pub code_hashes: Vec<CodeHash>,
}

impl<M: Middleware + 'static> UoPool<M> {
    async fn simulate_validation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulateValidationResult, SimulateValidationError> {
        match self
            .entry_point
            .simulate_validation(user_operation.clone())
            .await
        {
            Ok(simulate_validation_result) => Ok(simulate_validation_result),
            Err(entry_point_error) => match entry_point_error {
                EntryPointErr::FailedOp(failed_op) => {
                    Err(SimulateValidationError::UserOperationRejected {
                        message: format!("{failed_op}"),
                    })
                }
                _ => Err(SimulateValidationError::UserOperationRejected {
                    message: "unknown error".to_string(),
                }),
            },
        }
    }

    async fn simulate_validation_trace(
        &self,
        user_operation: &UserOperation,
    ) -> Result<GethTrace, SimulateValidationError> {
        match self
            .entry_point
            .simulate_validation_trace(user_operation.clone())
            .await
        {
            Ok(geth_trace) => Ok(geth_trace),
            Err(entry_point_error) => match entry_point_error {
                EntryPointErr::FailedOp(failed_op) => {
                    Err(SimulateValidationError::UserOperationRejected {
                        message: format!("{failed_op}"),
                    })
                }
                _ => Err(SimulateValidationError::UserOperationRejected {
                    message: "unknown error".to_string(),
                }),
            },
        }
    }

    fn extract_stake_info(
        &self,
        user_operation: &UserOperation,
        simulate_validation_result: &SimulateValidationResult,
        stake_info_by_entity: &mut [StakeInfo; NUMBER_LEVELS],
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
        stake_info_by_entity[0] = StakeInfo {
            address: if user_operation.init_code.len() >= 20 {
                Address::from_slice(&user_operation.init_code[0..20])
            } else {
                Address::zero()
            },
            stake: factory_info.0,
            unstake_delay: factory_info.1,
        };

        // account
        stake_info_by_entity[1] = StakeInfo {
            address: user_operation.sender,
            stake: sender_info.0,
            unstake_delay: sender_info.1,
        };

        // paymaster
        stake_info_by_entity[2] = StakeInfo {
            address: if user_operation.paymaster_and_data.len() >= 20 {
                Address::from_slice(&user_operation.paymaster_and_data[0..20])
            } else {
                Address::zero()
            },
            stake: paymaster_info.0,
            unstake_delay: paymaster_info.1,
        };
    }

    fn forbidden_opcodes(&self, trace: &JsTracerFrame) -> Result<(), SimulateValidationError> {
        for (index, _) in LEVEL_TO_ENTITY.iter().enumerate() {
            if let Some(level) = trace.number_levels.get(index) {
                for opcode in level.opcodes.keys() {
                    if FORBIDDEN_OPCODES.contains(opcode) {
                        return Err(SimulateValidationError::OpcodeValidation {
                            entity: LEVEL_TO_ENTITY[index].to_string(),
                            opcode: opcode.clone(),
                        });
                    }
                }
            }

            if let Some(level) = trace.number_levels.get(index) {
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
        stake_info_by_entity: &[StakeInfo; NUMBER_LEVELS],
        slots_by_entity: &mut HashMap<Address, HashSet<Bytes>>,
    ) {
        for kecc in keccak {
            for entity in stake_info_by_entity {
                if entity.address.is_zero() {
                    continue;
                }

                let entity_address_bytes =
                    Bytes::from([vec![0; 12], entity.address.to_fixed_bytes().to_vec()].concat());

                if kecc.starts_with(&entity_address_bytes) {
                    let k = keccak256(kecc.clone());
                    slots_by_entity
                        .entry(entity.address)
                        .or_insert(HashSet::new())
                        .insert(k.into());
                }
            }
        }
    }

    fn associated_with_slot(
        &self,
        address: &Address,
        slot: &String,
        slots_by_entity: &HashMap<Address, HashSet<Bytes>>,
    ) -> Result<bool, SimulateValidationError> {
        if *slot == address.to_string() {
            return Ok(true);
        }

        if !slots_by_entity.contains_key(address) {
            return Ok(false);
        }

        let slot_as_number = U256::from_str_radix(slot, 16)
            .map_err(|_| SimulateValidationError::StorageAccessValidation { slot: slot.clone() })?;

        if let Some(slots) = slots_by_entity.get(address) {
            for slot_entity in slots {
                let slot_entity_as_number = U256::from(slot_entity.as_ref());

                if slot_as_number >= slot_entity_as_number
                    && slot_as_number < (slot_entity_as_number + 128)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn storage_access(
        &self,
        user_operation: &UserOperation,
        stake_info_by_entity: &[StakeInfo; NUMBER_LEVELS],
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError> {
        let mut slots_by_entity = HashMap::new();
        self.parse_slots(
            trace.keccak.clone(),
            stake_info_by_entity,
            &mut slots_by_entity,
        );

        let mut slot_staked = String::new();

        for (index, stake_info) in stake_info_by_entity.iter().enumerate() {
            if let Some(level) = trace.number_levels.get(index) {
                for (address, access) in &level.access {
                    if *address == user_operation.sender || *address == self.entry_point.address() {
                        continue;
                    }

                    for slot in [
                        access.reads.keys().cloned().collect::<Vec<String>>(),
                        access.writes.keys().cloned().collect(),
                    ]
                    .concat()
                    {
                        slot_staked.clear();

                        if self.associated_with_slot(
                            &user_operation.sender,
                            &slot,
                            &slots_by_entity,
                        )? {
                            if user_operation.init_code.len() > 0 {
                                slot_staked = slot.clone();
                            } else {
                                continue;
                            }
                        } else if *address == stake_info.address
                            || self.associated_with_slot(
                                &stake_info.address,
                                &slot,
                                &slots_by_entity,
                            )?
                        {
                            slot_staked = slot.clone();
                        } else {
                            return Err(SimulateValidationError::StorageAccessValidation { slot });
                        }

                        if !slot_staked.is_empty() && stake_info.stake.is_zero() {
                            return Err(SimulateValidationError::StorageAccessValidation {
                                slot: slot_staked.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_call_stack(
        &self,
        trace: &JsTracerFrame,
        calls: &mut Vec<CallEntry>,
    ) -> Result<(), SimulateValidationError> {
        let mut stack: Vec<Call> = vec![];

        for call in trace.calls.iter() {
            if call.typ == *REVERT_OPCODE || call.typ == *RETURN_OPCODE {
                let top = stack.pop();

                if let Some(top) = top {
                    if top.typ.contains(CREATE_OPCODE.as_str()) {
                        calls.push(CallEntry {
                            typ: top.typ,
                            from: top.from,
                            to: top.to,
                            method: None,
                            ret: None,
                            rev: None,
                            value: None,
                        });
                    } else {
                        let method: Option<String> = {
                            if let Some(method) = top.method {
                                CONTRACTS_FUNCTIONS.get(method.as_ref()).cloned()
                            } else {
                                None
                            }
                        };

                        if call.typ == *REVERT_OPCODE {
                            calls.push(CallEntry {
                                typ: top.typ,
                                from: top.from,
                                to: top.to,
                                method,
                                ret: None,
                                rev: call.data.clone(),
                                value: top.value,
                            });
                        } else {
                            calls.push(CallEntry {
                                typ: top.typ,
                                from: top.from,
                                to: top.to,
                                method,
                                ret: call.data.clone(),
                                rev: None,
                                value: None,
                            });
                        }
                    }
                }
            } else {
                stack.push(call.clone());
            }
        }

        Ok(())
    }

    fn call_stack(
        &self,
        stake_info_by_entity: &[StakeInfo; NUMBER_LEVELS],
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError> {
        let mut calls: Vec<CallEntry> = vec![];
        self.parse_call_stack(trace, &mut calls)?;

        for (index, stake_info) in stake_info_by_entity.iter().enumerate() {
            if LEVEL_TO_ENTITY[index] == "paymaster" {
                let call = calls.iter().find(|call| {
                    call.method == Some(PAYMASTER_VALIDATION_FUNCTION.clone())
                        && call.to == Some(stake_info.address)
                });

                if let Some(call) = call {
                    if let Some(ret) = call.ret.as_ref() {
                        let validate_paymaster_return: ValidatePaymasterUserOpReturn =
                            AbiDecode::decode(ret).map_err(|_| {
                                SimulateValidationError::UserOperationRejected {
                                    message: "unknown error".to_string(),
                                }
                            })?;
                        let context = validate_paymaster_return.context;

                        if !context.is_empty()
                            && self
                                .reputation
                                .verify_stake("paymaster", Some(*stake_info))
                                .is_err()
                        {
                            return Err(SimulateValidationError::CallStackValidation {
                                message: "Paymaster that is not staked should not return context"
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_code_hashes(
        &self,
        contract_addresses: Vec<Address>,
        code_hashes: &mut Vec<CodeHash>,
    ) -> Result<(), SimulateValidationError> {
        let mut tasks: JoinSet<Option<(Address, H256)>> = JoinSet::new();

        for contract_address in contract_addresses {
            let eth_provider = self.eth_provider.clone();

            tasks.spawn(async move {
                match eth_provider.get_code(contract_address, None).await {
                    Ok(code) => Some((contract_address, keccak256(&code).into())),
                    Err(_) => None,
                }
            });
        }

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Some(code_hash)) => code_hashes.push(CodeHash {
                    address: code_hash.0,
                    hash: code_hash.1,
                }),
                Ok(None) | Err(_) => {
                    return Err(SimulateValidationError::UnknownError {
                        error: "failed to get code hashes".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    async fn code_hashes(
        &self,
        user_operation: &UserOperation,
        trace: &JsTracerFrame,
    ) -> Result<Vec<CodeHash>, SimulateValidationError> {
        let contract_addresses = trace
            .number_levels
            .iter()
            .flat_map(|level| {
                level
                    .contract_size
                    .keys()
                    .copied()
                    .collect::<Vec<Address>>()
            })
            .collect::<Vec<Address>>();

        let code_hashes: &mut Vec<CodeHash> = &mut vec![];
        self.get_code_hashes(contract_addresses, code_hashes)
            .await?;

        let user_operation_hash = user_operation.hash(&self.entry_point.address(), &self.chain_id);

        match self.mempool.has_code_hashes(&user_operation_hash) {
            Ok(true) => {
                // 2nd simulation
                let prev_code_hashes = self.mempool.get_code_hashes(&user_operation_hash);
                if !equal_code_hashes(code_hashes, &prev_code_hashes) {
                    Err(SimulateValidationError::CodeHashesValidation {
                        message: "modified code after 1st simulation".to_string(),
                    })
                } else {
                    Ok(code_hashes.to_vec())
                }
            }
            Ok(false) => {
                // 1st simulation
                Ok(code_hashes.to_vec())
            }
            Err(error) => Err(SimulateValidationError::UnknownError {
                error: error.to_string(),
            }),
        }
    }

    pub async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulationResult, SimulateValidationError> {
        let simulate_validation_result = self.simulate_validation(user_operation).await?;
        let geth_trace = self.simulate_validation_trace(user_operation).await?;

        trace!("Simulate user operation {user_operation:?} with trace {geth_trace:?}");

        let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace).map_err(|error| {
            SimulateValidationError::UserOperationRejected {
                message: error.to_string(),
            }
        })?;

        let mut stake_info_by_entity: [StakeInfo; NUMBER_LEVELS] = Default::default();
        self.extract_stake_info(
            user_operation,
            &simulate_validation_result,
            &mut stake_info_by_entity,
        );

        // may not invokes any forbidden opcodes
        self.forbidden_opcodes(&js_trace)?;

        // verify storage access
        self.storage_access(user_operation, &stake_info_by_entity, &js_trace)?;

        // verify call stack
        self.call_stack(&stake_info_by_entity, &js_trace)?;

        // verify code hashes
        let code_hashes = self.code_hashes(user_operation, &js_trace).await?;

        Ok(SimulationResult {
            simulate_validation_result,
            code_hashes,
        })
    }
}
