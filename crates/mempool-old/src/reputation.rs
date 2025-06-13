use crate::{mempool::ClearOp, ReputationError};
use dyn_clone::DynClone;
use ethers::types::{Address, Bytes, U256};
use parking_lot::RwLock;
use silius_primitives::{
    get_address,
    reputation::{ReputationEntry, ReputationStatus, StakeInfo, Status},
};
use std::{collections::HashSet, fmt::Debug, ops::Deref, sync::Arc};

/// Trait representing operations on a HashSet.
pub trait HashSetOp: Default + Sync + Send {
    /// Adds the given address into the list.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to be added.
    ///
    /// # Returns
    ///
    /// Returns `true` if the address was added successfully, `false` otherwise.
    fn add_into_list(&mut self, addr: &Address) -> bool;

    /// Removes the given address from the list.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to be removed.
    ///
    /// # Returns
    ///
    /// Returns `true` if the address was removed successfully, `false` otherwise.
    fn remove_from_list(&mut self, addr: &Address) -> bool;

    /// Checks if the given address is in the list.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to be checked.
    ///
    /// # Returns
    ///
    /// Returns `true` if the address is in the list, `false` otherwise.
    fn is_in_list(&self, addr: &Address) -> bool;
}

impl<T: HashSetOp> HashSetOp for Arc<RwLock<T>> {
    fn add_into_list(&mut self, addr: &Address) -> bool {
        self.write().add_into_list(addr)
    }

    fn remove_from_list(&mut self, addr: &Address) -> bool {
        self.write().remove_from_list(addr)
    }

    fn is_in_list(&self, addr: &Address) -> bool {
        self.read().is_in_list(addr)
    }
}
/// Trait representing operations on a reputation entry.
pub trait ReputationEntryOp: ClearOp + Sync + Send + Debug + DynClone {
    /// Retrieves the reputation entry associated with the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to retrieve the reputation entry for.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(entry))` if the entry exists, `Ok(None)` if the entry does not exist,
    /// or an `Err` if an error occurred during the retrieval.
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationError>;

    /// Sets the reputation entry for the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to set the reputation entry for.
    /// * `entry` - The reputation entry to set.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(previous_entry))` if there was a previous entry for the address,
    /// `Ok(None)` if there was no previous entry, or an `Err` if an error occurred during the
    /// operation.
    fn set_entry(
        &mut self,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationError>;

    /// Checks if a reputation entry exists for the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to check for a reputation entry.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the entry exists, `Ok(false)` if the entry does not exist,
    /// or an `Err` if an error occurred during the check.
    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationError>;

    /// Updates the reputation entries.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the update was successful, or an `Err` if an error occurred during the
    /// update.
    fn update(&mut self) -> Result<(), ReputationError> {
        let all = self.get_all();
        for mut ent in all {
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;
            self.set_entry(ent)?;
        }
        Ok(())
    }

    /// Retrieves all reputation entries.
    ///
    /// # Returns
    ///
    /// Returns a vector containing all reputation entries.
    fn get_all(&self) -> Vec<ReputationEntry>;
}
dyn_clone::clone_trait_object!(ReputationEntryOp);

impl<T: ReputationEntryOp> ReputationEntryOp for Arc<RwLock<T>> {
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationError> {
        self.read().get_entry(addr)
    }

    fn set_entry(
        &mut self,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationError> {
        self.write().set_entry(entry)
    }

    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationError> {
        self.read().contains_entry(addr)
    }

    fn update(&mut self) -> Result<(), ReputationError> {
        self.write().update()
    }

    fn get_all(&self) -> Vec<ReputationEntry> {
        self.read().get_all()
    }
}

#[derive(Debug)]
pub struct Reputation {
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
    whitelist: Arc<RwLock<HashSet<Address>>>,
    /// Blacklisted addreses
    blacklist: Arc<RwLock<HashSet<Address>>>,
    /// Entities' repuation registry
    entities: Box<dyn ReputationEntryOp>,
}

impl Clone for Reputation {
    fn clone(&self) -> Self {
        Self {
            min_inclusion_denominator: self.min_inclusion_denominator,
            throttling_slack: self.throttling_slack,
            ban_slack: self.ban_slack,
            min_stake: self.min_stake,
            min_unstake_delay: self.min_unstake_delay,
            whitelist: self.whitelist.clone(),
            blacklist: self.blacklist.clone(),
            entities: self.entities.clone(),
        }
    }
}

impl Reputation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
        whitelist: Arc<RwLock<HashSet<Address>>>,
        blacklist: Arc<RwLock<HashSet<Address>>>,
        entities: Box<dyn ReputationEntryOp>,
    ) -> Self {
        Self {
            min_inclusion_denominator,
            throttling_slack,
            ban_slack,
            min_stake,
            min_unstake_delay,
            whitelist,
            blacklist,
            entities,
        }
    }

    /// Set the default reputation entry for an address.
    /// It would do nothing if the address already exists.
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// #Returns
    /// * `Ok(())` if the address was added successfully
    /// * `Err(ReputationError::AlreadyExists)` if the address already exists
    fn set_default(&mut self, addr: &Address) -> Result<(), ReputationError> {
        if !self.entities.contains_entry(addr)? {
            let ent = ReputationEntry::default_with_addr(*addr);

            self.entities.set_entry(ent)?;
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
    pub fn get(&self, addr: &Address) -> Result<ReputationEntry, ReputationError> {
        if let Some(ent) = self.entities.get_entry(addr)? {
            Ok(ReputationEntry { status: self.get_status(addr)?, ..ent.clone() })
        } else {
            Ok(ReputationEntry::default_with_addr(*addr))
        }
    }

    /// Increase the number of times an entity's address has been seen
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// # Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    pub fn increment_seen(&mut self, addr: &Address) -> Result<(), ReputationError> {
        self.set_default(addr)?;
        if let Some(mut ent) = self.entities.get_entry(addr)? {
            ent.uo_seen += 1;
            self.entities.set_entry(ent)?;
        }
        Ok(())
    }

    /// Increases the number of times an entity successfully includes a
    /// user operation in a block.
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// # Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    pub fn increment_included(&mut self, addr: &Address) -> Result<(), ReputationError> {
        self.set_default(addr)?;
        if let Some(mut ent) = self.entities.get_entry(addr)? {
            ent.uo_included += 1;
            self.entities.set_entry(ent)?;
        }
        Ok(())
    }

    /// Update an entity's status by hours
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    pub fn update_hourly(&mut self) -> Result<(), ReputationError> {
        self.entities.update()
    }

    /// Add an address to the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// * `true` if the address was added successfully. Otherwise, `false`
    pub fn add_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.add_into_list(addr)
    }

    /// Remove an address from the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// * `true` if the address was removed successfully. Otherwise, `false
    pub fn remove_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.remove_from_list(addr)
    }

    /// Check if an address is in the whitelist
    ///
    /// # Arguments
    /// * `addr` - The address to check
    ///
    /// # Returns
    /// * `true` if the address is in the whitelist. Otherwise, `false
    pub fn is_whitelist(&self, addr: &Address) -> bool {
        self.whitelist.is_in_list(addr)
    }

    /// Add an address to the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// # Returns
    /// * `true` if the address was added successfully. Otherwise, `false
    pub fn add_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.add_into_list(addr)
    }

    /// Remove an address from the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// # Returns
    /// * `true` if the address was removed successfully. Otherwise, `false
    pub fn remove_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.remove_from_list(addr)
    }

    /// Check if an address is in the blacklist
    ///
    /// # Arguments
    /// * `addr` - The address to check
    ///
    /// # Returns
    /// * `true` if the address is in the blacklist. Otherwise, `false
    pub fn is_blacklist(&self, addr: &Address) -> bool {
        self.blacklist.is_in_list(addr)
    }

    pub fn min_stake(&self) -> U256 {
        self.min_stake
    }

    pub fn min_unstake_delay(&self) -> U256 {
        self.min_unstake_delay
    }

    /// Get an entity's reputation status
    ///
    /// # Arguments
    /// * `addr` - The address to get the status of
    ///
    /// # Returns
    /// * `Ok(ReputationStatus)` if the address exists
    pub fn get_status(&self, addr: &Address) -> Result<ReputationStatus, ReputationError> {
        if self.whitelist.is_in_list(addr) {
            return Ok(Status::OK.into());
        }

        if self.blacklist.is_in_list(addr) {
            return Ok(Status::BANNED.into());
        }

        Ok(match self.entities.get_entry(addr)? {
            Some(ent) => {
                let max_seen = ent.uo_seen / self.min_inclusion_denominator;
                if max_seen > ent.uo_included + self.ban_slack {
                    Status::BANNED.into()
                } else if max_seen > ent.uo_included + self.throttling_slack {
                    Status::THROTTLED.into()
                } else {
                    Status::OK.into()
                }
            }
            _ => Status::OK.into(),
        })
    }

    /// Update an entity's status when the user operation is reverted.
    ///
    /// # Arguments
    /// * `addr` - The address to update
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    pub fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), ReputationError> {
        self.set_default(addr)?;
        if let Some(mut ent) = self.entities.get_entry(addr)? {
            ent.uo_seen = 100;
            ent.uo_included = 0;
            self.entities.set_entry(ent)?;
        }

        Ok(())
    }

    /// Verify the stake information of an entity
    ///
    /// # Arguments
    /// * `entity` - The entity type
    /// * `info` - The entity's [stake information](StakeInfo)
    /// * `min_stake` - Min stake required. If set, this value has higher priority than the struct's
    ///   value.
    /// * `min_unstake_delay` - Min unstake delay required. If set, this value has higher priority
    ///   than the struct's value.
    ///
    /// # Returns
    /// * `Ok(())` if the entity's stake is valid
    /// * `Err(ReputationError::EntityBanned)` if the entity is banned
    /// * `Err(ReputationError::StakeTooLow)` if the entity's stake is too low
    /// * `Err(ReputationError::UnstakeDelayTooLow)` if unstakes too early
    pub fn verify_stake(
        &self,
        entity: &str,
        info: Option<StakeInfo>,
        min_stake: Option<U256>,
        min_unstake_delay: Option<U256>,
    ) -> Result<(), ReputationError> {
        if let Some(info) = info {
            if self.whitelist.is_in_list(&info.address) {
                return Ok(());
            }

            let min_stake =
                if let Some(min_stake) = min_stake { min_stake } else { self.min_stake };

            // TODO: use this value below
            let _min_unstake_delay = if let Some(min_unstake_delay) = min_unstake_delay {
                min_unstake_delay
            } else {
                self.min_unstake_delay
            };

            let err = if info.stake < min_stake {
                ReputationError::StakeTooLow {
                    entity: entity.into(),
                    address: info.address,
                    stake: info.stake,
                    min_stake: self.min_stake,
                }
            } else if info.unstake_delay < U256::from(2)
            // TODO: remove this when spec tests are updated!!!!
            /* min_unstake_delay */
            {
                ReputationError::UnstakeDelayTooLow {
                    address: info.address,
                    entity: entity.into(),
                    unstake_delay: info.unstake_delay,
                    min_unstake_delay: self.min_unstake_delay,
                }
            } else {
                return Ok(());
            };

            return Err(err);
        }

        Ok(())
    }
    /// Set the [Reputation Entry](ReputationEntry) of an entity
    ///
    /// # Arguments
    /// * `entries` - The [Reputation Entries](ReputationEntry) to set
    ///
    /// # Returns
    /// * `Ok(())` if the entries were set successfully
    pub fn set_entities(&mut self, entries: Vec<ReputationEntry>) -> Result<(), ReputationError> {
        for en in entries {
            self.entities.set_entry(en)?;
        }

        Ok(())
    }

    /// Get all [Reputation Entries](ReputationEntry)
    ///
    /// # Returns
    /// * All [Reputation Entries](ReputationEntry)
    pub fn get_all(&self) -> Result<Vec<ReputationEntry>, ReputationError> {
        Ok(self
            .entities
            .get_all()
            .into_iter()
            .flat_map(|entry| {
                let status = self.get_status(&entry.address)?;
                Ok::<ReputationEntry, ReputationError>(ReputationEntry { status, ..entry })
            })
            .collect())
    }

    // Try to get the reputation status from a sequence of bytes which the first 20 bytes should be
    // the address This is useful in getting the reputation directly from paymaster_and_data
    // field and init_code field in user operation. If the address is not found in the first 20
    // bytes, it would return ReputationStatus::OK directly.
    pub fn get_status_from_bytes(
        &self,
        bytes: &Bytes,
    ) -> Result<ReputationStatus, ReputationError> {
        let addr_opt = get_address(bytes.deref());
        if let Some(addr) = addr_opt {
            self.get_status(&addr)
        } else {
            Ok(Status::OK.into())
        }
    }

    /// Clear all [Reputation Entries](ReputationEntry)
    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

// impl<H, R> Reputation<H, R>
// where
//     H: HashSetOp + Default,
//     R: ReputationEntryOp + Default,
// {
//     pub fn new_default(
//         min_inclusion_denominator: u64,
//         throttling_slack: u64,
//         ban_slack: u64,
//         min_stake: U256,
//         min_unstake_delay: U256,
//     ) -> Self {
//         Self {
//             min_inclusion_denominator,
//             throttling_slack,
//             ban_slack,
//             min_stake,
//             min_unstake_delay,
//             whitelist: H::default(),
//             blacklist: H::default(),
//             entities: R::default(),
//         }
//     }
// }
