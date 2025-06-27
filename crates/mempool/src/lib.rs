use alloy_provider::Provider;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;
use silius_validator::validator::Validator;

use crate::{error::MempoolError, pool::UserOperationPool, reputation::Reputation};

mod error;
mod pool;
mod reputation;

pub struct Mempool<P: Provider> {
    pub user_operation_pool: UserOperationPool<P>,
    pub reputation: Reputation,
}

impl<P: Provider> Mempool<P> {
    pub fn new(db: SiliusDB, validator: Validator<P>) -> Self {
        Self {
            user_operation_pool: UserOperationPool::new(db.clone(), validator),
            reputation: Reputation::new(db),
        }
    }

    pub fn insert_user_operation(&self, user_operation: UserOperation) -> Result<(), MempoolError> {
        self.user_operation_pool
            .insert_user_operation(user_operation)?;
        Ok(())
    }

    pub fn get_user_operations(&self) -> Result<Vec<UserOperation>, MempoolError> {
        self.user_operation_pool.get_user_operations()
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.user_operation_pool.clear()?;
        self.reputation.clear()?;
        Ok(())
    }
}
