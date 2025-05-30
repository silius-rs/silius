use serde_json::Value;
use silius_storage::db::SiliusDB;

use crate::{handlers::version::client_version, types::error::ErrorData};

pub async fn web3_router(
    method: &str,
    _params: Vec<Value>,
    _db: &SiliusDB,
) -> Result<Value, ErrorData> {
    match method {
        "web3_clientVersion" => client_version().await,
        _ => Err(ErrorData::std(-32601)),
    }
}
