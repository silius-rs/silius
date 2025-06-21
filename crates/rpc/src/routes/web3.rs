use alloy_provider::Provider;
use serde_json::Value;
use silius_manager::SiliusManager;

use crate::{handlers::version::client_version, types::error::ErrorData};

pub async fn web3_router<P: Provider + Clone + 'static>(
    method: &str,
    _params: Vec<Value>,
    _manager: &SiliusManager<P>,
) -> Result<Value, ErrorData> {
    match method {
        "web3_clientVersion" => client_version().await,
        _ => Err(ErrorData::std(-32601)),
    }
}
