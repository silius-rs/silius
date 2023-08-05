//! Bundler is a crate for bundling transactions and sending them to the Ethereum execution client
#![allow(dead_code)]

mod bundler;
#[cfg(test)]
mod mock_relay;

pub use bundler::{Bundler, SendBundleMode};
