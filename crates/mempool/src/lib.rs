use silius_storage::db::SiliusDB;

use crate::{pool::UserOperationPool, reputation::Reputation};

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
}
