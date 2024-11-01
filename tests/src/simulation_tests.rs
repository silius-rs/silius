use crate::common::{
    deploy_entry_point, deploy_test_coin, deploy_test_opcode_account,
    deploy_test_opcode_account_factory, deploy_test_recursion_account,
    deploy_test_rules_account_factory, deploy_test_storage_account,
    deploy_test_storage_account_factory,
    gen::{
        EntryPointContract, TestOpcodesAccount, TestOpcodesAccountFactory, TestRulesAccount,
        TestStorageAccountFactory,
    },
    setup_database_mempool_reputation, setup_geth, setup_memory_mempool_reputation, ClientType,
    DeployedContract,
};
use alloy_chains::Chain;
use ethers::{
    abi::Token,
    prelude::BaseContract,
    providers::Middleware,
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
    utils::{parse_units, GethInstance},
};
use silius_contracts::EntryPoint;
use silius_mempool::{
    validate::{
        validator::{new_canonical, StandardValidator},
        UserOperationValidationOutcome, UserOperationValidator, UserOperationValidatorMode,
    },
    InvalidMempoolUserOperationError, Mempool, Reputation, SimulationError,
};
use silius_primitives::{
    constants::validation::entities::{FACTORY, PAYMASTER, SENDER},
    UserOperation, UserOperationSigned,
};
use std::{ops::Deref, sync::Arc, time::Duration};

struct TestContext<M>
where
    M: Middleware + 'static,
{
    pub client: Arc<M>,
    pub _geth: GethInstance,
    pub chain_id: u64,
    pub entry_point: DeployedContract<EntryPointContract<M>>,
    pub paymaster: DeployedContract<TestOpcodesAccount<M>>,
    pub opcodes_factory: DeployedContract<TestOpcodesAccountFactory<M>>,
    pub storage_factory: DeployedContract<TestStorageAccountFactory<M>>,
    pub storage_account: DeployedContract<TestRulesAccount<M>>,
    pub validator: StandardValidator<M>,
    pub mempool: Mempool,
    pub reputation: Reputation,
}

async fn setup_basic() -> eyre::Result<(
    Arc<ClientType>,
    DeployedContract<EntryPointContract<ClientType>>,
    u64,
    GethInstance,
    DeployedContract<TestOpcodesAccount<ClientType>>,
    DeployedContract<TestOpcodesAccountFactory<ClientType>>,
    DeployedContract<TestStorageAccountFactory<ClientType>>,
    DeployedContract<TestRulesAccount<ClientType>>,
)> {
    let chain_id = 1337u64;
    let (geth, _client, _) = setup_geth().await?;
    let client = Arc::new(_client);

    let ep = deploy_entry_point(client.clone()).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let paymaster = deploy_test_opcode_account(client.clone()).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

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
    tokio::time::sleep(Duration::from_millis(100)).await;

    let opcodes_factory = deploy_test_opcode_account_factory(client.clone()).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let storage_factory =
        deploy_test_storage_account_factory(client.clone(), test_coin.address).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let rules_factory = deploy_test_rules_account_factory(client.clone()).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let storage_account_call = rules_factory.contract().create("".into());
    let storage_account_address = storage_account_call.call().await?;

    storage_account_call.send().await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    ep.contract()
        .deposit_to(storage_account_address)
        .value(parse_units("1", "ether").unwrap())
        .send()
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((
        client.clone(),
        ep,
        chain_id,
        geth,
        paymaster,
        opcodes_factory,
        storage_factory,
        DeployedContract::new(
            TestRulesAccount::new(storage_account_address, client.clone()),
            storage_account_address,
        ),
    ))
}

async fn setup_database() -> eyre::Result<TestContext<ClientType>> {
    let (client, ep, chain_id, _geth, paymaster, opcodes_factory, storage_factory, storage_account) =
        setup_basic().await?;
    let (mempool, reputation) = setup_database_mempool_reputation();

    let entry_point = EntryPoint::new(client.clone(), ep.address);
    let c = Chain::from(chain_id);

    let validator =
        new_canonical(entry_point, c.clone(), U256::from(3000000_u64), U256::from(1u64));

    Ok(TestContext {
        client: client.clone(),
        _geth,
        chain_id,
        entry_point: ep,
        paymaster,
        opcodes_factory,
        storage_factory,
        storage_account,
        validator,
        mempool,
        reputation,
    })
}

async fn setup_memory() -> eyre::Result<TestContext<ClientType>> {
    let (client, ep, chain_id, _geth, paymaster, opcodes_factory, storage_factory, storage_account) =
        setup_basic().await?;
    let (mempool, reputation) = setup_memory_mempool_reputation();
    let entry_point = EntryPoint::new(client.clone(), ep.address);
    let c = Chain::from(chain_id);

    let validator =
        new_canonical(entry_point, c.clone(), U256::from(3000000_u64), U256::from(1u64));
    Ok(TestContext {
        client: client.clone(),
        _geth,
        chain_id,
        entry_point: ep,
        paymaster,
        opcodes_factory,
        storage_factory,
        storage_account,
        validator,
        mempool,
        reputation,
    })
}

async fn create_storage_factory_init_code(
    salt: u64,
    init_func: String,
) -> eyre::Result<(Bytes, Bytes)> {
    let c = setup_database().await?;
    let contract: &BaseContract = c.storage_factory.contract().deref().deref();
    let func = contract.abi().function("create")?;
    let init_func =
        func.encode_input(&[Token::Uint(U256::from(salt)), Token::String(init_func)])?;

    let mut init_code = vec![];
    init_code.extend_from_slice(c.storage_factory.address.as_bytes());
    init_code.extend_from_slice(init_func.as_ref());

    Ok((init_code.into(), init_func.into()))
}

async fn create_opcode_factory_init_code(init_func: String) -> eyre::Result<(Bytes, Bytes)> {
    let c = setup_database().await?;
    let contract: &BaseContract = c.opcodes_factory.contract().deref().deref();
    let token = vec![Token::String(init_func)];
    let func = contract.abi().function("create")?;
    let init_func = func.encode_input(&token)?;

    let mut init_code = vec![];
    init_code.extend_from_slice(c.opcodes_factory.address.as_bytes());
    init_code.extend_from_slice(&init_func);

    Ok((init_code.into(), init_func.into()))
}

async fn create_test_user_operation<M>(
    context: &TestContext<M>,
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> eyre::Result<UserOperationSigned>
where
    M: Middleware + 'static,
{
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
    eyre::ensure!(
        !head.iter().any(|i| *i != 0),
        format!("call init code returns non address data : {call_init_code_for_addr:?}")
    );

    let sender = Address::from_slice(address);

    Ok(UserOperationSigned {
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

fn existing_storage_account_user_operation<M>(
    context: &TestContext<M>,
    validate_rule: String,
    pm_rule: String,
) -> UserOperationSigned
where
    M: Middleware + 'static,
{
    let mut paymaster_and_data = vec![];
    paymaster_and_data.extend_from_slice(context.paymaster.address.as_bytes());
    paymaster_and_data.extend_from_slice(pm_rule.as_bytes());

    let sig = Bytes::from(validate_rule.as_bytes().to_vec());
    UserOperationSigned {
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

async fn validate<M>(
    context: &TestContext<M>,
    uo: UserOperationSigned,
) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError>
where
    M: Middleware + 'static,
{
    context
        .validator
        .validate_user_operation(
            &UserOperation::from_user_operation_signed(
                uo.hash(&context.entry_point.address, context.chain_id),
                uo.clone(),
            ),
            &context.mempool,
            &context.reputation,
            None,
            UserOperationValidatorMode::Simulation | UserOperationValidatorMode::SimulationTrace,
        )
        .await
}

async fn test_user_operation<M>(
    context: &TestContext<M>,
    validate_rule: String,
    pm_rule: Option<String>,
    init_code: Bytes,
    init_func: Bytes,
    factory_address: Address,
) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError>
where
    M: Middleware + 'static,
{
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
) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError> {
    let c = setup_database().await.expect("Setup context failed");
    let uo = existing_storage_account_user_operation(&c, validate_rule, pm_rule);
    validate(&c, uo).await
}

macro_rules! accept_plain_request {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let (init_code, init_func) = create_opcode_factory_init_code("".into()).await.unwrap();

            let c = $setup;
            test_user_operation(
                &c,
                "".into(),
                None,
                init_code,
                init_func,
                c.opcodes_factory.address,
            )
            .await
            .expect("succeed");
            Ok(())
        }
    };
}

accept_plain_request!(setup_database().await?, accept_plain_request_database);
accept_plain_request!(setup_memory().await?, accept_plain_request_memory);

macro_rules! reject_unkown_rule {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let (init_code, init_func) = create_opcode_factory_init_code("".into())
                .await
                .unwrap();
            let c = $setup;
            let res = test_user_operation(
                &c,
                "<unknown-rule>".into(),
                None,
                init_code.clone(),
                init_func.clone(),
                c.opcodes_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Validation { inner })) if inner.contains("unknown-rule")
            ));

            Ok(())
        }
    };
}

reject_unkown_rule!(setup_database().await?, reject_unkown_rule_database);
reject_unkown_rule!(setup_memory().await?, reject_unkown_rule_memory);

macro_rules! fail_with_bad_opcode_in_ctr {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let (init_code, init_func) = create_opcode_factory_init_code("coinbase".into())
                .await
                .unwrap();
            let c = $setup;
            let res = test_user_operation(
                &c,
                "".into(),
                None,
                init_code.clone(),
                init_func.clone(),
                c.opcodes_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Opcode { entity, opcode })) if entity==FACTORY && opcode == "COINBASE"
            ));

            Ok(())
        }
    };
}

fail_with_bad_opcode_in_ctr!(setup_database().await?, fail_with_bad_opcode_in_ctr_database);
fail_with_bad_opcode_in_ctr!(setup_memory().await?, fail_with_bad_opcode_in_ctr_memory);

macro_rules! fail_with_bad_opcode_in_paymaster {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let (init_code, init_func) = create_opcode_factory_init_code("".into())
                .await
                .unwrap();
            let c = $setup;
            let res = test_user_operation(
                &c,
                "".into(),
                Some("coinbase".into()),
                init_code,
                init_func,
                c.opcodes_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Opcode { entity, opcode })) if entity==PAYMASTER && opcode == "COINBASE"
            ));

            Ok(())
        }
    };
}

fail_with_bad_opcode_in_paymaster!(
    setup_database().await?,
    fail_with_bad_opcode_in_paymaster_database
);
fail_with_bad_opcode_in_paymaster!(setup_memory().await?, fail_with_bad_opcode_in_paymaster_memory);

macro_rules! fail_with_bad_opcode_in_validation {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let (init_code, init_func) = create_opcode_factory_init_code("".into())
                .await
                .unwrap();

            let res = test_user_operation(
                &c,
                "blockhash".into(),
                None,
                init_code,
                init_func,
                c.opcodes_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Opcode { entity, opcode })) if entity==SENDER && opcode == "BLOCKHASH"
            ));

            Ok(())
        }
    };
}

fail_with_bad_opcode_in_validation!(
    setup_database().await?,
    fail_with_bad_opcode_in_validation_database
);
fail_with_bad_opcode_in_validation!(
    setup_memory().await?,
    fail_with_bad_opcode_in_validation_memory
);

macro_rules!fail_if_create_too_many {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let (init_code, init_func) = create_opcode_factory_init_code("".into())
                .await
                .unwrap();

            let res = test_user_operation(
                &c,
                "create2".into(),
                None,
                init_code,
                init_func,
                c.opcodes_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Opcode { entity, opcode })) if entity==SENDER && opcode == "CREATE2"
            ));

            Ok(())
        }
    };
}

fail_if_create_too_many!(setup_database().await?, fail_if_create_too_many_database);
fail_if_create_too_many!(setup_memory().await?, fail_if_create_too_many_memory);

macro_rules! fail_referencing_self_token {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let (init_code, init_func) =
                create_storage_factory_init_code(0, "".into()).await.unwrap();

            let res = test_user_operation(
                &c,
                "balance-self".into(),
                None,
                init_code,
                init_func,
                c.storage_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Unstaked { .. }))
            ));

            Ok(())
        }
    };
}

fail_referencing_self_token!(setup_database().await?, fail_referencing_self_token_database);
fail_referencing_self_token!(setup_memory().await?, fail_referencing_self_token_memory);

macro_rules! test_existing_user_operation {
    ($setup:expr, $func_name: ident, $validate_rule: expr, $pm_rul:expr) => {
        #[tokio::test]
        async fn $func_name() -> eyre::Result<()> {
            let c = $setup;
            let uo = existing_storage_account_user_operation(&c, $validate_rule, $pm_rul);
            let res = validate(&c, uo).await;
            assert!(matches!(res, Ok(..)));
            Ok(())
        }
    };
}

test_existing_user_operation!(
    setup_database().await?,
    account_succeeds_referecing_its_own_balance_database,
    "balance-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_memory().await?,
    account_succeeds_referecing_its_own_balance_memory,
    "balance-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_database().await?,
    account_can_reference_its_own_allowance_on_other_contract_balance_database,
    "allowance-1-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_memory().await?,
    account_can_reference_its_own_allowance_on_other_contract_balance_memory,
    "allowance-1-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_database().await?,
    access_self_struct_data_database,
    "struct-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_memory().await?,
    access_self_struct_data_memory,
    "struct-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_database().await?,
    fail_if_referencing_self_token_balance_after_wallet_creation_database,
    "balance-self".into(),
    "".into()
);

test_existing_user_operation!(
    setup_memory().await?,
    fail_if_referencing_self_token_balance_after_wallet_creation_memory,
    "balance-self".into(),
    "".into()
);

macro_rules! fail_with_unstaked_paymaster_returning_context {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let pm = deploy_test_storage_account(c.client.clone()).await.expect("deploy succeed");
            let acct = deploy_test_recursion_account(c.client.clone(), c.entry_point.address)
                .await
                .expect("deploy succeed");

            let mut paymaster_and_data = vec![];
            paymaster_and_data.extend_from_slice(pm.address.as_bytes());
            paymaster_and_data.extend_from_slice("postOp-context".as_bytes());

            let uo = UserOperationSigned {
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
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::Unstaked { .. }))
            ));

            Ok(())
        }
    };
}

fail_with_unstaked_paymaster_returning_context!(
    setup_database().await?,
    fail_with_unstaked_paymaster_returning_context_database
);
fail_with_unstaked_paymaster_returning_context!(
    setup_memory().await?,
    fail_with_unstaked_paymaster_returning_context_memory
);

macro_rules! fail_with_validation_recursively_calls_handle_ops {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let acct = deploy_test_recursion_account(c.client.clone(), c.entry_point.address)
                .await
                .expect("deploy succeed");
            let uo = UserOperationSigned {
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
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(
                    SimulationError::CallStack { .. }
                ))
            ));

            Ok(())
        }
    };
}

fail_with_validation_recursively_calls_handle_ops!(
    setup_database().await?,
    fail_with_validation_recursively_calls_handle_ops_database
);
fail_with_validation_recursively_calls_handle_ops!(
    setup_memory().await?,
    fail_with_validation_recursively_calls_handle_ops_memory
);

macro_rules! succeed_with_inner_revert {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let (init_code, init_func) =
                create_storage_factory_init_code(0, "".into()).await.unwrap();
            test_user_operation(
                &c,
                "inner-revert".into(),
                None,
                init_code,
                init_func,
                c.storage_factory.address,
            )
            .await
            .expect("succeed");

            Ok(())
        }
    };
}

succeed_with_inner_revert!(setup_database().await?, succeed_with_inner_revert_database);
succeed_with_inner_revert!(setup_memory().await?, succeed_with_inner_revert_memory);

macro_rules! fail_with_inner_oog_revert {
    ($setup:expr, $name: ident) => {
        #[tokio::test]
        async fn $name() -> eyre::Result<()> {
            let c = $setup;
            let (init_code, init_func) =
                create_storage_factory_init_code(0, "".into()).await.unwrap();

            let res = test_user_operation(
                &c,
                "oog".into(),
                None,
                init_code,
                init_func,
                c.storage_factory.address,
            )
            .await;
            assert!(matches!(
                res,
                Err(InvalidMempoolUserOperationError::Simulation(SimulationError::OutOfGas { .. }))
            ));

            Ok(())
        }
    };
}

fail_with_inner_oog_revert!(setup_database().await?, fail_with_inner_oog_revert_database);
fail_with_inner_oog_revert!(setup_memory().await?, fail_with_inner_oog_revert_memory);
