use self::gen::{
    EntryPointContract, TestCoin, TestOpcodesAccount, TestOpcodesAccountFactory,
    TestRecursionAccount, TestRulesAccountFactory, TestStorageAccount, TestStorageAccountFactory,
    TracerTest,
};
use ethers::{
    prelude::{MiddlewareBuilder, NonceManagerMiddleware, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer},
    types::{Address, TransactionRequest, U256},
    utils::{Geth, GethInstance},
};
use std::{ops::Mul, sync::Arc, time::Duration};
use tempdir::TempDir;

pub mod gen;

pub const KEY_PHRASE: &str = "test test test test test test test test test test test junk";
pub type ClientType = NonceManagerMiddleware<SignerMiddleware<Provider<Http>, LocalWallet>>;

pub struct DeployedContract<C> {
    contract: C,
    pub address: Address,
}
impl<C> DeployedContract<C> {
    pub fn new(contract: C, addr: Address) -> Self {
        Self {
            contract,
            address: addr,
        }
    }

    pub fn contract(&self) -> &C {
        &self.contract
    }
}

pub async fn deploy_entry_point<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<EntryPointContract<M>>> {
    let (ep, receipt) = EntryPointContract::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(ep, addr))
}

pub async fn deploy_test_opcode_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccount<M>>> {
    let (acc, receipt) = TestOpcodesAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_opcode_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccountFactory<M>>> {
    let (factory, receipt) = TestOpcodesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_storage_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
    test_coin_addr: Address,
) -> anyhow::Result<DeployedContract<TestStorageAccountFactory<M>>> {
    let (factory, receipt) = TestStorageAccountFactory::deploy(client, test_coin_addr)?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_storage_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestStorageAccount<M>>> {
    let (acc, receipt) = TestStorageAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_recursion_account<M: Middleware + 'static>(
    client: Arc<M>,
    ep: Address,
) -> anyhow::Result<DeployedContract<TestRecursionAccount<M>>> {
    let (acc, receipt) = TestRecursionAccount::deploy(client, ep)?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_rules_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestRulesAccountFactory<M>>> {
    let (factory, receipt) = TestRulesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_tracer_test<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TracerTest<M>>> {
    let (factory, receipt) = TracerTest::deploy(client, ())?.send_with_receipt().await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_coin<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestCoin<M>>> {
    let (factory, receipt) = TestCoin::deploy(client, ())?.send_with_receipt().await?;
    let addr = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, addr))
}

pub async fn setup_geth() -> anyhow::Result<(GethInstance, ClientType, Provider<Http>)> {
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

    Ok((geth, client, provider))
}
