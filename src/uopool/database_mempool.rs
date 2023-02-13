use std::path::PathBuf;

use super::Mempool;
use crate::types::{
    user_operation::{UserOperation, UserOperationHash},
    utils::WrapAddress,
};
use ethers::types::{Address, U256};
use reth_db::{
    cursor::{DbCursorRO, DbDupCursorRO},
    database::{Database, DatabaseGAT},
    mdbx::{
        tx::{self, Tx},
        DatabaseFlags, Environment, EnvironmentFlags, EnvironmentKind, Geometry, Mode, PageSize,
        SyncMode, RO, RW,
    },
    table,
    table::DupSort,
    transaction::{DbTx, DbTxMut},
    Error, TableType,
};

table!(
    /// UserOperation DB
    ( UserOperationDB ) UserOperationHash | UserOperation
    // Sender to UserOperationHash
);

table!(
    /// Benefit for merklization is that hashed addresses/keys are sorted.
    ( SenderUserOperationDB ) WrapAddress | UserOperation
);

/// Default tables that should be present inside database.
pub const TABLES: [(TableType, &str); 2] = [
    (TableType::Table, UserOperationDB::const_name()),
    (TableType::DupSort, SenderUserOperationDB::const_name()),
];

impl DupSort for SenderUserOperationDB {
    type SubKey = WrapAddress;
}

#[derive(Debug)]
pub struct Env<E: EnvironmentKind> {
    /// Libmdbx-sys environment.
    pub inner: Environment<E>,
}

#[derive(Debug)]
pub struct DatabaseMempool<E: EnvironmentKind> {
    _path: PathBuf,
    env: Env<E>,
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

impl<E: EnvironmentKind> Mempool for DatabaseMempool<E> {
    type UserOperations = Vec<UserOperation>;
    type Error = DBError;
    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, DBError> {
        let hash = user_operation.hash(entry_point, chain_id);
        let tx = self.env.tx_mut()?;
        tx.put::<UserOperationDB>(hash, user_operation.clone())?;
        tx.put::<SenderUserOperationDB>(user_operation.sender.into(), user_operation)?;
        tx.commit()?;
        Ok(hash)
    }

    fn get(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, DBError> {
        let tx = self.env.tx()?;
        let res = tx.get::<UserOperationDB>(*user_operation_hash)?;
        tx.commit()?;
        Ok(res)
    }

    fn get_all_by_sender(&self, sender: &Address) -> Self::UserOperations {
        let wrap_sender: WrapAddress = (*sender).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<SenderUserOperationDB>()?;
                let res = cursor
                    .walk_dup(wrap_sender.clone(), Address::default().into())?
                    .map(|a| a.map(|(_, v)| v))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    fn remove(&mut self, user_operation_hash: &UserOperationHash) -> Result<(), DBError> {
        let tx = self.env.tx_mut()?;
        if let Some(user_op) = tx.get::<UserOperationDB>(*user_operation_hash)? {
            tx.delete::<UserOperationDB>(*user_operation_hash, None)?;
            tx.delete::<SenderUserOperationDB>(user_op.sender.into(), Some(user_op))?;
            tx.commit()?;
            Ok(())
        } else {
            Err(DBError::NotFound)
        }
    }

    #[cfg(debug_assertions)]
    fn get_all(&self) -> Self::UserOperations {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<UserOperationDB>()?;
                let res = c
                    .walk(UserOperationHash::default())?
                    .map(|a| a.map(|(_, v)| v))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    #[cfg(debug_assertions)]
    fn clear(&mut self) {
        self.env
            .tx_mut()
            .and_then(|tx| {
                tx.clear::<UserOperationDB>()?;
                tx.clear::<SenderUserOperationDB>()?;
                tx.commit()
            })
            .expect("Clear database failed");
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

impl<E: EnvironmentKind> DatabaseMempool<E> {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
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

        Ok(Self {
            _path: path,
            env: Env { inner: env },
        })
    }

    /// Creates all the defined tables, if necessary.
    pub fn create_tables(&self) -> Result<(), Error> {
        let tx = self
            .env
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

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::{H256, U256};
    use reth_db::mdbx::NoWriteMap;
    use tempdir::TempDir;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let entry_point = Address::random();
        let chain_id = U256::from(5);
        let senders = vec![Address::random(), Address::random(), Address::random()];

        let dir = TempDir::new("test-userop-db").unwrap();
        let mut mempool: DatabaseMempool<NoWriteMap> = DatabaseMempool::new(dir.into_path()).unwrap();
        mempool
            .create_tables()
            .expect("Create mdbx database tables failed");
        let mut user_operation: UserOperation;
        let mut user_operation_hash: UserOperationHash = Default::default();

        for i in 0..2 {
            user_operation = UserOperation {
                sender: senders[0],
                nonce: U256::from(i),
                ..UserOperation::random()
            };
            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );

            user_operation = UserOperation {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );
        }

        for i in 0..3 {
            user_operation = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );
        }

        assert_eq!(mempool.get_all().len(), 7);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[1]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 3);

        assert_eq!(mempool.remove(&user_operation_hash).unwrap(), ());
        assert_eq!(
            mempool.remove(&H256::random().into()).unwrap_err(),
            DBError::NotFound
        );

        assert_eq!(mempool.get_all().len(), 6);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 2);

        assert_eq!(mempool.clear(), ());

        assert_eq!(mempool.get_all().len(), 0);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 0);
    }
}
