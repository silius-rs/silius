use std::collections::HashMap;

use alloy_primitives::{Address, Bytes, U256};

#[derive(Default, Clone)]
pub struct ValidationResult {
    pub pre_op_gas: U256,
    pub prefund: U256,
    pub sig_failed: bool,
    pub paymaster_sig_failed: bool,
    pub valid_after: u64,
    pub valid_until: u64,
    pub entity_staked: HashMap<Address, bool>,
    pub paymaster_context: Bytes,
}
