pub mod common;

use aa_bundler::types::user_operation::UserOperation;
use common::gen::{
    EntryPointContract, TestOpcodesAccount, TestOpcodesAccountFactory, TestRulesAccount,
    TestRulesAccountFactory, TestStorageAccountFactory,
};
use common::DeployedContract;
use ethers::abi::Token;
use ethers::prelude::BaseContract;
use ethers::providers::{Http, Middleware};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Address;
use ethers::utils::{parse_units, AnvilInstance};
use ethers::{
    core::utils::Anvil,
    prelude::SignerMiddleware,
    providers::Provider,
    signers::{LocalWallet, Signer},
    types::{Bytes, U256},
};
use std::ops::Deref;
use std::{convert::TryFrom, sync::Arc, time::Duration};
use tokio::sync::OnceCell;

use crate::common::{
    deploy_entry_point, deploy_test_opcode_account, deploy_test_opcode_account_factory,
    deploy_test_recursion_account, deploy_test_rules_account_factory, deploy_test_storage_account,
    deploy_test_storage_account_factory,
};

struct TestContext<M> {
    pub client: Arc<M>,
    pub _anvil: AnvilInstance,
    pub entry_point: DeployedContract<EntryPointContract<M>>,
    pub paymaster: DeployedContract<TestOpcodesAccount<M>>,
    pub opcodes_factory: DeployedContract<TestOpcodesAccountFactory<M>>,
    pub storage_factory: DeployedContract<TestStorageAccountFactory<M>>,
    pub _rules_factory: DeployedContract<TestRulesAccountFactory<M>>,
    pub storage_account: DeployedContract<TestRulesAccount<M>>,
}

type ClientType = SignerMiddleware<Provider<Http>, LocalWallet>;

static CONTEXT: OnceCell<anyhow::Result<TestContext<ClientType>>> = OnceCell::const_new();

async fn get_global_context() -> &'static TestContext<ClientType> {
    let res = CONTEXT.get_or_init(setup).await;
    res.as_ref().expect("setup failed")
}

async fn setup() -> anyhow::Result<TestContext<ClientType>> {
    let anvil = Anvil::new().spawn();
    let provider =
        Provider::<Http>::try_from(anvil.endpoint())?.interval(Duration::from_millis(10u64));
    let wallet: LocalWallet = anvil.keys()[0].clone().into();

    let client = Arc::new(SignerMiddleware::new(
        provider,
        wallet.clone().with_chain_id(anvil.chain_id()),
    ));
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

    let opcodes_factory = deploy_test_opcode_account_factory(client.clone()).await?;
    let storage_factory = deploy_test_storage_account_factory(client.clone()).await?;
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
    Ok(TestContext::<ClientType> {
        client: client.clone(),
        _anvil: anvil,
        entry_point,
        paymaster,
        opcodes_factory,
        storage_factory,
        _rules_factory: rules_factory,
        storage_account: DeployedContract::new(
            TestRulesAccount::new(storage_account_address, client.clone()),
            storage_account_address,
        ),
    })
}

async fn create_storage_factory_init_code(
    salt: u64,
    init_func: String,
) -> anyhow::Result<(Bytes, Bytes)> {
    let context = get_global_context().await;
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
    let context = get_global_context().await;
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
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> anyhow::Result<UserOperation> {
    let context = get_global_context().await;

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

async fn existing_storage_account_user_op(validate_rule: String, pm_rule: String) -> UserOperation {
    let context = get_global_context().await;

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

fn validate(_user_op: UserOperation) -> anyhow::Result<()> {
    // TODO
    Ok(())
}

async fn test_user_op(
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> anyhow::Result<()> {
    let user_op = create_test_user_op(
        validate_rule,
        pm_rule,
        init_code,
        init_func,
        factory_address,
    )
    .await?;
    validate(user_op)
}

async fn test_existing_user_op(validate_rule: String, pm_rule: String) -> anyhow::Result<()> {
    let user_op = existing_storage_account_user_op(validate_rule, pm_rule).await;
    validate(user_op)
}

#[tokio::test]
async fn accept_plain_request() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        "".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect("succeed");
}

#[tokio::test]
#[should_panic]
async fn reject_unkown_rule() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        "<unknown-rule>".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect_err("unknown rule");
}

#[tokio::test]
#[should_panic]
async fn fail_with_bad_opcode_in_ctr() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("coinbase".to_string())
        .await
        .unwrap();
    test_user_op(
        "".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect_err("factory uses banned opcode: COINBASE");
}

#[tokio::test]
#[should_panic]
async fn fail_with_bad_opcode_in_paymaster() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        "".to_string(),
        Some("coinbase".to_string()),
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect_err("paymaster uses banned opcode: COINBASE");
}

#[tokio::test]
#[should_panic]
async fn fail_with_bad_opcode_in_validation() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        "blockhash".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect_err("account uses banned opcode: BLOCKHASH");
}

#[tokio::test]
// #[should_panic]
async fn fail_if_create_too_many() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_opcode_factory_init_code("".to_string())
        .await
        .unwrap();
    test_user_op(
        "create2".to_string(),
        None,
        init_code,
        init_func,
        context.opcodes_factory.address,
    )
    .await
    .expect("account uses banned opcode: CREATE2");
}

#[tokio::test]
#[should_panic]
async fn fail_referencing_self_token() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_op(
        "balance-self".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await
    .expect_err("unstaked account accessed");
}

#[tokio::test]
async fn account_succeeds_referecing_its_own_balance() {
    test_existing_user_op("balance-self".to_string(), "".to_string())
        .await
        .expect("succeed");
}

#[tokio::test]
#[should_panic]
async fn account_fail_to_read_allowance_of_address() {
    test_existing_user_op("allowance-self-1".to_string(), "".to_string())
        .await
        .expect_err("account has forbidden read");
}

#[tokio::test]
async fn account_can_reference_its_own_allowance_on_other_contract_balance() {
    test_existing_user_op("allowance-1-self".to_string(), "".to_string())
        .await
        .expect("succeed");
}

#[tokio::test]
async fn access_self_struct_data() {
    test_existing_user_op("struct-self".to_string(), "".to_string())
        .await
        .expect("succeed");
}

#[tokio::test]
#[should_panic]
async fn fail_to_access_other_address_struct_data() {
    test_existing_user_op("struct-1".to_string(), "".to_string())
        .await
        .expect_err("account has forbidden read");
}

#[tokio::test]
#[should_panic]
async fn fail_if_referencing_other_token_balance() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_op(
        "balance-1".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await
    .expect_err("account has forbidden read");
}

#[tokio::test]
async fn fail_if_referencing_self_token_balance_after_wallet_creation() {
    test_existing_user_op("balance-self".to_string(), "".to_string())
        .await
        .expect("succeed");
}

#[tokio::test]
#[should_panic]
async fn fail_with_unstaked_paymaster_returning_context() {
    let context = get_global_context().await;
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
    validate(user_op).expect_err("unstaked paymaster must not return context");
}

#[tokio::test]
#[should_panic]
async fn fail_with_validation_recursively_calls_handle_ops() {
    let context = get_global_context().await;
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
    validate(user_op).expect_err("illegal call into EntryPoint");
}

#[tokio::test]
async fn succeed_with_inner_revert() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_op(
        "inner-revert".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await
    .expect("succeed");
}

#[tokio::test]
#[should_panic]
async fn fail_with_inner_oog_revert() {
    let context = get_global_context().await;
    let (init_code, init_func) = create_storage_factory_init_code(0, "".to_string())
        .await
        .unwrap();
    test_user_op(
        "oog".to_string(),
        None,
        init_code,
        init_func,
        context.storage_factory.address,
    )
    .await
    .expect_err("oog");
}
