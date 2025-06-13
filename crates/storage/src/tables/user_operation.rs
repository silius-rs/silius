use std::sync::Arc;

use alloy_primitives::B256;
use redb::{Database, Durability, TableDefinition};
use silius_primitives::user_operation::UserOperation;

use crate::error::DatabaseError;

use super::{
    Bincode, MultimapTable, Table, entity::EntityUserOperationMultimapTable,
    sender::SenderUserOperationMultimapTable,
};

pub const USER_OPERATION_TABLE: TableDefinition<Bincode<B256>, Bincode<UserOperation>> =
    TableDefinition::new("user_operation");

pub struct UserOperationTable {
    pub db: Arc<Database>,
}

impl Table for UserOperationTable {
    type Key = B256;

    type Value = UserOperation;

    fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, DatabaseError> {
        let read_txn = self.db.begin_read()?;

        let table = read_txn.open_table(USER_OPERATION_TABLE)?;
        let result = table.get(key)?;
        Ok(result.map(|res| res.value()))
    }

    fn insert(&self, key: Self::Key, value: Self::Value) -> Result<(), DatabaseError> {
        // insert entry to sender_user_operation table
        let sender_user_operation_table = SenderUserOperationMultimapTable {
            db: self.db.clone(),
        };
        sender_user_operation_table.insert(value.sender, key)?;

        // insert entry to entity_user_operation table
        let entity_user_operation_table = EntityUserOperationMultimapTable {
            db: self.db.clone(),
        };
        entity_user_operation_table.insert(value.sender, key)?;
        if let Some(factory) = value.factory {
            entity_user_operation_table.insert(factory, key)?;
        }
        if let Some(paymaster) = value.paymaster {
            entity_user_operation_table.insert(paymaster, key)?;
        }

        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(USER_OPERATION_TABLE)?;
        table.insert(key, value)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn remove(&self, key: Self::Key) -> Result<(), DatabaseError> {
        let Some(value) = self.get(key)? else {
            return Ok(());
        };

        // remove entry from sender_user_operation table
        let sender_user_operation_table = SenderUserOperationMultimapTable {
            db: self.db.clone(),
        };
        sender_user_operation_table.remove(value.sender, key)?;

        // remove entry from entity_user_operation table
        let entity_user_operation_table = EntityUserOperationMultimapTable {
            db: self.db.clone(),
        };
        entity_user_operation_table.remove(value.sender, key)?;
        if let Some(factory) = value.factory {
            entity_user_operation_table.remove(factory, key)?;
        }
        if let Some(paymaster) = value.paymaster {
            entity_user_operation_table.remove(paymaster, key)?;
        }

        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(USER_OPERATION_TABLE)?;
        table.remove(key)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn clear(&self) -> Result<(), DatabaseError> {
        let sender_user_operation_table = SenderUserOperationMultimapTable {
            db: self.db.clone(),
        };
        let entity_user_operation_table = EntityUserOperationMultimapTable {
            db: self.db.clone(),
        };

        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(USER_OPERATION_TABLE)?;
        table.extract_if(|_, value| {
            let _ = sender_user_operation_table.remove_all(value.sender);
            let _ = entity_user_operation_table.remove_all(value.sender);
            if let Some(factory) = value.factory {
                let _ = entity_user_operation_table.remove_all(factory);
            }
            if let Some(paymaster) = value.paymaster {
                let _ = entity_user_operation_table.remove_all(paymaster);
            }
            true
        })?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }
}
