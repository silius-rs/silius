use anyhow;
use async_trait::async_trait;
use ethers::{
    middleware::SignerMiddleware,
    prelude::LocalWallet,
    providers::{Http, Middleware, Provider},
    types::{transaction::eip2718::TypedTransaction, Address, H256, U256},
};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const INIT_BLOCK: u64 = 17832041;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallBundleArgs {
    pub txs: Vec<TypedTransaction>,
    pub coinbase: Option<Address>,
    pub gas_limit: Option<u64>,
    pub base_fee: Option<U256>,
}

impl CallBundleArgs {
    pub fn new(txs: Vec<TypedTransaction>) -> Self {
        Self {
            txs,
            coinbase: None,
            gas_limit: None,
            base_fee: None,
        }
    }
}

#[rpc(server, namespace = "eth")]
pub trait MockFlashbotsRelay {
    #[method(name = "sendBundle")]
    async fn send_bundle(&self) -> RpcResult<MockPayload>;

    #[method(name = "callBundle")]
    async fn call_bundle(&self, call_bundle_args: CallBundleArgs) -> RpcResult<Vec<MockPayload>>;
}

#[derive(Debug, Clone)]
pub struct MockFlashbotsBlockBuilderRelay {
    pub mock_eth_client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MockPayload {
    // pub bundle_gas_price: U256,
    pub bundle_hash: H256,
    pub coinbase_diff: U256,
    // pub eth_sent_to_coinbase: U256,
    // pub gas_fees: U256,
    // pub results: Vec<MockJsonResult>,
    // pub state_block_number: U64,
    // pub total_gas_used: U256,
}

#[derive(Debug, Clone, Default)]
pub struct MockJsonResult {
    pub coinbase_diff: U256,
    pub eth_sent_to_coinbase: U256,
    pub from_address: Address,
    pub to_address: Address,
    pub gas_used: U256,
    pub tx_hash: H256,
}

impl MockFlashbotsBlockBuilderRelay {
    pub async fn new(port: u64) -> anyhow::Result<Self> {
        // Connect to the Anvil
        let url = format!("http://localhost:{}", port).to_string();
        let mock_eth_client = Provider::<Http>::try_from(&url)?;

        // Create a wallet and SignerMiddleware to deposit ETH into the Bundler
        let wallet = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse::<LocalWallet>()?;
        let client = Arc::new(SignerMiddleware::new(mock_eth_client.clone(), wallet));

        Ok(Self {
            mock_eth_client: client,
        })
    }
}

#[async_trait]
impl MockFlashbotsRelayServer for MockFlashbotsBlockBuilderRelay {
    async fn send_bundle(&self) -> RpcResult<MockPayload> {
        let port = 8545;
        let url = format!("http://localhost:{}", port).to_string();
        let provider = Provider::<Http>::try_from(&url).unwrap();

        Ok(MockPayload::default())
    }

    async fn call_bundle(&self, call_bundle_args: CallBundleArgs) -> RpcResult<Vec<MockPayload>> {
        let txs = call_bundle_args.txs;
        let block_number = INIT_BLOCK;
        let coinbase = call_bundle_args.coinbase.unwrap_or(
            "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990"
                .parse::<Address>()
                .expect("Failed to parse address"),
        );
        let _gas_limit = call_bundle_args.gas_limit.unwrap_or(700000);
        let _base_fee = call_bundle_args.base_fee.unwrap_or(
            self.mock_eth_client
                .get_block(block_number)
                .await
                .expect("Failed to get block")
                .unwrap()
                .base_fee_per_gas
                .unwrap(),
        );

        let mut res = Vec::new();
        for tx in txs {
            let mut mock_payload = MockPayload::default();
            let coinbase_before = self
                .mock_eth_client
                .get_balance(coinbase, None)
                .await
                .expect("Failed to get balance");

            let tx_hash = self
                .mock_eth_client
                .send_transaction(tx.clone(), None)
                .await
                .unwrap()
                .await
                .unwrap()
                .unwrap();
            mock_payload.bundle_hash = tx_hash.transaction_hash;

            let coinbase_after = self
                .mock_eth_client
                .get_balance(coinbase, None)
                .await
                .expect("Failed to get balance");

            let coinbase_diff = coinbase_after - coinbase_before;
            mock_payload.coinbase_diff = coinbase_diff;

            res.push(mock_payload);
        }

        return Ok(res);
    }
}
