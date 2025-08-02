use alloy_primitives::{Address, B256};
use silius_chain::error::ChainError;
use silius_primitives::entity::Entity;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TracingCheckError {
    #[error("account uses banned opcode: {0}")]
    BannedOpcode(String),

    #[error("{0} accesses undeployed contract address {1} with opcode {2}")]
    UndeployedContractAccess(Entity, Address, String),

    #[error("unstaked {0} accessed {1} slot {2}")]
    UnstakedEntitySlotAccess(Entity, Address, B256),

    #[error("{0} has forbidden {1} {2}{3} slot {4}")]
    ForbiddenSlotAccess(Entity, String, String, Address, B256),

    #[error("{0}")]
    Check(String),

    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for TracingCheckError {
    fn from(err: anyhow::Error) -> Self {
        TracingCheckError::Other(err.to_string())
    }
}
