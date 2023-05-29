use aa_bundler_contracts::EntryPoint;
use aa_bundler_primitives::{Chain, UoPoolMode, UserOperation};
use aa_bundler_uopool::canonical::simulation::{SimulateValidationError, SimulationResult};
use aa_bundler_uopool::{mempool_id, MemoryMempool, MemoryReputation, Reputation, UoPool};
use ethers::abi::Token;
use ethers::prelude::BaseContract;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Address;
use ethers::utils::{parse_units, GethInstance};
use ethers::{
    providers::Middleware,
    types::{Bytes, U256},
};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::common::deploy_test_coin;
use crate::common::{
    deploy_entry_point, deploy_test_opcode_account, deploy_test_opcode_account_factory,
    deploy_test_recursion_account, deploy_test_rules_account_factory, deploy_test_storage_account,
    deploy_test_storage_account_factory,
    gen::{
        EntryPointContract, TestOpcodesAccount, TestOpcodesAccountFactory, TestRulesAccount,
        TestRulesAccountFactory, TestStorageAccountFactory,
    },
    setup_geth, ClientType, DeployedContract,
};

struct TestContext<M: Middleware> {
    pub client: Arc<M>,
    pub _geth: GethInstance,
    pub entry_point: DeployedContract<EntryPointContract<M>>,
    pub paymaster: DeployedContract<TestOpcodesAccount<M>>,
    pub opcodes_factory: DeployedContract<TestOpcodesAccountFactory<M>>,
    pub storage_factory: DeployedContract<TestStorageAccountFactory<M>>,
    pub _rules_factory: DeployedContract<TestRulesAccountFactory<M>>,
    pub storage_account: DeployedContract<TestRulesAccount<M>>,
    pub uopool: UoPool<M>,
}

async fn setup() -> anyhow::Result<TestContext<ClientType>> {
    let chain_id = 1337u64;
    let (_geth, _client) = setup_geth().await?;
    let client = Arc::new(_client);
    let entry_point = deploy_entry_point(client.clone()).await?;
    let paymaster = deploy_test_opcode_account(client.clone()).await?;
    entry_point
        .contract()
        .deposit_to(paymaster.address)
        .value(parse_units("0.1", "ether").unwrap())
        .send()
        .await?;
    paymaster
        .contract()
        .add_stake(entry_point.address)
        .value(parse_units("0.1", "ether").unwrap())
        .send()
        .await?;

    let test_coin = deploy_test_coin(client.clone()).await?;
    let opcodes_factory = deploy_test_opcode_account_factory(client.clone()).await?;
    let storage_factory =
        deploy_test_storage_account_factory(client.clone(), test_coin.address).await?;
    let rules_factory = deploy_test_rules_account_factory(client.clone()).await?;

    let storage_account_call = rules_factory.contract().create("".to_string());
    let storage_account_address = storage_account_call.call().await?;

    storage_account_call.send().await?;

    entry_point
        .contract()
        .deposit_to(storage_account_address)
        .value(parse_units("1", "ether").unwrap())
        .send()
        .await?;

    let mempool_id = mempool_id(&entry_point.address, &U256::from(chain_id));
    let mut entrypoints_map = HashMap::new();
    entrypoints_map.insert(
        mempool_id,
        EntryPoint::new(client.clone(), entry_point.address),
    );
    let mempools = Box::new(MemoryMempool::default());
    let mut reputation = Box::new(MemoryReputation::default());
    reputation.init(10, 10, 10, 1u64.into(), 1u64.into());
    let pool = UoPool::new(
        EntryPoint::new(client.clone(), entry_point.address),
        mempools,
        reputation,
        client.clone(),
        U256::from(1500000000_u64),
        U256::from(1u64),
        Chain::from(chain_id),
        UoPoolMode::Standard,
    );

    Ok(TestContext::<ClientType> {
        client: client.clone(),
        _geth,
        entry_point,
        paymaster,
        opcodes_factory,
        storage_factory,
        _rules_factory: rules_factory,
        storage_account: DeployedContract::new(
            TestRulesAccount::new(storage_account_address, client.clone()),
            storage_account_address,
        ),
        uopool: pool,
    })
}

async fn create_storage_factory_init_code(
    salt: u64,
    init_func: String,
) -> anyhow::Result<(Bytes, Bytes)> {
    let context = setup().await?;
    let contract: &BaseContract = context.storage_factory.contract().deref().deref();

    let function = contract.abi().function("create")?;
    let init_func =
        function.encode_input(&[Token::Uint(U256::from(salt)), Token::String(init_func)])?;
    let mut init_code = vec![];
    init_code.extend_from_slice(context.storage_factory.address.as_bytes());
    init_code.extend_from_slice(init_func.as_ref());
    Ok((Bytes::from(init_code), Bytes::from(init_func)))
}
async fn create_opcode_factory_init_code(init_func: String) -> anyhow::Result<(Bytes, Bytes)> {
    let context = setup().await?;
    let contract: &BaseContract = context.opcodes_factory.contract().deref().deref();

    let token = vec![Token::String(init_func)];
    let function = contract.abi().function("create")?;
    let init_func = function.encode_input(&token)?;
    let mut init_code = vec![];
    init_code.extend_from_slice(context.opcodes_factory.address.as_bytes());
    init_code.extend_from_slice(&init_func);
    Ok((Bytes::from(init_code), Bytes::from(init_func)))
}

async fn create_test_user_op(
    context: &TestContext<ClientType>,
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> anyhow::Result<UserOperation> {
    let paymaster_and_data = if let Some(rule) = pm_rule {
        let mut data = vec![];
        data.extend_from_slice(context.paymaster.address.as_bytes());
        data.extend_from_slice(rule.as_bytes());
        Bytes::from(data)
    } else {
        Bytes::default()
    };

    let signature = Bytes::from(validate_rule.as_bytes().to_vec());

    let mut tx = TypedTransaction::default();
    tx.set_to(factory_address);
    tx.set_data(init_func);

    let call_init_code_for_addr = context.client.call(&tx, None).await?;

    let (head, address) = call_init_code_for_addr.split_at(12);
    anyhow::ensure!(
        !head.iter().any(|i| *i != 0),
        format!(
            "call init code returns non address data : {:?}",
            call_init_code_for_addr
        )
    );

    let sender = Address::from_slice(address);

    Ok(UserOperation {
        sender,
        nonce: U256::zero(),
        init_code,
        call_data: Bytes::default(),
        call_gas_limit: U256::from(1000000),
        verification_gas_limit: U256::from(1000000),
        pre_verification_gas: U256::from(50000),
        max_fee_per_gas: U256::from(0),
        max_priority_fee_per_gas: U256::from(0),
        paymaster_and_data,
        signature,
    })
}

fn existing_storage_account_user_op(
    context: &TestContext<ClientType>,
    validate_rule: String,
    pm_rule: String,
) -> UserOperation {
    let mut paymaster_and_data = vec![];
    paymaster_and_data.extend_from_slice(context.paymaster.address.as_bytes());
    paymaster_and_data.extend_from_slice(pm_rule.as_bytes());

    let signature = Bytes::from(validate_rule.as_bytes().to_vec());
    UserOperation {
        sender: context.storage_account.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: U256::from(1000000),
        verification_gas_limit: U256::from(1000000),
        pre_verification_gas: U256::from(50000),
        max_fee_per_gas: U256::from(0),
        max_priority_fee_per_gas: U256::from(0),
        paymaster_and_data: Bytes::from(paymaster_and_data),
        signature,
    }
}

async fn validate(
    context: &TestContext<ClientType>,
    user_op: UserOperation,
) -> Result<SimulationResult, SimulateValidationError> {
    context.uopool.simulate_user_operation(&user_op).await
}

async fn test_user_op(
    context: &TestContext<ClientType>,
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> Result<SimulationResult, SimulateValidationError> {
    let user_op = create_test_user_op(
        &context,
        validate_rule,
        pm_rule,
        init_code,
        init_func,
        factory_address,
    )
    .await
    .expect("Create test user operation failed.");
    validate(&context, user_op).await
}

async fn test_existing_user_op(
    validate_rule: String,
    pm_rule: String,
) -> Result<SimulationResult, SimulateValidationError> {
    let context = setup().await.expect("Setup context failed");

    let user_op = existing_storage_account_user_op(&context, validate_rule, pm_rule);
    validate(&context, user_op).await
}

#[tokio::test]
async fn accept_plain_request() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        &context,
        "".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect("succeed");
    Ok(())
}

#[tokio::test]
async fn reject_unkown_rule() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "<unknown-rule>".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::UserOperationRejected { message }) if message.contains("unknown-rule")
    ));
    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_ctr() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("coinbase".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::OpcodeValidation { entity, opcode }) if entity=="factory" && opcode == "COINBASE"
    ));
    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_paymaster() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "".to_string(),
        Some("coinbase".to_string()),
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::OpcodeValidation { entity, opcode }) if entity=="paymaster" && opcode == "COINBASE"
    ));
    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_validation() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "blockhash".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::OpcodeValidation { entity, opcode }) if entity=="account" && opcode == "BLOCKHASH"
    ));
    Ok(())
}

#[tokio::test]
async fn fail_if_create_too_many() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "create2".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::OpcodeValidation { entity, opcode }) if entity=="account" && opcode == "CREATE2"
    ));
    Ok(())
}

#[tokio::test]
async fn fail_referencing_self_token() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "balance-self".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::StorageAccessValidation { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn account_succeeds_referecing_its_own_balance() {
    let res = test_existing_user_op("balance-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn account_fail_to_read_allowance_of_address() {
    let res = test_existing_user_op("allowance-self-1".to_string(), "".to_string()).await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::StorageAccessValidation { .. })
    ));
}

#[tokio::test]
async fn account_can_reference_its_own_allowance_on_other_contract_balance() {
    let res = test_existing_user_op("allowance-1-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn access_self_struct_data() {
    let res = test_existing_user_op("struct-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn fail_to_access_other_address_struct_data() {
    let res = test_existing_user_op("struct-1".to_string(), "".to_string()).await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::StorageAccessValidation { .. })
    ));
}

#[tokio::test]
async fn fail_if_referencing_other_token_balance() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "balance-1".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::StorageAccessValidation { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn fail_if_referencing_self_token_balance_after_wallet_creation() {
    let res = test_existing_user_op("balance-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn fail_with_unstaked_paymaster_returning_context() -> anyhow::Result<()> {
    let context = setup().await?;
    let pm = deploy_test_storage_account(context.client.clone())
        .await
        .expect("deploy succeed");
    let acct = deploy_test_recursion_account(context.client.clone(), context.entry_point.address)
        .await
        .expect("deploy succeed");

    let mut paymaster_and_data = vec![];
    paymaster_and_data.extend_from_slice(pm.address.as_bytes());
    paymaster_and_data.extend_from_slice("postOp-context".as_bytes());

    let user_op = UserOperation {
        sender: acct.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: U256::from(0),
        verification_gas_limit: U256::from(50000),
        pre_verification_gas: U256::from(0),
        max_fee_per_gas: U256::from(0),
        max_priority_fee_per_gas: U256::from(0),
        paymaster_and_data: Bytes::from(paymaster_and_data),
        signature: Bytes::default(),
    };
    let res = validate(&context, user_op).await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::CallStackValidation { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn fail_with_validation_recursively_calls_handle_ops() -> anyhow::Result<()> {
    let context = setup().await?;
    let acct = deploy_test_recursion_account(context.client.clone(), context.entry_point.address)
        .await
        .expect("deploy succeed");
    let user_op = UserOperation {
        sender: acct.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: U256::from(0),
        verification_gas_limit: U256::from(50000),
        pre_verification_gas: U256::from(50000),
        max_fee_per_gas: U256::from(0),
        max_priority_fee_per_gas: U256::from(0),
        paymaster_and_data: Bytes::default(),
        signature: Bytes::from("handleOps".as_bytes().to_vec()),
    };
    let res = validate(&context, user_op).await;
    assert!(matches!(
        res,
        Err(SimulateValidationError::CallStackValidation { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn succeed_with_inner_revert() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_op(
        &context,
        "inner-revert".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await
    .expect("succeed");
    Ok(())
}

#[tokio::test]
async fn fail_with_inner_oog_revert() -> anyhow::Result<()> {
    let context = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    let res = test_user_op(
        &context,
        "oog".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await;
    assert!(matches!(res, Err(SimulateValidationError::OutOfGas { .. })));
    Ok(())
}
