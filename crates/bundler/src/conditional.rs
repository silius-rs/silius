use crate::bundler::SendBundleOp;
use ethers::{
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::LocalWallet,
    types::{transaction::eip2718::TypedTransaction, H256},
};
use silius_primitives::Wallet;
use std::{sync::Arc, time::Duration};
use tracing::trace;

/// A type alias for the Ethereum Conditional Signer client
#[derive(Clone)]
pub struct ConditionalClient<M>(pub Arc<SignerMiddleware<Arc<M>, LocalWallet>>);

#[async_trait::async_trait]
impl<M> SendBundleOp for ConditionalClient<M>
where
    M: Middleware + 'static,
{
    /// Send a bundle of [UserOperations](UserOperation) to the Ethereum execution client
    /// over conditional RPC method.
    ///
    /// # Arguments
    /// * `bundle` - Bundle of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash
    async fn send_bundle(&self, bundle: TypedTransaction) -> eyre::Result<H256> {
        trace!("Sending transaction to the execution client: {bundle:?}");

        let tx = self.0.send_transaction(bundle, None).await?.interval(Duration::from_millis(75));
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
        Self(Arc::new(signer))
    }
}
