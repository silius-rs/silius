use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::{keccak256, to_checksum},
};
use parking_lot::RwLock;
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::{fmt::Debug, sync::Arc};

pub type MempoolId = H256;

/// A thread safe wrapper around a [Mempool](Mempool)
///
/// The Mempool box provide a RwLock for the inner Mempool which could be multi-thread accessed
#[derive(Debug)]
pub struct MempoolBox<T, U, P, E>
where
    P: Mempool<UserOperations = T, CodeHashes = U, Error = E> + Send + Sync,
{
    inner: Arc<RwLock<P>>,
}

impl<T, U, P, E> Clone for MempoolBox<T, U, P, E>
where
    P: Mempool<UserOperations = T, CodeHashes = U, Error = E> + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T, U, P, E> MempoolBox<T, U, P, E>
where
    P: Mempool<UserOperations = T, CodeHashes = U, Error = E> + Send + Sync,
{
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

impl<T, U, M, E> Mempool for MempoolBox<T, U, M, E>
where
    T: Debug + IntoIterator<Item = UserOperation>,
    U: Debug + IntoIterator<Item = CodeHash>,
    M: Mempool<UserOperations = T, CodeHashes = U, Error = E> + Send + Sync,
    E: Debug,
{
    type CodeHashes = U;
    type Error = E;
    type UserOperations = T;

    fn get_all(&self) -> T {
        self.inner.read().get_all()
    }

    fn clear(&mut self) {
        self.inner.write().clear()
    }

    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, Self::Error> {
        self.inner.write().add(uo, ep, chain_id)
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &U,
    ) -> Result<(), Self::Error> {
        self.inner.write().set_code_hashes(uo_hash, hashes)
    }

    fn get(&self, uo_hash: &UserOperationHash) -> Result<Option<UserOperation>, Self::Error> {
        self.inner.read().get(uo_hash)
    }

    fn get_all_by_sender(&self, addr: &Address) -> Self::UserOperations {
        self.inner.read().get_all_by_sender(addr)
    }

    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Self::CodeHashes {
        self.inner.read().get_code_hashes(uo_hash)
    }

    fn get_number_by_sender(&self, addr: &Address) -> usize {
        self.inner.read().get_number_by_sender(addr)
    }

    fn get_number_by_entity(&self, addr: &Address) -> usize {
        self.inner.read().get_number_by_entity(addr)
    }

    fn get_prev_by_sender(&self, uo: &UserOperation) -> Option<UserOperation> {
        self.inner.read().get_prev_by_sender(uo)
    }

    fn get_sorted(&self) -> Result<Self::UserOperations, Self::Error> {
        self.inner.read().get_sorted()
    }

    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, Self::Error> {
        self.inner.read().has_code_hashes(uo_hash)
    }

    fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<(), Self::Error> {
        self.inner.write().remove(uo_hash)
    }

    fn remove_by_entity(&mut self, entity: &Address) -> Result<(), Self::Error> {
        self.inner.write().remove_by_entity(entity)
    }
}

pub fn mempool_id(ep: &Address, chain_id: &U256) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(ep, None).encode(), chain_id.encode()].concat()).as_slice(),
    )
}

/// Mempool trait that's implemented by [DatabaseMempool](DatabaseMempool) and [MemoryMempool](MemoryMempool)
/// See [DatabaseMempool](DatabaseMempool) and [MemoryMempool](MemoryMempool) for implementation details
pub trait Mempool: Debug {
    type UserOperations: IntoIterator<Item = UserOperation>;
    type CodeHashes: IntoIterator<Item = CodeHash>;
    type Error;

    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, Self::Error>;
    fn get(&self, uo_hash: &UserOperationHash) -> Result<Option<UserOperation>, Self::Error>;
    fn get_all_by_sender(&self, addr: &Address) -> Self::UserOperations;
    fn get_number_by_sender(&self, addr: &Address) -> usize;
    fn get_number_by_entity(&self, addr: &Address) -> usize;
    fn get_prev_by_sender(&self, uo: &UserOperation) -> Option<UserOperation> {
        self.get_all_by_sender(&uo.sender)
            .into_iter()
            .filter(|uo_prev| uo_prev.nonce == uo.nonce)
            .max_by_key(|uo_prev| uo_prev.max_priority_fee_per_gas)
    }
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, Self::Error>;
    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Self::CodeHashes,
    ) -> Result<(), Self::Error>;
    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Self::CodeHashes;
    fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<(), Self::Error>;
    fn remove_by_entity(&mut self, entity: &Address) -> Result<(), Self::Error>;
    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    fn get_sorted(&self) -> Result<Self::UserOperations, Self::Error>;
    fn get_all(&self) -> Self::UserOperations;
    fn clear(&mut self);
}
