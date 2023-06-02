use serde::Deserialize;

/// Default time interval for auto bundling mode (in seconds)
pub const DEFAULT_BUNDLE_INTERVAL: u64 = 10;

/// Bundling modes
#[derive(Debug, Deserialize)]
pub enum Mode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "manual")]
    Manual,
}
