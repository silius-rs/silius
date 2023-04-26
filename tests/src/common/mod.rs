use std::{ops::Mul, sync::Arc, time::Duration};

use aa_bundler_primitives::UserOperation;
use ethers::{
    prelude::{
        k256::ecdsa::SigningKey, MiddlewareBuilder, NonceManagerMiddleware, SignerMiddleware,
    },
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer, Wallet},
    types::{Address, Bytes, TransactionRequest, U256},
    utils::{Geth, GethInstance},
};
use tempdir::TempDir;

use self::gen::{
    EntryPointContract, TestOpcodesAccount, TestOpcodesAccountFactory, TestRecursionAccount,
    TestRulesAccountFactory, TestStorageAccount, TestStorageAccountFactory, TracerTest,
};
pub mod gen;

pub const KEY_PHRASE: &str = "test test test test test test test test test test test junk";
pub type ClientType = NonceManagerMiddleware<SignerMiddleware<Provider<Http>, LocalWallet>>;

pub struct DeployedContract<C> {
    contract: C,
    pub address: Address,
}
impl<C> DeployedContract<C> {
    pub fn new(contract: C, address: Address) -> Self {
        Self { contract, address }
    }

    pub fn contract(&self) -> &C {
        &self.contract
    }
}

pub async fn deploy_entry_point<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<EntryPointContract<M>>> {
    let (entry_point, receipt) = EntryPointContract::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(entry_point, address))
}

pub async fn deploy_test_opcode_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccount<M>>> {
    let (account, receipt) = TestOpcodesAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_opcode_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccountFactory<M>>> {
    let (factory, receipt) = TestOpcodesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn deploy_test_storage_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestStorageAccountFactory<M>>> {
    let (factory, receipt) = TestStorageAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn deploy_test_storage_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestStorageAccount<M>>> {
    let (account, receipt) = TestStorageAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_recursion_account<M: Middleware + 'static>(
    client: Arc<M>,
    entry_point_address: Address,
) -> anyhow::Result<DeployedContract<TestRecursionAccount<M>>> {
    let (account, receipt) = TestRecursionAccount::deploy(client, entry_point_address)?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_rules_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestRulesAccountFactory<M>>> {
    let (factory, receipt) = TestRulesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn deploy_tracer_test<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TracerTest<M>>> {
    let (factory, receipt) = TracerTest::deploy(client, ())?.send_with_receipt().await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn sign(
    user_op: &mut UserOperation,
    entry_point_address: &Address,
    chain_id: &U256,
    key: Wallet<SigningKey>,
) -> anyhow::Result<()> {
    let user_op_hash = user_op.hash(entry_point_address, chain_id);
    let signature = key.sign_message(user_op_hash.0.as_bytes()).await?;
    user_op.signature = Bytes::from(signature.to_vec());
    Ok(())
}

pub async fn setup_geth() -> anyhow::Result<(GethInstance, ClientType)> {
    let chain_id: u64 = 1337;
    let tmp_dir = TempDir::new("test_geth")?;
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(KEY_PHRASE)
        .build()?;

    let geth = Geth::new().data_dir(tmp_dir.path().to_path_buf()).spawn();
    let provider =
        Provider::<Http>::try_from(geth.endpoint())?.interval(Duration::from_millis(10u64));

    let client = SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(chain_id))
        .nonce_manager(wallet.address());

    let coinbase = client.get_accounts().await?[0];
    let tx = TransactionRequest::new()
        .to(wallet.address())
        .value(U256::from(10).pow(U256::from(18)).mul(100))
        .from(coinbase);
    provider.send_transaction(tx, None).await?.await?;

    Ok((geth, client))
}
