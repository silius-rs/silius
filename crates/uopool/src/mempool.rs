use std::{ops::DerefMut, sync::Arc};

use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::{keccak256, to_checksum},
};
use parking_lot::RwLock;
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};

use crate::DBError;

pub type MempoolId = H256;

pub fn mempool_id(ep: &Address, chain_id: &U256) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(ep, None).encode(), chain_id.encode()].concat()).as_slice(),
    )
}

#[derive(Debug)]
pub enum MempoolError {
    DBError(DBError),
}

impl From<DBError> for MempoolError {
    fn from(e: DBError) -> Self {
        Self::DBError(e)
    }
}

impl From<reth_db::Error> for MempoolError {
    fn from(e: reth_db::Error) -> Self {
        Self::DBError(e.into())
    }
}

impl From<MempoolError> for DBError {
    fn from(e: MempoolError) -> Self {
        match e {
            MempoolError::DBError(e) => e,
        }
    }
}

pub trait AddRemoveUserOp {
    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, MempoolError>;
    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError>;
}

impl<T: AddRemoveUserOp> AddRemoveUserOp for Arc<RwLock<T>> {
    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, MempoolError> {
        self.write().add(uo, ep, chain_id)
    }

    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        self.write().remove_by_uo_hash(uo_hash)
    }
}

pub trait AddRemoveUserOpHash {
    fn add(&mut self, address: &Address, uo_hash: UserOperationHash) -> Result<(), MempoolError>;
    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolError>;
}

impl<T: AddRemoveUserOpHash> AddRemoveUserOpHash for Arc<RwLock<T>> {
    fn add(&mut self, address: &Address, uo_hash: UserOperationHash) -> Result<(), MempoolError> {
        self.write().add(address, uo_hash)
    }

    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolError> {
        self.write().remove_uo_hash(address, uo_hash)
    }
}

pub trait UserOperationOp {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolError>;
    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolError>;
    fn get_all(&self) -> Vec<UserOperation>;
}

impl<T: UserOperationOp> UserOperationOp for Arc<RwLock<T>> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolError> {
        self.read().get_by_uo_hash(uo_hash)
    }

    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolError> {
        self.read().get_sorted()
    }

    fn get_all(&self) -> Vec<UserOperation> {
        self.read().get_all()
    }
}

pub trait UserOperationAddrOp {
    fn get_all_by_address(&self, addr: &Address) -> Vec<UserOperationHash>;
    fn get_number_by_address(&self, addr: &Address) -> usize {
        self.get_all_by_address(addr).len()
    }
}

impl<T: UserOperationAddrOp> UserOperationAddrOp for Arc<RwLock<T>> {
    fn get_all_by_address(&self, addr: &Address) -> Vec<UserOperationHash> {
        self.read().get_all_by_address(addr)
    }
}

pub trait UserOperationCodeHashOp {
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError>;
    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Vec<CodeHash>,
    ) -> Result<(), MempoolError>;
    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<Vec<CodeHash>, MempoolError>;
    fn remove_code_hashes(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError>;
}

impl<T: UserOperationCodeHashOp> UserOperationCodeHashOp for Arc<RwLock<T>> {
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        self.read().has_code_hashes(uo_hash)
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Vec<CodeHash>,
    ) -> Result<(), MempoolError> {
        self.write().set_code_hashes(uo_hash, hashes)
    }

    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<Vec<CodeHash>, MempoolError> {
        self.read().get_code_hashes(uo_hash)
    }

    fn remove_code_hashes(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        self.write().remove_code_hashes(uo_hash)
    }
}

pub trait ClearOp {
    fn clear(&mut self);
}

pub trait UserOperationAct: AddRemoveUserOp + UserOperationOp + ClearOp + Send + Sync {}
impl<T> UserOperationAct for T where T: AddRemoveUserOp + UserOperationOp + ClearOp + Send + Sync {}

impl<T: ClearOp> ClearOp for Arc<RwLock<T>> {
    fn clear(&mut self) {
        self.write().clear()
    }
}

pub trait UserOperationAddrAct:
    AddRemoveUserOpHash + UserOperationAddrOp + ClearOp + Send + Sync
{
}
impl<T> UserOperationAddrAct for T where
    T: AddRemoveUserOpHash + UserOperationAddrOp + ClearOp + Send + Sync
{
}

pub trait UserOperationCodeHashAct: UserOperationCodeHashOp + ClearOp + Send + Sync {}
impl<T> UserOperationCodeHashAct for T where T: UserOperationCodeHashOp + ClearOp + Send + Sync {}

pub struct Mempool<T, Y, X, Z>
where
    T: UserOperationAct,
    Y: UserOperationAddrAct,
    X: UserOperationAddrAct,
    Z: UserOperationCodeHashAct,
{
    user_operations: T,
    user_operations_by_sender: Y,
    user_operations_by_entity: X,
    user_operations_code_hashes: Z,
}

impl<T, Y, X, Z> Clone for Mempool<T, Y, X, Z>
where
    T: UserOperationAct + Clone,
    Y: UserOperationAddrAct + Clone,
    X: UserOperationAddrAct + Clone,
    Z: UserOperationCodeHashAct + Clone,
{
    fn clone(&self) -> Self {
        Self {
            user_operations: self.user_operations.clone(),
            user_operations_by_sender: self.user_operations_by_sender.clone(),
            user_operations_by_entity: self.user_operations_by_entity.clone(),
            user_operations_code_hashes: self.user_operations_code_hashes.clone(),
        }
    }
}

impl<T, Y, X, Z> Mempool<T, Y, X, Z>
where
    T: UserOperationAct,
    Y: UserOperationAddrAct,
    X: UserOperationAddrAct,
    Z: UserOperationCodeHashAct,
{
    pub fn new(
        user_operations: T,
        user_operations_by_sender: Y,
        user_operations_by_entity: X,
        user_operations_code_hashes: Z,
    ) -> Self {
        Self {
            user_operations,
            user_operations_by_sender,
            user_operations_by_entity,
            user_operations_code_hashes,
        }
    }
    pub fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, MempoolError> {
        let uo_hash = uo.hash(ep, chain_id);
        let (sender, factory, paymaster) = uo.get_entities();
        self.user_operations.add(uo, ep, chain_id)?;
        self.user_operations_by_sender.add(&sender, uo_hash)?;
        if let Some(factory) = factory {
            self.user_operations_by_entity.add(&factory, uo_hash)?;
        }
        if let Some(paymaster) = paymaster {
            self.user_operations_by_entity.add(&paymaster, uo_hash)?;
        }
        Ok(uo_hash)
    }
    pub fn get(&self, uo_hash: &UserOperationHash) -> Result<Option<UserOperation>, MempoolError> {
        self.user_operations.get_by_uo_hash(uo_hash)
    }
    pub fn get_all_by_sender(&self, addr: &Address) -> Vec<UserOperation> {
        let uos_by_sender = self.user_operations_by_sender.get_all_by_address(addr);
        uos_by_sender
            .iter()
            .flat_map(|uo_hash| self.user_operations.get_by_uo_hash(uo_hash))
            .flatten()
            .collect()
    }
    pub fn get_number_by_sender(&self, addr: &Address) -> usize {
        self.user_operations_by_sender.get_number_by_address(addr)
    }
    pub fn get_number_by_entity(&self, addr: &Address) -> usize {
        self.user_operations_by_entity.get_number_by_address(addr)
    }
    pub fn get_prev_by_sender(&self, uo: &UserOperation) -> Option<UserOperation> {
        self.user_operations_by_sender
            .get_all_by_address(&uo.sender)
            .iter()
            .flat_map(|uo_hash| self.get(uo_hash))
            .flatten()
            .filter(|uo_prev| uo_prev.nonce == uo.nonce)
            .max_by_key(|uo_prev| uo_prev.max_priority_fee_per_gas)
    }
    pub fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        self.user_operations_code_hashes.has_code_hashes(uo_hash)
    }
    pub fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Vec<CodeHash>,
    ) -> Result<(), MempoolError> {
        self.user_operations_code_hashes
            .set_code_hashes(uo_hash, hashes)
    }
    pub fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolError> {
        self.user_operations_code_hashes.get_code_hashes(uo_hash)
    }
    pub fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolError> {
        let uo = if let Some(user_op) = self.user_operations.get_by_uo_hash(uo_hash)? {
            user_op
        } else {
            return Ok(false);
        };

        let (sender, factory, paymaster) = uo.get_entities();

        self.user_operations.remove_by_uo_hash(uo_hash)?;

        self.user_operations_by_sender
            .remove_uo_hash(&sender, uo_hash)?;

        if let Some(factory) = factory {
            self.user_operations_by_entity
                .remove_uo_hash(&factory, uo_hash)?;
        }

        if let Some(paymaster) = paymaster {
            self.user_operations_by_entity
                .remove_uo_hash(&paymaster, uo_hash)?;
        }

        self.user_operations_code_hashes.remove_code_hashes(uo_hash);

        Ok(true)
    }
    pub fn remove_by_entity(&mut self, entity: &Address) -> Result<(), MempoolError> {
        let uos = self.user_operations_by_entity.get_all_by_address(entity);

        for uo_hash in uos {
            self.remove(&uo_hash)?;
        }

        Ok(())
    }
    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    pub fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolError> {
        self.user_operations.get_sorted()
    }
    pub fn get_all(&self) -> Vec<UserOperation> {
        self.user_operations.get_all()
    }
    pub fn clear(&mut self) {
        self.user_operations.clear();
        self.user_operations_by_sender.clear();
        self.user_operations_by_entity.clear();
        self.user_operations_code_hashes.clear();
    }
}
