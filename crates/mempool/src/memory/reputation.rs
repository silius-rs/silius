use crate::{
    mempool::ClearOp,
    reputation::{HashSetOp, ReputationEntryOp},
    ReputationError,
};
use ethers::types::Address;
use silius_primitives::reputation::ReputationEntry;
use std::collections::{HashMap, HashSet};

impl HashSetOp for HashSet<Address> {
    fn add_into_list(&mut self, addr: &Address) -> bool {
        self.insert(*addr)
    }

    fn remove_from_list(&mut self, addr: &Address) -> bool {
        self.remove(addr)
    }

    fn is_in_list(&self, addr: &Address) -> bool {
        self.contains(addr)
    }
}

impl ClearOp for HashMap<Address, ReputationEntry> {
    fn clear(&mut self) {
        self.clear()
    }
}

impl ReputationEntryOp for HashMap<Address, ReputationEntry> {
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationError> {
        Ok(self.get(addr).cloned())
    }

    fn set_entry(
        &mut self,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationError> {
        Ok(self.insert(entry.address, entry))
    }

    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationError> {
        Ok(self.contains_key(addr))
    }

    fn get_all(&self) -> Vec<ReputationEntry> {
        self.values().cloned().collect()
    }
}
#[cfg(test)]
mod tests {
    use crate::{utils::tests::reputation_test_case, Reputation};
    use ethers::types::{Address, U256};
    use silius_primitives::{
        constants::validation::reputation::{
            BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
        },
        reputation::ReputationEntry,
    };
    use std::collections::{HashMap, HashSet};

    #[tokio::test]
    async fn memory_reputation() {
        let reputation =
            Reputation::<HashSet<Address>, HashMap<Address, ReputationEntry>>::new_default(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                U256::from(1),
                U256::from(0),
            );
        reputation_test_case(reputation);
    }
}
