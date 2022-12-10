use crate::types::user_operation::UserOperation;
use ethers::types::{u256_from_f64_saturating, U256};
use std::ops::Deref;

pub const WARM_STORAGE_READ_COST: u64 = 100; // EIP 2929
pub const CALL_VALUE_TRANSFER_GAS: u64 = 9000; // Ethereum yellow paper
pub const CALL_GAS: u64 = 700; // EIP 150
pub const CALL_STIPEND: u64 = 2300; // Ethereum yellow paper

pub fn non_zero_value_call() -> U256 {
    U256::from(CALL_VALUE_TRANSFER_GAS + WARM_STORAGE_READ_COST + CALL_GAS + CALL_STIPEND)
}

pub struct Overhead {
    pub fixed: U256,
    pub per_user_op: U256,
    pub per_user_op_word: U256,
    pub zero_byte: U256,
    pub non_zero_byte: U256,
    pub bundle_size: U256,
    pub sig_size: U256,
}

impl Overhead {
    pub fn default() -> Self {
        Self {
            fixed: U256::from(21000),
            per_user_op: U256::from(18300),
            per_user_op_word: U256::from(4),
            zero_byte: U256::from(4),
            non_zero_byte: U256::from(16),
            bundle_size: U256::from(1),
            sig_size: U256::from(65),
        }
    }

    pub fn calculate_pre_verification_gas(&self, user_operation: &UserOperation) -> U256 {
        let user_operation_packed = user_operation.pack();
        let call_data_cost: U256 = U256::from(
            user_operation_packed
                .deref()
                .iter()
                .map(|&x| {
                    if x == 0 {
                        self.zero_byte.as_u128()
                    } else {
                        self.non_zero_byte.as_u128()
                    }
                })
                .sum::<u128>(),
        );
        u256_from_f64_saturating(
            (self.fixed.as_u128() as f64) / (self.bundle_size.as_u128() as f64)
                + (call_data_cost
                    + self.per_user_op
                    + self.per_user_op_word * user_operation_packed.len())
                .as_u128() as f64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::Bytes;
    use std::str::FromStr;

    #[test]
    fn pre_verification_gas_calculation() {
        let gas_overhead = Overhead::default();
        let user_operation = UserOperation {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xe19e9755942bb0bd0cccce25b1742596b8a8250b3bf2c3e70000000000000000000000001d9a2cb3638c2fc8bf9c01d088b79e75cd188b17000000000000000000000000789d9058feecf1948af429793e7f1eb4a75db2220000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000ab7e2cbfcfb6a5f33a75ad745c3e5fb48d689b5400000000000000000000000000000000000000000000000002c68af0bb14000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(21900),
            verification_gas_limit: U256::from(1218343),
            pre_verification_gas: U256::from(50780),
            max_fee_per_gas: U256::from(10064120791 as u64),
            max_priority_fee_per_gas: U256::from(1620899097),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0x4e69eb5e02d47ba28878655d61c59c20c3e9a2e6905381305626f6a5a2892ec12bd8dd59179f0642731e0e853af54a71ce422a1a234548c9dd1c559bd07df4461c").unwrap(),
        };

        assert_eq!(
            gas_overhead.calculate_pre_verification_gas(&user_operation),
            U256::from(48684)
        );
    }
}
