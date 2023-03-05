#[cfg(test)]
pub mod tests {
    use std::fmt::Debug;

    use ethers::types::{Address, H256, U256};

    use crate::{
        types::user_operation::{UserOperation, UserOperationHash},
        uopool::Mempool,
    };

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

            user_operation_hash = mempool
                .add(user_operation.clone(), &entry_point, &chain_id)
                .unwrap();
        }

        let sorted = mempool.get_sorted(2).unwrap();
        assert_eq!(sorted[0].max_priority_fee_per_gas, U256::from(3));
        assert_eq!(sorted[1].max_priority_fee_per_gas, U256::from(2));
        assert_eq!(sorted.len(), 2);

        let sorted = mempool.get_sorted(5).unwrap();
        assert_eq!(sorted[0].max_priority_fee_per_gas, U256::from(3));
        assert_eq!(sorted[1].max_priority_fee_per_gas, U256::from(2));
        assert_eq!(sorted[2].max_priority_fee_per_gas, U256::from(1));
        assert_eq!(sorted.len(), 3);
    }
}
