use const_hex::hex;
use core::fmt::Debug;
use ethers::{
    abi::{Hash, RawLog},
    contract::EthLogDecode,
    providers::Middleware,
    types::{Bytes, U256},
};
use silius_contracts::{
    decode_revert_string,
    executor_tracer::{ExecutorTracerResult, LogInfo},
    EntryPoint, EntryPointError, ExecutionResult, FailedOp, UserOperationEventFilter,
    UserOperationRevertReasonFilter,
};
use silius_primitives::UserOperationSigned;
use std::str::FromStr;

const FALL_BACK_BINARY_SEARCH_CUT_OFF: u64 = 30000;
const BASE_VGL_BUFFER: u64 = 25;
const MAX_CALL_GAS_LIMIT: u64 = 18_000_000;
const MAX_RETRY: u64 = 7;
const NON_ZERO_GAS: u64 = 12100; // should be different based on diferrent chain
const EXECUTION_REVERTED: &str = "execution reverted";
const EXECUTION_OOG: &str = "execution OOG";

fn is_prefund_not_paid<T: ToString>(err: T) -> bool {
    let s = err.to_string();
    s.contains("AA21 didn't pay prefund") ||
        s.contains("AA31 paymaster deposit too low") ||
        s.contains("AA95 out of gas")
}

fn is_validation_oog<T: ToString>(err: T) -> bool {
    let s = err.to_string();
    s.contains("validation OOG") ||
        s.contains("return data out of bounds") ||
        s.contains("AA40 over verificationGasLimit") ||
        s.contains("AA41 too little verificationGas") ||
        s.contains("AA51 prefund below actualGasCost") ||
        s.contains("AA13 initCode failed or OOG") ||
        s.contains("AA23 reverted (or OOG)") ||
        s.contains("AA33 reverted (or OOG")
}

fn is_execution_oog<T: ToString>(err: T) -> bool {
    err.to_string().contains(EXECUTION_OOG)
}
fn is_execution_revert<T: ToString>(err: T) -> bool {
    err.to_string().contains(EXECUTION_REVERTED)
}

#[derive(Debug, Default)]
struct TraceOutput {
    tracer_result: ExecutorTracerResult,
    execution_result: ExecutionResult,
    user_op_event: UserOperationEventFilter,
    user_op_revert_event: Option<UserOperationRevertReasonFilter>,
}

fn parse_simulate_handle_op_output(output: &str) -> Result<ExecutionResult, EntryPointError> {
    let output_b = Bytes::from_str(output).map_err(|e| EntryPointError::Other {
        inner: format!("parse simulate handle op output failed: {e:?}"),
    })?;
    if let Ok(decoded) =
        <ExecutionResult as ::ethers::core::abi::AbiDecode>::decode(output_b.as_ref())
    {
        return Ok(decoded);
    };

    if let Ok(decoded) = <FailedOp as ::ethers::core::abi::AbiDecode>::decode(output_b.as_ref()) {
        return Err(EntryPointError::FailedOp(decoded));
    };

    Err(EntryPointError::Other {
        inner: "output of parse simulate handle op is not valid".to_string(),
    })
}

fn parse_user_op_event<T: Debug + EthLogDecode>(event: &LogInfo) -> Result<T, EntryPointError> {
    let topics = event
        .topics
        .iter()
        .map(|t| {
            let mut hash_str = t.clone();
            if hash_str.len() % 2 != 0 {
                hash_str.insert(0, '0');
            };
            hex::decode(hash_str).map(|mut b| {
                b.resize(32, 0);
                Hash::from_slice(b.as_ref())
            })
        })
        .collect::<Result<Vec<Hash>, _>>()
        .map_err(|e| EntryPointError::Other {
            inner: format!(
                "simulate handle user op failed on parsing user op event topic hash, {e:?}"
            ),
        })?;
    let data = Bytes::from_str(event.data.as_str()).map_err(|e| EntryPointError::Other {
        inner: format!("simulate handle user op failed on parsing user op event data: {e:?}"),
    })?;
    let log = RawLog::from((topics, data.to_vec()));
    <T>::decode_log(&log).map_err(|err| EntryPointError::Other {
        inner: format!("simulate handle user op failed on parsing user op event: {err:?}"),
    })
}

async fn trace_simulate_handle_op<M: Middleware>(
    user_op: &UserOperationSigned,
    entry_point: &EntryPoint<M>,
) -> Result<TraceOutput, EntryPointError> {
    let geth_trace = entry_point.simulate_handle_op_trace(user_op.clone()).await?;

    let tracer_result: ExecutorTracerResult =
        ExecutorTracerResult::try_from(geth_trace).map_err(|e| EntryPointError::Other {
            inner: format!("Estimate trace simulate handle op decode error {e:?}"),
        })?;

    let execution_result = parse_simulate_handle_op_output(tracer_result.output.as_str())?;

    let user_op_event = tracer_result.user_op_event.as_ref().ok_or(EntryPointError::Other {
        inner: "Estimate trace simulate handle op user op event not found".to_string(),
    })?;
    let user_op_event = parse_user_op_event::<UserOperationEventFilter>(user_op_event)?;
    let user_op_revert_event = tracer_result
        .user_op_revert_event
        .as_ref()
        .and_then(|e| parse_user_op_event::<UserOperationRevertReasonFilter>(e).ok());

    if !user_op_event.success && !tracer_result.reverts.is_empty() {
        let revert_data = tracer_result.reverts[tracer_result.reverts.len() - 1].clone();
        if revert_data.is_empty() && user_op_revert_event.is_none() && tracer_result.execution_oog {
            return Err(EntryPointError::Other { inner: EXECUTION_OOG.to_string() });
        }
        if let Some(revert_event) = &user_op_revert_event {
            if let Some(error_str) = decode_revert_string(revert_event.revert_reason.clone()) {
                return Err(EntryPointError::ExecutionReverted(format!(
                    "User op execution revert with {error_str:?}, {revert_event:?}",
                )));
            };
        }
        return Err(EntryPointError::ExecutionReverted(format!(
            "{:?} , {:?} , {:?}, {:?}",
            tracer_result.error, execution_result, user_op_event, user_op_revert_event
        )));
    }

    Ok(TraceOutput { tracer_result, execution_result, user_op_event, user_op_revert_event })
}

pub async fn estimate_user_op_gas<M: Middleware>(
    user_op_ori: &UserOperationSigned,
    entry_point: &EntryPoint<M>,
) -> Result<(U256, U256), EntryPointError> {
    let mut iteration: u64 = 0;

    let mut user_op = user_op_ori.clone();
    user_op.verification_gas_limit = 0.into();
    user_op.call_gas_limit = 0.into();
    user_op.max_priority_fee_per_gas = user_op_ori.max_fee_per_gas;

    // Binary search
    let mut l: u64 = 0;
    let mut r: u64 = u64::MAX;
    let mut f: u64 = 0;

    while r - l >= FALL_BACK_BINARY_SEARCH_CUT_OFF {
        let m = (l + r) / 2;
        user_op.verification_gas_limit = m.into();
        match entry_point.simulate_handle_op(user_op.clone()).await {
            // VGL too high
            Ok(_) => {
                r = m - 1;
                f = m;
                continue;
            }
            Err(e) => {
                if is_prefund_not_paid(&e) {
                    r = m - 1;
                    continue;
                } else if is_validation_oog(&e) {
                    l = m + 1;
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
    if f == 0 {
        return Err(EntryPointError::Other {
            inner: "Could not find a valid verification gas limit".to_string(),
        });
    }
    let out: TraceOutput;
    let mut res: Result<(U256, U256), EntryPointError> = Ok((0u64.into(), 0u64.into()));
    loop {
        if iteration >= MAX_RETRY {
            return res;
        }
        f = (f * (100 + BASE_VGL_BUFFER)) / 100;
        user_op.verification_gas_limit = f.into();
        user_op.max_fee_per_gas = 0u64.into();
        user_op.max_priority_fee_per_gas = 0u64.into();
        user_op.call_gas_limit = MAX_CALL_GAS_LIMIT.into(); // max block gas limit, better set as a config parameter
        match trace_simulate_handle_op(&user_op, entry_point).await {
            Ok(o) => {
                out = o;
                break;
            }
            Err(e) => {
                iteration += 1;
                res = Err(e);
                continue;
            }
        }
    }
    let verification_gas_limit = user_op.verification_gas_limit;
    let mut call_gas_limit = if out.tracer_result.execution_gas_limit < NON_ZERO_GAS {
        NON_ZERO_GAS
    } else {
        out.tracer_result.execution_gas_limit
    };

    user_op.max_priority_fee_per_gas = user_op_ori.max_priority_fee_per_gas;
    user_op.max_fee_per_gas = user_op_ori.max_fee_per_gas;
    user_op.verification_gas_limit = verification_gas_limit;
    user_op.call_gas_limit = call_gas_limit.into();
    loop {
        match trace_simulate_handle_op(&user_op, entry_point).await {
            Ok(_) => break,
            Err(e) => {
                if is_execution_oog(&e) || is_execution_revert(&e) {
                    let mut l = call_gas_limit;
                    let mut r = u64::MAX;
                    let mut f = 0u64;
                    while r - l >= FALL_BACK_BINARY_SEARCH_CUT_OFF {
                        let m = (l + r) / 2;
                        user_op.call_gas_limit = m.into();
                        let res = trace_simulate_handle_op(&user_op, entry_point).await;
                        match res {
                            Ok(_) => {
                                r = m - 1;
                                f = m;
                                continue;
                            }
                            Err(err) => {
                                if is_prefund_not_paid(&err) {
                                    r = m - 1;
                                    continue;
                                } else if is_execution_oog(&err) || is_execution_revert(&err) {
                                    l = m + 1;
                                    continue;
                                } else {
                                    return Err(EntryPointError::Other {
                                        inner: "Could not find a valid call gas limit".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    call_gas_limit = f;
                } else {
                    return Err(EntryPointError::Other {
                        inner: format!("Trace handle op return unhandled error: {:?}", &e),
                    });
                }
            }
        }
    }
    Ok((verification_gas_limit, call_gas_limit.into()))
}
