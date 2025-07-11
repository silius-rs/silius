use std::sync::Arc;

use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::{db::SiliusDB, tables::Table};
use silius_validator::validator::Validator;

use crate::error::MempoolError;

pub struct UserOperationPool<P: Provider + 'static> {
    pub db: SiliusDB,
    pub chain: Arc<Chain<P>>,
    pub validator: Validator<P>,
}

impl<P: Provider + 'static> UserOperationPool<P> {
    pub fn new(db: SiliusDB, chain: Arc<Chain<P>>, validator: Validator<P>) -> Self {
        Self {
            db,
            chain,
            validator,
        }
    }

    pub async fn insert_user_operation(
        &self,
        user_operation: UserOperation,
    ) -> Result<(), MempoolError> {
        self.validator
            .validate_user_operation(&user_operation, &self.db, &self.chain)
            .await?;
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
