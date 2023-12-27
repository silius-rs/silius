use super::tables::TABLES;
use reth_db::{
    database::{Database, DatabaseGAT},
    mdbx::{
        tx::{self, Tx},
        DatabaseFlags, Environment, EnvironmentFlags, EnvironmentKind, Geometry, Mode, PageSize,
        SyncMode, RO, RW,
    },
    Error as RethDatabaseError, TableType,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fs, path::PathBuf};
use thiserror::Error;

// Code adapted from: https://github.com/paradigmxyz/reth/blob/main/crates/storage/db/src/implementation/mdbx/mod.rs
#[derive(Debug)]
pub struct Env<E: EnvironmentKind> {
    /// Libmdbx-sys environment.
    pub inner: Environment<E>,
}

impl<'a, E: EnvironmentKind> DatabaseGAT<'a> for Env<E> {
    type TX = tx::Tx<'a, RO, E>;
    type TXMut = tx::Tx<'a, RW, E>;
}

impl<E: EnvironmentKind> Database for Env<E> {
    fn tx(&self) -> Result<<Self as DatabaseGAT<'_>>::TX, RethDatabaseError> {
        Ok(Tx::new(
            self.inner.begin_ro_txn().map_err(|e| RethDatabaseError::InitTransaction(e.into()))?,
        ))
    }

    fn tx_mut(&self) -> Result<<Self as DatabaseGAT<'_>>::TXMut, RethDatabaseError> {
        Ok(Tx::new(
            self.inner.begin_rw_txn().map_err(|e| RethDatabaseError::InitTransaction(e.into()))?,
        ))
    }
}

/// Database error
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Internal database error
    #[error(transparent)]
    Internal(RethDatabaseError),
    /// Databse not found
    #[error("Database not found")]
    NotFound,
}

impl From<RethDatabaseError> for DatabaseError {
    fn from(value: RethDatabaseError) -> Self {
        DatabaseError::Internal(value)
    }
}

impl Serialize for DatabaseError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{self:?}"))
    }
}

// TODO: implement correct deserialization
impl<'de> Deserialize<'de> for DatabaseError {
    fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Ok(DatabaseError::NotFound)
    }
}

fn default_page_size() -> usize {
    let os_page_size = page_size::get();

    // source: https://gitflic.ru/project/erthink/libmdbx/blob?file=mdbx.h#line-num-821
    let libmdbx_max_page_size = 0x10000;

    // May lead to errors if it's reduced further because of the potential size of the
    // data.
    let min_page_size = 4096;

    os_page_size.clamp(min_page_size, libmdbx_max_page_size)
}

impl<E: EnvironmentKind> Env<E> {
    /// Sets up the database environment
    pub fn open(path: PathBuf) -> eyre::Result<Self> {
        fs::create_dir_all(&path)?;

        let env = Environment::new()
            .set_max_dbs(TABLES.len())
            .set_geometry(Geometry {
                size: Some(0..(1024 * 1024 * 1024 * 1024 * 4)), // TODO: reevaluate (4 tb)
                growth_step: Some(1024 * 1024 * 256),           // TODO: reevaluate (256 mb)
                shrink_threshold: None,
                page_size: Some(PageSize::Set(default_page_size())),
            })
            .set_flags(EnvironmentFlags {
                mode: Mode::ReadWrite { sync_mode: SyncMode::Durable },
                no_rdahead: true, // TODO: reevaluate
                coalesce: true,
                ..Default::default()
            })
            .open(path.as_path())
            .map_err(|e| RethDatabaseError::DatabaseLocation(e.into()))?;

        Ok(Self { inner: env })
    }

    /// Creates all the defined tables, if necessary
    pub fn create_tables(&self) -> Result<(), RethDatabaseError> {
        let tx =
            self.inner.begin_rw_txn().map_err(|e| RethDatabaseError::InitTransaction(e.into()))?;

        for (table_type, table) in TABLES {
            let flags = match table_type {
                TableType::Table => DatabaseFlags::default(),
                TableType::DupSort => DatabaseFlags::DUP_SORT,
            };

            tx.create_db(Some(table), flags)
                .map_err(|e| RethDatabaseError::TableCreation(e.into()))?;
        }

        tx.commit().map_err(|e| RethDatabaseError::Commit(e.into()))?;

        Ok(())
    }
}
