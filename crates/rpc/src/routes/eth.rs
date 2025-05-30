use serde_json::Value;
use silius_storage::db::SiliusDB;

use crate::{
    handlers::config::{chain_id, supported_entry_points},
    types::error::ErrorData,
};

pub async fn eth_router(
    method: &str,
    _params: Vec<Value>,
    _db: &SiliusDB,
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
