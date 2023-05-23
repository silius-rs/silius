use std::{sync::Arc, time::Duration};

use aa_bundler_contracts::EntryPointAPI;
use aa_bundler_primitives::{Chain, UserOperation, Wallet};
use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, H256},
};
use tracing::{info, trace};

#[derive(Clone)]
pub struct Bundler {
    pub wallet: Wallet,
    pub beneficiary: Address,
    pub entry_point: Address,
    pub chain: Chain,
    pub eth_client_address: String,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
        eth_client_address: String,
    ) -> Self {
        Self {
            wallet,
            beneficiary,
            entry_point,
            chain,
            eth_client_address,
        }
    }

    pub async fn send_next_bundle(&self, bundle: &Vec<UserOperation>) -> anyhow::Result<H256> {
        info!(
            "Creating the next bundle, got {} user operations",
            bundle.len()
        );
        if bundle.is_empty() {
            return Ok(H256::default());
        };
        let provider = Provider::<Http>::try_from(self.eth_client_address.clone())?;
        let client = Arc::new(SignerMiddleware::new(
            provider.clone(),
            self.wallet.signer.clone(),
        ));
        let entry_point = EntryPointAPI::new(self.entry_point, client.clone());
        let nonce = client
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await?;
        let mut tx: TypedTransaction = entry_point
            .handle_ops(
                bundle.clone().into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx
            .clone();
        tx.set_nonce(nonce).set_chain_id(self.chain.id());

        trace!("Prepare the transaction {tx:?} send to execution client!");
        let tx = client
            .send_transaction(tx, None)
            .await?
            .interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();
        trace!("Send bundle with transaction: {tx:?}");

        let tx_receipt = tx.await?;
        trace!("Bundle transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }
}
