use alloy_consensus::{Signed, TypedTransaction};
use alloy_primitives::TxHash;

use crate::BundleSubmitter;

pub struct NoopSubmitter {}

#[async_trait::async_trait]
impl BundleSubmitter for NoopSubmitter {
    async fn submit_bundle(&self, _bundle: Signed<TypedTransaction>) -> anyhow::Result<TxHash> {
        Ok(TxHash::default())
    }
}
