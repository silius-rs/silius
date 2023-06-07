use crate::reputation::Reputation;
use aa_bundler_primitives::reputation::{
    ReputationEntry, ReputationError, ReputationStatus, StakeInfo,
};
use educe::Educe;
use ethers::types::{Address, U256};
use std::collections::{HashMap, HashSet};

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryReputation {
    min_inclusion_denominator: u64,
    throttling_slack: u64,
    ban_slack: u64,
    min_stake: U256,
    min_unstake_delay: U256,

    entities: HashMap<Address, ReputationEntry>,
    whitelist: HashSet<Address>,
    blacklist: HashSet<Address>,
}

impl MemoryReputation {
    fn set(&mut self, addr: &Address) {
        if !self.entities.contains_key(addr) {
            let ent = ReputationEntry {
                address: *addr,
                uo_seen: 0,
                uo_included: 0,
                status: ReputationStatus::OK,
            };

            self.entities.insert(*addr, ent);
        }
    }
}

impl Reputation for MemoryReputation {
    type ReputationEntries = Vec<ReputationEntry>;

    fn init(
        &mut self,
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
    ) {
        self.min_inclusion_denominator = min_inclusion_denominator;
        self.throttling_slack = throttling_slack;
        self.ban_slack = ban_slack;
        self.min_stake = min_stake;
        self.min_unstake_delay = min_unstake_delay;
    }

    fn get(&mut self, addr: &Address) -> ReputationEntry {
        if let Some(ent) = self.entities.get(addr) {
            return *ent;
        }

        let ent = ReputationEntry {
            address: *addr,
            uo_seen: 0,
            uo_included: 0,
            status: ReputationStatus::OK,
        };

        self.entities.insert(*addr, ent);

        ent
    }

    fn increment_seen(&mut self, addr: &Address) {
        self.set(addr);
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen += 1;
        }
    }

    fn increment_included(&mut self, addr: &Address) {
        self.set(addr);
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_included += 1;
        }
    }

    fn update_hourly(&mut self) {
        for (_, ent) in self.entities.iter_mut() {
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;
        }
        self.entities
            .retain(|_, ent| ent.uo_seen > 0 || ent.uo_included > 0);
    }

    fn add_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.insert(*addr)
    }

    fn remove_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.remove(addr)
    }

    fn is_whitelist(&self, addr: &Address) -> bool {
        self.whitelist.contains(addr)
    }

    fn add_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.insert(*addr)
    }

    fn remove_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.remove(addr)
    }

    fn is_blacklist(&self, addr: &Address) -> bool {
        self.blacklist.contains(addr)
    }

    fn get_status(&self, addr: &Address) -> ReputationStatus {
        if self.is_whitelist(addr) {
            return ReputationStatus::OK;
        }

        if self.is_blacklist(addr) {
            return ReputationStatus::BANNED;
        }

        match self.entities.get(addr) {
            Some(ent) => {
                let min_expected_included = ent.uo_seen / self.min_inclusion_denominator;
                if min_expected_included <= ent.uo_included + self.throttling_slack {
                    ReputationStatus::OK
                } else if min_expected_included <= ent.uo_included + self.ban_slack {
                    ReputationStatus::THROTTLED
                } else {
                    ReputationStatus::BANNED
                }
            }
            _ => ReputationStatus::OK,
        }
    }

    fn update_handle_ops_reverted(&mut self, addr: &Address) {
        self.set(addr);
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen = 100;
            ent.uo_included = 0;
        }
    }

    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError> {
        if let Some(info) = info {
            if self.is_whitelist(&info.address) {
                return Ok(());
            }

            if let Some(ent) = self.entities.get(&info.address) {
                if ent.status == ReputationStatus::BANNED {
                    return Err(ReputationError::EntityBanned {
                        address: info.address,
                        title: title.to_string(),
                    });
                }
            }

            let err = if info.stake < self.min_stake {
                ReputationError::StakeTooLow {
                    address: info.address,
                    title: title.to_string(),
                    min_stake: self.min_stake,
                    min_unstake_delay: self.min_unstake_delay,
                }
            } else if info.unstake_delay < self.min_unstake_delay {
                ReputationError::UnstakeDelayTooLow {
                    address: info.address,
                    title: title.to_string(),
                    min_stake: self.min_stake,
                    min_unstake_delay: self.min_unstake_delay,
                }
            } else {
                return Ok(());
            };

            return Err(err);
        }

        Ok(())
    }

    fn set(&mut self, entries: Self::ReputationEntries) {
        for en in entries {
            self.entities.insert(en.address, en);
        }
    }

    fn get_all(&self) -> Self::ReputationEntries {
        self.entities.values().cloned().collect()
    }

    fn clear(&mut self) {
        self.entities.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_bundler_primitives::reputation::{
        BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
    };

    #[tokio::test]
    async fn memory_reputation() {
        let mut reputation: MemoryReputation = MemoryReputation::default();
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(1),
            U256::from(0),
        );

        let mut addrs: Vec<Address> = vec![];

        for _ in 0..5 {
            let addr = Address::random();
            assert_eq!(
                reputation.get(&addr),
                ReputationEntry {
                    address: addr,
                    uo_seen: 0,
                    uo_included: 0,
                    status: ReputationStatus::OK,
                }
            );
            addrs.push(addr);
        }

        assert_eq!(reputation.add_whitelist(&addrs[2]), true);
        assert_eq!(reputation.add_blacklist(&addrs[1]), true);

        assert_eq!(reputation.is_whitelist(&addrs[2]), true);
        assert_eq!(reputation.is_whitelist(&addrs[1]), false);
        assert_eq!(reputation.is_blacklist(&addrs[1]), true);
        assert_eq!(reputation.is_blacklist(&addrs[2]), false);

        assert_eq!(reputation.remove_whitelist(&addrs[2]), true);
        assert_eq!(reputation.remove_whitelist(&addrs[1]), false);
        assert_eq!(reputation.remove_blacklist(&addrs[1]), true);
        assert_eq!(reputation.remove_blacklist(&addrs[2]), false);

        assert_eq!(reputation.add_whitelist(&addrs[2]), true);
        assert_eq!(reputation.add_blacklist(&addrs[1]), true);

        assert_eq!(reputation.get_status(&addrs[2]), ReputationStatus::OK);
        assert_eq!(reputation.get_status(&addrs[1]), ReputationStatus::BANNED);
        assert_eq!(reputation.get_status(&addrs[3]), ReputationStatus::OK);

        assert_eq!(reputation.increment_seen(&addrs[2]), ());
        assert_eq!(reputation.increment_seen(&addrs[2]), ());
        assert_eq!(reputation.increment_seen(&addrs[3]), ());
        assert_eq!(reputation.increment_seen(&addrs[3]), ());

        assert_eq!(reputation.increment_included(&addrs[2]), ());
        assert_eq!(reputation.increment_included(&addrs[2]), ());
        assert_eq!(reputation.increment_included(&addrs[3]), ());

        assert_eq!(reputation.update_handle_ops_reverted(&addrs[3]), ());

        for _ in 0..250 {
            assert_eq!(reputation.increment_seen(&addrs[3]), ());
        }
        assert_eq!(
            reputation.get_status(&addrs[3]),
            ReputationStatus::THROTTLED
        );

        for _ in 0..500 {
            assert_eq!(reputation.increment_seen(&addrs[3]), ());
        }
        assert_eq!(reputation.get_status(&addrs[3]), ReputationStatus::BANNED);
    }
}
