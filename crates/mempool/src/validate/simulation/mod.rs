//! `simulation` module performs checks against a user operation's signature and
//! timestamp via a `eth_call` to the Ethereum execution client.
pub mod signature;
pub mod timestamp;
pub mod verification_extra_gas;
