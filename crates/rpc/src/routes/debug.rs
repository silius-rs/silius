use serde_json::Value;
use silius_storage::db::SiliusDB;

use crate::{handlers::mempool::clear_state, types::error::ErrorData};

pub async fn debug_router(
    method: &str,
    _params: Vec<Value>,
    db: &SiliusDB,
) -> Result<Value, ErrorData> {
    match method {
        "debug_bundler_clearState" => clear_state(db).await,
        // "debug_dumpMempool" => dump_mempool().await,
        // "debug_sendBundleNow" => send_bundle_now().await,
        // "debug_setBundlingMode" => set_bundling_mode().await,
        // "debug_setReputation" => set_reputation().await,
        // "debug_dumpReputation" => dump_reputation().await,
        // "debug_addUserOps" => add_user_ops().await,
        _ => Err(ErrorData::std(-32601)),
    }
}
