use silius_storage::db::SiliusDB;

use crate::{error::MempoolError, pool::UserOperationPool, reputation::Reputation};

mod error;
mod pool;
mod reputation;

pub struct Mempool {
    pub user_operation_pool: UserOperationPool,
    pub reputation: Reputation,
}

impl Mempool {
    pub fn new(db: SiliusDB) -> Self {
        Self {
            user_operation_pool: UserOperationPool::new(db.clone()),
            reputation: Reputation::new(db),
        }
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.user_operation_pool.clear()?;
        self.reputation.clear()?;
        Ok(())
    }
}
