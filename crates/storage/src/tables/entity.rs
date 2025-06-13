use std::{collections::HashSet, sync::Arc};

use alloy_primitives::{Address, B256};
use redb::{Database, Durability, MultimapTableDefinition};

use crate::error::DatabaseError;

use super::{Bincode, MultimapTable};

pub const ENTITY_USER_OPERATION_MULTIMAP_TABLE: MultimapTableDefinition<
    Bincode<Address>,
    Bincode<B256>,
> = MultimapTableDefinition::new("entity_user_operation");

pub struct EntityUserOperationMultimapTable {
    pub db: Arc<Database>,
}

impl MultimapTable for EntityUserOperationMultimapTable {
    type Key = Address;

    type GetValue = HashSet<B256>;

    type InsertValue = B256;

    type RemoveValue = B256;

    fn get(&self, key: Self::Key) -> Result<Option<Self::GetValue>, DatabaseError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_multimap_table(ENTITY_USER_OPERATION_MULTIMAP_TABLE)?;
        let result = table.get(key)?;
        let mut values = HashSet::new();
        for value in result {
            values.insert(value?.value());
        }
        Ok(Some(values))
    }

    fn insert(&self, key: Self::Key, value: Self::InsertValue) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_multimap_table(ENTITY_USER_OPERATION_MULTIMAP_TABLE)?;
        table.insert(key, value)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn remove(&self, key: Self::Key, value: Self::RemoveValue) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_multimap_table(ENTITY_USER_OPERATION_MULTIMAP_TABLE)?;
        table.remove(key, value)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn remove_all(&self, key: Self::Key) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_multimap_table(ENTITY_USER_OPERATION_MULTIMAP_TABLE)?;
        table.remove_all(key)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }
}
