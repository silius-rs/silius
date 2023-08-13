use super::{
    env::{DBError, Env},
    tables::EntitiesReputation,
    utils::WrapAddress,
};
use crate::Reputation;
use ethers::types::{Address, U256};
use reth_db::{
    cursor::{DbCursorRO, DbCursorRW},
    database::Database,
    mdbx::EnvironmentKind,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::reputation::{
    ReputationEntry, ReputationError, ReputationStatus, StakeInfo, Status,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug)]
pub struct DatabaseReputation<E: EnvironmentKind> {
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
    /// Libmdbx-sys environment.
    env: Arc<Env<E>>,
}

impl<E: EnvironmentKind> DatabaseReputation<E> {
    /// Spawns a new [DatabaseReputation](DatabaseReputation) instance.
    pub fn new(env: Arc<Env<E>>) -> Self {
        Self {
            min_inclusion_denominator: 0,
            throttling_slack: 0,
            ban_slack: 0,
            min_stake: U256::default(),
            min_unstake_delay: U256::default(),
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            env,
        }
    }
}

impl<E: EnvironmentKind> Reputation for DatabaseReputation<E> {
    type ReputationEntries = Vec<ReputationEntry>;
    type Error = DBError;

    /// Initializes the [Reputation](Reputation) database.
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

    /// Sets the [ReputationEntry](ReputationEntry) for a given address in the database
    ///
    /// # Arguments
    /// * `addr` - Address of the entity
    ///
    /// #Returns
    /// * `Ok(())` if the operation was successful
    /// * `Err(Self::Error)` if the operation failed
    fn set(&mut self, addr: &Address) -> Result<(), Self::Error> {
        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        tx.commit()?;

        if res.is_none() {
            let ent = ReputationEntry {
                address: *addr,
                uo_seen: 0,
                uo_included: 0,
                status: Status::OK.into(),
            };

            let tx = self.env.tx_mut()?;
            tx.put::<EntitiesReputation>((*addr).into(), ent.into())?;
            tx.commit()?;
        }

        Ok(())
    }

    /// Gets the [ReputationEntry](ReputationEntry) for a given address from the database
    ///
    /// # Arguments
    /// * `addr` - Address of the entity
    ///
    /// #Returns
    /// * `Ok(ReputationEntry)` if the operation was successful
    /// * `Err(Self::Error)` if the operation failed
    fn get(&mut self, addr: &Address) -> Result<ReputationEntry, DBError> {
        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        tx.commit()?;

        if let Some(ent) = res {
            Ok(ent.into())
        } else {
            let ent = ReputationEntry {
                address: *addr,
                uo_seen: 0,
                uo_included: 0,
                status: Status::OK.into(),
            };

            let tx = self.env.tx_mut()?;
            tx.put::<EntitiesReputation>((*addr).into(), ent.clone().into())?;
            tx.commit()?;

            Ok(ent)
        }
    }

    /// Increase the number of times an entity's address has been seen in the database
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// #Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn increment_seen(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;

        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx_mut()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        if let Some(ent) = res {
            let mut ent: ReputationEntry = ent.into();
            ent.uo_seen += 1;
            tx.put::<EntitiesReputation>((*addr).into(), ent.into())?;
        }
        tx.commit()?;

        Ok(())
    }

    /// Increases the number of times an entity's address successfully inlucde a [UserOperation](UserOperation) in a block in the database
    ///
    /// # Arguments
    /// * `addr` - The address to increment
    ///
    /// # Returns
    /// * `Ok(())` if the address was incremented successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn increment_included(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;

        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx_mut()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        if let Some(ent) = res {
            let mut ent: ReputationEntry = ent.into();
            ent.uo_included += 1;
            tx.put::<EntitiesReputation>((*addr).into(), ent.into())?;
        }
        tx.commit()?;

        Ok(())
    }

    /// Update an entity's status by hours
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn update_hourly(&mut self) -> Result<(), Self::Error> {
        let tx = self.env.tx_mut()?;
        let mut cursor = tx.cursor_write::<EntitiesReputation>()?;

        while let Ok(Some((addr_wrap, ent))) = cursor.next() {
            let mut ent: ReputationEntry = ent.into();
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;

            if ent.uo_seen > 0 || ent.uo_included > 0 {
                cursor.upsert(addr_wrap, ent.into())?;
            } else {
                cursor.delete_current()?;
            }
        }

        tx.commit()?;

        Ok(())
    }

    /// Add an address to the whitelist in the database
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// * `true` if the address was added successfully. Otherwise, `false`
    fn add_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.insert(*addr)
    }

    /// Remove an address from the whitelist in the database
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// * `true` if the address was removed successfully. Otherwise, `false
    fn remove_whitelist(&mut self, addr: &Address) -> bool {
        self.whitelist.remove(addr)
    }

    /// Check if an address is in the whitelist in the database
    ///
    /// # Arguments
    /// * `addr` - The address to check
    ///
    /// # Returns
    /// * `true` if the address is in the whitelist. Otherwise, `false
    fn is_whitelist(&self, addr: &Address) -> bool {
        self.whitelist.contains(addr)
    }

    /// Add an address to the blacklist in the database
    ///
    /// # Arguments
    /// * `addr` - The address to add
    ///
    /// # Returns
    /// * `true` if the address was added successfully. Otherwise, `false
    fn add_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.insert(*addr)
    }

    /// Remove an address from the blacklist in the database
    ///
    /// # Arguments
    /// * `addr` - The address to remove
    ///
    /// # Returns
    /// * `true` if the address was removed successfully. Otherwise, `false
    fn remove_blacklist(&mut self, addr: &Address) -> bool {
        self.blacklist.remove(addr)
    }

    /// Check if an address is in the blacklist in the database
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

        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;

        Ok(match res {
            Some(ent) => {
                let ent: ReputationEntry = ent.into();

                let min_expected_included = ent.uo_seen / self.min_inclusion_denominator;
                if min_expected_included <= ent.uo_included + self.throttling_slack {
                    Status::OK.into()
                } else if min_expected_included <= ent.uo_included + self.ban_slack {
                    Status::THROTTLED.into()
                } else {
                    Status::BANNED.into()
                }
            }
            None => Status::OK.into(),
        })
    }

    /// Update an entity's status when the [UserOperation](UserOperation) is reverted in the database
    ///
    /// # Arguments
    /// * `addr` - The address to update
    ///
    /// # Returns
    /// * `Ok(())` if the address was updated successfully
    /// * `Err(ReputationError::NotFound)` if the address does not exist
    fn update_handle_ops_reverted(&mut self, addr: &Address) -> Result<(), Self::Error> {
        self.set(addr)?;

        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx_mut()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        if let Some(ent) = res {
            let mut ent: ReputationEntry = ent.into();
            ent.uo_seen = 100;
            ent.uo_included = 0;
            tx.put::<EntitiesReputation>((*addr).into(), ent.into())?;
        }
        tx.commit()?;

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
    /// *`Err(ReputationError::EntityBanned)` if the entity is banned
    /// * `Err(ReputationError::InvalidStake)` if the entity's stake is invalid
    /// * `Err(ReputationError::UnknownError)` if an unknown error occurred
    /// * `Err(ReputationError::StakeTooLow)` if the entity's stake is too low
    /// * `Err(ReputationError::UnstakeDelayTooLow)` if unstakes too early
    fn verify_stake(&self, title: &str, info: Option<StakeInfo>) -> Result<(), ReputationError> {
        if let Some(info) = info {
            if self.is_whitelist(&info.address) {
                return Ok(());
            }

            let tx = self.env.tx().map_err(|_| ReputationError::UnknownError {
                message: "database error".into(),
            })?;
            let res = tx
                .get::<EntitiesReputation>(info.address.into())
                .map_err(|_| ReputationError::UnknownError {
                    message: "database error".into(),
                })?;
            if let Some(ent) = res {
                let ent: ReputationEntry = ent.into();
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

    /// Set the [reputation](ReputationEntries) of an entity in the database
    ///
    /// # Arguments
    /// * `entries` - The [reputation entries](ReputationEntries) to set
    ///
    /// # Returns
    /// * `Ok(())` if the entries were set successfully
    fn set_entities(&mut self, entries: Self::ReputationEntries) -> Result<(), Self::Error> {
        let tx = self.env.tx_mut()?;
        for entry in entries {
            let addr_wrap: WrapAddress = entry.address.into();
            tx.put::<EntitiesReputation>(addr_wrap, entry.into())?;
        }
        tx.commit()?;

        Ok(())
    }

    /// Get all [reputation entries](ReputationEntries)
    ///
    /// # Returns
    /// * All [reputation entries](ReputationEntries)
    fn get_all(&self) -> Self::ReputationEntries {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<EntitiesReputation>()?;
                let res: Vec<ReputationEntry> = c
                    .walk(Some(WrapAddress::default()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    /// Clear all [reputation entries](ReputationEntries)
    fn clear(&mut self) {
        self.env
            .tx_mut()
            .and_then(|tx| {
                tx.clear::<EntitiesReputation>()?;
                tx.commit()
            })
            .expect("Clear database failed");

        self.whitelist.clear();
        self.blacklist.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::{init_env, reputation::DatabaseReputation},
        utils::tests::reputation_test_case,
    };
    use reth_libmdbx::WriteMap;
    use std::sync::Arc;
    use tempdir::TempDir;

    #[tokio::test]
    async fn database_reputation() {
        let dir = TempDir::new("test-silius-db").unwrap();

        let env = init_env::<WriteMap>(dir.into_path()).unwrap();
        env.create_tables()
            .expect("Create mdbx database tables failed");
        let mempool: DatabaseReputation<WriteMap> = DatabaseReputation::new(Arc::new(env));

        reputation_test_case(mempool);
    }
}
