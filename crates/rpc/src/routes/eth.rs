use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;
use silius_primitives::user_operation::UserOperationBase;

use crate::{
    handlers::{
        config::{chain_id, supported_entry_points},
        mempool::send_user_operation,
    },
    types::{
        codes::{INVALID_PARAMS, METHOD_NOT_FOUND},
        error::ErrorData,
    },
};

pub async fn eth_router<P: Provider + Clone + 'static>(
    method: &str,
    params: Vec<Value>,
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    match method {
        "eth_sendUserOperation" => {
            if params.len() != 2 {
                return Err(ErrorData::std(INVALID_PARAMS));
            }

            let user_operation_base: UserOperationBase = serde_json::from_value(params[0].clone())
                .map_err(|_| ErrorData::std(INVALID_PARAMS))?;
            let entry_point_address: Address = serde_json::from_value(params[1].clone())
                .map_err(|_| ErrorData::std(INVALID_PARAMS))?;

            send_user_operation(user_operation_base, entry_point_address, manager).await
        }
        // "eth_estimateUserOperationGas" => estimate_user_operation_gas().await,
        // "eth_getUserOperationByHash" => get_user_operation_by_hash().await,
        // "eth_getUserOperationReceipt" => get_user_operation_receipt().await,
        "eth_supportedEntryPoints" => supported_entry_points().await,
        "eth_chainId" => chain_id().await,
        _ => Err(ErrorData::std(METHOD_NOT_FOUND)),
    }
}
