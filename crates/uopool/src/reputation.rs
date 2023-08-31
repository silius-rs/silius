use ethers::types::{Address, Bytes, U256};
use parking_lot::RwLock;
use silius_primitives::{
    get_address,
    reputation::{ReputationEntry, ReputationError, ReputationStatus, StakeInfo, Status},
};
use std::{fmt::Debug, ops::Deref, sync::Arc};

#[derive(Debug)]
pub struct ReputationBox<T, R, E>
where
    R: Reputation<ReputationEntries = T, Error = E> + Send + Sync + Debug,
{
    inner: Arc<RwLock<R>>,
}

impl<T, R, E> Clone for ReputationBox<T, R, E>
where
    R: Reputation<ReputationEntries = T, Error = E> + Send + Sync + Debug,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T, R, E> ReputationBox<T, R, E>
where
    R: Reputation<ReputationEntries = T, Error = E> + Send + Sync + Debug,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

impl<T, R, E> Reputation for ReputationBox<T, R, E>
where
    T: Debug + IntoIterator<Item = ReputationEntry>,
    R: Reputation<ReputationEntries = T, Error = E> + Send + Sync,
    E: Debug,
{
    type ReputationEntries = T;
    type Error = E;

    fn add_blacklist(&mut self, addr: &Address) -> bool {
        self.inner.write().add_blacklist(addr)
    }

    fn add_whitelist(&mut self, addr: &Address) -> bool {
        self.inner.write().add_whitelist(addr)
    }

    fn get_all(&self) -> Self::ReputationEntries {
        self.inner.read().get_all()
    }

    fn clear(&mut self) {
        self.inner.write().clear()
    }

    fn get(&mut self, addr: &Address) -> Result<ReputationEntry, Self::Error> {
        self.inner.write().get(addr)
    }
    fn get_status(&self, addr: &Address) -> Result<ReputationStatus, Self::Error> {
        self.inner.read().get_status(addr)
    }

    fn get_status_from_bytes(&self, bytes: &Bytes) -> Result<ReputationStatus, Self::Error> {
        self.inner.read().get_status_from_bytes(bytes)
    }

    fn increment_included(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.inner.write().increment_included(addr)
    }
    fn increment_seen(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.inner.write().increment_seen(addr)
    }

    fn init(
        &mut self,
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
    ) {
        self.inner.write().init(
            min_inclusion_denominator,
            throttling_slack,
            ban_slack,
            min_stake,
            min_unstake_delay,
        )
    }
    fn is_blacklist(&self, addr: &Address) -> bool {
        self.inner.read().is_blacklist(addr)
    }

    fn is_whitelist(&self, addr: &Address) -> bool {
        self.inner.read().is_whitelist(addr)
    }

    fn remove_blacklist(&mut self, addr: &Address) -> bool {
        self.inner.write().remove_blacklist(addr)
    }
    fn remove_whitelist(&mut self, addr: &Address) -> bool {
        self.inner.write().remove_whitelist(addr)
    }

    fn set(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.inner.write().set(addr)
    }

    fn set_entities(&mut self, entries: Self::ReputationEntries) -> Result<(), Self::Error> {
        self.inner.write().set_entities(entries)
    }

    fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.inner.write().update_handle_ops_reverted(addr)
    }

    fn update_hourly(&mut self) -> Result<(), Self::Error> {
        self.inner.write().update_hourly()
    }

    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError> {
        self.inner.read().verify_stake(title, info)
    }
}

/// Reputation trait is imeplemented by [DatabaseMempool](DatabaseMempool) and [MemoryMempool](MemoryMempool) according to [Reputation scoring and throttling/banning for global entities](https://eips.ethereum.org/EIPS/eip-4337#reputation-scoring-and-throttlingbanning-for-global-entities) requirements.
/// [UserOperation’s](UserOperation) storage access rules prevent them from interfere with each other. But “global” entities - paymasters, factories and aggregators are accessed by multiple UserOperations, and thus might invalidate multiple previously-valid UserOperations.
/// To prevent abuse, we need to throttle down (or completely ban for a period of time) an entity that causes invalidation of large number of UserOperations in the mempool. To prevent such entities from “sybil-attack”, we require them to stake with the system, and thus make such DoS attack very expensive.
pub trait Reputation: Debug {
    type ReputationEntries: IntoIterator<Item = ReputationEntry>;
    type Error;

    fn init(
        &mut self,
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
    );
    fn set(&mut self, addr: &Address) -> Result<(), Self::Error>;
    fn get(&mut self, addr: &Address) -> Result<ReputationEntry, Self::Error>;
    fn increment_seen(&mut self, addr: &Address) -> Result<(), Self::Error>;
    fn increment_included(&mut self, addr: &Address) -> Result<(), Self::Error>;
    fn update_hourly(&mut self) -> Result<(), Self::Error>;
    fn add_whitelist(&mut self, addr: &Address) -> bool;
    fn remove_whitelist(&mut self, addr: &Address) -> bool;
    fn is_whitelist(&self, addr: &Address) -> bool;
    fn add_blacklist(&mut self, addr: &Address) -> bool;
    fn remove_blacklist(&mut self, addr: &Address) -> bool;
    fn is_blacklist(&self, addr: &Address) -> bool;
    fn get_status(&self, addr: &Address) -> Result<ReputationStatus, Self::Error>;
    fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), Self::Error>;
    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError>;

    // Try to get the reputation status from a sequence of bytes which the first 20 bytes should be the address
    // This is useful in getting the reputation directly from paymaster_and_data field and init_code field in user operation.
    // If the address is not found in the first 20 bytes, it would return ReputationStatus::OK directly.
    fn get_status_from_bytes(&self, bytes: &Bytes) -> Result<ReputationStatus, Self::Error> {
        let addr_opt = get_address(bytes.deref());
        if let Some(addr) = addr_opt {
            self.get_status(&addr)
        } else {
            Ok(Status::OK.into())
        }
    }

    fn set_entities(&mut self, entries: Self::ReputationEntries) -> Result<(), Self::Error>;
    fn get_all(&self) -> Self::ReputationEntries;
    fn clear(&mut self);
}
