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

const FALL_BACK_BINARY_SEARCH_CUT_OFF: u128 = 30000;
const BASE_VGL_BUFFER: u128 = 25;
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
    user_operation_original: &UserOperationSigned,
    entry_point: &EntryPoint<M>,
) -> Result<(U256, U256), EntryPointError> {
    let mut iter: u64 = 0;

    let mut user_operation = user_operation_original.clone();
    user_operation.verification_gas_limit = 0.into();
    user_operation.call_gas_limit = 0.into();
    user_operation.max_priority_fee_per_gas = user_operation_original.max_fee_per_gas;

    // Binary search
    let mut l: u128 = 0;
    let mut r: u128 = u64::MAX.into();
    let mut f: u128 = 0;

    let mut err = EntryPointError::Other {
        inner: "Could not find a valid verification gas limit".to_string(),
    };

    while r - l >= FALL_BACK_BINARY_SEARCH_CUT_OFF {
        let m = (l + r) / 2;
        user_operation.verification_gas_limit = m.into();
        match entry_point.simulate_handle_op(user_operation.clone()).await {
            // VGL too high
            Ok(_) => {
                r = m - 1;
                f = m;
                continue;
            }
            Err(e) => {
                err = e.clone();
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
        return Err(err);
    }

    let out: TraceOutput;
    let mut res: Result<(U256, U256), EntryPointError> = Ok((0u64.into(), 0u64.into()));

    loop {
        if iter >= MAX_RETRY {
            return res;
        }
        f = (f * (100 + BASE_VGL_BUFFER)) / 100;
        user_operation.verification_gas_limit = f.into();
        user_operation.max_fee_per_gas = 0u64.into();
        user_operation.max_priority_fee_per_gas = 0u64.into();
        user_operation.call_gas_limit = MAX_CALL_GAS_LIMIT.into(); // max block gas limit, better set as a config parameter
        match trace_simulate_handle_op(&user_operation, entry_point).await {
            Ok(o) => {
                out = o;
                break;
            }
            Err(e) => {
                iter += 1;
                res = Err(e);
                continue;
            }
        }
    }

    let verification_gas_limit = user_operation.verification_gas_limit;
    let mut call_gas_limit = if out.tracer_result.execution_gas_limit < NON_ZERO_GAS {
        NON_ZERO_GAS
    } else {
        out.tracer_result.execution_gas_limit
    };

    user_operation.max_priority_fee_per_gas = user_operation_original.max_priority_fee_per_gas;
    user_operation.max_fee_per_gas = user_operation_original.max_fee_per_gas;
    user_operation.verification_gas_limit = verification_gas_limit;
    user_operation.call_gas_limit = call_gas_limit.into();

    loop {
        match trace_simulate_handle_op(&user_operation, entry_point).await {
            Ok(_) => break,
            Err(e) => {
                if is_execution_oog(&e) || is_execution_revert(&e) {
                    let mut l: u128 = call_gas_limit.into();
                    let mut r: u128 = u64::MAX.into();
                    let mut f: u128 = 0u128;
                    while r - l >= FALL_BACK_BINARY_SEARCH_CUT_OFF {
                        let m = (l + r) / 2;
                        user_operation.call_gas_limit = m.into();
                        let res = trace_simulate_handle_op(&user_operation, entry_point).await;
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
                    call_gas_limit = f.try_into().map_err(|e| EntryPointError::Other {
                        inner: format!("Trace handle op call gas limit convert error: {:?}", &e),
                    })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};
    use std::sync::Arc;

    #[tokio::test]
    #[ignore]
    async fn estimate_user_operation_gas() {
        let eth_client = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let ep = EntryPoint::<Provider<Http>>::new(
            eth_client.clone(),
            "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap(),
        );

        let uo = UserOperationSigned {
            sender: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".parse().unwrap(),
            nonce: "0x7295909a3da7b6dde1f89ca3a9ea7c0ea08d4ac74d111cae6421eb6302ac4cc6".into(),
            init_code: "0x3fab184622dc19b6109349b94811493bf2a453624fb673026dac4e898eb42df88a1b4990fc92f1b285988c17a1a0be3c3e9d634df745725cb0b9ab574c29160dd82c8621049564d9a722a589a899b19779d0171b".parse().unwrap(),
            call_data: "0x54c67e1420bcf01efe48af31d5259f47c0d6ae3550f89b9eed8344d729db6964ecf6a398b98a7fb26e7cd50fe077321d6e6e5803cc694773c97ec58e6380c7bbed6f67b63ec3a72f4558c2e7060badfc14c9d7e4e287c931960af6d34265726be9cff591059ccaeedac6809035722d17822f98570725953597e56e073324bdab8a38a2d638595b7202fe8c8805624575ded57aeec27bd1ec99079849442cb13d1ddaab9d64b9a146cea7596117d357b24861b33da24a23af117d3761e2661b6f5c1e0bbfdd4d764f8b6a37dd26779dc79cf99067104055854c56fc45b61e362aa4741db4be82f9e61f1a7c919ccc323daf3d394a11b3b2ac5570a201ed2709a15acdc651e243c900230f019642a57d32754ca46671b018083b6dbea9a920cb7e02ddb2be9a6440222f62beed3182dc98364f7c24d1b7b1d6fbbe767d748ce50c17e04585c4447d8522356819b1191aca1bed4e8cd318496d869c9c4bfe71a85e2c6beafdb299161d706915f8eed8f3bcff2de27e92b109d6380f6dd9e42af778ec2e0218d1e70eee9937081ec04e2c8d42b286f9533af292024eb820bf4c2d0d47aeafd925ce0f004d33e9d64b5ff983b0f0a2e05cc17c1c0e02f2bb87cde4b21dc8aa5d3707b7859f4ab587f9008810597fd9df20be48c0017d6e5979aa2a4588382bf8691b7a542679d25426fdd04b8625bdc4893aec921bc292846c25479d1af27e6ac9a859c5b7e24ea07f3b07f6d01622d324df211594f0276ecd8c0c962f94287c91eb371dbe1f913b9dbadb29e159a2d991000fab0b4c082e1864c14c32a0333e859fb76fb03af6d3cebb8acda50d3a5ddc722dbb5f7bc858d6c3c8028a4f8136b0539a2f878990df239bd05623f432712c93203c63b21a3fa21f8bd15986d25206cd915f2f3c03a0ec136df851c5e2c66215c9db7509db6f070cfd0bb9b8d7b6d9499dc0e7c2037d403dae96d34139abea2504db30411184ef3678a7943183f5a0cf49f6b85dd2202d6ed21819d1655c03247d8ea0e4429eeb5bebbfd4a40d23a443aedc64b9f651e5b78f0a996471f931ff290c1183660d3eff252fbfd6a920644b88ae0363bd15d5ed65e57ac26421ebf753c13b55a5e27c1d4b700bd9c4b4366f30b15d043fd873566d7bf42db8a89ab1563264c2c7f04254da55090e1c0d5a198feb2955c20fbc1a4dac9b505d1d021d1330c756e5".parse().unwrap(),
            call_gas_limit: "0xbc367".into(),
            verification_gas_limit: "0x7d36e".into(),
            pre_verification_gas: "0x108d9".into(),
            max_fee_per_gas: "0xae7aa6912".into(),
            max_priority_fee_per_gas: "0x1a13b8600".into(),
            paymaster_and_data: Bytes::default(),
            signature: "0xcbe8b7855dc1481374c37579f953876b778a4ee16f5408b18894d2306977651498b79128e5fedab6855d6b16f8466e8247e4ba601989d1c5fd24194b01b5e8514d".parse().unwrap(),
        };

        let res = estimate_user_op_gas(&uo, &ep).await;
        assert!(res.is_err());
    }
}
