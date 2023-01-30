use async_trait::async_trait;
use educe::Educe;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::types::reputation::{
    BadReputationError, ReputationEntry, ReputationError, ReputationStatus, StakeInfo,
};

use super::Reputation;

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryReputation {
    min_inclusion_denominator: u64,
    throttling_slack: u64,
    ban_slack: u64,
    min_stake: U256,
    min_unstake_delay: U256,

    entities: Arc<RwLock<HashMap<Address, ReputationEntry>>>,
    whitelist: Arc<RwLock<HashSet<Address>>>,
    blacklist: Arc<RwLock<HashSet<Address>>>,
}

#[async_trait]
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

    async fn get(&mut self, address: &Address) -> anyhow::Result<ReputationEntry> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get(address) {
            return Ok(*entity);
        }

        let entity = ReputationEntry {
            address: *address,
            uo_seen: 0,
            uo_included: 0,
            status: ReputationStatus::OK,
        };

        entities.insert(*address, entity);

        Ok(entity)
    }

    async fn increment_seen(&mut self, address: &Address) -> anyhow::Result<()> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get_mut(address) {
            entity.uo_seen += 1;
            return Ok(());
        }

        Err(anyhow::anyhow!("Entity not found"))
    }

    async fn increment_included(&mut self, address: &Address) -> anyhow::Result<()> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get_mut(address) {
            entity.uo_included += 1;
            return Ok(());
        }

        Err(anyhow::anyhow!("Entity not found"))
    }

    fn update_hourly(&mut self) {
        let mut entities = self.entities.write();
        for (_, entity) in entities.iter_mut() {
            entity.uo_seen = entity.uo_seen * 23 / 24;
            entity.uo_included = entity.uo_included * 23 / 24;
        }
        entities.retain(|_, entity| entity.uo_seen > 0 || entity.uo_included > 0);
    }

    async fn add_whitelist(&mut self, address: &Address) -> anyhow::Result<()> {
        self.whitelist.write().insert(*address);
        Ok(())
    }

    async fn remove_whitelist(&mut self, address: &Address) -> anyhow::Result<bool> {
        Ok(self.whitelist.write().remove(address))
    }

    async fn is_whitelist(&self, address: &Address) -> anyhow::Result<bool> {
        Ok(self.whitelist.read().contains(address))
    }

    async fn add_blacklist(&mut self, address: &Address) -> anyhow::Result<()> {
        self.blacklist.write().insert(*address);
        Ok(())
    }

    async fn remove_blacklist(&mut self, address: &Address) -> anyhow::Result<bool> {
        Ok(self.blacklist.write().remove(address))
    }

    async fn is_blacklist(&self, address: &Address) -> anyhow::Result<bool> {
        Ok(self.blacklist.read().contains(address))
    }

    async fn get_status(&self, address: &Address) -> anyhow::Result<ReputationStatus> {
        if self.is_whitelist(address).await? {
            return Ok(ReputationStatus::OK);
        }

        if self.is_blacklist(address).await? {
            return Ok(ReputationStatus::BANNED);
        }

        let entities = self.entities.read();

        match entities.get(address) {
            Some(entity) => {
                let min_expected_included = entity.uo_seen / self.min_inclusion_denominator;
                if min_expected_included <= entity.uo_included + self.throttling_slack {
                    Ok(ReputationStatus::OK)
                } else if min_expected_included <= entity.uo_included + self.ban_slack {
                    Ok(ReputationStatus::THROTTLED)
                } else {
                    Ok(ReputationStatus::BANNED)
                }
            }
            _ => Ok(ReputationStatus::OK),
        }
    }

    async fn update_handle_ops_reverted(&mut self, address: &Address) -> anyhow::Result<()> {
        if let Ok(mut entity) = self.get(address).await {
            entity.uo_seen = 100;
            entity.uo_included = 0;
        }

        Ok(())
    }

    async fn verify_stake(&self, title: &str, stake_info: Option<StakeInfo>) -> anyhow::Result<()> {
        if let Some(stake_info) = stake_info {
            if self.is_whitelist(&stake_info.address).await? {
                return Ok(());
            }

            let entities = self.entities.read();

            if let Some(entity) = entities.get(&stake_info.address) {
                let error = if entity.status == ReputationStatus::BANNED {
                    BadReputationError::EntityBanned {
                        address: stake_info.address,
                        title: title.to_string(),
                    }
                } else if stake_info.stake < self.min_stake {
                    BadReputationError::StakeTooLow {
                        address: stake_info.address,
                        title: title.to_string(),
                        stake: stake_info.stake,
                        min_stake: self.min_stake,
                        min_unstake_delay: self.min_unstake_delay,
                    }
                } else if stake_info.unstake_delay < self.min_unstake_delay {
                    BadReputationError::UnstakeDelayTooLow {
                        address: stake_info.address,
                        title: title.to_string(),
                        unstake_delay: stake_info.unstake_delay,
                        min_stake: self.min_stake,
                        min_unstake_delay: self.min_unstake_delay,
                    }
                } else {
                    return Ok(());
                };

                return Err(anyhow::anyhow!(serde_json::to_string(
                    &ReputationError::from(error)
                )?));
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn set(&mut self, reputation_entries: Self::ReputationEntries) {
        let mut entities = self.entities.write();

        for reputation in reputation_entries {
            entities.insert(reputation.address, reputation);
        }
    }

    #[cfg(debug_assertions)]
    fn get_all(&self) -> Self::ReputationEntries {
        self.entities.read().values().cloned().collect()
    }

    #[cfg(debug_assertions)]
    fn clear(&mut self) {
        self.entities.write().clear();
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
                reputation.get(&address).await.unwrap(),
                ReputationEntry {
                    address,
                    uo_seen: 0,
                    uo_included: 0,
                    status: ReputationStatus::OK,
                }
            );
            addresses.push(address);
        }

        assert_eq!(reputation.add_whitelist(&addresses[2]).await.unwrap(), ());
        assert_eq!(reputation.add_blacklist(&addresses[1]).await.unwrap(), ());

        assert_eq!(reputation.is_whitelist(&addresses[2]).await.unwrap(), true);
        assert_eq!(reputation.is_whitelist(&addresses[1]).await.unwrap(), false);
        assert_eq!(reputation.is_blacklist(&addresses[1]).await.unwrap(), true);
        assert_eq!(reputation.is_blacklist(&addresses[2]).await.unwrap(), false);

        assert_eq!(
            reputation.remove_whitelist(&addresses[2]).await.unwrap(),
            true
        );
        assert_eq!(
            reputation.remove_whitelist(&addresses[1]).await.unwrap(),
            false
        );
        assert_eq!(
            reputation.remove_blacklist(&addresses[1]).await.unwrap(),
            true
        );
        assert_eq!(
            reputation.remove_blacklist(&addresses[2]).await.unwrap(),
            false
        );

        assert_eq!(reputation.add_whitelist(&addresses[2]).await.unwrap(), ());
        assert_eq!(reputation.add_blacklist(&addresses[1]).await.unwrap(), ());

        assert_eq!(
            reputation.get_status(&addresses[2]).await.unwrap(),
            ReputationStatus::OK
        );
        assert_eq!(
            reputation.get_status(&addresses[1]).await.unwrap(),
            ReputationStatus::BANNED
        );
        assert_eq!(
            reputation.get_status(&addresses[3]).await.unwrap(),
            ReputationStatus::OK
        );

        assert_eq!(reputation.increment_seen(&addresses[2]).await.unwrap(), ());
        assert_eq!(reputation.increment_seen(&addresses[2]).await.unwrap(), ());
        assert_eq!(reputation.increment_seen(&addresses[3]).await.unwrap(), ());
        assert_eq!(reputation.increment_seen(&addresses[3]).await.unwrap(), ());

        assert_eq!(
            reputation.increment_included(&addresses[2]).await.unwrap(),
            ()
        );
        assert_eq!(
            reputation.increment_included(&addresses[2]).await.unwrap(),
            ()
        );
        assert_eq!(
            reputation.increment_included(&addresses[3]).await.unwrap(),
            ()
        );

        assert_eq!(
            reputation
                .update_handle_ops_reverted(&addresses[3])
                .await
                .unwrap(),
            ()
        );

        for _ in 0..250 {
            assert_eq!(reputation.increment_seen(&addresses[3]).await.unwrap(), ());
        }
        assert_eq!(
            reputation.get_status(&addresses[3]).await.unwrap(),
            ReputationStatus::THROTTLED
        );

        for _ in 0..500 {
            assert_eq!(reputation.increment_seen(&addresses[3]).await.unwrap(), ());
        }
        assert_eq!(
            reputation.get_status(&addresses[3]).await.unwrap(),
            ReputationStatus::BANNED
        );
    }
}
