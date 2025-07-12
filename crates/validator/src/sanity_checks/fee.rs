use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    sanity_checks::{SanityCheck, error::SanityCheckError},
};

pub struct FeeCheck;

#[async_trait::async_trait]
impl<P: Provider> SanityCheck<P> for FeeCheck {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        _db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), SanityCheckError> {
        if user_operation.max_priority_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(SanityCheckError::Check(format!(
                "Max priority fee per gas {} is greater than max fee per gas {}",
                user_operation.max_priority_fee_per_gas, user_operation.max_fee_per_gas
            )));
        }

        let base_fee_per_gas = chain.get_base_fee_per_gas().await?;

        if base_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(SanityCheckError::Check(format!(
                "Base fee per gas {} is greater than max fee per gas {}",
                base_fee_per_gas, user_operation.max_fee_per_gas
            )));
        }

        if user_operation.max_fee_per_gas < config.min_priority_fee_per_gas {
            return Err(SanityCheckError::Check(format!(
                "Max fee per gas {} is less than configured min priority fee per gas {}",
                user_operation.max_fee_per_gas, config.min_priority_fee_per_gas
            )));
        }

        Ok(())
    }
}
