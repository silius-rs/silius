use aa_bundler_primitives::{CodeHash, UserOperation, UserOperationHash};
use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::{keccak256, to_checksum},
};
use jsonrpsee::types::ErrorObject;
use std::fmt::Debug;

pub type MempoolId = H256;

pub type MempoolBox<T, U> =
    Box<dyn Mempool<UserOperations = T, CodeHashes = U, Error = anyhow::Error> + Send + Sync>;

pub type UoPoolError = ErrorObject<'static>;

pub fn mempool_id(entry_point: &Address, chain_id: &U256) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(entry_point, None).encode(), chain_id.encode()].concat()).as_slice(),
    )
}

pub trait Mempool: Debug {
    type UserOperations: IntoIterator<Item = UserOperation>;
    type CodeHashes: IntoIterator<Item = CodeHash>;
    type Error;
    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, Self::Error>;
    fn get(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, Self::Error>;
    fn get_all_by_sender(&self, sender: &Address) -> Self::UserOperations;
    fn get_number_by_sender(&self, sender: &Address) -> usize;
    fn has_code_hashes(&self, user_operation_hash: &UserOperationHash)
        -> Result<bool, Self::Error>;
    fn set_code_hashes(
        &mut self,
        user_operation_hash: &UserOperationHash,
        code_hashes: &Self::CodeHashes,
    ) -> Result<(), Self::Error>;
    fn get_code_hashes(&self, user_operation_hash: &UserOperationHash) -> Self::CodeHashes;
    fn remove(&mut self, user_operation_hash: &UserOperationHash) -> Result<(), Self::Error>;
    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    fn get_sorted(&self) -> Result<Self::UserOperations, Self::Error>;
    fn get_all(&self) -> Self::UserOperations;
    fn clear(&mut self);
}
