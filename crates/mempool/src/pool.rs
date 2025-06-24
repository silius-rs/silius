use silius_primitives::user_operation::UserOperation;
use silius_storage::{db::SiliusDB, tables::Table};
use silius_validation::validator::Validator;

use crate::error::MempoolError;

pub struct UserOperationPool {
    pub db: SiliusDB,
    pub validator: Validator,
}

impl UserOperationPool {
    pub fn new(db: SiliusDB, validator: Validator) -> Self {
        Self { db, validator }
    }

    pub fn insert_user_operation(&self, user_operation: UserOperation) -> Result<(), MempoolError> {
        self.db
            .user_operation_provider()
            .insert(user_operation.hash, user_operation.inner)?;
        Ok(())
    }

    pub fn get_user_operations(&self) -> Result<Vec<UserOperation>, MempoolError> {
        Ok(self.db.user_operation_provider().get_all()?)
    }

    pub fn clear(&self) -> Result<(), MempoolError> {
        self.db.user_operation_provider().clear()?;
        Ok(())
    }
}
