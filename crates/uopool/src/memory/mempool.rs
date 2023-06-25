use crate::mempool::Mempool;
use educe::Educe;
use ethers::types::{Address, U256};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::collections::{HashMap, HashSet};

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
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> anyhow::Result<UserOperationHash> {
        let uo_hash = uo.hash(ep, chain_id);

        self.user_operations_by_sender
            .entry(uo.sender)
            .or_insert_with(Default::default)
            .insert(uo_hash);
        self.user_operations.insert(uo_hash, uo);

        Ok(uo_hash)
    }

    fn get(&self, uo_hash: &UserOperationHash) -> anyhow::Result<Option<UserOperation>> {
        Ok(self.user_operations.get(uo_hash).cloned())
    }

    fn get_all_by_sender(&self, addr: &Address) -> Self::UserOperations {
        return if let Some(uos_by_sender) = self.user_operations_by_sender.get(addr) {
            uos_by_sender
                .iter()
                .filter_map(|uo_hash| self.user_operations.get(uo_hash).cloned())
                .collect()
        } else {
            vec![]
        };
    }

    fn get_prev_by_sender(&self, uo: &UserOperation) -> Option<UserOperation> {
        self.get_all_by_sender(&uo.sender)
            .iter()
            .find(|uo_prev| uo_prev.nonce == uo.nonce)
            .cloned()
    }

    fn get_number_by_sender(&self, addr: &Address) -> usize {
        return if let Some(uos_by_sender) = self.user_operations_by_sender.get(addr) {
            uos_by_sender.len()
        } else {
            0
        };
    }

    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> anyhow::Result<bool> {
        Ok(self.code_hashes_by_user_operation.contains_key(uo_hash))
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Self::CodeHashes,
    ) -> anyhow::Result<(), Self::Error> {
        self.code_hashes_by_user_operation
            .insert(*uo_hash, hashes.clone());
        Ok(())
    }

    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Self::CodeHashes {
        if let Some(hashes) = self.code_hashes_by_user_operation.get(uo_hash) {
            hashes.clone()
        } else {
            vec![]
        }
    }

    fn remove(&mut self, uo_hash: &UserOperationHash) -> anyhow::Result<()> {
        let uo: UserOperation;

        if let Some(user_op) = self.user_operations.get(uo_hash) {
            uo = user_op.clone();
        } else {
            return Err(anyhow::anyhow!("User operation not found"));
        }

        self.user_operations.remove(uo_hash);

        if let Some(uos) = self.user_operations_by_sender.get_mut(&uo.sender) {
            uos.remove(uo_hash);

            if uos.is_empty() {
                self.user_operations_by_sender.remove(&uo.sender);
            }
        }

        self.code_hashes_by_user_operation.remove(uo_hash);

        Ok(())
    }

    fn get_sorted(&self) -> anyhow::Result<Self::UserOperations> {
        let mut uos: Vec<UserOperation> = self.user_operations.values().cloned().collect();
        uos.sort_by(|a, b| {
            if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
            } else {
                a.nonce.cmp(&b.nonce)
            }
        });
        Ok(uos)
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
    use crate::utils::tests::mempool_test_case;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let mempool = MemoryMempool::default();
        mempool_test_case(mempool, "User operation not found");
    }
}
