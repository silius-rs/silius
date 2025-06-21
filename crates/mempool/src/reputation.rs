use silius_storage::db::SiliusDB;

use crate::error::MempoolError;

pub struct Reputation {
    pub db: SiliusDB,
}

impl Reputation {
    pub fn new(db: SiliusDB) -> Self {
        Self { db }
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        Ok(())
    }
}
