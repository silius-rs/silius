use super::utils::{
    WrapAddress, WrapCodeHashVec, WrapReputationEntry, WrapUserOpSet, WrapUserOperationHash,
    WrapUserOperationSigned,
};
use reth_db::tables;

tables! {
    /// Stores the user operations
    table UserOperations<Key = WrapUserOperationHash, Value = WrapUserOperationSigned>;

    /// Stores the hashes of user operations by sender
    /// Benefit for merklization is that hashed addresses/keys are sorted.
    table UserOperationsBySender<Key = WrapAddress, Value = WrapUserOpSet>;

    /// Stores the hashes of user operations by involved entities
    table UserOperationsByEntity<Key = WrapAddress, Value = WrapUserOpSet>;

    /// Stores the code hashes (needed during simulation)
    table CodeHashes<Key = WrapUserOperationHash, Value = WrapCodeHashVec>;

    /// Stores the reputation of entities
    table EntitiesReputation<Key = WrapAddress, Value = WrapReputationEntry>;
}
