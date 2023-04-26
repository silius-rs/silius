use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum Mode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "manual")]
    Manual,
}

pub const DEFAULT_INTERVAL: u64 = 10;
