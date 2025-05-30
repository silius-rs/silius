use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<Value>,
    pub id: Value,
}

impl Request {
    pub fn dump(&self) -> String {
        serde_json::to_string(self).expect("Should never failed")
    }
}
