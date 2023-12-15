use crate::mempool::{
    AddRemoveUserOp, AddRemoveUserOpHash, ClearOp, MempoolError, UserOperationAddrOp,
    UserOperationCodeHashOp, UserOperationOp,
};
use ethers::types::{Address, U256};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::collections::{HashMap, HashSet};

impl AddRemoveUserOp for HashMap<UserOperationHash, UserOperation> {
    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, MempoolError> {
        let uo_hash = uo.hash(ep, chain_id);
        self.insert(uo_hash, uo);

        Ok(uo_hash)
    }

    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        if let Some(user_op) = self.get(uo_hash) {
            user_op.clone()
        } else {
            return Ok(false);
        };
        self.remove(uo_hash);
        Ok(true)
    }
}

impl UserOperationOp for HashMap<UserOperationHash, UserOperation> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolError> {
        Ok(self.get(uo_hash).cloned())
    }

    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolError> {
        let mut uos: Vec<UserOperation> = self.values().cloned().collect();
        uos.sort_by(|a, b| {
            if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
            } else {
                a.nonce.cmp(&b.nonce)
            }
        });
        Ok(uos)
    }

    fn get_all(&self) -> Result<Vec<UserOperation>, MempoolError> {
        Ok(self.values().cloned().collect())
    }
}

impl UserOperationAddrOp for HashMap<Address, HashSet<UserOperationHash>> {
    fn get_all_by_address(&self, addr: &Address) -> Vec<UserOperationHash> {
        return if let Some(uos_by_relation) = self.get(addr) {
            uos_by_relation.iter().cloned().collect()
        } else {
            vec![]
        };
    }
}

impl AddRemoveUserOpHash for HashMap<Address, HashSet<UserOperationHash>> {
    fn add(&mut self, address: &Address, uo_hash: UserOperationHash) -> Result<(), MempoolError> {
        self.entry(*address).or_default().insert(uo_hash);
        Ok(())
    }

    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolError> {
        if let Some(uos) = self.get_mut(address) {
            uos.remove(uo_hash);

            if uos.is_empty() {
                self.remove(address);
            };
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl UserOperationCodeHashOp for HashMap<UserOperationHash, Vec<CodeHash>> {
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        Ok(self.contains_key(uo_hash))
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolError> {
        self.insert(*uo_hash, hashes.clone());
        Ok(())
    }

    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<Vec<CodeHash>, MempoolError> {
        if let Some(hashes) = self.get(uo_hash) {
            Ok(hashes.clone())
        } else {
            Ok(vec![])
        }
    }

    fn remove_code_hashes(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        self.remove(uo_hash);
        Ok(true)
    }
}

impl ClearOp for HashMap<UserOperationHash, Vec<CodeHash>> {
    fn clear(&mut self) {
        self.clear()
    }
}

impl ClearOp for HashMap<UserOperationHash, UserOperation> {
    fn clear(&mut self) {
        self.clear()
    }
}

impl ClearOp for HashMap<Address, HashSet<UserOperationHash>> {
    fn clear(&mut self) {
        self.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::tests::mempool_test_case, Mempool};

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let mempool = Mempool::new(
            HashMap::<UserOperationHash, UserOperation>::default(),
            HashMap::<Address, HashSet<UserOperationHash>>::default(),
            HashMap::<Address, HashSet<UserOperationHash>>::default(),
            HashMap::<UserOperationHash, Vec<CodeHash>>::default(),
        );
        mempool_test_case(mempool);
    }
}
