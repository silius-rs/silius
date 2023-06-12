use crate::{utils::equal_code_hashes, UoPool};
use aa_bundler_contracts::{
    entry_point::{
        EntryPointErr, SimulateValidationResult, ValidatePaymasterUserOpReturn, CONTRACTS_FUNCTIONS,
    },
    tracer::{Call, CallEntry, JsTracerFrame},
};
use aa_bundler_primitives::{
    consts::entities::{FACTORY, PAYMASTER},
    get_address,
    reputation::StakeInfo,
    simulation::{
        CodeHash, SimulationError, CREATE2_OPCODE, CREATE_OPCODE, EXPIRATION_TIMESTAMP_DIFF,
        FORBIDDEN_OPCODES, LEVEL_TO_ENTITY, NUMBER_LEVELS, PAYMASTER_VALIDATION_FUNCTION,
        RETURN_OPCODE, REVERT_OPCODE,
    },
    UoPoolMode, UserOperation,
};
use ethers::{
    abi::AbiDecode,
    providers::Middleware,
    types::{Address, Bytes, GethTrace, H256, U256},
    utils::keccak256,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationResult {
    pub code_hashes: Vec<CodeHash>,
    pub valid_after: Option<u64>,
    pub verification_gas_limit: U256,
    pub pre_fund: U256,
}

impl<M: Middleware + 'static> UoPool<M> {
    pub async fn simulate_validation(
        &self,
        uo: &UserOperation,
    ) -> Result<SimulateValidationResult, SimulationError> {
        match self.entry_point.simulate_validation(uo.clone()).await {
            Ok(res) => Ok(res),
            Err(err) => match err {
                EntryPointErr::FailedOp(f) => {
                    Err(SimulationError::Validation { message: f.reason })
                }
                _ => Err(SimulationError::UnknownError {
                    message: "Error when simulating validation on entry point".to_string(),
                }),
            },
        }
    }

    async fn simulate_validation_trace(
        &self,
        uo: &UserOperation,
    ) -> Result<GethTrace, SimulationError> {
        match self.entry_point.simulate_validation_trace(uo.clone()).await {
            Ok(trace) => Ok(trace),
            Err(err) => match err {
                EntryPointErr::FailedOp(f) => {
                    Err(SimulationError::Validation { message: f.reason })
                }
                _ => Err(SimulationError::UnknownError {
                    message: "Error when simulating validation on entry point".to_string(),
                }),
            },
        }
    }

    fn signature(&self, res: &SimulateValidationResult) -> Result<(), SimulationError> {
        let sig_check = match res {
            SimulateValidationResult::ValidationResult(res) => res.return_info.2,
            SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.2,
        };

        if sig_check {
            return Err(SimulationError::Signature {});
        }

        Ok(())
    }

    fn timestamps(&self, res: &SimulateValidationResult) -> Result<Option<u64>, SimulationError> {
        let (valid_after, valid_until) = match res {
            SimulateValidationResult::ValidationResult(res) => {
                (res.return_info.3, res.return_info.4)
            }
            SimulateValidationResult::ValidationResultWithAggregation(res) => {
                (res.return_info.3, res.return_info.4)
            }
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SimulationError::UnknownError {
                message: "Failed to get current timestamp".to_string(),
            })?
            .as_secs();

        if valid_until <= now + EXPIRATION_TIMESTAMP_DIFF {
            return Err(SimulationError::Expiration {
                valid_after,
                valid_until,
                paymaster: None, // TODO: fill with paymaster address this error was triggered by the paymaster
            });
        }

        if valid_after > now {
            return Ok(Some(valid_after));
        }

        Ok(None)
    }

    fn extract_stake_info(
        &self,
        uo: &UserOperation,
        res: &SimulateValidationResult,
        info: &mut [StakeInfo; NUMBER_LEVELS],
    ) {
        let (f_info, s_info, p_info) = match res {
            SimulateValidationResult::ValidationResult(res) => {
                (res.factory_info, res.sender_info, res.paymaster_info)
            }
            SimulateValidationResult::ValidationResultWithAggregation(res) => {
                (res.factory_info, res.sender_info, res.paymaster_info)
            }
        };

        // factory
        info[0] = StakeInfo {
            address: get_address(&uo.init_code).unwrap_or(Address::zero()),
            stake: f_info.0,
            unstake_delay: f_info.1,
        };

        // account
        info[1] = StakeInfo {
            address: uo.sender,
            stake: s_info.0,
            unstake_delay: s_info.1,
        };

        // paymaster
        info[2] = StakeInfo {
            address: get_address(&uo.paymaster_and_data).unwrap_or(Address::zero()),
            stake: p_info.0,
            unstake_delay: p_info.1,
        };
    }

    fn check_oog(&self, trace: &JsTracerFrame) -> Result<(), SimulationError> {
        for (i, _) in LEVEL_TO_ENTITY.iter().enumerate() {
            if let Some(l) = trace.number_levels.get(i) {
                if l.oog.unwrap_or(false) {
                    return Err(SimulationError::OutOfGas {});
                }
            }
        }
        Ok(())
    }

    fn forbidden_opcodes(&self, trace: &JsTracerFrame) -> Result<(), SimulationError> {
        for (i, _) in LEVEL_TO_ENTITY.iter().enumerate() {
            if let Some(l) = trace.number_levels.get(i) {
                for op in l.opcodes.keys() {
                    if FORBIDDEN_OPCODES.contains(op) {
                        return Err(SimulationError::ForbiddenOpcode {
                            entity: LEVEL_TO_ENTITY[i].to_string(),
                            opcode: op.clone(),
                        });
                    }
                }
            }

            if let Some(l) = trace.number_levels.get(i) {
                if let Some(c) = l.opcodes.get(&*CREATE2_OPCODE) {
                    if LEVEL_TO_ENTITY[i] == FACTORY && *c == 1 {
                        continue;
                    }
                    return Err(SimulationError::ForbiddenOpcode {
                        entity: LEVEL_TO_ENTITY[i].to_string(),
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
        info: &[StakeInfo; NUMBER_LEVELS],
        slots: &mut HashMap<Address, HashSet<Bytes>>,
    ) {
        for kecc in keccak {
            for entity in info {
                if entity.address.is_zero() {
                    continue;
                }

                let addr_b =
                    Bytes::from([vec![0; 12], entity.address.to_fixed_bytes().to_vec()].concat());

                if kecc.starts_with(&addr_b) {
                    let k = keccak256(kecc.clone());
                    slots
                        .entry(entity.address)
                        .or_insert(HashSet::new())
                        .insert(k.into());
                }
            }
        }
    }

    fn associated_with_slot(
        &self,
        addr: &Address,
        slot: &String,
        slots: &HashMap<Address, HashSet<Bytes>>,
    ) -> Result<bool, SimulationError> {
        if *slot == addr.to_string() {
            return Ok(true);
        }

        if !slots.contains_key(addr) {
            return Ok(false);
        }

        let slot_num = U256::from_str_radix(slot, 16)
            .map_err(|_| SimulationError::StorageAccess { slot: slot.clone() })?;

        if let Some(slots) = slots.get(addr) {
            for slot in slots {
                let slot_ent_num = U256::from(slot.as_ref());

                if slot_num >= slot_ent_num && slot_num < (slot_ent_num + 128) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn storage_access(
        &self,
        uo: &UserOperation,
        info: &[StakeInfo; NUMBER_LEVELS],
        trace: &JsTracerFrame,
    ) -> Result<(), SimulationError> {
        let mut slots = HashMap::new();
        self.parse_slots(trace.keccak.clone(), info, &mut slots);

        let mut slot_staked = String::new();

        for (i, stake_info) in info.iter().enumerate() {
            if let Some(l) = trace.number_levels.get(i) {
                for (addr, acc) in &l.access {
                    if *addr == uo.sender || *addr == self.entry_point.address() {
                        continue;
                    }

                    for slot in [
                        acc.reads.keys().cloned().collect::<Vec<String>>(),
                        acc.writes.keys().cloned().collect(),
                    ]
                    .concat()
                    {
                        slot_staked.clear();

                        if self.associated_with_slot(&uo.sender, &slot, &slots)? {
                            if uo.init_code.len() > 0 {
                                slot_staked = slot.clone();
                            } else {
                                continue;
                            }
                        } else if *addr == stake_info.address
                            || self.associated_with_slot(&stake_info.address, &slot, &slots)?
                        {
                            slot_staked = slot.clone();
                        } else {
                            return Err(SimulationError::StorageAccess { slot });
                        }

                        if !slot_staked.is_empty() && stake_info.stake.is_zero() {
                            return Err(SimulationError::StorageAccess {
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
    ) -> Result<(), SimulationError> {
        let mut st: Vec<Call> = vec![];

        for call in trace.calls.iter() {
            if call.typ == *REVERT_OPCODE || call.typ == *RETURN_OPCODE {
                let top = st.pop();

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
                        let m: Option<String> = {
                            if let Some(m) = top.method {
                                CONTRACTS_FUNCTIONS.get(m.as_ref()).cloned()
                            } else {
                                None
                            }
                        };

                        if call.typ == *REVERT_OPCODE {
                            calls.push(CallEntry {
                                typ: top.typ,
                                from: top.from,
                                to: top.to,
                                method: m,
                                ret: None,
                                rev: call.data.clone(),
                                value: top.value,
                            });
                        } else {
                            calls.push(CallEntry {
                                typ: top.typ,
                                from: top.from,
                                to: top.to,
                                method: m,
                                ret: call.data.clone(),
                                rev: None,
                                value: None,
                            });
                        }
                    }
                }
            } else {
                st.push(call.clone());
            }
        }

        Ok(())
    }

    fn call_stack(
        &self,
        info: &[StakeInfo; NUMBER_LEVELS],
        trace: &JsTracerFrame,
    ) -> Result<(), SimulationError> {
        let mut calls: Vec<CallEntry> = vec![];
        self.parse_call_stack(trace, &mut calls)?;

        let call = calls.iter().find(|call| {
            call.to.unwrap_or_default() == self.entry_point.address()
                && call.from.unwrap_or_default() != self.entry_point.address()
                && (call.method.is_some()
                    && call.method.clone().unwrap_or_default() != *"depositTo")
        });
        if call.is_some() {
            return Err(SimulationError::CallStack {
                message: format!("Illegal call into entry point during validation {call:?}"),
            });
        }

        for (i, stake_info) in info.iter().enumerate() {
            if LEVEL_TO_ENTITY[i] == PAYMASTER {
                let call = calls.iter().find(|call| {
                    call.method == Some(PAYMASTER_VALIDATION_FUNCTION.clone())
                        && call.to == Some(stake_info.address)
                });

                if let Some(call) = call {
                    if let Some(ret) = call.ret.as_ref() {
                        let validate_paymaster_return: ValidatePaymasterUserOpReturn =
                            AbiDecode::decode(ret).map_err(|_| SimulationError::Validation {
                                message: "Error during simulate validation on entry point"
                                    .to_string(),
                            })?;
                        let context = validate_paymaster_return.context;

                        if !context.is_empty()
                            && self
                                .reputation
                                .verify_stake(PAYMASTER, Some(*stake_info))
                                .is_err()
                        {
                            return Err(SimulationError::CallStack {
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
        addrs: Vec<Address>,
        hashes: &mut Vec<CodeHash>,
    ) -> Result<(), SimulationError> {
        let mut ts: JoinSet<Option<(Address, H256)>> = JoinSet::new();

        for addr in addrs {
            let eth_provider = self.eth_provider.clone();

            ts.spawn(async move {
                match eth_provider.get_code(addr, None).await {
                    Ok(code) => Some((addr, keccak256(&code).into())),
                    Err(_) => None,
                }
            });
        }

        while let Some(res) = ts.join_next().await {
            match res {
                Ok(Some(h)) => hashes.push(CodeHash {
                    address: h.0,
                    hash: h.1,
                }),
                Ok(None) | Err(_) => {
                    return Err(SimulationError::UnknownError {
                        message: "Failed to retrieve code hashes".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    async fn code_hashes(
        &self,
        uo: &UserOperation,
        trace: &JsTracerFrame,
    ) -> Result<Vec<CodeHash>, SimulationError> {
        let addrs = trace
            .number_levels
            .iter()
            .flat_map(|l| l.contract_size.keys().copied().collect::<Vec<Address>>())
            .collect::<Vec<Address>>();

        let hashes: &mut Vec<CodeHash> = &mut vec![];
        self.get_code_hashes(addrs, hashes).await?;

        let uo_hash = uo.hash(&self.entry_point.address(), &self.chain.id().into());

        match self.mempool.has_code_hashes(&uo_hash) {
            Ok(true) => {
                // 2nd simulation
                let hashes_prev = self.mempool.get_code_hashes(&uo_hash);
                if !equal_code_hashes(hashes, &hashes_prev) {
                    Err(SimulationError::CodeHashes {
                        message: "Modified code hashes after 1st simulation".to_string(),
                    })
                } else {
                    Ok(hashes.to_vec())
                }
            }
            Ok(false) => {
                // 1st simulation
                Ok(hashes.to_vec())
            }
            Err(err) => Err(SimulationError::UnknownError {
                message: err.to_string(),
            }),
        }
    }

    pub async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
        signature_check: bool,
    ) -> Result<SimulationResult, SimulationError> {
        let res = self.simulate_validation(user_operation).await?;

        // check signature
        if signature_check {
            self.signature(&res)?;
        }

        // check timestamps
        let valid_after = self.timestamps(&res)?;

        let mut code_hashes: Vec<CodeHash> = vec![];

        if self.mode == UoPoolMode::Standard {
            let geth_trace = self.simulate_validation_trace(user_operation).await?;

            let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace).map_err(|error| {
                SimulationError::Validation {
                    message: error.to_string(),
                }
            })?;

            let mut stake_info_by_entity: [StakeInfo; NUMBER_LEVELS] = Default::default();
            self.extract_stake_info(user_operation, &res, &mut stake_info_by_entity);

            // check if out of gas
            self.check_oog(&js_trace)?;

            // may not invokes any forbidden opcodes
            self.forbidden_opcodes(&js_trace)?;

            // verify storage access
            self.storage_access(user_operation, &stake_info_by_entity, &js_trace)?;

            // verify call stack
            self.call_stack(&stake_info_by_entity, &js_trace)?;

            // verify code hashes
            code_hashes = self.code_hashes(user_operation, &js_trace).await?;
        }

        Ok(SimulationResult {
            code_hashes,
            valid_after,
            verification_gas_limit: match res.clone() {
                SimulateValidationResult::ValidationResult(res) => res.return_info.0,
                SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.0,
            },
            pre_fund: match res {
                SimulateValidationResult::ValidationResult(res) => res.return_info.1,
                SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.1,
            },
        })
    }
}
