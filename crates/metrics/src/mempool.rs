use metrics::{counter, describe_counter, describe_gauge, gauge};
use silius_mempool::{
    AddRemoveUserOp, ClearOp, MempoolErrorKind, ReputationEntryOp, ReputationError, UserOperationOp,
};
use silius_primitives::{UserOperation, UserOperationHash};

const MEMPOOL_SIZE: &str = "silius_mempool_size";
const MEMPOOL_ADD_ERROR: &str = "silius_mempool_add_error";
const MEMPOOL_REMOVE_ERROR: &str = "silius_mempool_remove_error";
const REPUTATION_UO_SEEN: &str = "silius_reputation_uo_seen";
const REPUTATION_UO_INCLUDED: &str = "silius_reputation_uo_included";
const REPUTATION_STATUS: &str = "silius_reputation_status";
const REPUTATION_SET_ENTRY_ERROR: &str = "silius_reputation_set_entry.error";

#[derive(Clone, Debug)]
pub struct MetricsHandler<S: Clone> {
    inner: S,
}

impl<S: Clone> MetricsHandler<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: AddRemoveUserOp + Clone> AddRemoveUserOp for MetricsHandler<S> {
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind> {
        match self.inner.add(uo) {
            Ok(res) => {
                gauge!(MEMPOOL_SIZE).increment(1f64);
                Ok(res)
            }
            Err(e) => {
                counter!(MEMPOOL_ADD_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }

    fn remove_by_uo_hash(
        &mut self,
        uo_hash: &silius_primitives::UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
        match self.inner.remove_by_uo_hash(uo_hash) {
            Ok(res) => {
                gauge!(MEMPOOL_SIZE).decrement(1f64);
                Ok(res)
            }
            Err(e) => {
                counter!(MEMPOOL_REMOVE_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }
}

impl<S: UserOperationOp + Clone> UserOperationOp for MetricsHandler<S> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &silius_primitives::UserOperationHash,
    ) -> Result<Option<silius_primitives::UserOperation>, MempoolErrorKind> {
        self.inner.get_by_uo_hash(uo_hash)
    }

    fn get_sorted(&self) -> Result<Vec<silius_primitives::UserOperation>, MempoolErrorKind> {
        self.inner.get_sorted()
    }

    fn get_all(&self) -> Result<Vec<silius_primitives::UserOperation>, MempoolErrorKind> {
        self.inner.get_all()
    }
}

impl<S: ClearOp + Clone> ClearOp for MetricsHandler<S> {
    fn clear(&mut self) {
        self.inner.clear()
    }
}

impl<S: ReputationEntryOp + Clone> ReputationEntryOp for MetricsHandler<S> {
    fn get_entry(
        &self,
        addr: &ethers::types::Address,
    ) -> Result<Option<silius_primitives::reputation::ReputationEntry>, ReputationError> {
        self.inner.get_entry(addr)
    }

    fn set_entry(
        &mut self,
        entry: silius_primitives::reputation::ReputationEntry,
    ) -> Result<Option<silius_primitives::reputation::ReputationEntry>, ReputationError> {
        let addr = entry.address;
        match self.inner.set_entry(entry.clone()) {
            Ok(res) => {
                gauge!(REPUTATION_UO_SEEN, "address" => format!("{addr:x}"))
                    .set(entry.uo_seen as f64);
                gauge!(REPUTATION_UO_INCLUDED, "address" => format!("{addr:x}"))
                    .set(entry.uo_included as f64);
                gauge!(REPUTATION_STATUS, "address" => format!("{addr:x}"))
                    .set(entry.status as f64);
                Ok(res)
            }
            Err(e) => {
                counter!(REPUTATION_SET_ENTRY_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }

    fn contains_entry(&self, addr: &ethers::types::Address) -> Result<bool, ReputationError> {
        self.inner.contains_entry(addr)
    }

    fn get_all(&self) -> Vec<silius_primitives::reputation::ReputationEntry> {
        self.inner.get_all()
    }
}

pub fn describe_mempool_metrics() {
    describe_gauge!(MEMPOOL_SIZE, "The number of user operations in the mempool");
    describe_counter!(MEMPOOL_ADD_ERROR, "The number of errors when adding to the mempool");
    describe_counter!(MEMPOOL_REMOVE_ERROR, "The number of errors when removing from the mempool");
    describe_gauge!(REPUTATION_UO_SEEN, "The number of user operations seen for an address");
    describe_gauge!(
        REPUTATION_UO_INCLUDED,
        "The number of user operations included for an address"
    );
    describe_gauge!(REPUTATION_STATUS, "The status of an address");
    describe_counter!(
        REPUTATION_SET_ENTRY_ERROR,
        "The number of errors when setting a reputation entry"
    );
    counter!(MEMPOOL_ADD_ERROR).absolute(0);
    counter!(MEMPOOL_REMOVE_ERROR).absolute(0);
    counter!(REPUTATION_SET_ENTRY_ERROR).absolute(0);
    gauge!(MEMPOOL_SIZE).set(0f64);
    gauge!(REPUTATION_UO_SEEN).set(0f64);
    gauge!(REPUTATION_UO_INCLUDED).set(0f64);
    gauge!(REPUTATION_STATUS).set(0f64);
}
