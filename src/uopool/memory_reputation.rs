use educe::Educe;
use ethers::types::{Address, U256};
use std::collections::{HashMap, HashSet};

use crate::types::reputation::{BadReputationError, ReputationEntry, ReputationStatus, StakeInfo};

use super::Reputation;

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
    fn set(&mut self, address: &Address) {
        if !self.entities.contains_key(address) {
            let entity = ReputationEntry {
                address: *address,
                uo_seen: 0,
                uo_included: 0,
                status: ReputationStatus::OK,
            };

            self.entities.insert(*address, entity);
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

    fn get(&mut self, address: &Address) -> ReputationEntry {
        if let Some(entity) = self.entities.get(address) {
            return *entity;
        }

        let entity = ReputationEntry {
            address: *address,
            uo_seen: 0,
            uo_included: 0,
            status: ReputationStatus::OK,
        };

        self.entities.insert(*address, entity);

        entity
    }

    fn increment_seen(&mut self, address: &Address) {
        self.set(address);
        if let Some(entity) = self.entities.get_mut(address) {
            entity.uo_seen += 1;
        }
    }

    fn increment_included(&mut self, address: &Address) {
        self.set(address);
        if let Some(entity) = self.entities.get_mut(address) {
            entity.uo_included += 1;
        }
    }

    fn update_hourly(&mut self) {
        for (_, entity) in self.entities.iter_mut() {
            entity.uo_seen = entity.uo_seen * 23 / 24;
            entity.uo_included = entity.uo_included * 23 / 24;
        }
        self.entities
            .retain(|_, entity| entity.uo_seen > 0 || entity.uo_included > 0);
    }

    fn add_whitelist(&mut self, address: &Address) -> bool {
        self.whitelist.insert(*address)
    }

    fn remove_whitelist(&mut self, address: &Address) -> bool {
        self.whitelist.remove(address)
    }

    fn is_whitelist(&self, address: &Address) -> bool {
        self.whitelist.contains(address)
    }

    fn add_blacklist(&mut self, address: &Address) -> bool {
        self.blacklist.insert(*address)
    }

    fn remove_blacklist(&mut self, address: &Address) -> bool {
        self.blacklist.remove(address)
    }

    fn is_blacklist(&self, address: &Address) -> bool {
        self.blacklist.contains(address)
    }

    fn get_status(&self, address: &Address) -> ReputationStatus {
        if self.is_whitelist(address) {
            return ReputationStatus::OK;
        }

        if self.is_blacklist(address) {
            return ReputationStatus::BANNED;
        }

        match self.entities.get(address) {
            Some(entity) => {
                let min_expected_included = entity.uo_seen / self.min_inclusion_denominator;
                if min_expected_included <= entity.uo_included + self.throttling_slack {
                    ReputationStatus::OK
                } else if min_expected_included <= entity.uo_included + self.ban_slack {
                    ReputationStatus::THROTTLED
                } else {
                    ReputationStatus::BANNED
                }
            }
            _ => ReputationStatus::OK,
        }
    }

    fn update_handle_ops_reverted(&mut self, address: &Address) {
        self.set(address);
        if let Some(entity) = self.entities.get_mut(address) {
            entity.uo_seen = 100;
            entity.uo_included = 0;
        }
    }

    fn verify_stake(
        &self,
        title: &str,
        stake_info: Option<StakeInfo>,
    ) -> Result<(), BadReputationError> {
        if let Some(stake_info) = stake_info {
            if self.is_whitelist(&stake_info.address) {
                return Ok(());
            }

            if let Some(entity) = self.entities.get(&stake_info.address) {
                let error = if entity.status == ReputationStatus::BANNED {
                    BadReputationError::EntityBanned {
                        address: stake_info.address,
                        title: title.to_string(),
                    }
                } else if stake_info.stake < self.min_stake {
                    BadReputationError::StakeTooLow {
                        address: stake_info.address,
                        title: title.to_string(),
                        min_stake: self.min_stake,
                        min_unstake_delay: self.min_unstake_delay,
                    }
                } else if stake_info.unstake_delay < self.min_unstake_delay {
                    BadReputationError::UnstakeDelayTooLow {
                        address: stake_info.address,
                        title: title.to_string(),
                        min_stake: self.min_stake,
                        min_unstake_delay: self.min_unstake_delay,
                    }
                } else {
                    return Ok(());
                };

                return Err(error);
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn set(&mut self, reputation_entries: Self::ReputationEntries) {
        for reputation in reputation_entries {
            self.entities.insert(reputation.address, reputation);
        }
    }

    #[cfg(debug_assertions)]
    fn get_all(&self) -> Self::ReputationEntries {
        self.entities.values().cloned().collect()
    }

    #[cfg(debug_assertions)]
    fn clear(&mut self) {
        self.entities.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::uopool::{BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK};

    use super::*;

    #[tokio::test]
    async fn memory_reputation() {
        let mut reputation = MemoryReputation::default();
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(1),
            U256::from(0),
        );

        let mut addresses: Vec<Address> = vec![];

        for _ in 0..5 {
            let address = Address::random();
            assert_eq!(
                reputation.get(&address),
                ReputationEntry {
                    address,
                    uo_seen: 0,
                    uo_included: 0,
                    status: ReputationStatus::OK,
                }
            );
            addresses.push(address);
        }

        assert_eq!(reputation.add_whitelist(&addresses[2]), true);
        assert_eq!(reputation.add_blacklist(&addresses[1]), true);

        assert_eq!(reputation.is_whitelist(&addresses[2]), true);
        assert_eq!(reputation.is_whitelist(&addresses[1]), false);
        assert_eq!(reputation.is_blacklist(&addresses[1]), true);
        assert_eq!(reputation.is_blacklist(&addresses[2]), false);

        assert_eq!(reputation.remove_whitelist(&addresses[2]), true);
        assert_eq!(reputation.remove_whitelist(&addresses[1]), false);
        assert_eq!(reputation.remove_blacklist(&addresses[1]), true);
        assert_eq!(reputation.remove_blacklist(&addresses[2]), false);

        assert_eq!(reputation.add_whitelist(&addresses[2]), true);
        assert_eq!(reputation.add_blacklist(&addresses[1]), true);

        assert_eq!(reputation.get_status(&addresses[2]), ReputationStatus::OK);
        assert_eq!(
            reputation.get_status(&addresses[1]),
            ReputationStatus::BANNED
        );
        assert_eq!(reputation.get_status(&addresses[3]), ReputationStatus::OK);

        assert_eq!(reputation.increment_seen(&addresses[2]), ());
        assert_eq!(reputation.increment_seen(&addresses[2]), ());
        assert_eq!(reputation.increment_seen(&addresses[3]), ());
        assert_eq!(reputation.increment_seen(&addresses[3]), ());

        assert_eq!(reputation.increment_included(&addresses[2]), ());
        assert_eq!(reputation.increment_included(&addresses[2]), ());
        assert_eq!(reputation.increment_included(&addresses[3]), ());

        assert_eq!(reputation.update_handle_ops_reverted(&addresses[3]), ());

        for _ in 0..250 {
            assert_eq!(reputation.increment_seen(&addresses[3]), ());
        }
        assert_eq!(
            reputation.get_status(&addresses[3]),
            ReputationStatus::THROTTLED
        );

        for _ in 0..500 {
            assert_eq!(reputation.increment_seen(&addresses[3]), ());
        }
        assert_eq!(
            reputation.get_status(&addresses[3]),
            ReputationStatus::BANNED
        );
    }
}
