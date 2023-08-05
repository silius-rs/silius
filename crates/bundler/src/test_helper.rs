use anyhow;
use async_trait::async_trait;
use ethers::{
    middleware::SignerMiddleware,
    prelude::LocalWallet,
    providers::{Http, JsonRpcClient, Middleware, Provider},
    types::{Bytes, TxHash, H256, U256, U64},
};
use ethers_flashbots_test::{
    relay::SendBundleResponse, BundleRequest, BundleTransaction, PendingBundle, SimulatedBundle,
    SimulatedTransaction,
};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use std::sync::Arc;
use std::time::Duration;

pub const INIT_BLOCK: u64 = 17832062;

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

impl MockFlashbotsBlockBuilderRelay {
    pub async fn new(port: u64) -> anyhow::Result<Self> {
        let url = format!("http://localhost:{}", port).to_string();
        let mock_eth_client = Provider::<Http>::try_from(&url)?;

        // Create a wallet and SignerMiddleware
        let wallet = "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
            .parse::<LocalWallet>()?;
        let client = Arc::new(SignerMiddleware::new(mock_eth_client.clone(), wallet));

        Ok(Self {
            mock_eth_client: client,
        })
    }

    fn create_pending_bundle<'a, P: JsonRpcClient>(
        provider: &'a Provider<P>,
        bundle_req: &BundleRequest,
    ) -> PendingBundle<'a, P> {
        let bundle_hash = H256::from_low_u64_be(0);
        let block = bundle_req.block().unwrap_or(U64::from(INIT_BLOCK));
        let transactions: Vec<TxHash> = bundle_req
            .transactions()
            .iter()
            .map(|tx_hash| match tx_hash {
                BundleTransaction::Raw(_) => TxHash::zero(),
                _ => panic!("Not a raw transaction"),
            })
            .collect();

        // Create the PendingBundle
        PendingBundle::new(bundle_hash, block, transactions, &provider)
    }
}

#[async_trait]
impl MockFlashbotsRelayServer for MockFlashbotsBlockBuilderRelay {
    async fn send_bundle(&self, bundle_req: BundleRequest) -> RpcResult<SendBundleResponse> {
        let provider = self.mock_eth_client.inner().clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(12)).await;
            let _ = MockFlashbotsBlockBuilderRelay::create_pending_bundle(&provider, &bundle_req);
        });

        Ok(SendBundleResponse::default())
    }

    async fn call_bundle(&self, bundle_req: BundleRequest) -> RpcResult<SimulatedBundle> {
        let txs: Vec<Bytes> = bundle_req
            .transactions()
            .iter()
            .map(|tx| match tx {
                BundleTransaction::Raw(inner) => (*inner).clone(),
                _ => panic!("Not a raw transaction"),
            })
            .collect();

        let mut simulated_bundle = SimulatedBundle::default();
        simulated_bundle.simulation_block = INIT_BLOCK.into();
        let mut gas_used = U256::from(0);
        let mut gas_price = U256::from(0);
        for tx in txs {
            let mut simulated_transaction = SimulatedTransaction::default();
            let result = self
                .mock_eth_client
                .send_raw_transaction(tx)
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
