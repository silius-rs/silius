use crate::{
    chain::gas::{self, Overhead},
    types::user_operation::UserOperation,
    uopool::services::uopool::UoPoolService,
};
use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
};
use std::fmt;

pub const SANITY_CHECK_ERROR_CODE: i32 = -32602;

#[derive(Debug, PartialEq, Eq)]
pub enum BadUserOperationError {
    SenderOrInitCode {
        sender: Address,
        init_code: Bytes,
    },
    HighVerificationGasLimit {
        verification_gas_limit: U256,
        max_verification_gas: U256,
    },
    LowPreVerificationGas {
        pre_verification_gas: U256,
        calculated_pre_verification_gas: U256,
    },
    InvalidPaymasterAndData {
        paymaster_and_data: Bytes,
    },
    LowCallGasLimit {
        call_gas_limit: U256,
        non_zero_value_call: U256,
    },
    LowMaxFeePerGas {
        max_fee_per_gas: U256,
        max_fee_per_gas_estimated: U256,
    },
    HighMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        max_fee_per_gas: U256,
    },
}

impl fmt::Display for BadUserOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            BadUserOperationError::SenderOrInitCode { sender, init_code } => write!(
                f,
                "Either the sender {:?} is an existing contract, or the initCode {:?} is not empty (but not both)",
                sender, init_code
            ),
            BadUserOperationError::HighVerificationGasLimit {
                verification_gas_limit,
                max_verification_gas,
            } => write!(
                f,
                "Verification gas limit {} is higher than max verification gas {}",
                verification_gas_limit,
                max_verification_gas
            ),
            BadUserOperationError::LowPreVerificationGas {
                pre_verification_gas,
                calculated_pre_verification_gas
            } => write!(
                f,
                "Pre-verification gas {} is lower than calculated pre-verification gas {}",
                pre_verification_gas,
                calculated_pre_verification_gas
            ),
            BadUserOperationError::InvalidPaymasterAndData { paymaster_and_data } => write!(
                f,
                "Paymaster and data {:?} are inconsistent",
                paymaster_and_data
            ),
            BadUserOperationError::LowCallGasLimit {
                call_gas_limit,
                non_zero_value_call,
            } => write!(
                f,
                "Call gas limit {} is lower than CALL non-zero value {}",
                call_gas_limit, non_zero_value_call
            ),
            BadUserOperationError::LowMaxFeePerGas {
                max_fee_per_gas,
                max_fee_per_gas_estimated,
            } => write!(
                f,
                "Max fee per gas {} is lower than estimated max fee per gas {}",
                max_fee_per_gas, max_fee_per_gas_estimated
            ),
            BadUserOperationError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                max_fee_per_gas,
            } => write!(
                f,
                "Max priority fee per gas {} is higher than max fee per gas {}",
                max_priority_fee_per_gas, max_fee_per_gas
            ),
        };
    }
}

#[derive(Debug)]
pub enum UserOperationSanityCheckError<M: Middleware> {
    SanityCheck(BadUserOperationError),
    Internal(anyhow::Error),
    Middleware(M::Error),
}

impl<M: Middleware> From<anyhow::Error> for UserOperationSanityCheckError<M> {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSanityCheckError::Internal(e)
    }
}

impl<M: Middleware + 'static> UoPoolService<M> {
    async fn sender_or_init_code(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        let code = self
            .eth_provider
            .get_code(user_operation.sender, None)
            .await
            .map_err(UserOperationSanityCheckError::Middleware)?;
        if (code.is_empty() && user_operation.init_code.is_empty())
            || (!code.is_empty() && !user_operation.init_code.is_empty())
        {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::SenderOrInitCode {
                    sender: user_operation.sender,
                    init_code: user_operation.init_code.clone(),
                },
            ));
        }
        Ok(())
    }

    async fn verification_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        if user_operation.verification_gas_limit > self.max_verification_gas {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::HighVerificationGasLimit {
                    verification_gas_limit: user_operation.verification_gas_limit,
                    max_verification_gas: self.max_verification_gas,
                },
            ));
        }

        let calculated_pre_verification_gas =
            Overhead::default().calculate_pre_verification_gas(user_operation);
        if user_operation.pre_verification_gas < calculated_pre_verification_gas {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowPreVerificationGas {
                    pre_verification_gas: user_operation.pre_verification_gas,
                    calculated_pre_verification_gas,
                },
            ));
        }

        Ok(())
    }

    async fn call_gas_limit(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        let non_zero_value_call = gas::non_zero_value_call();
        if user_operation.call_gas_limit < non_zero_value_call {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowCallGasLimit {
                    call_gas_limit: user_operation.call_gas_limit,
                    non_zero_value_call,
                },
            ));
        }

        Ok(())
    }

    async fn max_fee_per_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        let (max_fee_per_gas_estimated, _) = self
            .eth_provider
            .estimate_eip1559_fees(None)
            .await
            .map_err(UserOperationSanityCheckError::Middleware)?;

        if user_operation.max_fee_per_gas < max_fee_per_gas_estimated {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowMaxFeePerGas {
                    max_fee_per_gas: user_operation.max_fee_per_gas,
                    max_fee_per_gas_estimated,
                },
            ));
        }

        if user_operation.max_priority_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::HighMaxPriorityFeePerGas {
                    max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
                    max_fee_per_gas: user_operation.max_fee_per_gas,
                },
            ));
        }

        Ok(())
    }

    pub async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        // condition 1
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        self.sender_or_init_code(user_operation).await?;

        // condition 2
        // If initCode is not empty, parse its first 20 bytes as a factory address. Record whether the factory is staked, in case the later simulation indicates that it needs to be. If the factory accesses global state, it must be staked - see reputation, throttling and banning section for details.
        // TODO: implement

        // condition 3
        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)
        self.verification_gas(user_operation).await?;

        // condition 4
        // The paymasterAndData is either empty, or start with the paymaster address, which is a contract that (i) currently has nonempty code on chain, (ii) has a sufficient deposit to pay for the UserOperation, and (iii) is not currently banned. During simulation, the paymaster's stake is also checked, depending on its storage usage - see reputation, throttling and banning section for details.
        // TODO: implement

        // condition 5
        // The callgas is at least the cost of a CALL with non-zero value.
        self.call_gas_limit(user_operation).await?;

        // condition 6
        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.
        self.max_fee_per_gas(user_operation).await?;

        // condition 7
        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch. A sender is exempt from this rule and may have multiple UserOperations in the pool and in a batch if it is staked (see reputation, throttling and banning section below), but this exception is of limited use to normal accounts.
        // TODO: implement

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        contracts::EntryPoint,
        uopool::{mempool_id, MempoolBox, MempoolId},
    };
    use ethers::providers::{Http, Provider};
    use parking_lot::RwLock;
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn user_operation_validation() {
        let chain_id = U256::from(5);
        let entry_point = "0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17"
            .parse::<Address>()
            .unwrap();
        let eth_provider = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let mut entry_points = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
        entry_points.insert(
            mempool_id(entry_point, chain_id),
            EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
        );

        let uo_pool_service = UoPoolService::new(
            Arc::new(RwLock::new(HashMap::<
                MempoolId,
                MempoolBox<Vec<UserOperation>>,
            >::new())),
            Arc::new(entry_points),
            eth_provider,
            U256::from(1500000),
            chain_id,
        );

        let user_operation_valid = UserOperation {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xe19e9755942bb0bd0cccce25b1742596b8a8250b3bf2c3e70000000000000000000000001d9a2cb3638c2fc8bf9c01d088b79e75cd188b17000000000000000000000000789d9058feecf1948af429793e7f1eb4a75db2220000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000ab7e2cbfcfb6a5f33a75ad745c3e5fb48d689b5400000000000000000000000000000000000000000000000002c68af0bb14000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(21900),
            verification_gas_limit: U256::from(1218343),
            pre_verification_gas: U256::from(50768),
            max_fee_per_gas: U256::from(3501638950_u64),
            max_priority_fee_per_gas: U256::from(2551157264_u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xb5a4efa90d560f95b508e6b0e7c2dc17a7e86928af551175fe2d9f6a1bd79a604e8a83a391d25c4b3dce56a0a1549c5f40d1a08c3f4b80982556efa768eca7f81c").unwrap(),
        };

        assert!(uo_pool_service
            .validate_user_operation(&user_operation_valid)
            .await
            .is_ok());

        // condition 1
        assert!(uo_pool_service
            .validate_user_operation(&UserOperation {
                sender: "0x6a98c1B9FD763eB693f40C407DC85106eBD74352"
                    .parse()
                    .unwrap(),
                init_code: Bytes::default(),
                ..user_operation_valid.clone()
            })
            .await
            .is_ok());
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    init_code: Bytes::default(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::SenderOrInitCode { .. },
            )
        ));
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    sender: "0x6a98c1B9FD763eB693f40C407DC85106eBD74352"
                        .parse()
                        .unwrap(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::SenderOrInitCode { .. },
            )
        ));

        // condition 2
        // TODO: implement

        // condition 3
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    verification_gas_limit: U256::from(2000000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::HighVerificationGasLimit { .. },
            )
        ));
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    pre_verification_gas: U256::from(25000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowPreVerificationGas { .. },
            )
        ));

        // condition 4
        // TODO: implement

        // condition 5
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    call_gas_limit: U256::from(12000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowCallGasLimit { .. },
            )
        ));

        // condition 6
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    max_fee_per_gas: U256::from(1001638950_u64),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::LowMaxFeePerGas { .. },
            )
        ));
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    max_priority_fee_per_gas: U256::from(5501638950_u64),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheck(
                BadUserOperationError::HighMaxPriorityFeePerGas { .. },
            )
        ));

        // condition 7
        // TODO: implement
    }
}
