use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::{reputation::ReputationState, user_operation::UserOperation};
use silius_storage::db::SiliusDB;
use silius_validator::validator::Validator;

use crate::{error::MempoolError, pool::UserOperationPool, reputation::Reputation};

mod error;
mod pool;
mod reputation;

pub struct Mempool<P: Provider + 'static> {
    pub user_operation_pool: UserOperationPool<P>,
    pub reputation: Reputation<P>,
}

impl<P: Provider + 'static> Mempool<P> {
    pub fn new(db: SiliusDB, chain: Arc<Chain<P>>, validator: Validator<P>) -> Self {
        Self {
            user_operation_pool: UserOperationPool::new(db.clone(), chain.clone(), validator),
            reputation: Reputation::new(db, chain),
        }
    }

    pub async fn insert_user_operation(
        &self,
        user_operation: UserOperation,
    ) -> Result<(), MempoolError> {
        self.user_operation_pool
            .insert_user_operation(user_operation)
            .await?;
        Ok(())
    }

    pub fn get_user_operations(&self) -> Result<Vec<UserOperation>, MempoolError> {
        self.user_operation_pool.get_user_operations()
    }

    pub fn get_reputation_state(&self, address: Address) -> Result<ReputationState, MempoolError> {
        self.reputation.get_state(address)
    }

    pub async fn is_staked(&self, address: Address) -> Result<bool, MempoolError> {
        self.reputation.is_staked(address).await
    }

    pub fn increment_ops_seen(&self, address: Address) -> Result<(), MempoolError> {
        self.reputation.increment_ops_seen(address)
    }

    pub fn increment_ops_included(&self, address: Address) -> Result<(), MempoolError> {
        self.reputation.increment_ops_included(address)
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.user_operation_pool.clear()?;
        self.reputation.clear()?;
        Ok(())
    }
}
