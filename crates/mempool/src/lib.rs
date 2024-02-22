//! The UserOperation alternative mempool implementation according to the [ERC-4337 specifications](https://eips.ethereum.org/EIPS/eip-4337#Alternative%20Mempools).
#![allow(dead_code)]

mod builder;
#[cfg(feature = "mdbx")]
mod database;
pub mod error;
mod estimate;
mod memory;
mod mempool;
pub mod metrics;
mod reputation;
mod uopool;
mod utils;
pub mod validate;

pub use builder::UoPoolBuilder;
#[cfg(feature = "mdbx")]
pub use database::{
    init_env,
    tables::{
        CodeHashes, EntitiesReputation, UserOperations, UserOperationsByEntity,
        UserOperationsBySender,
    },
    DatabaseError, DatabaseTable, WriteMap,
};
pub use error::{
    InvalidMempoolUserOperationError, MempoolError, MempoolErrorKind, ReputationError, SanityError,
    SimulationError,
};
pub use mempool::{
    mempool_id, AddRemoveUserOp, AddRemoveUserOpHash, ClearOp, Mempool, MempoolId,
    UserOperationAct, UserOperationAddrAct, UserOperationAddrOp, UserOperationCodeHashAct,
    UserOperationCodeHashOp, UserOperationOp,
};
pub use reputation::{HashSetOp, Reputation, ReputationEntryOp};
pub use uopool::UoPool;
pub use utils::Overhead;
pub use validate::{SanityCheck, SimulationCheck, SimulationTraceCheck};
