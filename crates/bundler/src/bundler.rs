use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, H256},
};
use silius_contracts::entry_point::EntryPointAPI;
use silius_primitives::{Chain, UserOperation, Wallet};
use std::{sync::Arc, time::Duration};
use tracing::{info, trace};

#[derive(Clone)]
pub struct Bundler {
    pub wallet: Wallet,
    pub eth_client_address: String,
    pub beneficiary: Address,
    pub entry_point: Address,
    pub chain: Chain,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        eth_client_address: String,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
    ) -> Self {
        Self {
            wallet,
            eth_client_address,
            beneficiary,
            entry_point,
            chain,
        }
    }

    pub async fn send_next_bundle(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        if uos.is_empty() {
            info!("Skipping creating a new bundle, no user operations");
            return Ok(H256::default());
        };

        info!("Creating a new bundle with {} user operations", uos.len());
        trace!("Bundle content: {uos:?}");

        let eth_client = Provider::<Http>::try_from(self.eth_client_address.clone())?;
        let client = Arc::new(SignerMiddleware::new(
            eth_client.clone(),
            self.wallet.signer.clone(),
        ));
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
            .tx
            .clone();
        tx.set_nonce(nonce).set_chain_id(self.chain.id());

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
}
