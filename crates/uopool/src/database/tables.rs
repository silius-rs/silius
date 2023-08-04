use super::utils::{
    WrapAddress, WrapCodeHash, WrapReputationEntry, WrapUserOperation, WrapUserOperationHash,
};
use reth_db::{dupsort, table, table::DupSort, TableType};

table!(
    /// Stores the user operations
    ( UserOperations ) WrapUserOperationHash | WrapUserOperation
);

table!(
    /// Stores the user operations by sender
    /// Benefit for merklization is that hashed addresses/keys are sorted.
    ( UserOperationsBySender ) WrapAddress | WrapUserOperation
);

dupsort!(
    /// Stores the code hashes (needed during simulation)
    ( CodeHashes ) WrapUserOperationHash | [WrapAddress] WrapCodeHash
);

table!(
    /// Stores the reputation of entities
    ( EntitiesReputation ) WrapAddress | WrapReputationEntry
);

/// Tables that should be present inside database
pub const TABLES: [(TableType, &str); 4] = [
    (TableType::Table, UserOperations::const_name()),
    (TableType::DupSort, UserOperationsBySender::const_name()),
    (TableType::DupSort, CodeHashes::const_name()),
    (TableType::Table, EntitiesReputation::const_name()),
];

impl DupSort for UserOperationsBySender {
    type SubKey = WrapAddress;
}
