use super::utils::{
    WrapAddress, WrapCodeHashVec, WrapReputationEntry, WrapUserOpSet, WrapUserOperationHash,
    WrapUserOperationSigned,
};
use reth_db::{table, TableType};

table!(
    /// Stores the user operations
    ( UserOperations ) WrapUserOperationHash | WrapUserOperationSigned
);

table!(
    /// Stores the hashes of user operations by sender
    /// Benefit for merklization is that hashed addresses/keys are sorted.
    ( UserOperationsBySender ) WrapAddress | WrapUserOpSet
);

table!(
    /// Stores the hashes of user operations by involved entities
    ( UserOperationsByEntity ) WrapAddress | WrapUserOpSet
);

table!(
    /// Stores the code hashes (needed during simulation)
    ( CodeHashes ) WrapUserOperationHash | WrapCodeHashVec
);

table!(
    /// Stores the reputation of entities
    ( EntitiesReputation ) WrapAddress | WrapReputationEntry
);

/// Tables that should be present inside database
pub const TABLES: [(TableType, &str); 5] = [
    (TableType::Table, UserOperations::const_name()),
    (TableType::Table, UserOperationsBySender::const_name()),
    (TableType::Table, UserOperationsByEntity::const_name()),
    (TableType::Table, CodeHashes::const_name()),
    (TableType::Table, EntitiesReputation::const_name()),
];
