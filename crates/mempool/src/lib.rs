use crate::{pool::UserOperationPool, reputation::Reputation};

mod pool;
mod reputation;

pub struct Mempool {
    pub user_operation_pool: UserOperationPool,
    pub reputation: Reputation,
}
