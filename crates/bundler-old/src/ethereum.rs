use crate::bundler::SendBundleOp;
use ethers::{
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::LocalWallet,
    types::{transaction::eip2718::TypedTransaction, H256},
};
use silius_primitives::{simulation::StorageMap, Wallet};
use std::{sync::Arc, time::Duration};
use tracing::trace;

/// A type alias for the Ethereum Signer client
#[derive(Clone)]
pub struct EthereumClient<M>(pub SignerMiddleware<Arc<M>, LocalWallet>);

#[async_trait::async_trait]
impl<M> SendBundleOp for EthereumClient<M>
where
    M: Middleware + 'static,
{
    /// Send a bundle of user operations to the Ethereum execution client
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
        _storage_map: StorageMap,
    ) -> eyre::Result<H256> {
        trace!("Sending transaction to the execution client: {bundle:?}");

        let tx = self.0.send_transaction(bundle, None).await?.interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();

        let tx_receipt = tx.await?;

        trace!("Transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }
}

impl<M> EthereumClient<M>
where
    M: Middleware + 'static,
{
    /// Create an Ethereum client
    ///
    /// # Arguments
    /// * `eth_client` - Connection to the Ethereum execution client
    /// * `wallet` - A [Wallet](Wallet) instance
    ///
    /// # Returns
    /// * `EthereumClient` - A [Ethereum Signer Middleware](EthereumClient)
    pub fn new(eth_client: Arc<M>, wallet: Wallet) -> Self {
        let signer = SignerMiddleware::new(eth_client, wallet.signer);
        Self(signer)
    }
}
