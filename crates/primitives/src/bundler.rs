//! Bundler-related primitives

use serde::Deserialize;
use strum_macros::{EnumString, EnumVariantNames};

/// Bundler modes
#[derive(Debug, Deserialize)]
pub enum Mode {
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
pub enum SendStrategy {
    /// Sends the bundle to the Ethereum execution client
    EthereumClient,
    /// Sends the bundle to the Flashbots relay
    Flashbots,
}
