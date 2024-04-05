use crate::{
    mempool::{
        AddRemoveUserOp, AddRemoveUserOpHash, ClearOp, UserOperationAddrOp,
        UserOperationCodeHashOp, UserOperationOp,
    },
    MempoolErrorKind,
};
use ethers::types::Address;
use silius_primitives::{
    simulation::CodeHash, UserOperation, UserOperationHash, UserOperationSigned,
};
use std::collections::{HashMap, HashSet};

impl AddRemoveUserOp for HashMap<UserOperationHash, UserOperationSigned> {
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind> {
        self.insert(uo.hash, uo.user_operation);
        Ok(uo.hash)
    }

    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        if let Some(user_op) = self.get(uo_hash) {
            user_op.clone()
        } else {
            return Ok(false);
        };
        self.remove(uo_hash);
        Ok(true)
    }
}

impl UserOperationOp for HashMap<UserOperationHash, UserOperationSigned> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolErrorKind> {
        if let Some(uo) = self.get(uo_hash) {
            Ok(Some(UserOperation::from_user_operation_signed(*uo_hash, uo.clone())))
        } else {
            Ok(None)
        }
    }

    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        let mut uos: Vec<UserOperation> = self
            .iter()
            .map(|(hash, uo)| UserOperation::from_user_operation_signed(*hash, uo.clone()))
            .collect();
        uos.sort_by(|a, b| {
            if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
            } else {
                a.nonce.cmp(&b.nonce)
            }
        });
        Ok(uos)
    }

    fn get_all(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        Ok(self
            .iter()
            .map(|(hash, uo)| UserOperation::from_user_operation_signed(*hash, uo.clone()))
            .collect())
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
    fn add(
        &mut self,
        address: &Address,
        uo_hash: UserOperationHash,
    ) -> Result<(), MempoolErrorKind> {
        self.entry(*address).or_default().insert(uo_hash);
        Ok(())
    }

    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
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
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        Ok(self.contains_key(uo_hash))
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolErrorKind> {
        self.insert(*uo_hash, hashes.clone());
        Ok(())
    }

    fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolErrorKind> {
        if let Some(hashes) = self.get(uo_hash) {
            Ok(hashes.clone())
        } else {
            Ok(vec![])
        }
    }

    fn remove_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
        self.remove(uo_hash);
        Ok(true)
    }
}

impl ClearOp for HashMap<UserOperationHash, Vec<CodeHash>> {
    fn clear(&mut self) {
        self.clear()
    }
}

impl ClearOp for HashMap<UserOperationHash, UserOperationSigned> {
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
            Box::new(HashMap::<UserOperationHash, UserOperationSigned>::default()),
            Box::new(HashMap::<Address, HashSet<UserOperationHash>>::default()),
            Box::new(HashMap::<Address, HashSet<UserOperationHash>>::default()),
            Box::new(HashMap::<UserOperationHash, Vec<CodeHash>>::default()),
        );
        mempool_test_case(mempool);
    }
}
