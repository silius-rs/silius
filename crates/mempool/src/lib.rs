use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;
use silius_validation::validator::Validator;

use crate::{error::MempoolError, pool::UserOperationPool, reputation::Reputation};

mod error;
mod pool;
mod reputation;

pub struct Mempool {
    pub user_operation_pool: UserOperationPool,
    pub reputation: Reputation,
}

impl Mempool {
    pub fn new(db: SiliusDB, validator: Validator) -> Self {
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

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.user_operation_pool.clear()?;
        self.reputation.clear()?;
        Ok(())
    }
}
