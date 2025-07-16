use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    tracing_check::{TracingCheck, error::TracingCheckError},
};

pub struct OpcodeCheck;

#[async_trait::async_trait]
impl<P: Provider> TracingCheck<P> for OpcodeCheck {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), TracingCheckError> {
        Ok(())
    }
}
