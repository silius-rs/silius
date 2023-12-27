//! Mempool/related primitives

use strum_macros::{EnumString, EnumVariantNames};

/// Verification modes for user operation mempool
#[derive(Clone, Copy, Debug, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Mode {
    Standard,
    Unsafe,
}
