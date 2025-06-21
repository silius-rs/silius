use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::{Value, json};
use silius_manager::SiliusManager;

use crate::types::error::ErrorData;

pub async fn clear_state<P: Provider + Clone + 'static>(
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    manager
        .mempool
        .clear()
        .map_err(|e| ErrorData::new(-32603, &e.to_string()))?;
    Ok(json!("ok"))
}
