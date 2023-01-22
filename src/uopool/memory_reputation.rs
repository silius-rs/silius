use async_trait::async_trait;
use educe::Educe;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::Reputation;

#[derive(Educe)]
#[educe(Debug)]
pub enum ReputationStatus {
    OK,
    THROTTLED,
    BANNED,
}

#[derive(Educe)]
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

    entites: Arc<RwLock<HashMap<Address, ReputationEntry>>>,
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
            entites: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    // hourly cron

    // add whitelist

    // add blacklist

    // init entity

    // increase seen

    // increase included

    // get status

    // check stake

    // crash handle ops

    // debug: set reputation

    // debug: clear reputation
}
