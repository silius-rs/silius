use crate::bundler::SendBundleOp;
use ethers::{
    middleware::SignerMiddleware,
    providers::{JsonRpcClient, Middleware},
    signers::LocalWallet,
    types::{transaction::eip2718::TypedTransaction, H256},
};
use jsonrpsee::types::{Id, Request};
use serde_json::value::RawValue;
use silius_primitives::{simulation::StorageMap, Wallet};
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
    /// * 'storage_map' - Storage map
    ///
    /// # Returns
    /// * `H256` - The transaction hash
    async fn send_bundle(&self, bundle: TypedTransaction, storage_map: StorageMap) -> eyre::Result<H256> {
        trace!("Sending transaction to the conditional endpoint: {bundle:?}");

        let value = serde_json::to_value(&bundle)?;
        let req_body = Request::new(
            "eth_sendRawTransactionConditional".into(),
            Some(&RawValue::from_string(r#"["hello"]"#.into()).unwrap()),
            Id::Number(1),
        );

        // let tx = self.0.send_transaction(bundle, None).await?.interval(Duration::from_millis(75));
        // let tx_hash = tx.tx_hash();

        // let tx_receipt = tx.await?;

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
