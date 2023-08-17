use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::{keccak256, to_checksum},
};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::fmt::Debug;

pub type MempoolId = H256;

pub type MempoolBox<T, U> =
    Box<dyn Mempool<UserOperations = T, CodeHashes = U, Error = anyhow::Error> + Send + Sync>;

pub fn mempool_id(ep: &Address, chain_id: &U256) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(ep, None).encode(), chain_id.encode()].concat()).as_slice(),
    )
}

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
    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    fn get_sorted(&self) -> Result<Self::UserOperations, Self::Error>;
    fn get_all(&self) -> Self::UserOperations;
    fn clear(&mut self);
}
