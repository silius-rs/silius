use serde_json::Value;
use silius_node_version::silius_node_version;

use crate::types::error::ErrorData;

pub async fn client_version() -> Result<Value, ErrorData> {
    Ok(Value::from(silius_node_version()))
}
