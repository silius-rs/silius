use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use silius_primitives::network_spec::network_spec;

use crate::types::error::ErrorData;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct SupportedEntryPoints {
    supported_entry_points: Vec<Address>,
}

impl SupportedEntryPoints {
    pub fn new(supported_entry_points: Vec<Address>) -> Self {
        Self {
            supported_entry_points,
        }
    }
}

pub async fn supported_entry_points() -> Result<Value, ErrorData> {
    Ok(json!(SupportedEntryPoints::new(vec![
        network_spec().entry_point_address,
    ])))
}

pub async fn chain_id() -> Result<Value, ErrorData> {
    Ok(json!(format!("0x{:x}", network_spec().chain_id())))
}
