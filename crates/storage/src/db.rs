use std::{path::PathBuf, sync::Arc};

use redb::{Builder, Database};

use crate::error::DatabaseError;

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
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }
}
