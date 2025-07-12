use std::sync::Arc;

use alloy_primitives::Address;
use redb::{Database, Durability, TableDefinition};
use silius_primitives::reputation::ReputationEntry;

use super::Bincode;
use crate::{error::DatabaseError, tables::Table};

pub const REPUTATION_TABLE: TableDefinition<Bincode<Address>, Bincode<ReputationEntry>> =
    TableDefinition::new("reputation");

pub struct ReputationTable {
    pub db: Arc<Database>,
}

impl Table for ReputationTable {
    type Key = Address;

    type Value = ReputationEntry;

    fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, DatabaseError> {
        let read_txn = self.db.begin_read()?;

        let table = read_txn.open_table(REPUTATION_TABLE)?;
        let result = table.get(key)?;
        Ok(result.map(|res| res.value()))
    }

    fn insert(&self, key: Self::Key, value: Self::Value) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(REPUTATION_TABLE)?;
        table.insert(key, value)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn remove(&self, key: Self::Key) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(REPUTATION_TABLE)?;
        table.remove(key)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }

    fn clear(&self) -> Result<(), DatabaseError> {
        let mut write_txn = self.db.begin_write()?;
        write_txn.set_durability(Durability::Immediate);
        let mut table = write_txn.open_table(REPUTATION_TABLE)?;
        table.extract_if(|_, _| true)?;
        drop(table);
        write_txn.commit()?;
        Ok(())
    }
}

impl ReputationTable {
    pub fn increment_ops_seen(&self, address: Address) -> Result<(), DatabaseError> {
        let entry = self.get(address)?.unwrap_or_default();
        self.insert(
            address,
            ReputationEntry {
                ops_seen: entry.ops_seen + 1,
                ops_included: entry.ops_included,
            },
        )
    }

    pub fn increment_ops_included(&self, address: Address) -> Result<(), DatabaseError> {
        let entry = self.get(address)?.unwrap_or_default();
        self.insert(
            address,
            ReputationEntry {
                ops_seen: entry.ops_seen,
                ops_included: entry.ops_included + 1,
            },
        )
    }
}
