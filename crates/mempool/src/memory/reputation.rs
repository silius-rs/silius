use crate::{
    mempool::ClearOp,
    reputation::{HashSetOp, ReputationEntryOp, ReputationOpError},
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
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationOpError> {
        Ok(self.get(addr).cloned())
    }

    fn set_entry(
        &mut self,
        addr: &Address,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationOpError> {
        Ok(self.insert(*addr, entry))
    }

    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationOpError> {
        Ok(self.contains_key(addr))
    }

    fn update(&mut self) -> Result<(), ReputationOpError> {
        for (_, ent) in self.iter_mut() {
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;
        }
        self.retain(|_, ent| ent.uo_seen > 0 || ent.uo_included > 0);

        Ok(())
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
