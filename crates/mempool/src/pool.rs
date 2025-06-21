use silius_storage::db::SiliusDB;
use silius_storage::tables::Table;

use crate::error::MempoolError;

pub struct UserOperationPool {
    pub db: SiliusDB,
}

impl UserOperationPool {
    pub fn new(db: SiliusDB) -> Self {
        Self { db }
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.db.user_operation_provider().clear()?;
        Ok(())
    }
}
