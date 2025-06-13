use crate::MempoolErrorKind;
use dyn_clone::DynClone;
use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::{keccak256, to_checksum},
};
use parking_lot::RwLock;
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::sync::Arc;

pub type MempoolId = H256;

pub fn mempool_id(ep: &Address, chain_id: u64) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(ep, None).encode(), U256::from(chain_id).encode()].concat())
            .as_slice(),
    )
}

/// AddRemoveUserOp describe the ability to add and remove user operation
pub trait AddRemoveUserOp {
    /// Adds a [UserOperation](UserOperation) to the mempool
    ///
    /// # Arguments
    /// * `uo` - The [UserOperation](UserOperation) to add
    ///
    /// # Returns
    /// * `Ok(UserOperationHash)` - The hash of the [UserOperation](UserOperation) that was added
    /// * `Err(MempoolErrorKind)` - If the [UserOperation](UserOperation) could not be added
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind>;
    /// Removes a [UserOperation](UserOperation) by its hash
    ///
    /// # Arguments
    /// * `uo_hash` - The hash of the [UserOperation](UserOperation) to remove
    ///
    /// # Returns
    /// * `Ok(bool)` - true if the [UserOperation](UserOperation) was removed, false means it was
    ///   not found
    /// * `Err(MempoolErrorKind)` - If there are some  internal errors
    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind>;
}

impl<T: AddRemoveUserOp> AddRemoveUserOp for Arc<RwLock<T>> {
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind> {
        self.write().add(uo)
    }

    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        self.write().remove_by_uo_hash(uo_hash)
    }
}

/// AddRemoveUserOpHash describe the ability to add and remove user operation hash set
/// associated with an address
pub trait AddRemoveUserOpHash {
    ///
    /// Adds a user operation hash to the an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address associated with the user operation.
    /// * `uo_hash` - The hash of the user operation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the user operation hash was successfully added to the mempool,
    /// otherwise returns an error of type `MempoolErrorKind`.
    fn add(
        &mut self,
        address: &Address,
        uo_hash: UserOperationHash,
    ) -> Result<(), MempoolErrorKind>;

    /// Removes a user operation hash from an address.
    ///
    /// This function removes the specified user operation hash from the mempool
    /// associated with the given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address associated with the mempool.
    /// * `uo_hash` - The user operation hash to be removed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether the removal was successful or not.
    /// - If the user operation hash was successfully removed, `Ok(true)` is returned.
    /// - If the user operation hash was not found in the mempool, `Ok(false)` is returned.
    /// - If an error occurred during the removal process, an `Err` variant is returned.
    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind>;
}

impl<T: AddRemoveUserOpHash> AddRemoveUserOpHash for Arc<RwLock<T>> {
    fn add(
        &mut self,
        address: &Address,
        uo_hash: UserOperationHash,
    ) -> Result<(), MempoolErrorKind> {
        self.write().add(address, uo_hash)
    }

    fn remove_uo_hash(
        &mut self,
        address: &Address,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
        self.write().remove_uo_hash(address, uo_hash)
    }
}

/// Trait representing operations on user operations.
pub trait UserOperationOp {
    /// Retrieves a user operation by its hash.
    ///
    /// # Arguments
    ///
    /// * `uo_hash` - The hash of the user operation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(UserOperation))` if the user operation is found,
    /// `Ok(None)` if the user operation is not found, or an `Err(MempoolErrorKind)` if an error
    /// occurs.
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolErrorKind>;

    /// Retrieves all user operations sorted by max_priority_fee_per_gas.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<UserOperation>)` containing all user operations sorted in the specified
    /// order, or an `Err(MempoolErrorKind)` if an error occurs.
    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolErrorKind>;

    /// Retrieves all user operations.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<UserOperation>)` containing all user operations,
    /// or an `Err(MempoolErrorKind)` if an error occurs.
    fn get_all(&self) -> Result<Vec<UserOperation>, MempoolErrorKind>;
}

impl<T: UserOperationOp> UserOperationOp for Arc<RwLock<T>> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolErrorKind> {
        self.read().get_by_uo_hash(uo_hash)
    }

    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        self.read().get_sorted()
    }

    fn get_all(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        self.read().get_all()
    }
}

/// Trait for operations related to user operation addresses.
pub trait UserOperationAddrOp {
    /// Retrieves all user operation hashes associated with the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to retrieve user operation hashes for.
    ///
    /// # Returns
    ///
    /// A vector containing all user operation hashes associated with the given address.
    fn get_all_by_address(&self, addr: &Address) -> Vec<UserOperationHash>;

    /// Retrieves the number of user operation hashes associated with the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to retrieve the number of user operation hashes for.
    ///
    /// # Returns
    ///
    /// The number of user operation hashes associated with the given address.
    fn get_number_by_address(&self, addr: &Address) -> usize {
        self.get_all_by_address(addr).len()
    }
}

impl<T: UserOperationAddrOp> UserOperationAddrOp for Arc<RwLock<T>> {
    fn get_all_by_address(&self, addr: &Address) -> Vec<UserOperationHash> {
        self.read().get_all_by_address(addr)
    }
}

/// Trait for managing user operation code hashes in a memory pool.
pub trait UserOperationCodeHashOp {
    /// Checks if the given user operation hash has associated code hashes in the memory pool.
    ///
    /// # Arguments
    ///
    /// * `uo_hash` - The user operation hash to check.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether the user operation hash has associated code hashes.
    ///
    /// - If the user operation hash has associated code hashes, `Ok(true)` is returned.
    /// - If the user operation hash does not have associated code hashes, `Ok(false)` is returned.
    /// - If an error occurs during the operation, an `Err` variant is returned with a
    ///   `MempoolErrorKind`.
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind>;

    /// Sets the code hashes for the given user operation hash in the memory pool.
    ///
    /// # Arguments
    ///
    /// * `uo_hash` - The user operation hash to set the code hashes for.
    /// * `hashes` - The code hashes to associate with the user operation hash.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the success or failure of the operation.
    ///
    /// - If the code hashes are successfully set, `Ok(())` is returned.
    /// - If an error occurs during the operation, an `Err` variant is returned with a
    ///   `MempoolErrorKind`.
    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolErrorKind>;

    /// Retrieves the code hashes associated with the given user operation hash from the memory
    /// pool.
    ///
    /// # Arguments
    ///
    /// * `uo_hash` - The user operation hash to retrieve the code hashes for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the code hashes associated with the user operation hash.
    ///
    /// - If the code hashes are successfully retrieved, `Ok(Vec<CodeHash>)` is returned with the
    ///   code hashes.
    /// - If an error occurs during the operation, an `Err` variant is returned with a
    ///   `MempoolErrorKind`.
    fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolErrorKind>;

    /// Removes the code hashes associated with the given user operation hash from the memory pool.
    ///
    /// # Arguments
    ///
    /// * `uo_hash` - The user operation hash to remove the code hashes for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether the code hashes were successfully removed.
    ///
    /// - If the code hashes are successfully removed, `Ok(true)` is returned.
    /// - If the user operation hash does not have associated code hashes, `Ok(false)` is returned.
    /// - If an error occurs during the operation, an `Err` variant is returned with a
    ///   `MempoolErrorKind`.
    fn remove_code_hashes(&mut self, uo_hash: &UserOperationHash)
        -> Result<bool, MempoolErrorKind>;
}

impl<T: UserOperationCodeHashOp> UserOperationCodeHashOp for Arc<RwLock<T>> {
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        self.read().has_code_hashes(uo_hash)
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolErrorKind> {
        self.write().set_code_hashes(uo_hash, hashes)
    }

    fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolErrorKind> {
        self.read().get_code_hashes(uo_hash)
    }

    fn remove_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
        self.write().remove_code_hashes(uo_hash)
    }
}

/// A trait for clearing operation.
pub trait ClearOp {
    /// Clears the operation.
    fn clear(&mut self);
}

pub trait UserOperationAct:
    AddRemoveUserOp + UserOperationOp + ClearOp + Send + Sync + DynClone
{
}

dyn_clone::clone_trait_object!(UserOperationAct);
impl<T> UserOperationAct for T where
    T: AddRemoveUserOp + UserOperationOp + ClearOp + Send + Sync + Clone
{
}

impl<T: ClearOp> ClearOp for Arc<RwLock<T>> {
    fn clear(&mut self) {
        self.write().clear()
    }
}

pub trait UserOperationAddrAct:
    AddRemoveUserOpHash + UserOperationAddrOp + ClearOp + Send + Sync + DynClone
{
}

dyn_clone::clone_trait_object!(UserOperationAddrAct);
impl<T> UserOperationAddrAct for T where
    T: AddRemoveUserOpHash + UserOperationAddrOp + ClearOp + Send + Sync + Clone
{
}

pub trait UserOperationCodeHashAct:
    UserOperationCodeHashOp + ClearOp + Send + Sync + DynClone
{
}

dyn_clone::clone_trait_object!(UserOperationCodeHashAct);
impl<T> UserOperationCodeHashAct for T where
    T: UserOperationCodeHashOp + ClearOp + Send + Sync + Clone
{
}

#[derive(Clone)]
pub struct Mempool {
    user_operations: Box<dyn UserOperationAct>,
    user_operations_by_sender: Box<dyn UserOperationAddrAct>,
    user_operations_by_entity: Box<dyn UserOperationAddrAct>,
    user_operations_code_hashes: Box<dyn UserOperationCodeHashAct>,
}

impl Mempool {
    pub fn new(
        user_operations: Box<dyn UserOperationAct>,
        user_operations_by_sender: Box<dyn UserOperationAddrAct>,
        user_operations_by_entity: Box<dyn UserOperationAddrAct>,
        user_operations_code_hashes: Box<dyn UserOperationCodeHashAct>,
    ) -> Self {
        Self {
            user_operations,
            user_operations_by_sender,
            user_operations_by_entity,
            user_operations_code_hashes,
        }
    }

    pub fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind> {
        let (sender, factory, paymaster) = uo.get_entities();
        let uo_hash = uo.hash;
        self.user_operations.add(uo)?;
        self.user_operations_by_sender.add(&sender, uo_hash)?;
        if let Some(factory) = factory {
            self.user_operations_by_entity.add(&factory, uo_hash)?;
        }
        if let Some(paymaster) = paymaster {
            self.user_operations_by_entity.add(&paymaster, uo_hash)?;
        }
        Ok(uo_hash)
    }

    pub fn get(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolErrorKind> {
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

    pub fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        self.user_operations_code_hashes.has_code_hashes(uo_hash)
    }

    pub fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolErrorKind> {
        self.user_operations_code_hashes.set_code_hashes(uo_hash, hashes)
    }

    pub fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolErrorKind> {
        self.user_operations_code_hashes.get_code_hashes(uo_hash)
    }

    pub fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        let uo = if let Some(user_op) = self.user_operations.get_by_uo_hash(uo_hash)? {
            user_op
        } else {
            return Ok(false);
        };

        let (sender, factory, paymaster) = uo.get_entities();

        self.user_operations.remove_by_uo_hash(uo_hash)?;

        self.user_operations_by_sender.remove_uo_hash(&sender, uo_hash)?;

        if let Some(factory) = factory {
            self.user_operations_by_entity.remove_uo_hash(&factory, uo_hash)?;
        }

        if let Some(paymaster) = paymaster {
            self.user_operations_by_entity.remove_uo_hash(&paymaster, uo_hash)?;
        }

        self.user_operations_code_hashes.remove_code_hashes(uo_hash)?;

        Ok(true)
    }

    pub fn remove_by_entity(&mut self, entity: &Address) -> Result<(), MempoolErrorKind> {
        let uos = self.user_operations_by_entity.get_all_by_address(entity);

        for uo_hash in uos {
            self.remove(&uo_hash)?;
        }

        Ok(())
    }

    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    pub fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        self.user_operations.get_sorted()
    }

    pub fn get_all(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        self.user_operations.get_all()
    }

    pub fn clear(&mut self) {
        self.user_operations.clear();
        self.user_operations_by_sender.clear();
        self.user_operations_by_entity.clear();
        self.user_operations_code_hashes.clear();
    }
}
