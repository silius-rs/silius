use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{JSONRPC_VERSION, error::ErrorData};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorData>,
    pub id: Value,
}

impl Response {
    pub fn with_result(mut self, result: Value) -> Self {
        self.result = Some(result);
        self
    }

    pub fn with_error(mut self, error: ErrorData) -> Self {
        self.error = Some(error);
        self
    }

    pub fn dump(&self) -> String {
        serde_json::to_string(self).expect("Should never failed")
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            result: None,
            error: None,
            id: Value::Null,
        }
    }
}
