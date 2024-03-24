use crate::common::{
    deploy_tracer_test,
    gen::{ExecSelfResultFilter, TracerTest},
    setup_geth, ClientType, DeployedContract,
};
use ethers::{
    abi::{RawLog, Token},
    contract::EthLogDecode,
    prelude::BaseContract,
    providers::Middleware,
    types::{
        Bytes, GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
        TransactionRequest, H256,
    },
    utils::GethInstance,
};
use silius_contracts::tracer::{JsTracerFrame, JS_TRACER};
use std::{ops::Deref, str::FromStr, sync::Arc};

struct Context<M> {
    _geth: GethInstance,
    client: Arc<M>,
    tracer_test: DeployedContract<TracerTest<M>>,
}

async fn setup() -> eyre::Result<Context<ClientType>> {
    let (_geth, _client, _) = setup_geth().await?;
    let client = Arc::new(_client);

    let tracer_test = deploy_tracer_test(client.clone()).await?;
    Ok(Context { _geth, client, tracer_test })
}

async fn trace_call<M: Middleware + 'static>(
    c: &Context<M>,
    func_data: Bytes,
) -> eyre::Result<JsTracerFrame> {
    let req = TransactionRequest::new().to(c.tracer_test.address).data(func_data);
    let res = c
        .client
        .clone()
        .debug_trace_call(
            req,
            None,
            GethDebugTracingCallOptions {
                tracing_options: GethDebugTracingOptions {
                    disable_storage: None,
                    disable_stack: None,
                    enable_memory: None,
                    enable_return_data: None,
                    tracer: Some(GethDebugTracerType::JsTracer(JS_TRACER.into())),
                    tracer_config: None,
                    timeout: None,
                },
                state_overrides: None,
                block_overrides: None,
            },
        )
        .await?;
    let frames: JsTracerFrame = res.try_into().unwrap();
    Ok(frames)
}

async fn trace_exec_self<M: Middleware + 'static>(
    c: &Context<M>,
    func_data: Vec<u8>,
    use_number: bool,
    extra_wrapper: bool,
) -> eyre::Result<JsTracerFrame> {
    let contract: &BaseContract = c.tracer_test.contract().deref().deref();
    let func = contract.abi().function("execSelf")?;
    let exec_test_call_gas =
        func.encode_input(&[Token::Bytes(func_data), Token::Bool(use_number)])?;
    if extra_wrapper {
        let exec_2_test_call_gas =
            func.encode_input(&[Token::Bytes(exec_test_call_gas), Token::Bool(use_number)])?;
        trace_call(c, Bytes::from(exec_2_test_call_gas)).await
    } else {
        trace_call(c, Bytes::from(exec_test_call_gas)).await
    }
}

#[tokio::test]
async fn count_opcode_depth_bigger_than_1() -> eyre::Result<()> {
    let c = setup().await?;
    let contract: &BaseContract = c.tracer_test.contract().deref().deref();
    let func_data = contract.abi().function("callTimeStamp")?.encode_input(&[])?;
    let ret = trace_exec_self(&c, func_data, false, true).await?;
    let log: ExecSelfResultFilter = ExecSelfResultFilter::decode_log(&RawLog::from((
        ret.logs[0]
            .topics
            .clone()
            .into_iter()
            .map(|i| H256::from_str(i.as_str()).unwrap())
            .collect::<Vec<H256>>(),
        ret.logs[0].data.to_vec(),
    )))?;
    assert_eq!(log.success, true);
    assert_eq!(*ret.calls_from_entry_point[0].opcodes.get("TIMESTAMP").unwrap(), 1);
    Ok(())
}

#[tokio::test]
async fn trace_exec_self_should_revert() -> eyre::Result<()> {
    let c = setup().await?;
    let ret = trace_exec_self(&c, Bytes::from_str("0xdead")?.to_vec(), true, true).await?;
    assert!(ret.debug.join(",").matches("execution reverted").collect::<Vec<&str>>().len() > 0);
    assert_eq!(ret.logs.len(), 1);

    let log: ExecSelfResultFilter = ExecSelfResultFilter::decode_log(&RawLog::from((
        ret.logs[0]
            .topics
            .clone()
            .into_iter()
            .map(|i| H256::from_str(i.as_str()).unwrap())
            .collect::<Vec<H256>>(),
        ret.logs[0].data.to_vec(),
    )))?;
    assert_eq!(log.success, false);
    Ok(())
}

#[tokio::test]
async fn trace_exec_self_call_itself() -> eyre::Result<()> {
    let c = setup().await?;
    let contract: &BaseContract = c.tracer_test.contract().deref().deref();
    let inner_call = contract.abi().function("doNothing")?.encode_input(&[])?;
    let exec_inner = contract
        .abi()
        .function("execSelf")?
        .encode_input(&[Token::Bytes(inner_call.to_vec()), Token::Bool(false)])?;
    let ret = trace_exec_self(&c, exec_inner, true, true).await?;
    assert_eq!(ret.logs.len(), 2);
    ret.logs.iter().for_each(|l| {
        let log_params: ExecSelfResultFilter = ExecSelfResultFilter::decode_log(&RawLog::from((
            l.topics
                .clone()
                .into_iter()
                .map(|i| H256::from_str(i.as_str()).unwrap())
                .collect::<Vec<H256>>(),
            l.data.to_vec(),
        )))
        .unwrap();
        assert_eq!(log_params.success, true);
    });
    Ok(())
}

#[tokio::test]
async fn should_report_direct_use_of_gas_opcode() -> eyre::Result<()> {
    let c = setup().await?;
    let contract: &BaseContract = c.tracer_test.contract().deref().deref();
    let func_data = contract.abi().function("testCallGas")?.encode_input(&[])?;
    let ret = trace_exec_self(&c, func_data, false, false).await?;
    assert_eq!(*ret.calls_from_entry_point[0].opcodes.get("GAS").unwrap(), 1);
    Ok(())
}

#[tokio::test]
async fn should_ignore_gas_used_as_part_of_call() -> eyre::Result<()> {
    let c = setup().await?;
    let contract: &BaseContract = c.tracer_test.contract().deref().deref();
    let do_nothing = contract.abi().function("doNothing")?.encode_input(&[])?;
    let call_do_nothing = contract
        .abi()
        .function("execSelf")?
        .encode_input(&[Token::Bytes(do_nothing.to_vec()), Token::Bool(false)])?;
    let ret = trace_exec_self(&c, call_do_nothing, false, false).await?;
    assert_eq!(ret.calls_from_entry_point[0].opcodes.get("GAS"), None);
    Ok(())
}
