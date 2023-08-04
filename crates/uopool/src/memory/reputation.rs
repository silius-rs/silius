use crate::reputation::Reputation;
use educe::Educe;
use ethers::types::{Address, U256};
use silius_primitives::reputation::{
    ReputationEntry, ReputationError, ReputationStatus, StakeInfo, Status,
};
use std::collections::{HashMap, HashSet};

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryReputation {
    min_inclusion_denominator: u64,
    throttling_slack: u64,
    ban_slack: u64,
    min_stake: U256,
    min_unstake_delay: U256,
    whitelist: HashSet<Address>,
    blacklist: HashSet<Address>,

    entities: HashMap<Address, ReputationEntry>,
}

impl Reputation for MemoryReputation {
    type ReputationEntries = Vec<ReputationEntry>;
    type Error = anyhow::Error;

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

    fn set(&mut self, addr: &Address) -> Result<(), Self::Error> {
        if !self.entities.contains_key(addr) {
            let ent = ReputationEntry {
                address: *addr,
                uo_seen: 0,
                uo_included: 0,
                status: Status::OK.into(),
            };

            self.entities.insert(*addr, ent);
        }

        Ok(())
    }

    fn get(&mut self, addr: &Address) -> Result<ReputationEntry, Self::Error> {
        if let Some(ent) = self.entities.get(addr) {
            return Ok(ent.clone());
        }

        let ent = ReputationEntry {
            address: *addr,
            uo_seen: 0,
            uo_included: 0,
            status: Status::OK.into(),
        };

        self.entities.insert(*addr, ent.clone());

        Ok(ent)
    }

    fn increment_seen(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen += 1;
        }
        Ok(())
    }

    fn increment_included(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_included += 1;
        }
        Ok(())
    }

    fn update_hourly(&mut self) -> Result<(), Self::Error> {
        for (_, ent) in self.entities.iter_mut() {
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;
        }
        self.entities
            .retain(|_, ent| ent.uo_seen > 0 || ent.uo_included > 0);

        Ok(())
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

    fn get_status(&self, addr: &Address) -> Result<ReputationStatus, Self::Error> {
        if self.is_whitelist(addr) {
            return Ok(Status::OK.into());
        }

        if self.is_blacklist(addr) {
            return Ok(Status::BANNED.into());
        }

        Ok(match self.entities.get(addr) {
            Some(ent) => {
                let min_expected_included = ent.uo_seen / self.min_inclusion_denominator;
                if min_expected_included <= ent.uo_included + self.throttling_slack {
                    Status::OK.into()
                } else if min_expected_included <= ent.uo_included + self.ban_slack {
                    Status::THROTTLED.into()
                } else {
                    Status::BANNED.into()
                }
            }
            _ => Status::OK.into(),
        })
    }

    fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen = 100;
            ent.uo_included = 0;
        }

        Ok(())
    }

    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError> {
        if let Some(info) = info {
            if self.is_whitelist(&info.address) {
                return Ok(());
            }

            if let Some(ent) = self.entities.get(&info.address) {
                if Status::from(ent.status) == Status::BANNED {
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

    fn set_entities(&mut self, entries: Self::ReputationEntries) -> Result<(), Self::Error> {
        for en in entries {
            self.entities.insert(en.address, en);
        }

        Ok(())
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
    use crate::{utils::tests::reputation_test_case, MemoryReputation};

    #[tokio::test]
    async fn memory_reputation() {
        let reputation = MemoryReputation::default();
        reputation_test_case(reputation);
    }
}
