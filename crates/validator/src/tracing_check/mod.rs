use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{config::ValidatorConfig, tracing_check::error::TracingCheckError};

pub mod code;
pub mod error;
pub mod opcode;
pub mod storage;

#[async_trait::async_trait]
pub trait TracingCheck<P: Provider> {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
        frame: &Erc7562Frame,
    ) -> Result<(), TracingCheckError>;
}
