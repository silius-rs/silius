use educe::Educe;
use ethers::{
    abi::AbiEncode,
    types::{Address, U256},
};
use jsonrpsee::types::{error::ErrorCode, ErrorObject};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const MIN_INCLUSION_RATE_DENOMINATOR: u64 = 10;
pub const THROTTLING_SLACK: u64 = 10;
pub const BAN_SLACK: u64 = 50;
const ENTITY_BANNED_ERROR_CODE: i32 = -32504;
const STAKE_TOO_LOW_ERROR_CODE: i32 = -32505;

pub type ReputationError = ErrorObject<'static>;

#[derive(Clone, Copy, Educe, PartialEq, Eq, Serialize, Deserialize)]
#[educe(Debug)]
pub enum ReputationStatus {
    OK,
    THROTTLED,
    BANNED,
}

#[derive(Clone, Copy, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct ReputationEntry {
    pub address: Address,
    pub uo_seen: u64,
    pub uo_included: u64,
    pub status: ReputationStatus,
}

#[derive(Clone, Copy, Default, Educe, Eq, PartialEq, Serialize, Deserialize)]
#[educe(Debug)]
pub struct StakeInfo {
    pub address: Address,
    pub stake: U256,
    pub unstake_delay: U256, // seconds
}

pub enum BadReputationError {
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
    Internal(anyhow::Error),
}

impl From<BadReputationError> for ReputationError {
    fn from(error: BadReputationError) -> Self {
        match error {
            BadReputationError::EntityBanned { address, title } => ReputationError::owned(
                ENTITY_BANNED_ERROR_CODE,
                format!("{title} with address {address} is banned",),
                Some(json!({
                    title: address.to_string(),
                })),
            ),
            BadReputationError::StakeTooLow {
                address,
                title,
                min_stake,
                min_unstake_delay,
            } => ReputationError::owned(
                STAKE_TOO_LOW_ERROR_CODE,
                format!(
                    "{title} with address {address} stake is lower than {min_stake}",
                ),
                Some(json!({
                    title: address.to_string(),
                    "minimumStake": AbiEncode::encode_hex(min_stake),
                    "minimumUnstakeDelay": AbiEncode::encode_hex(min_unstake_delay),
                })),
            ),
            BadReputationError::UnstakeDelayTooLow {
                address,
                title,
                min_stake,
                min_unstake_delay,
            } => ReputationError::owned(
                STAKE_TOO_LOW_ERROR_CODE,
                format!(
                    "{title} with address {address} unstake delay is lower than {min_unstake_delay}",
                ),
                Some(json!({
                    title: address.to_string(),
                    "minimumStake": AbiEncode::encode_hex(min_stake),
                    "minimumUnstakeDelay": AbiEncode::encode_hex(min_unstake_delay),
                })),
            ),
            BadReputationError::Internal(_) => ReputationError::from(ErrorCode::InternalError),
        }
    }
}
