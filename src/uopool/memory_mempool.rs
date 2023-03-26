use educe::Educe;
use ethers::types::{Address, U256};
use std::collections::{HashMap, HashSet};

use crate::types::{
    simulation::CodeHash,
    user_operation::{UserOperation, UserOperationHash},
};

use super::Mempool;

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    user_operations: HashMap<UserOperationHash, UserOperation>, // user_operation_hash -> user_operation
    user_operations_by_sender: HashMap<Address, HashSet<UserOperationHash>>, // sender -> user_operations
    code_hashes_by_user_operation: HashMap<UserOperationHash, Vec<CodeHash>>, // user_operation_hash -> (contract_address -> code_hash)
}

impl Mempool for MemoryMempool {
    type UserOperations = Vec<UserOperation>;
    type CodeHashes = Vec<CodeHash>;
    type Error = anyhow::Error;

    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> anyhow::Result<UserOperationHash> {
        let hash = user_operation.hash(entry_point, chain_id);

        self.user_operations_by_sender
            .entry(user_operation.sender)
            .or_insert_with(Default::default)
            .insert(hash);
        self.user_operations.insert(hash, user_operation);

        Ok(hash)
    }

    fn get(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> anyhow::Result<Option<UserOperation>> {
        Ok(self.user_operations.get(user_operation_hash).cloned())
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

    fn get_number_by_sender(&self, sender: &Address) -> usize {
        return if let Some(user_operations_by_sender) = self.user_operations_by_sender.get(sender) {
            user_operations_by_sender.len()
        } else {
            0
        };
    }

    fn has_code_hashes(&self, user_operation_hash: &UserOperationHash) -> anyhow::Result<bool> {
        Ok(self
            .code_hashes_by_user_operation
            .contains_key(user_operation_hash))
    }

    fn set_code_hashes(
        &mut self,
        user_operation_hash: &UserOperationHash,
        code_hashes: &Self::CodeHashes,
    ) -> anyhow::Result<(), Self::Error> {
        self.code_hashes_by_user_operation
            .insert(*user_operation_hash, code_hashes.clone());
        Ok(())
    }

    fn get_code_hashes(&self, user_operation_hash: &UserOperationHash) -> Self::CodeHashes {
        if let Some(code_hashes) = self.code_hashes_by_user_operation.get(user_operation_hash) {
            code_hashes.clone()
        } else {
            vec![]
        }
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

        self.code_hashes_by_user_operation
            .remove(user_operation_hash);

        Ok(())
    }

    fn get_sorted(&self) -> anyhow::Result<Self::UserOperations> {
        let mut user_operations: Vec<UserOperation> =
            self.user_operations.values().cloned().collect();
        user_operations.sort_by(|a, b| b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas));
        Ok(user_operations)
    }

    fn get_all(&self) -> Self::UserOperations {
        self.user_operations.values().cloned().collect()
    }

    fn clear(&mut self) {
        self.user_operations.clear();
        self.user_operations_by_sender.clear();
        self.code_hashes_by_user_operation.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uopool::utils::tests::mempool_test_case;
    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let mempool = MemoryMempool::default();
        mempool_test_case(mempool, "User operation not found");
    }
}
