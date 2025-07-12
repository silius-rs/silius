use alloy_primitives::{
    U256,
    aliases::{U48, U112},
};
use serde::{Deserialize, Serialize};

pub const BAN_SLACK: u64 = 50;
pub const MIN_INCLUSION_DENOMINATOR: u64 = 10;
pub const THROTTLING_SLACK: u64 = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReputationEntry {
    pub ops_seen: u64,
    pub ops_included: u64,
}

impl ReputationEntry {
    pub fn max_seen(&self) -> u64 {
        self.ops_seen / MIN_INCLUSION_DENOMINATOR
    }

    pub fn state(&self) -> ReputationState {
        let max_seen = self.max_seen();

        if max_seen > self.ops_included + BAN_SLACK {
            ReputationState::Banned
        } else if max_seen > self.ops_included + THROTTLING_SLACK {
            ReputationState::Throttled
        } else {
            ReputationState::Ok
        }
    }

    pub fn inclusion_rate(&self) -> f64 {
        self.ops_included as f64 / self.ops_seen as f64
    }
}

pub enum ReputationState {
    Ok,
    Banned,
    Throttled,
}

pub struct DepositInfo {
    pub deposit: U256,
    pub staked: bool,
    pub stake: U112,
    pub unstake_delay_sec: u32,
    pub withdraw_time: U48,
}
