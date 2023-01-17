use async_trait::async_trait;
use educe::Educe;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::types::user_operation::{UserOperation, UserOperationHash};

use super::Mempool;

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    user_operations: Arc<RwLock<HashMap<UserOperationHash, UserOperation>>>, // user_operation_hash -> user_operation
    user_operations_by_sender: Arc<RwLock<HashMap<Address, HashSet<UserOperationHash>>>>, // sender -> user_operations
}

#[async_trait]
impl Mempool for MemoryMempool {
    type UserOperations = Vec<UserOperation>;

    async fn add(
        &mut self,
        user_operation: &UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> anyhow::Result<UserOperationHash> {
        let hash = user_operation.hash(entry_point, chain_id);

        let mut user_operations = self.user_operations.write();
        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        user_operations_by_sender
            .entry(user_operation.sender)
            .or_insert_with(Default::default)
            .insert(hash);
        user_operations.insert(hash, user_operation.clone());

        Ok(hash)
    }

    async fn get(&self, user_operation_hash: &UserOperationHash) -> anyhow::Result<UserOperation> {
        if let Some(user_operation) = self.user_operations.read().get(user_operation_hash) {
            return Ok(user_operation.clone());
        } else {
            return Err(anyhow::anyhow!("User operation not found"));
        }
    }

    async fn get_all(&self) -> anyhow::Result<Self::UserOperations> {
        Ok(self.user_operations.read().values().cloned().collect())
    }

    async fn get_all_by_sender(&self, sender: &Address) -> anyhow::Result<Self::UserOperations> {
        let user_operations = self.user_operations.read();

        if let Some(user_operations_by_sender) = self.user_operations_by_sender.read().get(sender) {
            return Ok(user_operations_by_sender
                .iter()
                .filter_map(|hash| user_operations.get(hash).cloned())
                .collect());
        } else {
            return Ok(vec![]);
        }
    }

    async fn remove(&mut self, user_operation_hash: &UserOperationHash) -> anyhow::Result<()> {
        let user_operation: UserOperation;
        let mut user_operations = self.user_operations.write();

        if let Some(uo) = user_operations.get(user_operation_hash) {
            user_operation = uo.clone();
        } else {
            return Err(anyhow::anyhow!("User operation not found"));
        }

        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        user_operations.remove(user_operation_hash);

        if let Some(uos) = user_operations_by_sender.get_mut(&user_operation.sender) {
            uos.remove(user_operation_hash);

            if uos.is_empty() {
                user_operations_by_sender.remove(&user_operation.sender);
            }
        }

        Ok(())
    }

    async fn clear(&mut self) -> anyhow::Result<()> {
        let mut user_operations = self.user_operations.write();
        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        user_operations.clear();
        user_operations_by_sender.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::{H256, U256};

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let entry_point = Address::random();
        let chain_id = U256::from(5);
        let senders = vec![Address::random(), Address::random(), Address::random()];

        let mut mempool = MemoryMempool::default();
        let mut user_operation: UserOperation;
        let mut user_operation_hash: UserOperationHash = Default::default();

        for i in 0..2 {
            user_operation = UserOperation {
                sender: senders[0],
                nonce: U256::from(i),
                ..UserOperation::random()
            };
            user_operation_hash = mempool
                .add(&user_operation, &entry_point, &chain_id)
                .await
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).await.unwrap(),
                user_operation
            );

            user_operation = UserOperation {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(&user_operation, &entry_point, &chain_id)
                .await
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).await.unwrap(),
                user_operation
            );
        }

        for i in 0..3 {
            user_operation = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(&user_operation, &entry_point, &chain_id)
                .await
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).await.unwrap(),
                user_operation
            );
        }

        assert_eq!(mempool.get_all().await.unwrap().len(), 7);
        assert_eq!(
            mempool.get_all_by_sender(&senders[0]).await.unwrap().len(),
            2
        );
        assert_eq!(
            mempool.get_all_by_sender(&senders[1]).await.unwrap().len(),
            2
        );
        assert_eq!(
            mempool.get_all_by_sender(&senders[2]).await.unwrap().len(),
            3
        );

        assert_eq!(mempool.remove(&user_operation_hash).await.unwrap(), ());
        assert_eq!(
            mempool
                .remove(&H256::random())
                .await
                .unwrap_err()
                .to_string(),
            anyhow::anyhow!("User operation not found").to_string()
        );

        assert_eq!(mempool.get_all().await.unwrap().len(), 6);
        assert_eq!(
            mempool.get_all_by_sender(&senders[0]).await.unwrap().len(),
            2
        );
        assert_eq!(
            mempool.get_all_by_sender(&senders[2]).await.unwrap().len(),
            2
        );

        assert_eq!(mempool.clear().await.unwrap(), ());

        assert_eq!(mempool.get_all().await.unwrap().len(), 0);
        assert_eq!(
            mempool.get_all_by_sender(&senders[0]).await.unwrap().len(),
            0
        );
    }
}
