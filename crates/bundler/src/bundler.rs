use aa_bundler_contracts::entry_point::EntryPointAPI;
use aa_bundler_primitives::{consts::flashbots_relay_endpoints, Chain, UserOperation, Wallet};
use ethers::{
    prelude::{LocalWallet, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{
        transaction::eip2718::TypedTransaction, Address, Eip1559TransactionRequest, H256, U64,
    },
};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware, PendingBundleError::BundleNotIncluded};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tracing::{info, trace};
use url::Url;

type FlashbotsClientType<M> =
    Arc<SignerMiddleware<FlashbotsMiddleware<M, LocalWallet>, LocalWallet>>;

type EthClientType<M> = Arc<SignerMiddleware<M, LocalWallet>>;

/// The `SendBundleMode` determines whether to send the bundle to a Ethereum execution client or to Flashbots relay
#[derive(Clone, Debug, PartialEq)]
pub enum SendBundleMode {
    /// Send the bundle to a Ethereum execution client
    EthClient,
    /// Send the bundle to Flashbots relay
    Flashbots,
}

/// The `Bundler` struct is used to represent a bundler with necessary properties
#[derive(Clone, Debug)]
pub struct Bundler {
    /// Wallet instance representing the bundler's wallet.
    pub wallet: Wallet,
    /// URL of an Ethereum client.
    pub eth_client_address: String,
    /// Beneficiary address where the gas is refunded after execution
    pub beneficiary: Address,
    /// Entry point contract address
    pub entry_point: Address,
    /// [Chain](Chain) instance representing the blockchain network to be used
    pub chain: Chain,
    /// Send bundle mode determines whether to send the bundle to a regular Eth execution client or to Flashbots
    pub send_bundle_mode: SendBundleMode,
    /// Block Builder relay endpoints
    pub relay_endpoints: Option<Vec<String>>,
    /// Ethereum signer middleware
    pub eth_client: Option<EthClientType<Provider<Http>>>,
    /// Flashbots signer middleware
    pub fb_client: Option<FlashbotsClientType<Provider<Http>>>,
}

impl Bundler {
    /// Create a new `Bundler` instance
    /// if `send_bundle_mode` is `SendBundleMode::Flashbots` and `relay_endpoints` is `None`, the default Flashbots relay endpoint will be used
    ///
    /// # Returns
    /// * `Self` - A new `Bundler` instance
    pub fn new(
        wallet: Wallet,
        eth_client_address: String,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
        send_bundle_mode: SendBundleMode,
        relay_endpoints: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        if !(chain.id() == 1 || chain.id() == 5 || chain.id() == 11155111)
            && send_bundle_mode == SendBundleMode::Flashbots
        {
            panic!("Flashbots is only supported on Mainnet, Goerli and Sepolia");
        };

        match send_bundle_mode {
            SendBundleMode::EthClient => {
                let eth_client = Provider::<Http>::try_from(eth_client_address.clone())?;
                let client = Arc::new(SignerMiddleware::new(
                    eth_client.clone(),
                    wallet.signer.clone(),
                ));

                Ok(Self {
                    wallet,
                    eth_client_address,
                    beneficiary,
                    entry_point,
                    chain,
                    send_bundle_mode,
                    relay_endpoints: None,
                    eth_client: Some(client),
                    fb_client: None,
                })
            }
            SendBundleMode::Flashbots => match relay_endpoints {
                None => {
                    let mut relay_endpoints = Vec::new();
                    relay_endpoints.push(flashbots_relay_endpoints::FLASHBOTS.to_string());
                    let fb_client = generate_fb_middleware(
                        eth_client_address.clone(),
                        Some(relay_endpoints.clone()),
                        wallet.clone(),
                    )?;

                    Ok(Self {
                        wallet,
                        eth_client_address,
                        beneficiary,
                        entry_point,
                        chain,
                        send_bundle_mode,
                        relay_endpoints: Some(relay_endpoints),
                        eth_client: None,
                        fb_client: Some(fb_client),
                    })
                }
                Some(relay_endpoints) => {
                    let fb_client = generate_fb_middleware(
                        eth_client_address.clone(),
                        Some(relay_endpoints.clone()),
                        wallet.clone(),
                    )?;

                    Ok(Self {
                        wallet,
                        eth_client_address,
                        beneficiary,
                        entry_point,
                        chain,
                        send_bundle_mode,
                        relay_endpoints: Some(relay_endpoints),
                        eth_client: None,
                        fb_client: Some(fb_client),
                    })
                }
            },
        }
    }

    /// Send a bundle of [UserOperations](UserOperation) to the Ethereum execution client or Flashbots' relay, depending on the [SendBundleMode](SendBundleMode)
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle or transaction
    pub async fn send_next_bundle(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        if uos.is_empty() {
            info!("Skipping creating a new bundle, no user operations");
            return Ok(H256::default());
        };

        info!("Creating a new bundle with {} user operations", uos.len());
        trace!("Bundle content: {uos:?}");

        match self.send_bundle_mode {
            SendBundleMode::EthClient => self.send_next_bundle_eth(uos).await,
            SendBundleMode::Flashbots => self.send_next_bundle_flashbots(uos).await,
        }
    }

    /// Helper function to generate a [TypedTransaction](TypedTransaction) from an array of user operations.
    ///
    /// # Arguments
    /// * `client` - A provider that implements [Middleware](Middleware) trait
    ///
    /// # Returns
    /// * `TypedTransaction` - A [TypedTransaction](TypedTransaction)
    #[allow(clippy::ptr_arg)]
    async fn generate_tx<M>(
        &self,
        client: Arc<M>,
        uos: &Vec<UserOperation>,
    ) -> anyhow::Result<TypedTransaction>
    where
        M: Middleware + 'static,
    {
        let ep = EntryPointAPI::new(self.entry_point, client.clone());

        let nonce = client
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await?;
        let mut tx: TypedTransaction = ep
            .handle_ops(
                uos.clone().into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx;

        match self.chain.id() {
            // Mumbai
            80001u64 => {
                tx.set_nonce(nonce).set_chain_id(self.chain.id());
            }
            _ => {
                let accesslist = client
                    .clone()
                    .create_access_list(&tx, None)
                    .await?
                    .access_list;
                tx.set_access_list(accesslist.clone());
                let estimated_gas = client.clone().estimate_gas(&tx, None).await?;

                let (max_fee_per_gas, max_priority_fee) =
                    client.clone().estimate_eip1559_fees(None).await?;

                tx = TypedTransaction::Eip1559(Eip1559TransactionRequest {
                    to: tx.to().cloned(),
                    from: Some(self.wallet.signer.address()),
                    data: tx.data().cloned(),
                    chain_id: Some(U64::from(self.chain.id())),
                    max_priority_fee_per_gas: Some(max_priority_fee),
                    max_fee_per_gas: Some(max_fee_per_gas),
                    gas: Some(estimated_gas),
                    nonce: Some(nonce),
                    value: None,
                    access_list: accesslist,
                });
            }
        };

        Ok(tx.into())
    }

    /// Send a bundle of [UserOperations](UserOperation) to the Ethereum execution client
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash
    async fn send_next_bundle_eth(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        let client = match self.eth_client.clone() {
            Some(client) => client,
            None => return Err(anyhow::anyhow!("No Ethereum client provided")),
        };

        let tx = self.generate_tx(client.clone(), uos).await?;

        trace!("Sending transaction to the execution client: {tx:?}");

        let tx = client
            .send_transaction(tx, None)
            .await?
            .interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();

        let tx_receipt = tx.await?;

        trace!("Transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }

    // TODO: add more relay endpoints support
    /// Send a bundle of [UserOperations](UserOperation) to the Flashbots relay.
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle
    #[allow(clippy::needless_return)]
    async fn send_next_bundle_flashbots(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        let client = match self.fb_client.clone() {
            Some(client) => client,
            None => return Err(anyhow::anyhow!("No Flashbots client provided")),
        };

        let tx = self.generate_tx(client.clone(), uos).await?;

        let bundle_req = generate_bundle_req(client.clone(), tx).await?;

        match simulate_fb_bundle(client.clone(), &bundle_req).await {
            Ok(_) => {}
            Err(e) => return Err(anyhow::anyhow!("Bundle simulation failed: {:?}", e)),
        };

        let bundle_hash = send_fb_bundle(client.clone(), bundle_req.clone()).await?;

        Ok(bundle_hash)
    }
}

/// Send a Flashbots bundle and check for status
///
/// # Arguments
/// * `client` - An [Flashbots SignerMiddleware](FlashbotsSignerMiddleware)
/// * `bundle` - A [BundleRequest](BundleRequest) sent to Flashbots relay
///
/// # Returns
/// * `H256` - The transaction hash of the bundle
async fn send_fb_bundle<M: Middleware + 'static>(
    client: FlashbotsClientType<M>,
    bundle: BundleRequest,
) -> anyhow::Result<H256> {
    // Send the Flashbots bundle and check for status
    let handle: JoinHandle<Result<(bool, H256), anyhow::Error>> = tokio::spawn(async move {
        let pending_bundle = match client.inner().send_bundle(&bundle).await {
            Ok(bundle) => bundle,
            Err(e) => return Err(anyhow::anyhow!("Failed to send bundle: {:?}", e)),
        };

        let bundle_hash = pending_bundle.bundle_hash;

        match pending_bundle.await {
            Ok(_) => return Ok((true, bundle_hash)),
            Err(BundleNotIncluded) => {
                return Err(anyhow::anyhow!("Bundle not included in the target block"));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Bundle rejected: {:?}", e));
            }
        };
    });

    match handle.await {
        Ok(Ok((_, bundle_hash))) => {
            info!("Bundle included");
            Ok(bundle_hash)
        }
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::anyhow!("Task panicked: {:?}", e)),
    }
}

/// Simulate a Flashbots bundle
///
/// # Arguments
/// * `client` - An [Flashbots SignerMiddleware](FlashbotsSignerMiddleware)
/// * `bundle` - A [BundleRequest](BundleRequest) sent to Flashbots relay
async fn simulate_fb_bundle<M: Middleware + 'static>(
    client: FlashbotsClientType<M>,
    bundle: &BundleRequest,
) -> anyhow::Result<()> {
    let simulated_bundle = client.inner().simulate_bundle(&bundle).await?;

    // Currently there's only 1 tx per bundle
    for tx in simulated_bundle.transactions {
        trace!("Simulate bundle: {:?}", tx);
        if let Some(err) = &tx.error {
            return Err(anyhow::anyhow!(
                "Transaction failed simulation with error: {:?}",
                err
            ));
        }
        if let Some(revert) = &tx.revert {
            return Err(anyhow::anyhow!(
                "Transaction failed simulation with revert: {:?}",
                revert
            ));
        }
    }

    Ok(())
}

/// Generate a Flashbots bundle request
///
/// # Arguments
/// * `client` - An [Flashbots SignerMiddleware](FlashbotsSignerMiddleware)
/// * `tx` - A [EIP-1559 TypedTransaction](TypedTransaction)
///
/// # Returns
/// * `BundleRequest` - A [BundleRequest](BundleRequest)
async fn generate_bundle_req<M: Middleware + 'static>(
    client: FlashbotsClientType<M>,
    tx: TypedTransaction,
) -> anyhow::Result<BundleRequest> {
    let typed_tx = TypedTransaction::Eip1559(tx.into());
    let raw_signed_tx = match client.signer().sign_transaction(&typed_tx).await {
        Ok(tx) => typed_tx.rlp_signed(&tx),
        Err(e) => return Err(anyhow::anyhow!("Failed to sign transaction: {:?}", e)),
    };
    let mut bundle_req = BundleRequest::new();
    bundle_req = bundle_req.push_transaction(raw_signed_tx);

    // Simulate the Flashbots bundle
    let block_num = client.get_block_number().await?;
    bundle_req = bundle_req
        .set_block(block_num + 1)
        .set_simulation_block(block_num)
        .set_simulation_timestamp(0);

    Ok(bundle_req)
}

/// Create a Flashbots middleware
///
/// # Arguments
/// * `eth_client_address` - The URL of an Ethereum execution client
/// * `relay_endpoints` - An array of Flashbots relay endpoints
/// * `wallet` - A [Wallet](Wallet)
///
/// # Returns
/// * `FlashbotsClientType` - A [Flashbots Signer Middleware](FlashbotsClientType)
pub(crate) fn generate_fb_middleware(
    eth_client_address: String,
    relay_endpoints: Option<Vec<String>>,
    wallet: Wallet,
) -> anyhow::Result<FlashbotsClientType<Provider<Http>>> {
    // Only support one relay endpoint for now
    let relay_endpoint: &str = relay_endpoints.as_ref().unwrap().first().unwrap();

    let provider = Provider::<Http>::try_from(eth_client_address.clone())?;

    let bundle_signer = match wallet.fb_signer {
        Some(ref signer) => signer,
        None => return Err(anyhow::anyhow!("No Flashbots signer provided")),
    };

    let mut fb_middleware = FlashbotsMiddleware::new(
        provider.clone(),
        Url::parse(relay_endpoint.clone())?,
        bundle_signer.clone(),
    );
    fb_middleware.set_simulation_relay(
        Url::parse(relay_endpoint.clone()).expect("Failed to parse simulation relay URL"),
        bundle_signer.clone(),
    );

    let client = Arc::new(SignerMiddleware::new(fb_middleware, wallet.signer.clone()));

    Ok(client)
}

#[cfg(test)]
mod test {
    use crate::test_helper::MockFlashbotsRelayServer;
    use crate::{
        bundler::generate_fb_middleware,
        test_helper::{MockFlashbotsBlockBuilderRelay, INIT_BLOCK},
        Bundler, SendBundleMode,
    };
    use aa_bundler_primitives::{consts::flashbots_relay_endpoints, Chain, Wallet};
    use alloy_primitives::{Address as alloy_Address, U256 as alloy_U256};
    use alloy_sol_types::{sol, SolCall};
    use ethers::{
        contract::abigen,
        providers::Middleware,
        signers::Signer,
        types::{
            transaction::eip2718::TypedTransaction, Address, Eip1559TransactionRequest,
            NameOrAddress, H160, U256, U64,
        },
        utils::{parse_units, Anvil, AnvilInstance},
    };
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::http_client::HttpClientBuilder;
    use jsonrpsee::rpc_params;
    use jsonrpsee::server::{ServerBuilder, ServerHandle};
    use std::env;

    sol! {
        #[derive(Debug)]
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts);
    }

    abigen!(
        WETH,
        "https://etherscan.io/address/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
    );

    // Testing key
    const KEY_PHRASE: &str = "test test test test test test test test test test test junk";
    // Deplolyed SimpleAccount address on Goerli
    const SIMPLE_ACCOUNT: &str = "0xf850679aFA7A1675D24D21092b584AbA709d35F8";
    // UO signing key
    const SIGNING_KEY: &str = "0xdf1f39dd322a0cb54da8724bf1baf639f0d34916d529adbe2942a28b47dbed4a";

    struct TestContext {
        pub bundler: Bundler,
        pub entry_point: Address,
        pub anvil: AnvilInstance,
    }

    async fn setup() -> anyhow::Result<TestContext> {
        std::env::set_var("RUST_LOG", "info");
        tracing_subscriber::fmt::init();

        dotenv::dotenv().ok();
        let eth_client_address = env::var("HTTP_RPC").expect("HTTP_RPC env var not set");
        let ep_address = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse::<Address>()?;
        let wallet = Wallet::from_phrase(KEY_PHRASE, &U256::from(1), true)?;

        // Start Anvil and expose the port at 8545
        let port = 8545u16;
        let anvil = Anvil::new()
            .port(port)
            .fork(eth_client_address.clone())
            .fork_block_number(INIT_BLOCK.clone())
            .spawn();

        // Create a bundler and connect to the Anvil
        let bundler = Bundler::new(
            wallet.clone(),
            "http://localhost:8545".to_string(),
            wallet.signer.address(),
            ep_address,
            Chain::from(1),
            SendBundleMode::Flashbots,
            Some(vec![flashbots_relay_endpoints::FLASHBOTS_GOERLI.to_string()]),
        )
        .expect("Failed to create bundler");

        Ok(TestContext {
            bundler,
            entry_point: ep_address,
            anvil,
        })
    }

    async fn start_mock_server() -> anyhow::Result<(ServerHandle, MockFlashbotsBlockBuilderRelay)> {
        // Start a mock server connecting to the Anvil, exposing the port at 3001
        let mock_relay = MockFlashbotsBlockBuilderRelay::new(8545u64).await.unwrap();
        let server = ServerBuilder::new()
            .build("127.0.0.1:3001".to_string())
            .await?;
        let handle = server
            .start(mock_relay.clone().into_rpc())
            .expect("Failed to start mock server");

        Ok((handle, mock_relay))
    }

    #[tokio::test]
    async fn test_send_bundle_flashbots() -> anyhow::Result<()> {
        let ctx = setup().await?;
        let (_handle, mock_relay) = start_mock_server().await?;

        let bundler = ctx.bundler;
        let depositor = mock_relay.mock_eth_client.clone();
        let address = bundler.wallet.signer.address();

        // Create a Flashbots signer middleware
        let client = generate_fb_middleware(
            "http://localhost:8545".to_string(),
            Some(vec!["http://127.0.0.1:3001".to_string()]),
            bundler.wallet.clone(),
        )?;

        let depositor_weth_instance = WETH::new(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?,
            depositor.clone(),
        );
        let bundler_weth_instance = WETH::new(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?,
            client.clone(),
        );

        // Deposit 500 ETH to get WETH and transfer to the bundler
        let value = U256::from(parse_units("500.0", "ether").unwrap());
        let _ = depositor_weth_instance
            .deposit()
            .value(value)
            .send()
            .await?
            .await?;
        let _ = depositor_weth_instance
            .transfer(address.clone(), value)
            .send()
            .await?
            .await?;

        // Approve the UniswapV2 Router from the Bundler
        let _ = bundler_weth_instance
            .approve(
                "0xf164fC0Ec4E93095b804a4795bBe1e041497b92a".parse::<H160>()?,
                U256::MAX,
            )
            .send()
            .await?
            .await?;

        let balance = bundler_weth_instance.balance_of(address).call().await?;
        println!("Balance: {:?}", balance);

        let path = vec![
            // WETH address
            alloy_Address::parse_checksummed("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", None)
                .unwrap(),
            // USDt address
            alloy_Address::parse_checksummed("0xdAC17F958D2ee523a2206206994597C13D831ec7", None)
                .unwrap(),
        ];

        let swap_eth = swapExactETHForTokensCall {
            amountOutMin: alloy_U256::from(0),
            path: path.clone(),
            to: SIMPLE_ACCOUNT.parse::<alloy_Address>().unwrap(),
            deadline: alloy_U256::from(0),
        };

        let call_data = swap_eth.encode();
        let nonce = client.clone().get_transaction_count(address, None).await?;

        let tx_req = TypedTransaction::Eip1559(Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(
                "0xf164fC0Ec4E93095b804a4795bBe1e041497b92a".parse::<H160>()?,
            )),
            from: Some(address),
            data: Some(call_data.into()),
            chain_id: Some(U64::from(1)),
            max_fee_per_gas: Some(U256::from(1000000000000u64)),
            max_priority_fee_per_gas: Some(U256::from(1000000000000u64)),
            gas: Some(U256::from(1000000u64)),
            nonce: Some(nonce),
            value: None,
            access_list: Default::default(),
        });

        println!("{:?}", tx_req);

        // let url = "http://127.0.0.1:3001";
        // let client = HttpClientBuilder::default().build(url)?;
        // let params = rpc_params![tx];
        // let response: Result<String, _> = client.request("eth_callBundle", params).await;
        // println!("{:?}", response);

        Ok(())
    }
}
