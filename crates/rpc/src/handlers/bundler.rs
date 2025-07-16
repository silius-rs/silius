use std::sync::Arc;

use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;

use crate::types::{codes::INTERNAL_ERROR, error::ErrorData};

pub async fn send_bundle_now<P: Provider + Clone + 'static>(
    manager: &Arc<SiliusManager<P>>,
) -> Result<Value, ErrorData> {
    match manager.submit_bundle().await {
        Ok(tx_hash) => Ok(tx_hash.to_string().into()),
        Err(e) => Err(ErrorData::new(INTERNAL_ERROR, &e.to_string())),
    }
}
