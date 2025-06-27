use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{config::ValidatorConfig, sanity_checks::error::SanityCheckError};

pub mod error;
pub mod max_fee;

#[async_trait::async_trait]
pub trait SanityCheck<P: Provider> {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), SanityCheckError>;
}
