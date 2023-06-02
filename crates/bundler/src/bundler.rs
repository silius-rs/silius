use aa_bundler_contracts::entry_point::EntryPointAPI;
use aa_bundler_primitives::{Chain, UserOperation, Wallet};
use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, H256},
};
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Bundler {
    pub wallet: Wallet,
    pub eth_provider_address: String,
    pub beneficiary: Address,
    pub entry_point: Address,
    pub chain: Chain,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        eth_provider: String,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
    ) -> Self {
        Self {
            wallet,
            eth_provider_address: eth_provider,
            beneficiary,
            entry_point,
            chain,
        }
    }

    pub async fn send_next_bundle(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        if uos.is_empty() {
            return Ok(H256::default());
        };

        let provider = Provider::<Http>::try_from(self.eth_provider_address.clone())?;
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
                uos.clone().into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx
            .clone();
        tx.set_nonce(nonce).set_chain_id(self.chain.id());

        let tx = client
            .send_transaction(tx, None)
            .await?
            .interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();

        let _tx_receipt = tx.await?;

        Ok(tx_hash)
    }
}
