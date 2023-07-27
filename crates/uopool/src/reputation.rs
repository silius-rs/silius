use ethers::types::{Address, Bytes, U256};
use silius_primitives::{
    get_address,
    reputation::{ReputationEntry, ReputationError, ReputationStatus, StakeInfo},
};
use std::{fmt::Debug, ops::Deref};

pub type ReputationBox<T> = Box<dyn Reputation<ReputationEntries = T> + Send + Sync>;

pub trait Reputation: Debug {
    type ReputationEntries: IntoIterator<Item = ReputationEntry>;

    fn init(
        &mut self,
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
    );
    fn get(&mut self, addr: &Address) -> ReputationEntry;
    fn increment_seen(&mut self, addr: &Address);
    fn increment_included(&mut self, addr: &Address);
    fn update_hourly(&mut self);
    fn add_whitelist(&mut self, addr: &Address) -> bool;
    fn remove_whitelist(&mut self, addr: &Address) -> bool;
    fn is_whitelist(&self, addr: &Address) -> bool;
    fn add_blacklist(&mut self, addr: &Address) -> bool;
    fn remove_blacklist(&mut self, addr: &Address) -> bool;
    fn is_blacklist(&self, addr: &Address) -> bool;
    fn get_status(&self, addr: &Address) -> ReputationStatus;
    fn update_handle_ops_reverted(&mut self, addr: &Address);
    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError>;

    // Try to get the reputation status from a sequence of bytes which the first 20 bytes should be the address
    // This is useful in getting the reputation directly from paymaster_and_data field and init_code field in user operation.
    // If the address is not found in the first 20 bytes, it would return ReputationStatus::OK directly.
    fn get_status_from_bytes(&self, bytes: &Bytes) -> ReputationStatus {
        let addr_opt = get_address(bytes.deref());
        if let Some(addr) = addr_opt {
            self.get_status(&addr)
        } else {
            ReputationStatus::OK
        }
    }

    fn set(&mut self, entries: Self::ReputationEntries);
    fn get_all(&self) -> Self::ReputationEntries;
    fn clear(&mut self);
}
