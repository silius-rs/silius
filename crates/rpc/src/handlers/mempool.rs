use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::Provider;
use serde_json::{Value, json};
use silius_manager::SiliusManager;
use silius_primitives::user_operation::UserOperationBase;

use crate::types::error::ErrorData;

pub async fn send_user_operation<P: Provider + Clone + 'static>(
    user_operation: UserOperationBase,
    entry_point: Address,
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    // TODO: implement send user operation
    Ok(json!("ok"))
}

pub async fn clear_state<P: Provider + Clone + 'static>(
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    manager
        .mempool
        .clear()
        .map_err(|e| ErrorData::new(-32603, &e.to_string()))?;
    Ok(json!("ok"))
}
