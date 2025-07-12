use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::reputation::ReputationState;
use silius_storage::{db::SiliusDB, tables::Table};

use crate::error::MempoolError;

pub struct Reputation<P: Provider + 'static> {
    pub db: SiliusDB,
    pub chain: Arc<Chain<P>>,
}

impl<P: Provider + 'static> Reputation<P> {
    pub fn new(db: SiliusDB, chain: Arc<Chain<P>>) -> Self {
        Self { db, chain }
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        Ok(())
    }

    pub fn get_state(&self, address: Address) -> Result<ReputationState, MempoolError> {
        Ok(self
            .db
            .reputation_provider()
            .get(address)?
            .unwrap_or_default()
            .state())
    }

    pub async fn is_staked(&self, address: Address) -> Result<bool, MempoolError> {
        let deposit_info = self.chain.get_deposit_info(address).await?;
        Ok(deposit_info.staked)
    }

    pub fn increment_ops_seen(&self, address: Address) -> Result<(), MempoolError> {
        self.db
            .reputation_provider()
            .increment_ops_seen(address)
            .map_err(MempoolError::Database)
    }

    pub fn increment_ops_included(&self, address: Address) -> Result<(), MempoolError> {
        self.db
            .reputation_provider()
            .increment_ops_included(address)
            .map_err(MempoolError::Database)
    }
}
