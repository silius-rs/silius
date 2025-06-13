use crate::{
    mempool::Mempool,
    validate::{utils::extract_stake_info, SimulationTraceCheck, SimulationTraceHelper},
    Reputation, SimulationError,
};
use ethers::{abi::AbiDecode, providers::Middleware};
use silius_contracts::{
    entry_point::{ValidatePaymasterUserOpReturn, SELECTORS_NAMES},
    tracer::{Call, CallEntry, JsTracerFrame},
};
use silius_primitives::{
    constants::validation::entities::{LEVEL_TO_ENTITY, PAYMASTER},
    simulation::{
        CREATE_OPCODE, RETURN_OPCODE, REVERT_OPCODE, VALIDATE_PAYMASTER_USER_OP_FUNCTION,
    },
    UserOperation,
};

#[derive(Clone)]
pub struct CallStack;

impl CallStack {
    /// The helper method that parses the call stack.
    ///
    /// # Arguments
    /// `trace` - The [JsTracerFrame] that contains the call stack to parse
    /// `calls` - The vector of [CallEntry] that will be filled with the parsed call stack
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
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
                                SELECTORS_NAMES.get(m.as_ref()).cloned()
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
                                value: top.value,
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
}

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for CallStack {
    /// The method implementation that performs the call stack trace check.
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool,
        reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        if helper.stake_info.is_none() {
            helper.stake_info = Some(extract_stake_info(uo, helper.simulate_validation_result));
        }

        let mut calls: Vec<CallEntry> = vec![];
        self.parse_call_stack(helper.js_trace, &mut calls)?;

        for call in calls.iter() {
            // [OP-052] - may call depositTo(sender) with any value from either the sender or
            // factory [OP-053] - may call the fallback function from the sender with
            // any value
            if call.to.unwrap_or_default() == helper.entry_point.address() &&
                call.from.unwrap_or_default() != helper.entry_point.address() &&
                (call.method.is_some() &&
                    call.method.clone().unwrap_or_default() != *"depositTo")
            {
                // [OP-054] - any other access to the EntryPoint is forbidden
                return Err(SimulationError::CallStack {
                    inner: "Illegal call into entry point during validation {call:?}".into(),
                });
            }

            // [OP-061] - CALL with value is forbidden. The only exception is a call to the
            // EntryPoint described above
            if call.to.unwrap_or_default() != helper.entry_point.address() &&
                !call.value.unwrap_or_default().is_zero()
            {
                return Err(SimulationError::CallStack { inner: "Illegal call {call:?}".into() });
            }

            // paymaster
            for (i, stake_info) in helper.stake_info.unwrap_or_default().iter().enumerate() {
                if LEVEL_TO_ENTITY[i] == PAYMASTER &&
                    call.method == Some(VALIDATE_PAYMASTER_USER_OP_FUNCTION.clone()) &&
                    call.to == Some(stake_info.address)
                {
                    if let Some(ret) = call.ret.as_ref() {
                        let validate_paymaster_return: ValidatePaymasterUserOpReturn =
                            AbiDecode::decode(ret).map_err(|_| SimulationError::Validation {
                                inner: "Error during simulate validation on entry point".into(),
                            })?;
                        let context = validate_paymaster_return.context;

                        // [EREP-050] - an unstaked paymaster may not return a context
                        // This will be removed in the future
                        if !context.is_empty() &&
                            reputation
                                .verify_stake(
                                    PAYMASTER,
                                    Some(*stake_info),
                                    helper.val_config.min_stake,
                                    helper.val_config.min_unstake_delay,
                                )
                                .is_err()
                        {
                            return Err(SimulationError::Unstaked {
                                entity: PAYMASTER.into(),
                                address: stake_info.address,
                                inner: "must not return context".into(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
