use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::{Value, json};
use silius_manager::SiliusManager;

use crate::types::error::ErrorData;

pub async fn send_bundle_now<P: Provider + Clone + 'static>(
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    // TODO: implement send bundle now
    Ok(json!("ok"))
}
