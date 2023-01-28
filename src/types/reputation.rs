use educe::Educe;
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Educe, PartialEq, Eq, Serialize, Deserialize)]
#[educe(Debug)]
pub enum ReputationStatus {
    OK,
    THROTTLED,
    BANNED,
}

#[derive(Clone, Copy, Educe, Serialize, Deserialize)]
#[educe(Debug)]
pub struct ReputationEntry {
    pub address: Address,
    pub uo_seen: u64,
    pub uo_included: u64,
    pub status: ReputationStatus,
}

#[derive(Clone, Copy, Educe, Serialize, Deserialize)]
#[educe(Debug)]
pub struct StakeInfo {
    pub address: Address,
    pub stake: U256,
    pub unstake_delay: U256, // seconds
}
