//! `simulation` module performs checks against a [UserOperation's](UserOperation) signature and
//! timestamp via a `eth_call` to the Ethereum execution client.
pub mod signature;
pub mod timestamp;
