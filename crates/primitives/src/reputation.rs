use educe::Educe;
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

pub const MIN_INCLUSION_RATE_DENOMINATOR: u64 = 10;
pub const THROTTLING_SLACK: u64 = 10;
pub const BAN_SLACK: u64 = 50;

// If the paymaster is throttle, maximum amount in one bundle is 1.
pub const THROTTLED_MAX_INCLUDE: u64 = 1;

/// All possible reputation statuses
#[derive(Clone, Copy, Educe, PartialEq, Eq, Serialize, Deserialize)]
#[educe(Debug)]
pub enum ReputationStatus {
    OK,
    THROTTLED,
    BANNED,
}

/// Reputation entry for entities
#[derive(Clone, Copy, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct ReputationEntry {
    pub address: Address,
    pub uo_seen: u64,
    pub uo_included: u64,
    pub status: ReputationStatus,
}

/// Stake info
#[derive(Clone, Copy, Default, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct StakeInfo {
    pub address: Address,
    pub stake: U256,
    pub unstake_delay: U256, // seconds
}

impl StakeInfo {
    pub fn is_staked(&self) -> bool {
        self.stake > U256::zero() && self.unstake_delay > U256::zero()
    }
}

/// Error object for reputation
pub enum ReputationError {
    EntityBanned {
        address: Address,
        title: String,
    },
    StakeTooLow {
        address: Address,
        title: String,
        min_stake: U256,
        min_unstake_delay: U256,
    },
    UnstakeDelayTooLow {
        address: Address,
        title: String,
        min_stake: U256,
        min_unstake_delay: U256,
    },
    UnknownError {
        message: String,
    },
}
