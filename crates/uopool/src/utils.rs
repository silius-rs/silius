use ethers::types::{u256_from_f64_saturating, Address, H256, U256};
use silius_primitives::{simulation::CodeHash, UserOperation};
use std::{collections::HashMap, ops::Deref};

pub fn equal_code_hashes(hashes: &Vec<CodeHash>, hashes_prev: &Vec<CodeHash>) -> bool {
    if hashes_prev.len() != hashes.len() {
        return false;
    }

    let hashes_map = hashes
        .iter()
        .map(|h: &CodeHash| (h.address, h.hash))
        .collect::<HashMap<Address, H256>>();

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

/// Struct to calculate the pre-verification gas of a [UserOperation](UserOperation)
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
    /// Calculates the pre-verification gas of a [UserOperation](UserOperation)
    /// The function first packs the [UserOperation](UserOperation) by calling the [pack](UserOperation::pack) method, then extracts the call data for gas calculation.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to calculate the pre-verification gas for
    ///
    /// # Returns
    /// The pre-verification gas of the [UserOperation](UserOperation)
    pub fn calculate_pre_verification_gas(&self, uo: &UserOperation) -> U256 {
        let uo_pack = uo.pack();
        let call_data: U256 = U256::from(
            uo_pack
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
        let len_in_word = ((uo_pack.len() + 31) as f64) / 32_f64;
        u256_from_f64_saturating(
            (self.fixed.as_u128() as f64) / (self.bundle_size.as_u128() as f64)
                + ((call_data + self.per_user_op).as_u128() as f64)
                + (self.per_user_op_word.as_u128() as f64) * len_in_word,
        )
    }
}

/// Helper function to calculate the valid gas of a [UserOperation](UserOperation)
/// The function is invoked by the [check_valid_gas](crates::uopool::validate::sanity::check_valid_gas) method.
///
/// # Arguments
/// `gas_price` - The gas price
/// `gas_incr_perc` - The gas increase percentage
///
/// # Returns
/// The valid gas of the [UserOperation](UserOperation)
pub fn calculate_valid_gas(gas_price: U256, gas_incr_perc: U256) -> U256 {
    let gas_price = gas_price.as_u64() as f64;
    let gas_incr_perc = gas_incr_perc.as_u64() as f64;
    ((gas_price * (1.0 + gas_incr_perc / 100.0)).ceil() as u64).into()
}

/// Helper function to calculate the call gas limit of a [UserOperation](UserOperation)
/// The function is invoked by the [estimate_user_operation_gas](crates::uopool::estimate::estimate_user_operation_gas) method.
///
/// # Arguments
/// `paid` - The paid gas
/// `pre_op_gas` - The pre-operation gas
/// `fee_per_gas` - The fee per gas
///
/// # Returns
/// The call gas limit of the [UserOperation](UserOperation)
pub fn calculate_call_gas_limit(paid: U256, pre_op_gas: U256, fee_per_gas: U256) -> U256 {
    paid / fee_per_gas - pre_op_gas + Overhead::default().fixed
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{mempool::Mempool, Reputation};
    use ethers::types::{Address, Bytes, H256, U256};
    use silius_primitives::{
        reputation::{
            ReputationEntry, Status, BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
        },
        UserOperation, UserOperationHash,
    };
    use std::fmt::Debug;

    #[test]
    fn pre_verification_gas_calculation() {
        let gas_oh = Overhead::default();
        let uo = UserOperation {
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

    pub fn mempool_test_case<T>(mut mempool: T, not_found_error_message: &str)
    where
        T: Mempool<UserOperations = Vec<UserOperation>> + Debug,
        T::Error: Debug + ToString,
    {
        let ep = Address::random();
        let chain_id = U256::from(5);
        let senders = vec![Address::random(), Address::random(), Address::random()];

        let mut uo: UserOperation;
        let mut uo_hash: UserOperationHash = Default::default();
        for i in 0..2 {
            uo = UserOperation {
                sender: senders[0],
                nonce: U256::from(i),
                ..UserOperation::random()
            };
            uo_hash = mempool.add(uo.clone(), &ep, &chain_id).unwrap();

            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap(), uo);

            uo = UserOperation {
                sender: senders[1],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            uo_hash = mempool.add(uo.clone(), &ep, &chain_id).unwrap();

            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap(), uo);
        }

        for i in 0..3 {
            uo = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                ..UserOperation::random()
            };

            uo_hash = mempool.add(uo.clone(), &ep, &chain_id).unwrap();

            assert_eq!(mempool.get(&uo_hash).unwrap().unwrap(), uo);
        }

        assert_eq!(mempool.get_all().len(), 7);
        assert_eq!(mempool.get_all_by_sender(&senders[0]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[1]).len(), 2);
        assert_eq!(mempool.get_all_by_sender(&senders[2]).len(), 3);

        assert_eq!(mempool.remove(&uo_hash).unwrap(), ());
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
            uo = UserOperation {
                sender: senders[2],
                nonce: U256::from(i),
                max_priority_fee_per_gas: U256::from(i + 1),
                ..UserOperation::random()
            };

            mempool.add(uo.clone(), &ep, &chain_id).unwrap();
        }

        let sorted = mempool.get_sorted().unwrap();
        assert_eq!(sorted[0].max_priority_fee_per_gas, U256::from(3));
        assert_eq!(sorted[1].max_priority_fee_per_gas, U256::from(2));
        assert_eq!(sorted[2].max_priority_fee_per_gas, U256::from(1));
        assert_eq!(sorted.len(), 3);
    }

    pub fn reputation_test_case<T>(mut reputation: T)
    where
        T: Reputation<ReputationEntries = Vec<ReputationEntry>> + Debug,
        T::Error: Debug + ToString,
    {
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(1),
            U256::from(0),
        );

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

        assert_eq!(
            Status::from(reputation.get_status(&addrs[2]).unwrap()),
            Status::OK
        );
        assert_eq!(
            Status::from(reputation.get_status(&addrs[1]).unwrap()),
            Status::BANNED
        );
        assert_eq!(
            Status::from(reputation.get_status(&addrs[3]).unwrap()),
            Status::OK
        );

        assert_eq!(reputation.increment_seen(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());

        assert_eq!(reputation.increment_included(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_included(&addrs[2]).unwrap(), ());
        assert_eq!(reputation.increment_included(&addrs[3]).unwrap(), ());

        assert_eq!(
            reputation.update_handle_ops_reverted(&addrs[3]).unwrap(),
            ()
        );

        for _ in 0..250 {
            assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        }
        assert_eq!(
            Status::from(reputation.get_status(&addrs[3]).unwrap()),
            Status::THROTTLED
        );

        for _ in 0..500 {
            assert_eq!(reputation.increment_seen(&addrs[3]).unwrap(), ());
        }
        assert_eq!(
            Status::from(reputation.get_status(&addrs[3]).unwrap()),
            Status::BANNED
        );
    }
}
