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
    /// Minimum denominator for calculating the minimum expected inclusions
    min_inclusion_denominator: u64,
    /// Constant for calculating the throttling thrshold
    throttling_slack: u64,
    /// Constant for calculating the ban thrshold
    ban_slack: u64,
    /// Minimum stake amount
    min_stake: U256,
    /// Minimum time requuired to unstake
    min_unstake_delay: U256,
    /// Whitelisted addresses
    whitelist: HashSet<Address>,
    /// Blacklisted addreses
    blacklist: HashSet<Address>,
    /// Entities' repuation registry
    entities: HashMap<Address, ReputationEntry>,
}

impl Reputation for MemoryReputation {
    /// An array of [`ReputationEntry`](silius_primitives::reputation::ReputationEntry)
    type ReputationEntries = Vec<ReputationEntry>;
    type Error = anyhow::Error;

    /// Initialize an instance of the [MemoryReputation](MemoryReputation)
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

    /// Add an address to the entity registry
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// #Returns
    /// * `Ok(())` if the address was added successfully
    /// * `Err(ReputationError::AlreadyExists)` if the address already exists
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

    /// Get an entity's [ReputationEntry](ReputationEntry) by address
    ///
    /// # Arguments
    /// * `addr` - The address to get
    ///
    /// # Returns
    /// * `Ok(ReputationEntry)` if the address exists
    /// * `Err(ReputationError::NotFound)` if the address does not exist
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

    /// Increase the number of times an entity's address has been seen
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// # Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn increment_seen(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen += 1;
        }
        Ok(())
    }

    /// Increases the number of times an entity successfully inlucdes a [UserOperation](UserOperation) in a block
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// # Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn increment_included(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_included += 1;
        }
        Ok(())
    }

    /// Update an entity's status by hours
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn update_hourly(&mut self) -> Result<(), Self::Error> {
        for (_, ent) in self.entities.iter_mut() {
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;
        }
        self.entities
            .retain(|_, ent| ent.uo_seen > 0 || ent.uo_included > 0);

        Ok(())
    }

    /// Add an address to the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// * `true` if the address was added successfully. Otherwise, `false`
    fn add_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.insert(*addr)
    }

    /// Remove an address from the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// * `true` if the address was removed successfully. Otherwise, `false
    fn remove_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.remove(addr)
    }

    /// Check if an address is in the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to check
    ///
    /// # Returns
    /// * `true` if the address is in the whitelist. Otherwise, `false
    fn is_whitelist(&self, addr: &Address) -> bool {
        self.whitelist.contains(addr)
    }

    /// Add an address to the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// # Returns
    /// * `true` if the address was added successfully. Otherwise, `false
    fn add_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.insert(*addr)
    }

    /// Remove an address from the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// # Returns
    /// * `true` if the address was removed successfully. Otherwise, `false
    fn remove_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.remove(addr)
    }

    /// Check if an address is in the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to check
    ///
    /// # Returns
    /// * `true` if the address is in the blacklist. Otherwise, `false
    fn is_blacklist(&self, addr: &Address) -> bool {
        self.blacklist.contains(addr)
    }

    /// Get an entity's reputation status
    ///
    /// # Arguments
    /// * `addr` - The address to get the status of
    ///
    /// # Returns
    /// * `Ok(ReputationStatus)` if the address exists
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

    /// Update an entity's status when the [UserOperation](UserOperation) is reverted
    ///
    /// # Arguments
    /// * `addr` - The address to update
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;
        if let Some(ent) = self.entities.get_mut(addr) {
            ent.uo_seen = 100;
            ent.uo_included = 0;
        }

        Ok(())
    }

    /// Verify the stake information of an entity
    ///
    /// # Arguments
    /// * `title` - The entity's name
    /// * `info` - The entity's [stake information](StakeInfo)
    ///
    /// # Returns
    /// * `Ok(())` if the entity's stake is valid
    /// * `Err(ReputationError::EntityBanned)` if the entity is banned
    /// * `Err(ReputationError::StakeTooLow)` if the entity's stake is too low
    /// * `Err(ReputationError::UnstakeDelayTooLow)` if unstakes too early
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

    /// Set the [reputation](ReputationEntries) of an entity
    ///
    /// # Arguments
    /// * `entries` - The [reputation entries](ReputationEntries) to set
    ///
    /// # Returns
    /// * `Ok(())` if the entries were set successfully
    fn set_entities(&mut self, entries: Self::ReputationEntries) -> Result<(), Self::Error> {
        for en in entries {
            self.entities.insert(en.address, en);
        }

        Ok(())
    }

    /// Get all [reputation entries](ReputationEntries)
    ///
    /// # Returns
    /// * All [reputation entries](ReputationEntries)
    fn get_all(&self) -> Self::ReputationEntries {
        self.entities.values().cloned().collect()
    }

    /// Clear all [reputation entries](ReputationEntries)
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
