use async_trait::async_trait;
use educe::Educe;
use ethers::{
    abi::AbiEncode,
    types::{Address, U256},
};
use parking_lot::RwLock;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::types::reputation::{ReputationEntry, ReputationStatus, StakeInfo};

use super::{Reputation, ReputationError, ENTITY_BANNED_ERROR_CODE, STAKE_TOO_LOW_ERROR_CODE};

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
                    ReputationError::owned(
                        ENTITY_BANNED_ERROR_CODE,
                        format!("{title} with address {} is banned", stake_info.address),
                        Some(json!({
                            title: stake_info.address.to_string(),
                        })),
                    )
                } else if stake_info.stake < self.min_stake {
                    ReputationError::owned(
                        STAKE_TOO_LOW_ERROR_CODE,
                        format!(
                            "{title} with address {} stake {} is lower than {}",
                            stake_info.address, stake_info.stake, self.min_stake
                        ),
                        Some(json!({
                            title: stake_info.address.to_string(),
                            "minimumStake": AbiEncode::encode_hex(self.min_stake),
                            "minimumUnstakeDelay": AbiEncode::encode_hex(self.min_unstake_delay),
                        })),
                    )
                } else if stake_info.unstake_delay < self.min_unstake_delay {
                    ReputationError::owned(
                        STAKE_TOO_LOW_ERROR_CODE,
                        format!(
                            "{title} with address {} unstake delay {} is lower than {}",
                            stake_info.address, stake_info.unstake_delay, self.min_unstake_delay
                        ),
                        Some(json!({
                            title: stake_info.address.to_string(),
                            "minimumStake": AbiEncode::encode_hex(self.min_stake),
                            "minimumUnstakeDelay": AbiEncode::encode_hex(self.min_unstake_delay),
                        })),
                    )
                } else {
                    return Ok(());
                };

                return Err(anyhow::anyhow!(serde_json::to_string(&error)?));
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

// tests
