use std::{error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorData {
    pub code: i32,
    pub message: String,
    pub data: Value,
}

impl ErrorData {
    pub fn new(code: i32, message: &str) -> Self {
        Self {
            code,
            message: String::from(message),
            data: Value::Null,
        }
    }

    pub fn std(code: i32) -> Self {
        match code {
            -32700 => ErrorData::new(-32700, "Parse error"),
            -32600 => ErrorData::new(-32600, "Invalid Request"),
            -32601 => ErrorData::new(-32601, "Method not found"),
            -32602 => ErrorData::new(-32602, "Invalid params"),
            -32603 => ErrorData::new(-32603, "Internal error"),
            _ => panic!("Undefined pre-defined error codes"),
        }
    }

    pub fn dump(&self) -> String {
        serde_json::to_string(self).expect("Should never failed")
    }
}

impl error::Error for ErrorData {}

impl fmt::Display for ErrorData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.code, self.message, self.data)
    }
}
