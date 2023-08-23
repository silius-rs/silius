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

/// The `SendBundleMode` determines whether to send the bundle to a Ethereum execution client or to Flashbots relay
#[derive(Clone, Debug, PartialEq)]
pub enum SendBundleMode {
    /// Send the bundle to a Ethereum execution client
    EthClient,
    /// Send the bundle to Flashbots relay
    Flashbots,
}
