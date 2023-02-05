use educe::Educe;
use ethers::types::{Address, U256};
use std::collections::{HashMap, HashSet};

use crate::types::user_operation::{UserOperation, UserOperationHash};

use super::Mempool;

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    user_operations: HashMap<UserOperationHash, UserOperation>, // user_operation_hash -> user_operation
    user_operations_by_sender: HashMap<Address, HashSet<UserOperationHash>>, // sender -> user_operations
}

impl Mempool for MemoryMempool {
    type UserOperations = Vec<UserOperation>;

    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> UserOperationHash {
        let hash = user_operation.hash(entry_point, chain_id);

        self.user_operations_by_sender
            .entry(user_operation.sender)
            .or_insert_with(Default::default)
            .insert(hash);
        self.user_operations.insert(hash, user_operation);

        hash
    }

    fn get(&self, user_operation_hash: &UserOperationHash) -> anyhow::Result<UserOperation> {
        return if let Some(user_operation) = self.user_operations.get(user_operation_hash) {
            Ok(user_operation.clone())
        } else {
            Err(anyhow::anyhow!("User operation not found"))
        };
    }

    fn get_all_by_sender(&self, sender: &Address) -> Self::UserOperations {
        return if let Some(user_operations_by_sender) = self.user_operations_by_sender.get(sender) {
            user_operations_by_sender
                .iter()
                .filter_map(|hash| self.user_operations.get(hash).cloned())
                .collect()
        } else {
            vec![]
        };
    }

    fn remove(&mut self, user_operation_hash: &UserOperationHash) -> anyhow::Result<()> {
        let user_operation: UserOperation;

        if let Some(uo) = self.user_operations.get(user_operation_hash) {
            user_operation = uo.clone();
        } else {
            return Err(anyhow::anyhow!("User operation not found"));
        }

        self.user_operations.remove(user_operation_hash);

        if let Some(uos) = self
            .user_operations_by_sender
            .get_mut(&user_operation.sender)
        {
            uos.remove(user_operation_hash);

            if uos.is_empty() {
                self.user_operations_by_sender
                    .remove(&user_operation.sender);
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn get_all(&self) -> Self::UserOperations {
        self.user_operations.values().cloned().collect()
    }

    #[cfg(debug_assertions)]
    fn clear(&mut self) {
        self.user_operations.clear();
        self.user_operations_by_sender.clear();
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
            user_operation_hash = mempool.add(user_operation.clone(), &entry_point, &chain_id);

            assert_eq!(mempool.get(&user_operation_hash).unwrap(), user_operation);

            user_operation = UserOperation {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool.add(user_operation.clone(), &entry_point, &chain_id);

            assert_eq!(mempool.get(&user_operation_hash).unwrap(), user_operation);
        }

        for i in 0..3 {
            user_operation = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool.add(user_operation.clone(), &entry_point, &chain_id);

            assert_eq!(mempool.get(&user_operation_hash).unwrap(), user_operation);
        }

        assert_eq!(mempool.get_all().len(), 7);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[1]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 3);

        assert_eq!(mempool.remove(&user_operation_hash).unwrap(), ());
        assert_eq!(
            mempool.remove(&H256::random()).unwrap_err().to_string(),
            anyhow::anyhow!("User operation not found").to_string()
        );

        assert_eq!(mempool.get_all().len(), 6);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 2);

        assert_eq!(mempool.clear(), ());

        assert_eq!(mempool.get_all().len(), 0);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 0);
    }
}
