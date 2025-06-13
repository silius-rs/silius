use async_trait::async_trait;
use ethers::{
    middleware::SignerMiddleware,
    prelude::LocalWallet,
    providers::{Http, Middleware, Provider},
    types::{Bytes, H256, U256, U64},
};
use ethers_flashbots_test::{relay::SendBundleResponse, SimulatedBundle, SimulatedTransaction};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub const INIT_BLOCK: u64 = 17832062;

// Testing key
pub const KEY_PHRASE: &str = "test test test test test test test test test test test junk";

#[rpc(server, namespace = "eth")]
pub trait MockFlashbotsRelay {
    #[method(name = "sendBundle")]
    async fn send_bundle(&self, bundle_req: BundleRequest) -> RpcResult<SendBundleResponse>;

    #[method(name = "callBundle")]
    async fn call_bundle(&self, bundle_req: BundleRequest) -> RpcResult<SimulatedBundle>;
}

#[derive(Debug, Clone)]
pub struct MockFlashbotsBlockBuilderRelay {
    pub mock_eth_client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

#[cfg(test)]
impl MockFlashbotsBlockBuilderRelay {
    pub async fn new(port: u64) -> eyre::Result<Self> {
        let url = format!("http://127.0.0.1:{port}");
        let mock_eth_client = Provider::<Http>::try_from(&url)?;

        // Create a wallet and SignerMiddleware
        let wallet = "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
            .parse::<LocalWallet>()?;
        let client = Arc::new(SignerMiddleware::new(mock_eth_client.clone(), wallet));

        Ok(Self { mock_eth_client: client })
    }
}

#[cfg(test)]
#[async_trait]
impl MockFlashbotsRelayServer for MockFlashbotsBlockBuilderRelay {
    async fn send_bundle(&self, _bundle_req: BundleRequest) -> RpcResult<SendBundleResponse> {
        let _provider = self.mock_eth_client.inner().clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(12)).await;
            // let _ = MockFlashbotsBlockBuilderRelay::create_pending_bundle(&provider,
            // &bundle_req);
        });

        Ok(SendBundleResponse::default())
    }

    async fn call_bundle(&self, bundle_req: BundleRequest) -> RpcResult<SimulatedBundle> {
        let txs: Vec<_> = bundle_req.transactions;

        let mut simulated_bundle = SimulatedBundle::default();
        simulated_bundle.simulation_block = INIT_BLOCK.into();
        let mut gas_used = U256::from(0);
        let mut gas_price = U256::from(0);
        for tx in txs {
            let mut simulated_transaction = SimulatedTransaction::default();
            let result = self
                .mock_eth_client
                .send_raw_transaction(tx.into())
                .await
                .unwrap()
                .await
                .unwrap()
                .unwrap();

            simulated_transaction.hash = result.transaction_hash;
            simulated_transaction.gas_used = result.gas_used.unwrap();
            simulated_transaction.gas_price = result.effective_gas_price.unwrap();
            simulated_transaction.from = result.from;
            simulated_transaction.to = result.to;

            gas_used += result.gas_used.unwrap();
            gas_price += result.effective_gas_price.unwrap();

            simulated_bundle.transactions.push(simulated_transaction);
        }
        simulated_bundle.gas_used = gas_used;
        simulated_bundle.gas_price = gas_price;

        Ok(simulated_bundle)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRequest {
    #[serde(rename = "txs")]
    // #[serde(serialize_with = "serialize_txs")]
    // transactions: Vec<BundleTransaction>,
    pub transactions: Vec<Bytes>,
    #[serde(rename = "revertingTxHashes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    revertible_transaction_hashes: Vec<H256>,

    #[serde(rename = "blockNumber")]
    #[serde(skip_serializing_if = "Option::is_none")]
    target_block: Option<U64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    min_timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    max_timestamp: Option<u64>,

    #[serde(rename = "stateBlockNumber")]
    #[serde(skip_serializing_if = "Option::is_none")]
    simulation_block: Option<U64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timestamp")]
    simulation_timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "baseFee")]
    simulation_basefee: Option<u64>,
}
