use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;

use crate::{handlers::mempool::clear_state, types::error::ErrorData};

pub async fn debug_router<P: Provider + Clone + 'static>(
    method: &str,
    _params: Vec<Value>,
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    match method {
        "debug_bundler_clearState" => clear_state(manager).await,
        // "debug_dumpMempool" => dump_mempool().await,
        // "debug_sendBundleNow" => send_bundle_now().await,
        // "debug_setBundlingMode" => set_bundling_mode().await,
        // "debug_setReputation" => set_reputation().await,
        // "debug_dumpReputation" => dump_reputation().await,
        // "debug_addUserOps" => add_user_ops().await,
        _ => Err(ErrorData::std(-32601)),
    }
}
