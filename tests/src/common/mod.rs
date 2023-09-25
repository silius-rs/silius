use self::gen::{
    EntryPointContract, TestCoin, TestOpcodesAccount, TestOpcodesAccountFactory,
    TestRecursionAccount, TestRulesAccountFactory, TestStorageAccount, TestStorageAccountFactory,
    TracerTest,
};
use ethers::{
    prelude::{MiddlewareBuilder, NonceManagerMiddleware, SignerMiddleware},
    providers::{Middleware, Provider, Ws},
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer},
    types::{Address, TransactionRequest, U256},
    utils::{Geth, GethInstance},
};
use std::{ops::Mul, sync::Arc, time::Duration};
use tempdir::TempDir;

pub mod gen;

pub const SEED_PHRASE: &str = "test test test test test test test test test test test junk";
pub type ClientType = NonceManagerMiddleware<SignerMiddleware<Provider<Ws>, LocalWallet>>;

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
) -> eyre::Result<DeployedContract<EntryPointContract<M>>> {
    let (ep, receipt) = EntryPointContract::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(ep, addr))
}

pub async fn deploy_test_opcode_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TestOpcodesAccount<M>>> {
    let (acc, receipt) = TestOpcodesAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_opcode_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TestOpcodesAccountFactory<M>>> {
    let (factory, receipt) = TestOpcodesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_storage_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
    test_coin_addr: Address,
) -> eyre::Result<DeployedContract<TestStorageAccountFactory<M>>> {
    let (factory, receipt) = TestStorageAccountFactory::deploy(client, test_coin_addr)?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_storage_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TestStorageAccount<M>>> {
    let (acc, receipt) = TestStorageAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_recursion_account<M: Middleware + 'static>(
    client: Arc<M>,
    ep: Address,
) -> eyre::Result<DeployedContract<TestRecursionAccount<M>>> {
    let (acc, receipt) = TestRecursionAccount::deploy(client, ep)?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(acc, addr))
}

pub async fn deploy_test_rules_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TestRulesAccountFactory<M>>> {
    let (factory, receipt) = TestRulesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_tracer_test<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TracerTest<M>>> {
    let (factory, receipt) = TracerTest::deploy(client, ())?.send_with_receipt().await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(factory, addr))
}

pub async fn deploy_test_coin<M: Middleware + 'static>(
    client: Arc<M>,
) -> eyre::Result<DeployedContract<TestCoin<M>>> {
    let (factory, receipt) = TestCoin::deploy(client, ())?.send_with_receipt().await?;
    let addr = receipt.contract_address.unwrap_or(Address::zero());
    Ok(DeployedContract::new(factory, addr))
}

pub async fn setup_geth() -> eyre::Result<(GethInstance, ClientType, Provider<Ws>)> {
    let chain_id: u64 = 1337;
    let tmp_dir = TempDir::new("test_geth")?;
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(SEED_PHRASE)
        .build()?;

    let geth = Geth::new().data_dir(tmp_dir.path().to_path_buf()).spawn();
    let provider = Provider::<Ws>::connect(geth.ws_endpoint())
        .await?
        .interval(Duration::from_millis(5u64));

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
