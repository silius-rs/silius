use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;

use crate::{
    handlers::config::{chain_id, supported_entry_points},
    types::error::ErrorData,
};

pub async fn eth_router<P: Provider + Clone + 'static>(
    method: &str,
    _params: Vec<Value>,
    _manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    match method {
        // "eth_sendUserOperation" => send_user_operation().await,
        // "eth_estimateUserOperationGas" => estimate_user_operation_gas().await,
        // "eth_getUserOperationByHash" => get_user_operation_by_hash().await,
        // "eth_getUserOperationReceipt" => get_user_operation_receipt().await,
        "eth_supportedEntryPoints" => supported_entry_points().await,
        "eth_chainId" => chain_id().await,
        _ => Err(ErrorData::std(-32601)),
    }
}
