use super::utils::{
    WrapAddress, WrapCodeHashVec, WrapReputationEntry, WrapU64, WrapUserOperationHash,
    WrapUserOperationHashSet, WrapUserOperationSigned,
};
use reth_db::{table, TableType};

table!(
    /// Stores the user operations
    ( UserOperations ) WrapUserOperationHash | WrapUserOperationSigned
);

table!(
    /// Stores the hashes of user operations by sender
    /// Benefit for merklization is that hashed addresses/keys are sorted.
    ( UserOperationsBySender ) WrapAddress | WrapUserOperationHashSet
);

table!(
    /// Stores the hashes of user operations by involved entities
    ( UserOperationsByEntity ) WrapAddress | WrapUserOperationHashSet
);

table!(
    /// Stores the code hashes (needed during simulation)
    ( CodeHashes ) WrapUserOperationHash | WrapCodeHashVec
);

table!(
    /// Stores the reputation of entities
    ( EntitiesReputation ) WrapAddress | WrapReputationEntry
);

table!(
    /// Stores timestamps of user operations (when they were received)
    /// UNIX timestamps are rounded by 10 seconds.
    ( Timestamps ) WrapUserOperationHash | WrapU64
);

table!(
    /// Stores the hashes of user operations by timestamp
    ( UserOperationsByTimestamp ) WrapU64 | WrapUserOperationHashSet
);

/// Tables that should be present inside database
pub const TABLES: [(TableType, &str); 7] = [
    (TableType::Table, UserOperations::const_name()),
    (TableType::Table, UserOperationsBySender::const_name()),
    (TableType::Table, UserOperationsByEntity::const_name()),
    (TableType::Table, CodeHashes::const_name()),
    (TableType::Table, EntitiesReputation::const_name()),
    (TableType::Table, Timestamps::const_name()),
    (TableType::Table, UserOperationsByTimestamp::const_name()),
];
