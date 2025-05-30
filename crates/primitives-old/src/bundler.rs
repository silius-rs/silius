//! Bundler-related primitives

use serde::Deserialize;
use strum_macros::{EnumString, EnumVariantNames};

/// Bundle modes
#[derive(Debug, Deserialize)]
pub enum BundleMode {
    /// Sends bundles automatically every x seconds
    #[serde(rename = "auto")]
    Auto(u64),
    /// Sends bundles upon request
    #[serde(rename = "manual")]
    Manual,
}

/// Determines the mode how bundler sends the bundle
#[derive(Clone, Copy, Debug, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum BundleStrategy {
    /// Sends the bundle to the Ethereum execution client
    EthereumClient,
    /// Sends the bundle to the Flashbots relay
    Flashbots,
    /// Send the bundle to the Ethereum execution client over conditional RPC method
    Conditional,
    /// Sends the bundle to the Fastlane relay
    Fastlane,
}
