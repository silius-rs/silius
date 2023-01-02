use educe::Educe;
use ethers::types::{Address, U256};
use std::collections::{HashMap, HashSet};

use crate::types::user_operation::{UserOperation, UserOperationHash};

use super::Mempool;

#[derive(Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    user_operations: HashMap<UserOperationHash, UserOperation>, // user_operation_hash -> user_operation
    user_operations_by_entry_point: HashMap<Address, HashSet<UserOperationHash>>, // entry_point -> user_operations
    user_operations_by_sender: HashMap<Address, HashMap<Address, HashSet<UserOperationHash>>>, // entry_point -> sender -> user_operations
}

impl Mempool for MemoryMempool {
    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<()> {
        let hash = user_operation.hash(entry_point, chain_id);
        if self.user_operations.contains_key(&hash) {
            // TODO: implement replace user operation
            return Err(anyhow::anyhow!("User operation already exists"));
        }
        self.user_operations_by_entry_point
            .get_mut(&entry_point)
            .unwrap()
            .insert(hash);
        self.user_operations_by_sender
            .get_mut(&entry_point)
            .unwrap()
            .get_mut(&user_operation.sender)
            .unwrap_or(&mut Default::default())
            .insert(hash);
        self.user_operations.insert(hash, user_operation);
        Ok(())
    }

    fn get(&self, user_operation_hash: UserOperationHash) -> anyhow::Result<UserOperation> {
        if !self.user_operations.contains_key(&user_operation_hash) {
            return Err(anyhow::anyhow!("User operation not found"));
        }
        Ok(self
            .user_operations
            .get(&user_operation_hash)
            .unwrap()
            .clone())
    }

    fn all(&self) -> anyhow::Result<Vec<UserOperation>> {
        Ok(self.user_operations.values().cloned().collect())
    }

    fn all_by_entry_point(&self, entry_point: Address) -> anyhow::Result<Vec<UserOperation>> {
        Ok(self
            .user_operations_by_entry_point
            .get(&entry_point)
            .unwrap()
            .iter()
            .map(|hash| self.user_operations.get(hash).unwrap().clone())
            .collect())
    }

    fn all_by_sender(
        &self,
        sender: Address,
        entry_point: Address,
    ) -> anyhow::Result<Vec<UserOperation>> {
        Ok(self
            .user_operations_by_sender
            .get(&entry_point)
            .unwrap()
            .get(&sender)
            .unwrap_or(&Default::default())
            .iter()
            .map(|hash| self.user_operations.get(hash).unwrap().clone())
            .collect())
    }

    fn remove(
        &mut self,
        user_operation_hash: UserOperationHash,
        entry_point: Address,
    ) -> anyhow::Result<()> {
        if !self.user_operations.contains_key(&user_operation_hash) {
            return Err(anyhow::anyhow!("User operation not found"));
        }
        let user_operation = self.user_operations.get(&user_operation_hash).unwrap();
        self.user_operations_by_entry_point
            .get_mut(&entry_point)
            .unwrap()
            .remove(&user_operation_hash);
        self.user_operations_by_sender
            .get_mut(&entry_point)
            .unwrap()
            .get_mut(&user_operation.sender)
            .unwrap()
            .remove(&user_operation_hash);
        self.user_operations.remove(&user_operation_hash);
        Ok(())
    }

    fn clear(&mut self) -> anyhow::Result<()> {
        for (_, value) in self.user_operations_by_entry_point.iter_mut() {
            value.clear();
        }
        for (_, value) in self.user_operations_by_sender.iter_mut() {
            value.clear();
        }
        self.user_operations.clear();
        Ok(())
    }
}

impl MemoryMempool {
    pub fn new(entry_points: Vec<Address>) -> anyhow::Result<Self> {
        if entry_points.len() < 1 {
            return Err(anyhow::anyhow!("At least 1 entry point is required"));
        }

        let mut user_operations_by_entry_point = HashMap::new();
        let mut user_operations_by_sender = HashMap::new();

        for entry_point in entry_points {
            user_operations_by_entry_point.insert(entry_point, Default::default());
            user_operations_by_sender.insert(entry_point, Default::default());
        }

        Ok(Self {
            user_operations: Default::default(),
            user_operations_by_entry_point,
            user_operations_by_sender,
        })
    }
}

// tests
