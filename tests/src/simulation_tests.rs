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
use silius_contracts::EntryPoint;
use silius_primitives::consts::entities::{ACCOUNT, FACTORY, PAYMASTER};
use silius_primitives::simulation::SimulationError;
use silius_primitives::{Chain, UoPoolMode, UserOperation};
use silius_uopool::canonical::simulation::SimulationResult;
use silius_uopool::{mempool_id, MemoryMempool, MemoryReputation, Reputation, UoPool};
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
    let ep = deploy_entry_point(client.clone()).await?;
    let paymaster = deploy_test_opcode_account(client.clone()).await?;
    ep.contract()
        .deposit_to(paymaster.address)
        .value(parse_units("0.1", "ether").unwrap())
        .send()
        .await?;
    paymaster
        .contract()
        .add_stake(ep.address)
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

    ep.contract()
        .deposit_to(storage_account_address)
        .value(parse_units("1", "ether").unwrap())
        .send()
        .await?;

    let m_id = mempool_id(&ep.address, &U256::from(chain_id));
    let mut ep_map = HashMap::new();
    ep_map.insert(m_id, EntryPoint::new(client.clone(), ep.address));
    let mempools = Box::new(MemoryMempool::default());
    let mut reputation = Box::new(MemoryReputation::default());
    reputation.init(10, 10, 10, 1u64.into(), 1u64.into());
    let pool = UoPool::new(
        EntryPoint::new(client.clone(), ep.address),
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
        entry_point: ep,
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
    let c = setup().await?;
    let contract: &BaseContract = c.storage_factory.contract().deref().deref();
    let func = contract.abi().function("create")?;
    let init_func =
        func.encode_input(&[Token::Uint(U256::from(salt)), Token::String(init_func)])?;

    let mut init_code = vec![];
    init_code.extend_from_slice(c.storage_factory.address.as_bytes());
    init_code.extend_from_slice(init_func.as_ref());

    Ok((init_code.into(), init_func.into()))
}
async fn create_opcode_factory_init_code(init_func: String) -> anyhow::Result<(Bytes, Bytes)> {
    let c = setup().await?;
    let contract: &BaseContract = c.opcodes_factory.contract().deref().deref();
    let token = vec![Token::String(init_func)];
    let func = contract.abi().function("create")?;
    let init_func = func.encode_input(&token)?;

    let mut init_code = vec![];
    init_code.extend_from_slice(c.opcodes_factory.address.as_bytes());
    init_code.extend_from_slice(&init_func);

    Ok((init_code.into(), init_func.into()))
}

async fn create_test_user_operation(
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

    let sig = Bytes::from(validate_rule.as_bytes().to_vec());

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
        call_gas_limit: 1000000.into(),
        verification_gas_limit: 1000000.into(),
        pre_verification_gas: 50000.into(),
        max_fee_per_gas: U256::zero(),
        max_priority_fee_per_gas: U256::zero(),
        paymaster_and_data,
        signature: sig,
    })
}

fn existing_storage_account_user_operation(
    context: &TestContext<ClientType>,
    validate_rule: String,
    pm_rule: String,
) -> UserOperation {
    let mut paymaster_and_data = vec![];
    paymaster_and_data.extend_from_slice(context.paymaster.address.as_bytes());
    paymaster_and_data.extend_from_slice(pm_rule.as_bytes());

    let sig = Bytes::from(validate_rule.as_bytes().to_vec());
    UserOperation {
        sender: context.storage_account.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: 1000000.into(),
        verification_gas_limit: 1000000.into(),
        pre_verification_gas: 50000.into(),
        max_fee_per_gas: U256::zero(),
        max_priority_fee_per_gas: U256::zero(),
        paymaster_and_data: Bytes::from(paymaster_and_data),
        signature: sig,
    }
}

async fn validate(
    context: &TestContext<ClientType>,
    user_op: UserOperation,
) -> Result<SimulationResult, SimulationError> {
    context.uopool.simulate_user_operation(&user_op, true).await
}

async fn test_user_operation(
    context: &TestContext<ClientType>,
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> Result<SimulationResult, SimulationError> {
    let uo = create_test_user_operation(
        &context,
        validate_rule,
        pm_rule,
        init_code,
        init_func,
        factory_address,
    )
    .await
    .expect("Create test user operation failed.");
    validate(&context, uo).await
}

async fn test_existing_user_operation(
    validate_rule: String,
    pm_rule: String,
) -> Result<SimulationResult, SimulationError> {
    let c = setup().await.expect("Setup context failed");
    let uo = existing_storage_account_user_operation(&c, validate_rule, pm_rule);
    validate(&c, uo).await
}

#[tokio::test]
async fn accept_plain_request() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();

    test_user_operation(
        &c,
        "".to_string(),
        None,
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await
    .expect("succeed");

    Ok(())
}

#[tokio::test]
async fn reject_unkown_rule() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "<unknown-rule>".to_string(),
        None,
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulationError::Validation { message }) if message.contains("unknown-rule")
    ));

    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_ctr() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("coinbase".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "".to_string(),
        None,
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulationError::ForbiddenOpcode { entity, opcode }) if entity==FACTORY && opcode == "COINBASE"
    ));

    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_paymaster() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "".to_string(),
        Some("coinbase".to_string()),
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulationError::ForbiddenOpcode { entity, opcode }) if entity==PAYMASTER && opcode == "COINBASE"
    ));

    Ok(())
}

#[tokio::test]
async fn fail_with_bad_opcode_in_validation() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "blockhash".to_string(),
        None,
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulationError::ForbiddenOpcode { entity, opcode }) if entity==ACCOUNT && opcode == "BLOCKHASH"
    ));

    Ok(())
}

#[tokio::test]
async fn fail_if_create_too_many() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "create2".to_string(),
        None,
        init_code,
        init_func,
        c.opcodes_factory.address,
    )
    .await;
    assert!(matches!(
        res,
        Err(SimulationError::ForbiddenOpcode { entity, opcode }) if entity==ACCOUNT && opcode == "CREATE2"
    ));

    Ok(())
}

#[tokio::test]
async fn fail_referencing_self_token() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "balance-self".to_string(),
        None,
        init_code,
        init_func,
        c.storage_factory.address,
    )
    .await;
    assert!(matches!(res, Err(SimulationError::Unstaked { .. })));

    Ok(())
}

#[tokio::test]
async fn account_succeeds_referecing_its_own_balance() {
    let res = test_existing_user_operation("balance-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn account_fail_to_read_allowance_of_address() {
    let res = test_existing_user_operation("allowance-self-1".to_string(), "".to_string()).await;
    assert!(matches!(res, Err(SimulationError::StorageAccess { .. })));
}

#[tokio::test]
async fn account_can_reference_its_own_allowance_on_other_contract_balance() {
    let res = test_existing_user_operation("allowance-1-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn access_self_struct_data() {
    let res = test_existing_user_operation("struct-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn fail_to_access_other_address_struct_data() {
    let res = test_existing_user_operation("struct-1".to_string(), "".to_string()).await;
    assert!(matches!(res, Err(SimulationError::StorageAccess { .. })));
}

#[tokio::test]
async fn fail_if_referencing_other_token_balance() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "balance-1".to_string(),
        None,
        init_code,
        init_func,
        c.storage_factory.address,
    )
    .await;
    assert!(matches!(res, Err(SimulationError::StorageAccess { .. })));

    Ok(())
}

#[tokio::test]
async fn fail_if_referencing_self_token_balance_after_wallet_creation() {
    let res = test_existing_user_operation("balance-self".to_string(), "".to_string()).await;
    assert!(matches!(res, Ok(..)));
}

#[tokio::test]
async fn fail_with_unstaked_paymaster_returning_context() -> anyhow::Result<()> {
    let c = setup().await?;
    let pm = deploy_test_storage_account(c.client.clone())
        .await
        .expect("deploy succeed");
    let acct = deploy_test_recursion_account(c.client.clone(), c.entry_point.address)
        .await
        .expect("deploy succeed");

    let mut paymaster_and_data = vec![];
    paymaster_and_data.extend_from_slice(pm.address.as_bytes());
    paymaster_and_data.extend_from_slice("postOp-context".as_bytes());

    let uo = UserOperation {
        sender: acct.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: U256::zero(),
        verification_gas_limit: 50000.into(),
        pre_verification_gas: U256::zero(),
        max_fee_per_gas: U256::zero(),
        max_priority_fee_per_gas: U256::zero(),
        paymaster_and_data: Bytes::from(paymaster_and_data),
        signature: Bytes::default(),
    };

    let res = validate(&c, uo).await;
    assert!(matches!(res, Err(SimulationError::Unstaked { .. })));

    Ok(())
}

#[tokio::test]
async fn fail_with_validation_recursively_calls_handle_ops() -> anyhow::Result<()> {
    let c = setup().await?;
    let acct = deploy_test_recursion_account(c.client.clone(), c.entry_point.address)
        .await
        .expect("deploy succeed");
    let uo = UserOperation {
        sender: acct.address,
        nonce: U256::zero(),
        init_code: Bytes::default(),
        call_data: Bytes::default(),
        call_gas_limit: U256::zero(),
        verification_gas_limit: 50000.into(),
        pre_verification_gas: 50000.into(),
        max_fee_per_gas: U256::zero(),
        max_priority_fee_per_gas: U256::zero(),
        paymaster_and_data: Bytes::default(),
        signature: Bytes::from("handleOps".as_bytes().to_vec()),
    };

    let res = validate(&c, uo).await;
    assert!(matches!(res, Err(SimulationError::CallStack { .. })));

    Ok(())
}

#[tokio::test]
async fn succeed_with_inner_revert() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_operation(
        &c,
        "inner-revert".to_string(),
        None,
        init_code,
        init_func,
        c.storage_factory.address,
    )
    .await
    .expect("succeed");

    Ok(())
}

#[tokio::test]
async fn fail_with_inner_oog_revert() -> anyhow::Result<()> {
    let c = setup().await?;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();

    let res = test_user_operation(
        &c,
        "oog".to_string(),
        None,
        init_code,
        init_func,
        c.storage_factory.address,
    )
    .await;
    assert!(matches!(res, Err(SimulationError::OutOfGas { .. })));

    Ok(())
}
