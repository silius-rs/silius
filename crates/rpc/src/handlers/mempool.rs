use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::Provider;
use serde_json::{Value, json};
use silius_manager::SiliusManager;
use silius_primitives::{
    network_spec::network_spec,
    user_operation::{UserOperation, UserOperationBase},
};

use crate::types::{
    codes::{INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST},
    error::ErrorData,
};

pub async fn send_user_operation<P: Provider + Clone + 'static>(
    user_operation_base: UserOperationBase,
    entry_point_address: Address,
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    if entry_point_address != network_spec().entry_point_address {
        return Err(ErrorData::new(
            INVALID_PARAMS,
            "Entry point address not supported",
        ));
    }

    let user_operation_hash = manager
        .chain
        .get_user_operation_hash(user_operation_base.to_packed_user_operation())
        .await
        .map_err(|e| ErrorData::new(INVALID_REQUEST, &e.to_string()))?;
    let user_operation = UserOperation::new(user_operation_base, user_operation_hash);

    manager
        .mempool
        .insert_user_operation(user_operation)
        .await?;

    Ok(user_operation_hash.to_string().into())
}

pub async fn clear_state<P: Provider + Clone + 'static>(
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    manager
        .mempool
        .clear()
        .map_err(|e| ErrorData::new(INTERNAL_ERROR, &e.to_string()))?;
    Ok(json!("ok"))
}
