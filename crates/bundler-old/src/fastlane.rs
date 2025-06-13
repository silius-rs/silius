use crate::bundler::SendBundleOp;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::LocalWallet,
    types::{
        transaction::{
            conditional::{AccountStorage, ConditionalOptions},
            eip2718::TypedTransaction,
        },
        Address, BlockNumber, H256,
    },
};
use serde::Deserialize;
use silius_primitives::{
    constants::fastlane_relay_endpoints::FASTLANE_VALIDATORS, simulation::StorageMap, Wallet,
};
use std::{collections::HashMap, sync::Arc};
use tracing::trace;

/// A type alias for the Ethereum Conditional Signer client
#[derive(Clone)]
pub struct FastlaneClient<M> {
    pub client: SignerMiddleware<Arc<M>, LocalWallet>,
    pub polygon_client: Provider<Http>,
    pub relay_client: Provider<Http>,
}

/// Validators participating in the Fastlane relay network
#[derive(Deserialize, Debug)]
pub struct FastlaneValidators {
    validators: Vec<Address>,
}

#[async_trait::async_trait]
impl<M> SendBundleOp for FastlaneClient<M>
where
    M: Middleware + 'static,
{
    /// Send a bundle of user operations to the Ethereum execution client
    /// over conditional RPC method.
    ///
    /// # Arguments
    /// * `bundle` - Bundle of user operations as [TypedTransaction](TypedTransaction).
    /// * 'storage_map' - Storage map
    ///
    /// # Returns
    /// * `H256` - The transaction hash
    async fn send_bundle(
        &self,
        bundle: TypedTransaction,
        storage_map: StorageMap,
    ) -> eyre::Result<H256> {
        trace!("Sending transaction to the conditional endpoint: {bundle:?}");

        let mut known_accounts: HashMap<Address, AccountStorage> = HashMap::default();

        for (k, v) in storage_map.root_hashes {
            known_accounts.insert(k, AccountStorage::RootHash(v));
        }

        for (k, v) in storage_map.slots {
            known_accounts.insert(k, AccountStorage::SlotValues(v));
        }

        let signed_tx = self.client.sign_transaction(bundle).await?;

        let prefix: Option<String> = Some("pfl".to_string());
        let block = self.client.get_block(BlockNumber::Latest).await?;

        let mut options = ConditionalOptions { known_accounts, ..Default::default() };

        if let Some(block) = block {
            options.timestamp_min = Some(block.timestamp.as_u64());
            options.timestamp_max = Some(block.timestamp.as_u64() + 420); // around 15 minutes
        }

        // check if the current validator is participating in the Fastlane protocol
        let fastlane_validators =
            reqwest::get(FASTLANE_VALIDATORS).await?.json::<FastlaneValidators>().await?;
        let current_validator: Address =
            self.polygon_client.request("bor_getCurrentProposer", ()).await?;

        if !fastlane_validators.validators.contains(&current_validator) {
            trace!("Current validator is not participating in the Fastlane protocol");
            return Err(eyre::eyre!(
                "Current validator is not participating in the Fastlane protocol"
            ));
        }

        let tx =
            self.relay_client.send_raw_transaction_conditional(signed_tx, prefix, options).await?;
        let tx_hash = tx.tx_hash();

        let tx_receipt = tx.await?;

        trace!("Transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }
}

impl<M> FastlaneClient<M>
where
    M: Middleware + 'static,
{
    /// Create an Conditional client
    ///
    /// # Arguments
    /// * `eth_client` - Connection to the Ethereum execution client
    /// * `polygon_client` - Connection to the Polygon execution client
    /// * `relay_client` - Connection to the Fastlane relay client
    /// * `wallet` - A [Wallet](Wallet) instance
    ///
    /// # Returns
    /// * `ConditionalClient` - A [Ethereum Signer Middleware](ConditionalClient)
    pub fn new(
        eth_client: Arc<M>,
        polygon_client: Provider<Http>,
        relay_client: Provider<Http>,
        wallet: Wallet,
    ) -> Self {
        let signer = SignerMiddleware::new(eth_client, wallet.clone().signer);
        Self { client: signer, polygon_client, relay_client }
    }
}
