use educe::Educe;
use ethers::{
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, U256},
};
use serde::{Deserialize, Serialize};

pub type ReputationStatus = u8;

pub const MIN_INCLUSION_RATE_DENOMINATOR: u64 = 10;
pub const THROTTLING_SLACK: u64 = 10;
pub const BAN_SLACK: u64 = 50;

// If the paymaster is throttle, maximum amount in one bundle is 1.
pub const THROTTLED_MAX_INCLUDE: u64 = 1;

/// All possible reputation statuses
#[derive(Default, Clone, Educe, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[educe(Debug)]
pub enum Status {
    #[default]
    OK,
    THROTTLED,
    BANNED,
}

impl From<Status> for ReputationStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::OK => 0,
            Status::THROTTLED => 1,
            Status::BANNED => 2,
        }
    }
}

impl From<ReputationStatus> for Status {
    fn from(status: ReputationStatus) -> Self {
        match status {
            0 => Status::OK,
            1 => Status::THROTTLED,
            2 => Status::BANNED,
            _ => Status::OK,
        }
    }
}

/// Reputation entry for entities
#[derive(
    Default,
    Clone,
    Educe,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EthAbiCodec,
    EthAbiType,
)]
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
