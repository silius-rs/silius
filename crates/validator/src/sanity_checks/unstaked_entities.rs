use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    sanity_checks::{SanityCheck, error::SanityCheckError},
};

pub struct UnstakedEntitiesCheck;

#[async_trait::async_trait]
impl<P: Provider> SanityCheck<P> for UnstakedEntitiesCheck {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        _db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), SanityCheckError> {
        Ok(())
    }
}
