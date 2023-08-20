use crate::validate::{utils::extract_stake_info, SimulationTraceCheck, SimulationTraceHelper};
use ethers::{abi::AbiDecode, providers::Middleware};
use silius_contracts::{
    entry_point::{ValidatePaymasterUserOpReturn, CONTRACTS_FUNCTIONS},
    tracer::{Call, CallEntry, JsTracerFrame},
};
use silius_primitives::{
    consts::entities::{LEVEL_TO_ENTITY, PAYMASTER},
    simulation::{
        SimulationCheckError, CREATE_OPCODE, PAYMASTER_VALIDATION_FUNCTION, RETURN_OPCODE,
        REVERT_OPCODE,
    },
    UserOperation,
};

pub struct CallStack;

impl CallStack {
    /// The helper method that parses the call stack.
    ///
    /// # Arguments
    /// `trace` - The [JsTracerFrame] that contains the call stack to parse
    /// `calls` - The vector of [CallEntry] that will be filled with the parsed call stack
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    fn parse_call_stack(
        &self,
        trace: &JsTracerFrame,
        calls: &mut Vec<CallEntry>,
    ) -> Result<(), SimulationCheckError> {
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
    /// The [check_user_operation] method implementation that performs the call stack trace check.
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError> {
        if helper.stake_info.is_none() {
            helper.stake_info = Some(extract_stake_info(uo, helper.simulate_validation_result));
        }

        let mut calls: Vec<CallEntry> = vec![];
        self.parse_call_stack(helper.js_trace, &mut calls)?;

        for call in calls.iter() {
            if call.to.unwrap_or_default() == helper.entry_point.address()
                && call.from.unwrap_or_default() != helper.entry_point.address()
                && (call.method.is_some()
                    && call.method.clone().unwrap_or_default() != *"depositTo")
            {
                return Err(SimulationCheckError::CallStack {
                    message: format!("Illegal call into entry point during validation {call:?}"),
                });
            }

            if call.to.unwrap_or_default() != helper.entry_point.address()
                && !call.value.unwrap_or_default().is_zero()
            {
                return Err(SimulationCheckError::CallStack {
                    message: format!("Illegal call {call:?}"),
                });
            }

            // paymaster
            for (i, stake_info) in helper.stake_info.unwrap_or_default().iter().enumerate() {
                if LEVEL_TO_ENTITY[i] == PAYMASTER
                    && call.method == Some(PAYMASTER_VALIDATION_FUNCTION.clone())
                    && call.to == Some(stake_info.address)
                {
                    if let Some(ret) = call.ret.as_ref() {
                        let validate_paymaster_return: ValidatePaymasterUserOpReturn =
                            AbiDecode::decode(ret).map_err(|_| {
                                SimulationCheckError::Validation {
                                    message: "Error during simulate validation on entry point"
                                        .to_string(),
                                }
                            })?;
                        let context = validate_paymaster_return.context;

                        if !context.is_empty()
                            && helper
                                .reputation
                                .verify_stake(PAYMASTER, Some(*stake_info))
                                .is_err()
                        {
                            return Err(SimulationCheckError::Unstaked {
                                entity: PAYMASTER.to_string(),
                                message: "must not return context".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
