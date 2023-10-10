use super::utils::{as_checksum_addr, as_hex_string, as_u64};
use educe::Educe;
use ethers::{
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, U256},
};
use serde::{Deserialize, Serialize};

pub type ReputationStatus = u64;

/// All possible reputation statuses
#[derive(Default, Clone, Educe, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[educe(Debug)]
#[serde(rename_all = "lowercase")]
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
    #[serde(rename = "opsSeen", serialize_with = "as_hex_string")]
    pub uo_seen: u64,
    #[serde(rename = "opsIncluded", serialize_with = "as_hex_string")]
    pub uo_included: u64,
    #[serde(default, serialize_with = "as_hex_string")]
    pub status: ReputationStatus,
}

/// Stake info
#[derive(Clone, Copy, Default, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct StakeInfo {
    #[serde(rename = "addr", serialize_with = "as_checksum_addr")]
    pub address: Address,
    #[serde(serialize_with = "as_u64")]
    pub stake: U256,
    #[serde(rename = "unstakeDelaySec", serialize_with = "as_u64")]
    pub unstake_delay: U256, // seconds
}

impl StakeInfo {
    pub fn is_staked(&self) -> bool {
        self.stake > U256::zero() && self.unstake_delay > U256::zero()
    }
}

/// Stake info response for RPC
#[derive(Clone, Copy, Default, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct StakeInfoResponse {
    #[serde(rename = "stakeInfo")]
    pub stake_info: StakeInfo,
    #[serde(rename = "isStaked")]
    pub is_staked: bool,
}

/// Error object for reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationError {
    EntityBanned {
        address: Address,
        entity: String,
    },
    ThrottledLimit {
        address: Address,
        entity: String,
    },
    UnstakedEntityVerification {
        address: Address,
        entity: String,
        message: String,
    },
    StakeTooLow {
        address: Address,
        entity: String,
        min_stake: U256,
        min_unstake_delay: U256,
    },
    UnstakeDelayTooLow {
        address: Address,
        entity: String,
        min_stake: U256,
        min_unstake_delay: U256,
    },
    UnknownError {
        message: String,
    },
}
