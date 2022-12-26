use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
};

use crate::{
    chain::gas::{self, Overhead},
    types::user_operation::UserOperation,
    uopool::services::uopool::UoPoolService,
};

#[derive(Debug, PartialEq, Eq)]
pub enum BadUserOperationError {
    SenderOrInitCode {
        sender: Address,
        init_code: Bytes,
    },
    HighVerificationGasLimit {
        verification_gas_limit: U256,
    },
    LowPreVerificationGas {
        pre_verification_gas: U256,
    },
    InvalidPaymasterAndData {
        paymaster_and_data: Bytes,
    },
    LowCallGasLimit {
        call_gas_limit: U256,
        non_zero_call_value: U256,
    },
    HighMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        max_fee_per_gas: U256,
    },
    LowMaxFeePerGas {
        max_fee_per_gas: U256,
        max_fee_per_gas_estimated: U256,
    },
}

#[derive(Debug)]
pub enum UserOperationSanityCheckError<M: Middleware> {
    SanityCheckError(BadUserOperationError),
    InternalError(anyhow::Error),
    MiddlewareError(M::Error),
}

impl<M: Middleware> From<anyhow::Error> for UserOperationSanityCheckError<M> {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSanityCheckError::InternalError(e)
    }
}

impl<M: Middleware + 'static> UoPoolService<M> {
    async fn sender_or_init_code(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        let code = self
            .eth_provider
            .get_code(user_operation.sender, None)
            .await
            .map_err(UserOperationSanityCheckError::MiddlewareError)?;
        if (code.is_empty() && user_operation.init_code.is_empty())
            || (!code.is_empty() && !user_operation.init_code.is_empty())
        {
            return Err(UserOperationSanityCheckError::SanityCheckError(
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
        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)
        if user_operation.verification_gas_limit > self.max_verification_gas {
            return Err(UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::HighVerificationGasLimit {
                    verification_gas_limit: user_operation.verification_gas_limit,
                },
            ));
        }

        let gas_overhead = Overhead::default();
        if user_operation.pre_verification_gas
            < gas_overhead.calculate_pre_verification_gas(user_operation)
        {
            return Err(UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowPreVerificationGas {
                    pre_verification_gas: user_operation.pre_verification_gas,
                },
            ));
        }

        Ok(())
    }

    async fn call_gas_limit(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        // The callgas is at least the cost of a CALL with non-zero value.
        if user_operation.call_gas_limit < gas::non_zero_value_call() {
            return Err(UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowCallGasLimit {
                    call_gas_limit: user_operation.call_gas_limit,
                    non_zero_call_value: gas::non_zero_value_call(),
                },
            ));
        }

        Ok(())
    }

    async fn max_fee_per_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.
        let (max_fee_per_gas_estimated, _) = self
            .eth_provider
            .estimate_eip1559_fees(None)
            .await
            .map_err(UserOperationSanityCheckError::MiddlewareError)?;

        if user_operation.max_fee_per_gas < max_fee_per_gas_estimated {
            return Err(UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowMaxFeePerGas {
                    max_fee_per_gas: user_operation.max_fee_per_gas,
                    max_fee_per_gas_estimated,
                },
            ));
        }

        if user_operation.max_priority_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::HighMaxPriorityFeePerGas {
                    max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
                    max_fee_per_gas: user_operation.max_fee_per_gas,
                },
            ));
        }

        Ok(())
    }

    async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSanityCheckError<M>> {
        self.sender_or_init_code(user_operation).await?;
        self.verification_gas(user_operation).await?;

        // condition 3
        // The paymaster is either the zero address or is a contract which (i) currently has nonempty code on chain, (ii) has registered with sufficient stake value, (iii) has a sufficient deposit to pay for the UserOperation, and (v) is not currently banned.
        // TODO: implement

        self.call_gas_limit(user_operation).await?;
        self.max_fee_per_gas(user_operation).await?;

        // condition 6
        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch.
        // TODO: implement
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::uopool::UserOperationPool;
    use ethers::providers::Provider;
    use std::{str::FromStr, sync::Arc};

    use super::*;

    #[tokio::test]
    async fn user_operation_validation() {
        let uo_pool_service = UoPoolService::new(
            Arc::new(UserOperationPool::new()),
            Arc::new(Provider::try_from("https://rpc-mumbai.maticvigil.com/").unwrap()),
            "0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17"
                .parse()
                .unwrap(),
            U256::from(1500000),
        );

        let user_operation_valid = UserOperation {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xe19e9755942bb0bd0cccce25b1742596b8a8250b3bf2c3e70000000000000000000000001d9a2cb3638c2fc8bf9c01d088b79e75cd188b17000000000000000000000000789d9058feecf1948af429793e7f1eb4a75db2220000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000ab7e2cbfcfb6a5f33a75ad745c3e5fb48d689b5400000000000000000000000000000000000000000000000002c68af0bb14000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(21900),
            verification_gas_limit: U256::from(1218343),
            pre_verification_gas: U256::from(50768),
            max_fee_per_gas: U256::from(3501638950 as u64),
            max_priority_fee_per_gas: U256::from(2551157264 as u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xb5a4efa90d560f95b508e6b0e7c2dc17a7e86928af551175fe2d9f6a1bd79a604e8a83a391d25c4b3dce56a0a1549c5f40d1a08c3f4b80982556efa768eca7f81c").unwrap(),
        };

        assert_eq!(
            uo_pool_service
                .validate_user_operation(&user_operation_valid)
                .await
                .unwrap(),
            ()
        );

        // sender_or_init_code
        assert_eq!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    sender: "0x6a98c1B9FD763eB693f40C407DC85106eBD74352"
                        .parse()
                        .unwrap(),
                    init_code: Bytes::default(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap(),
            ()
        );
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    init_code: Bytes::default(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheckError(
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
            UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::SenderOrInitCode { .. },
            )
        ));

        // verification_gas
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    verification_gas_limit: U256::from(2000000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheckError(
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
            UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowPreVerificationGas { .. },
            )
        ));

        // condition 3
        // TODO: implement

        // call_gas_limit
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    call_gas_limit: U256::from(12000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowCallGasLimit { .. },
            )
        ));

        // max_fee_per_gas
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    max_fee_per_gas: U256::from(1001638950 as u64),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::LowMaxFeePerGas { .. },
            )
        ));
        assert!(matches!(
            uo_pool_service
                .validate_user_operation(&UserOperation {
                    max_priority_fee_per_gas: U256::from(5501638950 as u64),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            UserOperationSanityCheckError::SanityCheckError(
                BadUserOperationError::HighMaxPriorityFeePerGas { .. },
            )
        ));

        // condition 6
        // TODO: implement
    }
}
