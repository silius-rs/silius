use ethers::types::{Address, H256, U256};
use silius_primitives::{simulation::CodeHash, UserOperationSigned};
use std::{collections::HashMap, ops::Deref};

pub fn equal_code_hashes(hashes: &[CodeHash], hashes_prev: &Vec<CodeHash>) -> bool {
    if hashes_prev.len() != hashes.len() {
        return false;
    }

    let hashes_map =
        hashes.iter().map(|h: &CodeHash| (h.address, h.hash)).collect::<HashMap<Address, H256>>();

    for hash_prev in hashes_prev {
        if let Some(hash) = hashes_map.get(&hash_prev.address) {
            if hash != &hash_prev.hash {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

/// Struct to calculate the pre-verification gas of a user operation
// https://github.com/eth-infinitism/bundler/blob/main/packages/sdk/src/calcPreVerificationGas.ts#L44-L51
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
    /// Calculates the pre-verification gas of a [UserOperation](UserOperationSigned)
    /// The function first packs the [UserOperation](UserOperationSigned), then extracts the call
    /// data for gas calculation.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperationSigned) to calculate the pre-verification gas for
    ///
    /// # Returns
    /// The pre-verification gas of the [UserOperation](UserOperationSigned)
    pub fn calculate_pre_verification_gas(&self, uo: &UserOperationSigned) -> U256 {
        let uo_pack = uo.pack();

        let call_data = uo_pack.deref().iter().fold(U256::zero(), |acc, &x| {
            let byte_cost = if x == 0 { &self.zero_byte } else { &self.non_zero_byte };
            acc.saturating_add(*byte_cost)
        });

        // per_user_op_word * (uo_pack.len() + 31) / 32
        // -> (per_user_op_word * (uo_pack.len() + 31)) / 32
        // -> (per_user_op_word * (uo_pack.len() + 31)) / 32 + rounding_const
        let word_cost = div_ceil(
            self.per_user_op_word.saturating_mul(U256::from(uo_pack.len() + 31)),
            U256::from(32),
        );

        // fixed / bundle_size
        // -> fixed / bundle_size + rounding_const
        let fixed_divided_by_bundle_size = div_ceil(self.fixed, self.bundle_size);

        fixed_divided_by_bundle_size
            .saturating_add(call_data)
            .saturating_add(self.per_user_op)
            .saturating_add(word_cost)
    }
}

/// Helper function to calculate the valid gas of a [UserOperation](UserOperation)
/// The function is invoked by the
/// [check_valid_gas](crates::uopool::validate::sanity::check_valid_gas) method.
///
/// # Arguments
/// `gas_price` - The gas price
/// `gas_incr_perc` - The gas increase percentage
///
/// # Returns
/// The valid gas of the [UserOperation](UserOperation)
pub fn calculate_valid_gas(gas_price: U256, gas_incr_perc: U256) -> U256 {
    // (gas_price * (1 + gas_incr_perc / 100)
    // -> (100 / 100) * (gas_price * ( 1 + gas_incr_perc / 100 ))
    // -> (gas_price * ( 100 + gas_incr_perc )) / 100
    // -> (gas_price * ( 100 + gas_incr_perc )) / 100 + rounding_const
    let denominator = U256::from(100);
    let numerator = gas_price.saturating_mul(gas_incr_perc.saturating_add(denominator));
    div_ceil(numerator, denominator)
}

/// Helper function to calculate the call gas limit of a [UserOperation](UserOperation)
/// The function is invoked by the
/// [estimate_user_operation_gas](crates::uopool::estimate::estimate_user_operation_gas) method.
///
/// # Arguments
/// `paid` - The paid gas
/// `pre_op_gas` - The pre-operation gas
/// `fee_per_gas` - The fee per gas
///
/// # Returns
/// The call gas limit of the [UserOperation](UserOperation)
pub fn calculate_call_gas_limit(paid: U256, pre_op_gas: U256, fee_per_gas: U256) -> U256 {
    // paid / fee_per_gas - pre_op_gas + Overhead::default().fixed
    // -> (paid / fee_per_gas + rounding_cost) - pre_op_gas + Overhead::default().fixed
    div_ceil(paid, fee_per_gas).saturating_sub(pre_op_gas).saturating_add(Overhead::default().fixed)
}

/// Performs division and rounds up to the nearest integer.
///
/// This function takes a numerator and a denominator of type `U256`,
/// performs the division, and rounds up if there is a remainder.
///
/// # Examples
///
/// ```ignore
/// use ethers::types::U256;
///
/// let result = div_ceil(U256::from(10), U256::from(3));
/// assert_eq!(result, U256::from(4));
/// ```
pub fn div_ceil(numerator: U256, denominator: U256) -> U256 {
    let rounding_const =
        U256::from(if numerator.checked_rem(denominator).unwrap_or_default() > U256::zero() {
            1
        } else {
            0
        });
    numerator.checked_div(denominator).unwrap_or_default().saturating_add(rounding_const)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{mempool::Mempool, Reputation};
    use ethers::types::{Address, Bytes, H256, U256};
    use silius_primitives::{
        reputation::{ReputationEntry, Status},
        UserOperation, UserOperationHash, UserOperationSigned,
    };

    #[test]
    fn pre_verification_gas_calculation() {
        let gas_oh = Overhead::default();
        let uo = UserOperationSigned {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::zero(),
            init_code: "0xe19e9755942bb0bd0cccce25b1742596b8a8250b3bf2c3e70000000000000000000000001d9a2cb3638c2fc8bf9c01d088b79e75cd188b17000000000000000000000000789d9058feecf1948af429793e7f1eb4a75db2220000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_data: "0x80c5c7d0000000000000000000000000ab7e2cbfcfb6a5f33a75ad745c3e5fb48d689b5400000000000000000000000000000000000000000000000002c68af0bb14000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_gas_limit: 21900.into(),
            verification_gas_limit: 1218343.into(),
            pre_verification_gas: 50780.into(),
            max_fee_per_gas: 10064120791_u64.into(),
            max_priority_fee_per_gas: 1620899097.into(),
            paymaster_and_data: Bytes::default(),
            signature: "0x4e69eb5e02d47ba28878655d61c59c20c3e9a2e6905381305626f6a5a2892ec12bd8dd59179f0642731e0e853af54a71ce422a1a234548c9dd1c559bd07df4461c".parse().unwrap(),
        };

        assert_eq!(gas_oh.calculate_pre_verification_gas(&uo), 45340.into());
    }

    #[test]
    fn pre_verification_gas_calculation_with_large_user_operation() {
        let gas_oh = Overhead::default();
        let uo = UserOperationSigned {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::max_value(),
            init_code: Bytes::from(vec![255; 1024]), // Large init_code
            call_data: Bytes::from(vec![255; 1024]), // Large call_data
            call_gas_limit: U256::max_value(),
            verification_gas_limit: U256::max_value(),
            pre_verification_gas: U256::max_value(),
            max_fee_per_gas: U256::max_value(),
            max_priority_fee_per_gas: U256::max_value(),
            paymaster_and_data: Bytes::from(vec![255; 1024]), // Large paymaster_and_data
            signature: Bytes::from(vec![255; 1024]),          // Large signature
        };

        assert_eq!(gas_oh.calculate_pre_verification_gas(&uo), 110020.into());
    }

    #[test]
    fn pre_verification_gas_calculation_with_large_per_user_op_word() {
        let gas_oh = Overhead {
            fixed: U256::from(21000),
            per_user_op: U256::from(18300),
            per_user_op_word: U256::from(10000),
            zero_byte: U256::from(4),
            non_zero_byte: U256::from(16),
            bundle_size: U256::from(1),
            sig_size: U256::from(65),
        };
        let uo = UserOperationSigned {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::max_value(),
            init_code: Bytes::from(vec![255; 1024]), // Large init_code
            call_data: Bytes::from(vec![255; 1024]), // Large call_data
            call_gas_limit: U256::max_value(),
            verification_gas_limit: U256::max_value(),
            pre_verification_gas: U256::max_value(),
            max_fee_per_gas: U256::max_value(),
            max_priority_fee_per_gas: U256::max_value(),
            paymaster_and_data: Bytes::from(vec![255; 1024]), // Large paymaster_and_data
            signature: Bytes::from(vec![255; 1024]),          // Large signature
        };

        assert_eq!(gas_oh.calculate_pre_verification_gas(&uo), 1549132.into());
    }

    /// This test occurred overflow when previous `calculate_pre_verification_gas` is used.
    /// previous `calculate_pre_verification_gas` is https://github.com/silius-rs/silius/blob/bd79ea0e610adff8d77ba128f53befa8401a4d77/crates/uopool/src/utils.rs#L63-L84
    #[test]
    fn pre_verification_gas_calculation_overflow() {
        let gas_oh = Overhead {
            fixed: U256::max_value(),
            per_user_op: U256::max_value(),
            per_user_op_word: U256::max_value(),
            zero_byte: U256::max_value(),
            non_zero_byte: U256::max_value(),
            bundle_size: U256::from(1), // To avoid division by zero
            sig_size: U256::max_value(),
        };

        let uo = UserOperationSigned {
            sender: Address::default(),
            nonce: U256::max_value(),
            init_code: Bytes::from(vec![255; 1024]), // Large init_code
            call_data: Bytes::from(vec![255; 1024]), // Large call_data
            call_gas_limit: U256::max_value(),
            verification_gas_limit: U256::max_value(),
            pre_verification_gas: U256::max_value(),
            max_fee_per_gas: U256::max_value(),
            max_priority_fee_per_gas: U256::max_value(),
            paymaster_and_data: Bytes::from(vec![255; 1024]), // Large paymaster_and_data
            signature: Bytes::from(vec![255; 1024]),          // Large signature
        };

        // This test is mainly to check if the function can handle the overflow scenario without
        // panicking. We don't have a specific expected value in this case.
        let _ = gas_oh.calculate_pre_verification_gas(&uo);
    }

    #[test]
    fn valid_gas_calculation_when_no_round_up_case() {
        let gas_price = U256::from(100);
        let gas_incr_perc = U256::from(10);
        let valid_gas = calculate_valid_gas(gas_price, gas_incr_perc);
        assert_eq!(valid_gas, 110.into());
    }

    #[test]
    fn valid_gas_calculation_when_round_up_case() {
        let gas_price = U256::from(10);
        let gas_incr_perc = U256::from(11);
        assert_eq!(calculate_valid_gas(gas_price, gas_incr_perc), 12.into());
    }

    #[test]
    fn call_gas_limit_calculation() {
        let paid = U256::from(100);
        let pre_op_gas = U256::from(10);
        let fee_per_gas = U256::from(1);
        assert_eq!(calculate_call_gas_limit(paid, pre_op_gas, fee_per_gas), 21090.into());
    }

    #[test]
    fn call_gas_limit_calculation_with_zero_divide() {
        let paid = U256::from(100);
        let pre_op_gas = U256::from(10);
        let fee_per_gas = U256::from(0);
        assert_eq!(calculate_call_gas_limit(paid, pre_op_gas, fee_per_gas), 21000.into());
    }

    #[test]
    fn div_ceil_divisible_calculation() {
        assert_eq!(div_ceil(U256::from(10), U256::from(2)), 5.into());
    }

    #[test]
    fn div_ceil_no_divisible_calculation() {
        assert_eq!(div_ceil(U256::from(10), U256::from(3)), 4.into());
    }

    pub fn mempool_test_case(mut mempool: Mempool) {
        let ep = Address::random();
        let chain_id = 5_u64;
        let senders = vec![Address::random(), Address::random(), Address::random()];

        let mut uo: UserOperationSigned;
        let mut uo_hash: UserOperationHash = Default::default();
        for i in 0..2 {
            uo = UserOperationSigned {
                sender: senders[0],
                nonce: U256::from(i),
                ..UserOperationSigned::random()
            };
            uo_hash = uo.hash(&ep, chain_id);

            assert_eq!(
                mempool
                    .add(UserOperation::from_user_operation_signed(uo_hash, uo.clone()))
                    .unwrap(),
                uo_hash
            );
            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap().user_operation, uo);

            uo = UserOperationSigned {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperationSigned::random()
            };
            uo_hash = uo.hash(&ep, chain_id);

            assert_eq!(
                mempool
                    .add(UserOperation::from_user_operation_signed(uo_hash, uo.clone()))
                    .unwrap(),
                uo_hash
            );
            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap().user_operation, uo);
        }

        for i in 0..3 {
            uo = UserOperationSigned {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperationSigned::random()
            };
            uo_hash = uo.hash(&ep, chain_id);

            assert_eq!(
                mempool
                    .add(UserOperation::from_user_operation_signed(uo_hash, uo.clone()))
                    .unwrap(),
                uo_hash
            );
            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap().user_operation, uo);
        }

        assert_eq!(mempool.get_all().unwrap().len(), 7);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[1]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 3);

        assert_eq!(mempool.remove(&uo_hash).unwrap(), true);
        assert_eq!(mempool.remove(&H256::random().into()).unwrap(), false);

        assert_eq!(mempool.get_all().unwrap().len(), 6);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 2);

        assert_eq!(mempool.clear(), ());

        assert_eq!(mempool.get_all().unwrap().len(), 0);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 0);

        for i in 0..3 {
            uo = UserOperationSigned {
                sender: senders[2],
                nonce: U256::from(i),
                max_priority_fee_per_gas: U256::from(i + 1),
                ..UserOperationSigned::random()
            };
            uo_hash = uo.hash(&ep, chain_id);

            assert_eq!(
                mempool
                    .add(UserOperation::from_user_operation_signed(uo_hash, uo.clone()))
                    .unwrap(),
                uo_hash
            );
        }

        let sorted = mempool.get_sorted().unwrap();
        assert_eq!(sorted[0].max_priority_fee_per_gas, U256::from(3));
        assert_eq!(sorted[1].max_priority_fee_per_gas, U256::from(2));
        assert_eq!(sorted[2].max_priority_fee_per_gas, U256::from(1));
        assert_eq!(sorted.len(), 3);
        assert_eq!(mempool.clear(), ());

        uo = UserOperationSigned {
            sender: Address::random(),
            nonce: U256::from(0),
            max_priority_fee_per_gas: U256::from(1),
            ..UserOperationSigned::random()
        };
        uo_hash = uo.hash(&ep, chain_id);
        assert_eq!(
            mempool.add(UserOperation::from_user_operation_signed(uo_hash, uo.clone())).unwrap(),
            uo_hash
        );
        let code_hashes = vec![CodeHash { address: Address::random(), hash: H256::random() }];
        mempool.set_code_hashes(&uo_hash, code_hashes.clone()).unwrap();

        assert!(mempool.has_code_hashes(&uo_hash).unwrap());

        let code_hashes_get = mempool.get_code_hashes(&uo_hash).unwrap();
        assert_eq!(code_hashes, code_hashes_get);
    }

    pub fn reputation_test_case(mut reputation: Reputation) {
        let mut addrs: Vec<Address> = vec![];

        for _ in 0..5 {
            let addr = Address::random();
            assert_eq!(
                reputation.get(&addr).unwrap(),
                ReputationEntry {
                    address: addr,
                    uo_seen: 0,
                    uo_included: 0,
                    status: Status::OK.into(),
                }
            );
            addrs.push(addr);
        }

        assert_eq!(reputation.add_whitelist(&addrs[2]), true);
        assert_eq!(reputation.add_blacklist(&addrs[1]), true);

        assert_eq!(reputation.is_whitelist(&addrs[2]), true);
        assert_eq!(reputation.is_whitelist(&addrs[1]), false);
        assert_eq!(reputation.is_blacklist(&addrs[1]), true);
        assert_eq!(reputation.is_blacklist(&addrs[2]), false);

        assert_eq!(reputation.remove_whitelist(&addrs[2]), true);
        assert_eq!(reputation.remove_whitelist(&addrs[1]), false);
        assert_eq!(reputation.remove_blacklist(&addrs[1]), true);
        assert_eq!(reputation.remove_blacklist(&addrs[2]), false);

        assert_eq!(reputation.add_whitelist(&addrs[2]), true);
        assert_eq!(reputation.add_blacklist(&addrs[1]), true);

        assert_eq!(Status::from(reputation.get_status(&addrs[2]).unwrap()), Status::OK);
        assert_eq!(Status::from(reputation.get_status(&addrs[1]).unwrap()), Status::BANNED);
        assert_eq!(Status::from(reputation.get_status(&addrs[3]).unwrap()), Status::OK);

        assert_eq!(reputation.increment_seen(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());

        assert_eq!(reputation.increment_included(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_included(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_included(&addrs[3]).unwrap(), ());

        assert_eq!(reputation.update_handle_ops_reverted(&addrs[3]).unwrap(), ());

        for _ in 0..250 {
            assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        }
        assert_eq!(Status::from(reputation.get_status(&addrs[3]).unwrap()), Status::THROTTLED);

        for _ in 0..500 {
            assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        }
        assert_eq!(Status::from(reputation.get_status(&addrs[3]).unwrap()), Status::BANNED);
    }
}
