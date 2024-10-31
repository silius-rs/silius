use crate::bundler::SendBundleOp;
use alloy_chains::{Chain, NamedChain};
use ethers::{
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::LocalWallet,
    types::{
        transaction::{
            conditional::{AccountStorage, ConditionalOptions},
            eip2718::TypedTransaction,
        },
        Address, H256,
    },
};
use silius_primitives::{simulation::StorageMap, Wallet};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::trace;

/// A type alias for the Ethereum Conditional Signer client
#[derive(Clone)]
pub struct ConditionalClient<M>(pub SignerMiddleware<Arc<M>, LocalWallet>);

#[async_trait::async_trait]
impl<M> SendBundleOp for ConditionalClient<M>
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

        let signed_tx = self.0.sign_transaction(bundle).await?;

        let prefix: Option<String> = if self.0.get_chainid().await? ==
            Chain::from_named(NamedChain::Polygon).id().into() ||
            self.0.get_chainid().await? == Chain::from_named(NamedChain::PolygonAmoy).id().into()
        {
            Some("bor".to_string())
        } else {
            None
        };

        let tx = self
            .0
            .send_raw_transaction_conditional(
                signed_tx,
                prefix,
                ConditionalOptions { known_accounts, ..Default::default() },
            )
            .await?
            .interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();

        let tx_receipt = tx.await?;

        trace!("Transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }
}

impl<M> ConditionalClient<M>
where
    M: Middleware + 'static,
{
    /// Create an Conditional client
    ///
    /// # Arguments
    /// * `eth_client` - Connection to the Ethereum execution client
    /// * `wallet` - A [Wallet](Wallet) instance
    ///
    /// # Returns
    /// * `ConditionalClient` - A [Ethereum Signer Middleware](ConditionalClient)
    pub fn new(eth_client: Arc<M>, wallet: Wallet) -> Self {
        let signer = SignerMiddleware::new(eth_client, wallet.signer);
        Self(signer)
    }
}
