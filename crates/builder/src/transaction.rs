use std::sync::Arc;

use alloy_consensus::{Signed, TypedTransaction};
use alloy_primitives::TxHash;
use alloy_provider::{Provider, network::eip2718::Encodable2718};
use silius_chain::Chain;

use crate::BundleSubmitter;

pub struct TransactionSubmitter<P: Provider + 'static> {
    pub chain: Arc<Chain<P>>,
}

#[async_trait::async_trait]
impl<P: Provider + 'static> BundleSubmitter for TransactionSubmitter<P> {
    async fn submit_bundle(&self, bundle: Signed<TypedTransaction>) -> anyhow::Result<TxHash> {
        let pending_tx = self
            .chain
            .provider()
            .send_raw_transaction(&bundle.encoded_2718())
            .await?;
        Ok(pending_tx.tx_hash().clone())
    }
}
