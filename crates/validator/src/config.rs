use alloy_primitives::U256;

const DEFAULT_MAX_VERIFICATION_GAS: u64 = 10_000_000;
const DEFAULT_MIN_PRIORITY_FEE_PER_GAS: u64 = 1;

pub struct ValidatorConfig {
    pub min_priority_fee_per_gas: U256,
    pub max_verification_gas: U256,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            min_priority_fee_per_gas: U256::from(DEFAULT_MIN_PRIORITY_FEE_PER_GAS),
            max_verification_gas: U256::from(DEFAULT_MAX_VERIFICATION_GAS),
        }
    }
}
