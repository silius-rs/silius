use serde_json::{Value, json};
use silius_storage::{db::SiliusDB, tables::Table};

use crate::types::error::ErrorData;

pub async fn clear_state(db: &SiliusDB) -> Result<Value, ErrorData> {
    db.user_operation_provider()
        .clear()
        .map_err(|e| ErrorData::new(-32603, &e.to_string()))?;
    Ok(json!("ok"))
}
