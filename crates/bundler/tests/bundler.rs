mod common;

use alloy_chains::Chain;
use alloy_primitives::{Address as alloy_Address, U256 as alloy_U256};
use alloy_sol_types::{sol, SolCall};
use common::{MockFlashbotsBlockBuilderRelay, MockFlashbotsRelayServer, INIT_BLOCK, KEY_PHRASE};
use ethers::{
    contract::abigen,
    providers::{Http, Middleware, Provider, Ws},
    signers::Signer,
    types::{
        transaction::eip2718::TypedTransaction, Address, Eip1559TransactionRequest, NameOrAddress,
        H160, U256, U64,
    },
    utils::{parse_units, Anvil, AnvilInstance},
};
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use silius_bundler::{Bundler, FlashbotsClient, SendBundleOp};
use silius_primitives::{
    constants::{entry_point::ADDRESS, flashbots_relay_endpoints},
    Wallet,
};
use std::sync::Arc;

sol! {
    #[derive(Debug)]
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
}

sol! {
    #[derive(Debug)]
    function approve(address guy, uint wad) public returns (bool);
}

abigen!(
    WETH,
    r#"[
        function deposit() public payable
        function withdraw(uint wad) public
        function totalSupply() public view returns (uint)
        function approve(address guy, uint wad) public returns (bool)
        function transfer(address dst, uint wad) public returns (bool)
        function transferFrom(address src, address dst, uint wad) public returns (bool)
    ]"#
);

struct TestContext<M, S>
where
    M: Middleware + 'static,
    S: SendBundleOp,
{
    pub bundler: Bundler<M, S>,
    pub _entry_point: Address,
    pub _anvil: AnvilInstance,
}

async fn setup() -> eyre::Result<TestContext<Provider<Ws>, FlashbotsClient<Provider<Ws>>>> {
    dotenv::dotenv().ok();
    // RPC URL to mainnet
    let eth_client_address = std::env::var("HTTP_RPC_URL").expect("HTTP_RPC_URL env var not set");

    // Start Anvil and expose the port at 8545
    let port = 8545u16;
    let anvil = Anvil::new()
        .port(port)
        .fork(eth_client_address.clone())
        .fork_block_number(INIT_BLOCK.clone())
        .block_time(1u64)
        .spawn();

    let eth_client = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
    let ep_address = ADDRESS.parse::<Address>()?;
    let wallet = Wallet::from_phrase(KEY_PHRASE, anvil.chain_id(), true)?;

    let client = Arc::new(FlashbotsClient::new(
        eth_client.clone(),
        Some(vec![flashbots_relay_endpoints::FLASHBOTS.into()]),
        wallet.clone(),
    )?);

    // Create a bundler and connect to the Anvil
    let bundler = Bundler::new(
        wallet.clone(),
        wallet.signer.address(),
        ep_address,
        Chain::from(1),
        U256::from(100000000000000000u64),
        eth_client,
        client,
        true,
    );

    Ok(TestContext { bundler, _entry_point: ep_address, _anvil: anvil })
}

async fn start_mock_server() -> eyre::Result<(ServerHandle, MockFlashbotsBlockBuilderRelay)> {
    // Start a mock server connecting to the Anvil, exposing the port at 3001
    let mock_relay = MockFlashbotsBlockBuilderRelay::new(8545u64).await.unwrap();
    let server = ServerBuilder::new().build("127.0.0.1:3001".to_string()).await?;
    let handle = server.start(mock_relay.clone().into_rpc());

    Ok((handle, mock_relay))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_send_bundle_flashbots() -> eyre::Result<()> {
    let ctx = setup().await?;
    let (_handle, mock_relay) = start_mock_server().await?;

    let bundler = ctx.bundler;
    let depositor = mock_relay.mock_eth_client.clone();
    let address = bundler.wallet.signer.address();

    let eth_client = Arc::new(Provider::<Http>::try_from("http://127.0.0.1:8545".to_string())?);

    // Create a Flashbots signer middleware
    let client = FlashbotsClient::new(
        eth_client.clone(),
        Some(vec!["http://127.0.0.1:3001".into()]),
        bundler.wallet.clone(),
    )?;

    let depositor_weth_instance =
        WETH::new("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?, depositor.clone());

    // Deposit 500 ETH to get WETH and transfer to the bundler
    let value = U256::from(parse_units("500.0", "ether").unwrap());
    let _ = depositor_weth_instance.deposit().value(value).send().await?.await?;

    let _ = depositor_weth_instance.transfer(address.clone(), value.clone()).send().await?.await?;

    let balance_before = eth_client.get_balance(address, None).await?;

    // Create approve calldata
    let approve = approveCall {
        // UniswapV2Router address
        guy: alloy_Address::parse_checksummed("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", None)
            .unwrap(),
        wad: alloy_U256::MAX,
    };
    let approve_call_data = approve.abi_encode();

    let path = vec![
        // WETH address
        alloy_Address::parse_checksummed("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", None)
            .unwrap(),
        // USDT address
        alloy_Address::parse_checksummed("0xdAC17F958D2ee523a2206206994597C13D831ec7", None)
            .unwrap(),
    ];

    // Create swap calldata
    let swap_eth = swapExactTokensForTokensCall {
        amountIn: alloy_U256::from(10),
        amountOutMin: alloy_U256::from(0),
        path: path.clone(),
        to: alloy_Address::from_slice(address.clone().as_bytes()),
        deadline: alloy_U256::MAX,
    };
    let swap_call_data = swap_eth.abi_encode();

    let nonce = eth_client.get_transaction_count(address, None).await?;

    // Craft a bundle with approve() and swapExactETHForTokens()
    let approve_tx_req = TypedTransaction::Eip1559(Eip1559TransactionRequest {
        to: Some(NameOrAddress::Address(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?,
        )),
        from: Some(address),
        data: Some(approve_call_data.into()),
        chain_id: Some(U64::from(1)),
        max_fee_per_gas: Some(U256::from(1000000000000u64)),
        max_priority_fee_per_gas: Some(U256::from(1000000000000u64)),
        gas: Some(U256::from(1000000u64)),
        nonce: Some(nonce.clone()),
        value: None,
        access_list: Default::default(),
    });

    let swap_tx_req = TypedTransaction::Eip1559(Eip1559TransactionRequest {
        to: Some(NameOrAddress::Address(
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse::<H160>()?,
        )),
        from: Some(address),
        data: Some(swap_call_data.into()),
        chain_id: Some(U64::from(1)),
        max_fee_per_gas: Some(U256::from(1000000000000u64)),
        max_priority_fee_per_gas: Some(U256::from(1000000000000u64)),
        gas: Some(U256::from(9000000u64)),
        nonce: Some(nonce.clone() + 1),
        value: None,
        access_list: Default::default(),
    });

    // Simulate the bundle
    let sim_bundle_req = client
        .generate_bundle_req(vec![approve_tx_req.clone(), swap_tx_req.clone()], true)
        .await?
        .set_simulation_block(U64::from(INIT_BLOCK.clone()));

    // Swap on Anvil as mock to simulation. In reality, no real state change should happen
    let simulation_res = client.simulate_flashbots_bundle(&sim_bundle_req).await?;

    assert_eq!(simulation_res.transactions.len(), 2);
    assert_eq!(simulation_res.transactions[0].from, address);

    let balance_after = eth_client.get_balance(address, None).await?;
    assert_ne!(balance_before, balance_after);

    // Send the bundle
    let bundle_req =
        client.generate_bundle_req(vec![approve_tx_req.clone(), swap_tx_req.clone()], true).await?;

    let result = client.send_flashbots_bundle(bundle_req.clone()).await;
    assert!(matches!(
        result,
        Err(ref e) if e.to_string() == "Bundle not included in the target block"
    ));

    Ok(())
}
