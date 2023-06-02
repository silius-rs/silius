use aa_bundler_primitives::{CodeHash, UserOperation};
use ethers::types::{u256_from_f64_saturating, Address, H256, U256};
use lazy_static::__Deref;
use std::collections::HashMap;

pub fn equal_code_hashes(code_hashes: &Vec<CodeHash>, prev_code_hashes: &Vec<CodeHash>) -> bool {
    if prev_code_hashes.len() != code_hashes.len() {
        return false;
    }

    let code_hashes_map = code_hashes
        .iter()
        .map(|code_hash| (code_hash.address, code_hash.hash))
        .collect::<HashMap<Address, H256>>();

    for code_hash in prev_code_hashes {
        if let Some(hash) = code_hashes_map.get(&code_hash.address) {
            if hash != &code_hash.hash {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

// https://github.com/eth-infinitism/bundler/blob/main/packages/sdk/src/calcPreVerificationGas.ts#L44-L52
pub struct Overhead {
    pub fixed: U256,
    pub per_user_op: U256,
    pub per_user_op_word: U256,
    pub zero_byte: U256,
    pub non_zero_byte: U256,
    pub bundle_size: U256,
    pub sig_size: U256,
}

impl Default for Overhead {
    fn default() -> Self {
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
}

impl Overhead {
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
        let length_in_word = ((user_operation_packed.len() + 31) as f64) / 32_f64;
        u256_from_f64_saturating(
            (self.fixed.as_u128() as f64) / (self.bundle_size.as_u128() as f64)
                + ((call_data_cost + self.per_user_op).as_u128() as f64)
                + (self.per_user_op_word.as_u128() as f64) * length_in_word,
        )
    }
}

pub fn calculate_valid_gas(gas_price: U256, gas_increase_perc: U256) -> U256 {
    let gas_price = gas_price.as_u64() as f64;
    let gas_increase_perc = gas_increase_perc.as_u64() as f64;
    U256::from((gas_price * (1.0 + gas_increase_perc / 100.0)).ceil() as u64)
}

pub fn calculate_call_gas_limit(paid: U256, pre_op_gas: U256, fee_per_gas: U256) -> U256 {
    paid / fee_per_gas - pre_op_gas + Overhead::default().fixed
}

#[cfg(test)]
pub mod tests {
    use std::{fmt::Debug, str::FromStr};

    use aa_bundler_primitives::{UserOperation, UserOperationHash};
    use ethers::types::{Address, Bytes, H256, U256};

    use super::*;
    use crate::mempool::Mempool;

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
            max_fee_per_gas: U256::from(10064120791_u64),
            max_priority_fee_per_gas: U256::from(1620899097),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0x4e69eb5e02d47ba28878655d61c59c20c3e9a2e6905381305626f6a5a2892ec12bd8dd59179f0642731e0e853af54a71ce422a1a234548c9dd1c559bd07df4461c").unwrap(),
        };

        assert_eq!(
            gas_overhead.calculate_pre_verification_gas(&user_operation),
            U256::from(45340)
        );
    }

    pub fn mempool_test_case<T>(mut mempool: T, not_found_error_message: &str)
    where
        T: Mempool<UserOperations = Vec<UserOperation>> + Debug,
        T::Error: Debug + ToString,
    {
        let entry_point = Address::random();
        let chain_id = U256::from(5);
        let senders = vec![Address::random(), Address::random(), Address::random()];

        let mut user_operation: UserOperation;
        let mut user_operation_hash: UserOperationHash = Default::default();
        for i in 0..2 {
            user_operation = UserOperation {
                sender: senders[0],
                nonce: U256::from(i),
                ..UserOperation::random()
            };
            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );

            user_operation = UserOperation {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );
        }

        for i in 0..3 {
            user_operation = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();

            assert_eq!(
                mempool.get(&user_operation_hash).unwrap().unwrap(),
                user_operation
            );
        }

        assert_eq!(mempool.get_all().len(), 7);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[1]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 3);

        assert_eq!(mempool.remove(&user_operation_hash).unwrap(), ());
        assert_eq!(
            mempool
                .remove(&H256::random().into())
                .unwrap_err()
                .to_string(),
            not_found_error_message
        );

        assert_eq!(mempool.get_all().len(), 6);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 2);

        assert_eq!(mempool.clear(), ());

        assert_eq!(mempool.get_all().len(), 0);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 0);

        for i in 0..3 {
            user_operation = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                max_priority_fee_per_gas: U256::from(i + 1),
                ..UserOperation::random()
            };

            mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();
        }

        let sorted = mempool.get_sorted().unwrap();
        assert_eq!(sorted[0].max_priority_fee_per_gas, U256::from(3));
        assert_eq!(sorted[1].max_priority_fee_per_gas, U256::from(2));
        assert_eq!(sorted[2].max_priority_fee_per_gas, U256::from(1));
        assert_eq!(sorted.len(), 3);
    }
}
