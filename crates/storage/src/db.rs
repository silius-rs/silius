use std::{fs, io, path::PathBuf, sync::Arc};

use redb::{Builder, Database};
use tracing::info;

use crate::{
    error::DatabaseError,
    tables::{
        entity::{ENTITY_USER_OPERATION_MULTIMAP_TABLE, EntityUserOperationMultimapTable},
        sender::{SENDER_USER_OPERATION_MULTIMAP_TABLE, SenderUserOperationMultimapTable},
        user_operation::{USER_OPERATION_TABLE, UserOperationTable},
    },
};

pub const REDB_CACHE_SIZE: usize = 1_024 * 1_024 * 1_024;
pub const REDB_FILE: &str = "silius.redb";

#[derive(Clone, Debug)]
pub struct SiliusDB {
    pub db: Arc<Database>,
}

impl SiliusDB {
    pub fn new(silius_dir: PathBuf) -> Result<Self, DatabaseError> {
        let silius_file = silius_dir.join(REDB_FILE);

        let db = Builder::new()
            .set_cache_size(REDB_CACHE_SIZE)
            .create(&silius_file)?;

        let write_txn = db.begin_write()?;
        write_txn.open_table(USER_OPERATION_TABLE)?;
        write_txn.open_multimap_table(ENTITY_USER_OPERATION_MULTIMAP_TABLE)?;
        write_txn.open_multimap_table(SENDER_USER_OPERATION_MULTIMAP_TABLE)?;
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn user_operation_provider(&self) -> UserOperationTable {
        UserOperationTable {
            db: self.db.clone(),
        }
    }

    pub fn entity_user_operation_multimap_provider(&self) -> EntityUserOperationMultimapTable {
        EntityUserOperationMultimapTable {
            db: self.db.clone(),
        }
    }

    pub fn sender_user_operation_multimap_provider(&self) -> SenderUserOperationMultimapTable {
        SenderUserOperationMultimapTable {
            db: self.db.clone(),
        }
    }
}

pub fn reset_db(db_path: PathBuf) -> anyhow::Result<()> {
    if fs::read_dir(&db_path)?.next().is_none() {
        info!("Data directory at {db_path:?} is already empty.");
        return Ok(());
    }

    info!(
        "Are you sure you want to clear the contents of the data directory at {db_path:?}? (y/n):"
    );
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        for entry in fs::read_dir(&db_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        info!("Database contents cleared successfully.");
    } else {
        info!("Operation canceled by user.");
    }
    Ok(())
}
