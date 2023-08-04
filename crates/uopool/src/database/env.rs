use reth_db::{
    database::{Database, DatabaseGAT},
    mdbx::{
        tx::{self, Tx},
        DatabaseFlags, Environment, EnvironmentFlags, EnvironmentKind, Geometry, Mode, PageSize,
        SyncMode, RO, RW,
    },
    Error, TableType,
};
use std::{fmt::Display, path::PathBuf};

use super::tables::TABLES;

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
    fn tx(&self) -> Result<<Self as DatabaseGAT<'_>>::TX, Error> {
        Ok(Tx::new(
            self.inner
                .begin_ro_txn()
                .map_err(|e| Error::InitTransaction(e.into()))?,
        ))
    }

    fn tx_mut(&self) -> Result<<Self as DatabaseGAT<'_>>::TXMut, Error> {
        Ok(Tx::new(
            self.inner
                .begin_rw_txn()
                .map_err(|e| Error::InitTransaction(e.into()))?,
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DBError {
    DBInternalError(Error),
    NotFound,
}

impl From<Error> for DBError {
    fn from(value: Error) -> Self {
        DBError::DBInternalError(value)
    }
}

impl Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
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
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        let env = Environment::new()
            .set_max_dbs(TABLES.len())
            .set_geometry(Geometry {
                size: Some(0..(1024 * 1024 * 1024 * 1024 * 4)), // TODO: reevaluate (4 tb)
                growth_step: Some(1024 * 1024 * 256),           // TODO: reevaluate (256 mb)
                shrink_threshold: None,
                page_size: Some(PageSize::Set(default_page_size())),
            })
            .set_flags(EnvironmentFlags {
                mode: Mode::ReadWrite {
                    sync_mode: SyncMode::Durable,
                },
                no_rdahead: true, // TODO: reevaluate
                coalesce: true,
                ..Default::default()
            })
            .open(path.as_path())
            .map_err(|e| Error::DatabaseLocation(e.into()))?;

        Ok(Self { inner: env })
    }

    /// Creates all the defined tables, if necessary
    pub fn create_tables(&self) -> Result<(), Error> {
        let tx = self
            .inner
            .begin_rw_txn()
            .map_err(|e| Error::InitTransaction(e.into()))?;

        for (table_type, table) in TABLES {
            let flags = match table_type {
                TableType::Table => DatabaseFlags::default(),
                TableType::DupSort => DatabaseFlags::DUP_SORT,
            };

            tx.create_db(Some(table), flags)
                .map_err(|e| Error::TableCreation(e.into()))?;
        }

        tx.commit().map_err(|e| Error::Commit(e.into()))?;

        Ok(())
    }
}
