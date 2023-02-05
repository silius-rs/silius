use crate::{
    chain::gas::{self, Overhead},
    types::{
        sanity_check::{BadUserOperationError, SanityCheckResult},
        user_operation::UserOperation,
    },
    uopool::{services::uopool::UoPoolService, MempoolId},
};
use ethers::{
    prelude::gas_oracle::GasOracle,
    providers::Middleware,
    types::Address,
};

impl<M: Middleware + 'static> UoPoolService<M> {
    async fn sender_or_init_code(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        let code = self
            .eth_provider
            .get_code(user_operation.sender, None)
            .await
            .map_err(|error| BadUserOperationError::Middleware(error))?;
        if (code.is_empty() && user_operation.init_code.is_empty())
            || (!code.is_empty() && !user_operation.init_code.is_empty())
        {
            return Err(BadUserOperationError::SenderOrInitCode {
                sender: user_operation.sender,
                init_code: user_operation.init_code.clone(),
            });
        }
        Ok(())
    }

    async fn factory_staked(
        &self,
        user_operation: &UserOperation,
        mempool_id: &MempoolId,
    ) -> Result<(), BadUserOperationError<M>> {
        if !user_operation.init_code.is_empty() {
            let factory_address = if &user_operation.init_code.len() >= &20 {
                Address::from_slice(&user_operation.init_code[0..20])
            } else {
                Address::zero()
            };

            // if let Some(entry_point) = self.entry_points.get(&mempool_id) {
            //     let stake_info = entry_point
            //     .get_stake_info(factory_address)
            //     .await
            //     .map_err(|_| BadUserOperationError::FactoryStaked {
            //         factory: factory_address,
            //     })?;

            //     let reputations = self.reputations.read();

            //     if let Some(reputation) = reputations.get(&mempool_id) {
            //         reputation
            //         .verify_stake("factory", None)
            //         .await
            //         .is_ok();
            //         self.sanity_check_results
            //                 .write()
            //                 .entry(user_operation.hash(&entry_point.address(), &self.chain_id))
            //                 .or_insert_with(Default::default)
            //                 .insert(SanityCheckResult::FactoryStaked);
            //     }
            // }

            return Err(BadUserOperationError::FactoryStaked {
                factory: factory_address,
            });
        }

        Ok(())
    }

    fn verification_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        if user_operation.verification_gas_limit > self.max_verification_gas {
            return Err(BadUserOperationError::HighVerificationGasLimit {
                verification_gas_limit: user_operation.verification_gas_limit,
                max_verification_gas: self.max_verification_gas,
            });
        }

        let calculated_pre_verification_gas =
            Overhead::default().calculate_pre_verification_gas(user_operation);
        if user_operation.pre_verification_gas < calculated_pre_verification_gas {
            return Err(BadUserOperationError::LowPreVerificationGas {
                pre_verification_gas: user_operation.pre_verification_gas,
                calculated_pre_verification_gas,
            });
        }

        Ok(())
    }

    fn call_gas_limit(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        let non_zero_value_call = gas::non_zero_value_call();
        if user_operation.call_gas_limit < non_zero_value_call {
            return Err(BadUserOperationError::LowCallGasLimit {
                call_gas_limit: user_operation.call_gas_limit,
                non_zero_value_call,
            });
        }

        Ok(())
    }

    async fn max_fee_per_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        let base_fee_estimation = self
            .gas_oracle
            .fetch()
            .await
            .map_err(|error| BadUserOperationError::GasOracleError(error))?;

        if user_operation.max_priority_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(BadUserOperationError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
                max_fee_per_gas: user_operation.max_fee_per_gas,
            });
        }

        if base_fee_estimation + user_operation.max_priority_fee_per_gas
            > user_operation.max_fee_per_gas
        {
            return Err(BadUserOperationError::LowMaxFeePerGas {
                max_fee_per_gas: user_operation.max_fee_per_gas,
                max_fee_per_gas_estimated: base_fee_estimation
                    + user_operation.max_priority_fee_per_gas,
            });
        }

        Ok(())
    }

    pub async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
        mempool_id: &MempoolId,
    ) -> Result<(), BadUserOperationError<M>> {
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        self.sender_or_init_code(user_operation).await?;

        // If initCode is not empty, parse its first 20 bytes as a factory address. Record whether the factory is staked, in case the later simulation indicates that it needs to be. If the factory accesses global state, it must be staked - see reputation, throttling and banning section for details.
        self.factory_staked(user_operation, mempool_id).await?;

        // simulation checks and enums meces noter kaj je potrebno potem pri simulation preveriti

        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)
        self.verification_gas(user_operation)?;

        // The paymasterAndData is either empty, or start with the paymaster address, which is a contract that (i) currently has nonempty code on chain, (ii) has a sufficient deposit to pay for the UserOperation, and (iii) is not currently banned. During simulation, the paymaster's stake is also checked, depending on its storage usage - see reputation, throttling and banning section for details.
        // TODO: implement

        // The callgas is at least the cost of a CALL with non-zero value.
        self.call_gas_limit(user_operation)?;

        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.
        self.max_fee_per_gas(user_operation).await?;

        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch. A sender is exempt from this rule and may have multiple UserOperations in the pool and in a batch if it is staked (see reputation, throttling and banning section below), but this exception is of limited use to normal accounts.
        // TODO: implement

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        contracts::EntryPoint,
        types::reputation::ReputationEntry,
        uopool::{mempool_id, MempoolBox, MempoolId, ReputationBox},
    };
    use ethers::{
        prelude::gas_oracle::ProviderOracle,
        providers::{Http, Provider},
        types::{Address, Bytes, U256},
    };
    use parking_lot::RwLock;
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use super::*;

    #[tokio::test]
    async fn user_operation_sanity_check() {
        let chain_id = U256::from(5);
        let entry_point = "0x602aB3881Ff3Fa8dA60a8F44Cf633e91bA1FdB69"
            .parse::<Address>()
            .unwrap();
        let mempool_id = mempool_id(&entry_point, &chain_id);
        let eth_provider = Arc::new(Provider::try_from("https://rpc.ankr.com/eth_goerli").unwrap());
        let gas_oracle = Arc::new(ProviderOracle::new(
            Provider::try_from("https://rpc.ankr.com/eth_goerli").unwrap(),
        ));
        let mut entry_points = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
        entry_points.insert(
            mempool_id,
            EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
        );

        let max_priority_fee_per_gas = U256::from(1500000000_u64);
        let max_fee_per_gas = max_priority_fee_per_gas + gas_oracle.fetch().await.unwrap();

        let uo_pool_service = UoPoolService::new(
            Arc::new(entry_points),
            Arc::new(RwLock::new(HashMap::<
                MempoolId,
                MempoolBox<Vec<UserOperation>>,
            >::new())),
            Arc::new(RwLock::new(HashMap::<
                MempoolId,
                ReputationBox<Vec<ReputationEntry>>,
            >::new())),
            eth_provider,
            gas_oracle,
            U256::from(1500000),
            chain_id,
        );

        let user_operation_valid = UserOperation {
            sender: "0xeF5b78898D61b7020A6DB5a39608C4B02f95b50f".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a8000000000000000000000000000000000000000000000000000001861645d91d").unwrap(),
            call_data: Bytes::from_str("0xb61d27f6000000000000000000000000ef5b78898d61b7020a6db5a39608c4b02f95b50f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(22016),
            verification_gas_limit: U256::from(413910),
            pre_verification_gas: U256::from(48480),
            max_fee_per_gas: max_fee_per_gas,
            max_priority_fee_per_gas: max_priority_fee_per_gas,
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };

        // valid user operation
        assert!(uo_pool_service
            .validate_user_operation(&user_operation_valid, &mempool_id)
            .await
            .is_ok());

        // smart contract wallet already deployed
        assert!(uo_pool_service
            .validate_user_operation(
                &UserOperation {
                    sender: "0x6f55C6b12CdF6D77A77bc3b8639Ac77468b3f5e9"
                        .parse()
                        .unwrap(),
                    init_code: Bytes::default(),
                    ..user_operation_valid.clone()
                },
                &mempool_id
            )
            .await
            .is_ok());

        // sender or init_code
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        init_code: Bytes::default(),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::SenderOrInitCode { .. },
        ));

        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        sender: "0x6f55C6b12CdF6D77A77bc3b8639Ac77468b3f5e9"
                            .parse()
                            .unwrap(),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::SenderOrInitCode { .. },
        ));

        // // TODO: implement

        // verification gas
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        verification_gas_limit: U256::from(2000000),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::HighVerificationGasLimit { .. },
        ));

        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        pre_verification_gas: U256::from(25000),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::LowPreVerificationGas { .. },
        ));

        // // TODO: implement

        // call gas
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        call_gas_limit: U256::from(12000),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::LowCallGasLimit { .. },
        ));

        // max fee per gas and max priority fee per gas
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        max_priority_fee_per_gas: U256::from(1500000000_u64 * 3),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::HighMaxPriorityFeePerGas { .. },
        ));

        assert!(matches!(
            uo_pool_service
                .validate_user_operation(
                    &UserOperation {
                        max_fee_per_gas: U256::from(1500000000_u64 + 10),
                        ..user_operation_valid.clone()
                    },
                    &mempool_id
                )
                .await
                .unwrap_err(),
            BadUserOperationError::LowMaxFeePerGas { .. },
        ));

        // TODO: implement
    }
}
