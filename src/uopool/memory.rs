use educe::Educe;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::types::user_operation::{UserOperation, UserOperationHash};

use super::{Mempool, MempoolId};

pub type UserOperationsBySender = HashMap<Address, HashSet<UserOperationHash>>;

#[derive(Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    user_operations: Arc<RwLock<HashMap<UserOperationHash, UserOperation>>>, // user_operation_hash -> user_operation
    user_operations_by_entry_point: Arc<RwLock<HashMap<MempoolId, HashSet<UserOperationHash>>>>, // mempool_id -> user_operations
    user_operations_by_sender: Arc<RwLock<HashMap<MempoolId, UserOperationsBySender>>>, // mempool_id -> sender -> user_operations
}

impl Mempool for MemoryMempool {
    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<()> {
        let hash = user_operation.hash(entry_point, chain_id);

        // replace user operation
        if self.user_operations.read().contains_key(&hash) {
            let user_operations = self.user_operations.read();
            let user_operation_old = user_operations.get(&hash).unwrap();
            let max_priority_fee_per_gas_diff = user_operation.max_priority_fee_per_gas
                - user_operation_old.max_priority_fee_per_gas;
            let max_fee_per_gas_diff =
                user_operation.max_fee_per_gas - user_operation_old.max_fee_per_gas;
            if !(user_operation.sender == user_operation_old.sender
                && user_operation.nonce == user_operation_old.nonce
                && user_operation.max_priority_fee_per_gas
                    > user_operation_old.max_priority_fee_per_gas
                && max_priority_fee_per_gas_diff == max_fee_per_gas_diff)
            {
                return Err(anyhow::anyhow!("User operation already exists"));
            }
        }

        let mut user_operations = self.user_operations.write();
        let mut user_operations_by_entry_point = self.user_operations_by_entry_point.write();
        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        let id = MemoryMempool::id(entry_point, chain_id);
        user_operations_by_entry_point
            .get_mut(&id)
            .unwrap()
            .insert(hash);
        user_operations_by_sender
            .get_mut(&id)
            .unwrap()
            .get_mut(&user_operation.sender)
            .unwrap_or(&mut Default::default())
            .insert(hash);
        user_operations.insert(hash, user_operation);

        Ok(())
    }

    fn get(&self, user_operation_hash: UserOperationHash) -> anyhow::Result<UserOperation> {
        if !self
            .user_operations
            .read()
            .contains_key(&user_operation_hash)
        {
            return Err(anyhow::anyhow!("User operation not found"));
        }
        Ok(self
            .user_operations
            .read()
            .get(&user_operation_hash)
            .unwrap()
            .clone())
    }

    fn all(&self) -> anyhow::Result<Vec<UserOperation>> {
        Ok(self.user_operations.read().values().cloned().collect())
    }

    fn all_by_entry_point(
        &self,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let id = MemoryMempool::id(entry_point, chain_id);
        Ok(self
            .user_operations_by_entry_point
            .read()
            .get(&id)
            .unwrap()
            .iter()
            .map(|hash| self.user_operations.read().get(hash).unwrap().clone())
            .collect())
    }

    fn all_by_sender(
        &self,
        sender: Address,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let id = MemoryMempool::id(entry_point, chain_id);
        Ok(self
            .user_operations_by_sender
            .read()
            .get(&id)
            .unwrap()
            .get(&sender)
            .unwrap_or(&Default::default())
            .iter()
            .map(|hash| self.user_operations.read().get(hash).unwrap().clone())
            .collect())
    }

    fn remove(
        &mut self,
        user_operation_hash: UserOperationHash,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<()> {
        if !self
            .user_operations
            .read()
            .contains_key(&user_operation_hash)
        {
            return Err(anyhow::anyhow!("User operation not found"));
        }

        let user_operations = self.user_operations.read();

        let user_operation = user_operations.get(&user_operation_hash).unwrap();
        let id = MemoryMempool::id(entry_point, chain_id);

        let mut user_operations = self.user_operations.write();
        let mut user_operations_by_entry_point = self.user_operations_by_entry_point.write();
        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        user_operations_by_entry_point
            .get_mut(&id)
            .unwrap()
            .remove(&user_operation_hash);
        user_operations_by_sender
            .get_mut(&id)
            .unwrap()
            .get_mut(&user_operation.sender)
            .unwrap()
            .remove(&user_operation_hash);
        user_operations.remove(&user_operation_hash);

        Ok(())
    }

    fn clear(&mut self) -> anyhow::Result<()> {
        let mut user_operations = self.user_operations.write();
        let mut user_operations_by_entry_point = self.user_operations_by_entry_point.write();
        let mut user_operations_by_sender = self.user_operations_by_sender.write();

        for (_, value) in user_operations_by_entry_point.iter_mut() {
            value.clear();
        }
        for (_, value) in user_operations_by_sender.iter_mut() {
            value.clear();
        }
        user_operations.clear();

        Ok(())
    }
}

impl MemoryMempool {
    pub fn new(entry_points: Vec<Address>, chain_id: U256) -> anyhow::Result<Self> {
        if entry_points.is_empty() {
            return Err(anyhow::anyhow!("At least 1 entry point is required"));
        }

        let mut user_operations_by_entry_point =
            HashMap::<MempoolId, HashSet<UserOperationHash>>::new();
        let mut user_operations_by_sender = HashMap::<MempoolId, UserOperationsBySender>::new();

        for entry_point in entry_points {
            let id = MemoryMempool::id(entry_point, chain_id);
            user_operations_by_entry_point.insert(id, Default::default());
            user_operations_by_sender.insert(id, Default::default());
        }

        Ok(Self {
            user_operations: Default::default(),
            user_operations_by_entry_point: Arc::new(RwLock::new(user_operations_by_entry_point)),
            user_operations_by_sender: Arc::new(RwLock::new(user_operations_by_sender)),
        })
    }
}

// tests
