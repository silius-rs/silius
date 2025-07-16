use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;

use crate::{
    handlers::{bundler::send_bundle_now, mempool::clear_state},
    types::{codes::METHOD_NOT_FOUND, error::ErrorData},
};

pub async fn debug_router<P: Provider + Clone + 'static>(
    method: &str,
    _params: Vec<Value>,
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    match method {
        "debug_bundler_clearState" => clear_state(manager).await,
        // "debug_bundler_dumpMempool" => dump_mempool().await,
        "debug_bundler_sendBundleNow" => send_bundle_now(manager).await,
        // "debug_bundler_setBundlingMode" => set_bundling_mode().await,
        // "debug_bundler_setReputation" => set_reputation().await,
        // "debug_bundler_dumpReputation" => dump_reputation().await,
        // "debug_bundler_addUserOps" => add_user_ops().await,
        _ => Err(ErrorData::std(METHOD_NOT_FOUND)),
    }
}
