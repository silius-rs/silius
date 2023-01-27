use async_trait::async_trait;
use educe::Educe;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::Reputation;

#[derive(Clone, Copy, Educe)]
#[educe(Debug)]
pub enum ReputationStatus {
    OK,
    THROTTLED,
    BANNED,
}

#[derive(Clone, Copy, Educe)]
#[educe(Debug)]
pub struct ReputationEntry {
    address: Address,
    uo_seen: u64,
    uo_included: u64,
    status: ReputationStatus,
}

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryReputation {
    min_inclusion_denominator: u64,
    throttling_slack: u64,
    ban_slack: u64,
    min_stake: U256,
    min_unstake_delay: u64,

    entities: Arc<RwLock<HashMap<Address, ReputationEntry>>>,
    whitelist: Arc<RwLock<HashSet<Address>>>,
    blacklist: Arc<RwLock<HashSet<Address>>>,
}

#[async_trait]
impl Reputation for MemoryReputation {
    fn new(
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: u64,
    ) -> Self {
        Self {
            min_inclusion_denominator,
            throttling_slack,
            ban_slack,
            min_stake,
            min_unstake_delay,
            entities: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    async fn get(&mut self, address: Address) -> anyhow::Result<ReputationEntry> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get(&address) {
            return Ok(*entity);
        }

        let entity = ReputationEntry {
            address,
            uo_seen: 0,
            uo_included: 0,
            status: ReputationStatus::OK,
        };

        entities.insert(address, entity);

        Ok(entity)
    }

    async fn increment_seen(&mut self, address: Address) -> anyhow::Result<()> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get_mut(&address) {
            entity.uo_seen += 1;
            return Ok(());
        }

        Err(anyhow::anyhow!("Entity not found"))
    }

    async fn increment_included(&mut self, address: Address) -> anyhow::Result<()> {
        let mut entities = self.entities.write();

        if let Some(entity) = entities.get_mut(&address) {
            entity.uo_included += 1;
            return Ok(());
        }

        Err(anyhow::anyhow!("Entity not found"))
    }

    async fn update_hourly(&mut self) -> anyhow::Result<()> {
        let mut entities = self.entities.write();
        for (_, entity) in entities.iter_mut() {
            entity.uo_seen = entity.uo_seen * 23 / 24;
            entity.uo_included = entity.uo_included * 23 / 24;
        }
        entities.retain(|_, entity| entity.uo_seen > 0 || entity.uo_included > 0);
        Ok(())
    }

    async fn add_whitelist(&mut self, address: Address) -> anyhow::Result<()> {
        self.whitelist.write().insert(address);
        Ok(())
    }

    async fn remove_whitelist(&mut self, address: Address) -> anyhow::Result<bool> {
        Ok(self.whitelist.write().remove(&address))
    }

    async fn is_whitelist(&self, address: Address) -> anyhow::Result<bool> {
        Ok(self.whitelist.read().contains(&address))
    }

    async fn add_blacklist(&mut self, address: Address) -> anyhow::Result<()> {
        self.blacklist.write().insert(address);
        Ok(())
    }

    async fn remove_blacklist(&mut self, address: Address) -> anyhow::Result<bool> {
        Ok(self.blacklist.write().remove(&address))
    }

    async fn is_blacklist(&self, address: Address) -> anyhow::Result<bool> {
        Ok(self.blacklist.read().contains(&address))
    }

    async fn get_status(&self, address: Address) -> anyhow::Result<ReputationStatus> {
        if self.is_whitelist(address).await? {
            return Ok(ReputationStatus::OK);
        }

        if self.is_blacklist(address).await? {
            return Ok(ReputationStatus::BANNED);
        }

        let entities = self.entities.read();

        match entities.get(&address) {
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

    // check stake

    // crash handle ops

    // debug: set reputation

    // debug: clear reputation
}

// tests
