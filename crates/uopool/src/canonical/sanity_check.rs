use aa_bundler_primitives::{
    ReputationStatus, SanityCheckError, StakeInfo, UserOperation, UserOperationHash,
    EXECUTION_ERROR_CODE, SANITY_CHECK_ERROR_CODE,
};
use ethers::{
    providers::Middleware,
    types::{Address, BlockNumber, Bytes, U256},
};
use jsonrpsee::types::error::ErrorCode;

use crate::{
    utils::{calculate_valid_gas, Overhead},
    UoPool,
};

const MAX_UOS_PER_UNSTAKED_SENDER: usize = 4;
const GAS_INCREASE_PERC: u64 = 10;

#[derive(Debug)]
pub enum BadUserOperationError<M: Middleware> {
    SenderOrInitCode {
        sender: Address,
        init_code: Bytes,
    },
    FactoryVerification {
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
    PaymasterVerification {
        paymaster_and_data: Bytes,
    },
    LowCallGasLimit {
        call_gas_limit: U256,
        call_gas_estimation: U256,
    },
    LowMaxFeePerGas {
        max_fee_per_gas: U256,
        max_fee_per_gas_estimated: U256,
    },
    HighMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        max_fee_per_gas: U256,
    },
    LowMaxPriorityFeePerGas {
        max_priority_fee_per_gas: U256,
        min_priority_fee_per_gas: U256,
    },
    SenderVerification {
        sender: Address,
    },
    UserOperationExecution {
        message: String,
    },
    Middleware(M::Error),
    UnknownError {
        error: String,
    },
}

impl<M: Middleware> From<BadUserOperationError<M>> for SanityCheckError {
    fn from(error: BadUserOperationError<M>) -> Self {
        match error {
            BadUserOperationError::SenderOrInitCode { sender, init_code } => {
                SanityCheckError::owned(
                    SANITY_CHECK_ERROR_CODE,
                    format!(
                        "Either the sender {sender} is an existing contract, or the initCode {init_code} is not empty (but not both)",
                    ),
                    None::<bool>,
                )
            },
            BadUserOperationError::FactoryVerification { init_code } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!("Init code {init_code} is not valid (factory check)",),
                None::<bool>,
            ),
            BadUserOperationError::HighVerificationGasLimit {
                verification_gas_limit,
                max_verification_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Verification gas limit {verification_gas_limit} is higher than max verification gas {max_verification_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowPreVerificationGas {
                pre_verification_gas,
                calculated_pre_verification_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Pre-verification gas {pre_verification_gas} is lower than calculated pre-verification gas {calculated_pre_verification_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::PaymasterVerification { paymaster_and_data } => {
                SanityCheckError::owned(
                    SANITY_CHECK_ERROR_CODE,
                    format!(
                        "Paymaster and data {paymaster_and_data} is invalid (paymaster check)",
                    ),
                    None::<bool>,
                )
            },
            BadUserOperationError::LowCallGasLimit {
                call_gas_limit,
                call_gas_estimation,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Call gas limit {call_gas_limit} is lower than call gas estimation {call_gas_estimation}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowMaxFeePerGas {
                max_fee_per_gas,
                max_fee_per_gas_estimated,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max fee per gas {max_fee_per_gas} is lower than estimated max fee per gas {max_fee_per_gas_estimated}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                max_fee_per_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is higher than max fee per gas {max_fee_per_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::LowMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                min_priority_fee_per_gas,
            } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is lower than min priority fee per gas {min_priority_fee_per_gas}",
                ),
                None::<bool>,
            ),
            BadUserOperationError::SenderVerification { sender } => SanityCheckError::owned(
                SANITY_CHECK_ERROR_CODE,
                format!("Sender {sender} is invalid (sender check)",),
                None::<bool>,
            ),
            BadUserOperationError::UserOperationExecution { message } => {
                SanityCheckError::owned(
                    EXECUTION_ERROR_CODE,
                    message,
                    None::<bool>,
                )
            },
            BadUserOperationError::Middleware(_) => {
                SanityCheckError::from(ErrorCode::InternalError)
            },
            BadUserOperationError::UnknownError { error } => {
                SanityCheckError::owned(
                    SANITY_CHECK_ERROR_CODE,
                    error,
                    None::<bool>,
                )
            },
        }
    }
}

#[derive(Debug)]
pub struct SanityCheckResult {
    pub user_operation_hash: Option<UserOperationHash>,
}

impl<M: Middleware + 'static> UoPool<M> {
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

    async fn verify_paymaster(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        if !user_operation.paymaster_and_data.is_empty() {
            let paymaster_address = if user_operation.paymaster_and_data.len() >= 20 {
                Address::from_slice(&user_operation.paymaster_and_data[0..20])
            } else {
                return Err(BadUserOperationError::PaymasterVerification {
                    paymaster_and_data: user_operation.paymaster_and_data.clone(),
                });
            };

            let code = self
                .eth_provider
                .get_code(paymaster_address, None)
                .await
                .map_err(|error| BadUserOperationError::Middleware(error))?;

            if code.is_empty() {
                return Err(BadUserOperationError::PaymasterVerification {
                    paymaster_and_data: user_operation.paymaster_and_data.clone(),
                });
            }

            let deposit_info = self
                .entry_point
                .get_deposit_info(&paymaster_address)
                .await
                .map_err(|_| BadUserOperationError::PaymasterVerification {
                    paymaster_and_data: user_operation.paymaster_and_data.clone(),
                })?;

            if U256::from(deposit_info.deposit) < user_operation.max_fee_per_gas
                || self.reputation.get_status(&paymaster_address) == ReputationStatus::BANNED
            {
                return Err(BadUserOperationError::PaymasterVerification {
                    paymaster_and_data: user_operation.paymaster_and_data.clone(),
                });
            }
        }

        Ok(())
    }

    async fn call_gas_limit(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        let call_gas_estimation = self
            .entry_point
            .estimate_call_gas(user_operation.clone())
            .await
            .map_err(|error| BadUserOperationError::UnknownError {
                error: format!("{error:?}"),
            })?;

        if user_operation.call_gas_limit >= call_gas_estimation {
            return Ok(());
        }

        Err(BadUserOperationError::LowCallGasLimit {
            call_gas_limit: user_operation.call_gas_limit,
            call_gas_estimation,
        })
    }

    async fn max_fee_per_gas(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), BadUserOperationError<M>> {
        if user_operation.max_priority_fee_per_gas > user_operation.max_fee_per_gas {
            return Err(BadUserOperationError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
                max_fee_per_gas: user_operation.max_fee_per_gas,
            });
        }

        let block = self
            .eth_provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|error| BadUserOperationError::Middleware(error))?;

        let base_fee_per_gas = if let Some(block) = block {
            if let Some(base_fee_per_gas) = block.base_fee_per_gas {
                base_fee_per_gas
            } else {
                return Err(BadUserOperationError::UnknownError {
                    error: "Can't get base fee per gas".to_string(),
                });
            }
        } else {
            return Err(BadUserOperationError::UnknownError {
                error: "Can't get base fee per gas".to_string(),
            });
        };

        if base_fee_per_gas + user_operation.max_priority_fee_per_gas
            > user_operation.max_fee_per_gas
        {
            return Err(BadUserOperationError::LowMaxFeePerGas {
                max_fee_per_gas: user_operation.max_fee_per_gas,
                max_fee_per_gas_estimated: base_fee_per_gas
                    + user_operation.max_priority_fee_per_gas,
            });
        }

        if user_operation.max_priority_fee_per_gas < self.min_priority_fee_per_gas {
            return Err(BadUserOperationError::LowMaxPriorityFeePerGas {
                max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
                min_priority_fee_per_gas: self.min_priority_fee_per_gas,
            });
        }

        Ok(())
    }

    async fn verify_sender(
        &self,
        user_operation: &UserOperation,
    ) -> Result<Option<UserOperationHash>, BadUserOperationError<M>> {
        if self.mempool.get_number_by_sender(&user_operation.sender) == 0 {
            return Ok(None);
        }

        let user_operation_prev = self
            .mempool
            .get_all_by_sender(&user_operation.sender)
            .iter()
            .find(|user_operation_prev| user_operation_prev.nonce == user_operation.nonce)
            .cloned();

        match user_operation_prev {
            Some(user_operation_prev) => {
                if user_operation.max_fee_per_gas
                    >= calculate_valid_gas(
                        user_operation_prev.max_fee_per_gas,
                        U256::from(GAS_INCREASE_PERC),
                    )
                    && user_operation.max_priority_fee_per_gas
                        >= calculate_valid_gas(
                            user_operation_prev.max_priority_fee_per_gas,
                            U256::from(GAS_INCREASE_PERC),
                        )
                {
                    Ok(Some(
                        user_operation_prev.hash(&self.entry_point.address(), &self.chain_id),
                    ))
                } else {
                    Err(BadUserOperationError::SenderVerification {
                        sender: user_operation.sender,
                    })
                }
            }
            None => {
                if self.mempool.get_number_by_sender(&user_operation.sender)
                    < MAX_UOS_PER_UNSTAKED_SENDER
                {
                    return Ok(None);
                }

                let deposit_info = self
                    .entry_point
                    .get_deposit_info(&user_operation.sender)
                    .await
                    .map_err(|_| BadUserOperationError::SenderVerification {
                        sender: user_operation.sender,
                    })?;
                match self.reputation.verify_stake(
                    "account",
                    Some(StakeInfo {
                        address: user_operation.sender,
                        stake: U256::from(deposit_info.stake),
                        unstake_delay: U256::from(deposit_info.unstake_delay_sec),
                    }),
                ) {
                    Ok(_) => Ok(None),
                    Err(_) => Err(BadUserOperationError::SenderVerification {
                        sender: user_operation.sender,
                    }),
                }
            }
        }
    }

    pub async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SanityCheckResult, BadUserOperationError<M>> {
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        self.sender_or_init_code(user_operation).await?;

        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)
        self.verification_gas(user_operation)?;

        // The paymasterAndData is either empty, or start with the paymaster address, which is a contract that (i) currently has nonempty code on chain, (ii) has a sufficient deposit to pay for the UserOperation, and (iii) is not currently banned. During simulation, the paymaster's stake is also checked, depending on its storage usage - see reputation, throttling and banning section for details.
        self.verify_paymaster(user_operation).await?;

        // The callgas is at least the cost of a CALL with non-zero value.
        self.call_gas_limit(user_operation).await?;

        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.
        self.max_fee_per_gas(user_operation).await?;

        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch. A sender is exempt from this rule and may have multiple UserOperations in the pool and in a batch if it is staked (see reputation, throttling and banning section below), but this exception is of limited use to normal accounts.
        let user_operation_prev_hash = self.verify_sender(user_operation).await?;

        Ok(SanityCheckResult {
            user_operation_hash: user_operation_prev_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use aa_bundler_contracts::EntryPoint;
    use aa_bundler_primitives::{BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK};
    use ethers::{
        providers::{Http, Provider},
        types::{Address, Bytes, U256},
    };
    use std::{str::FromStr, sync::Arc};

    use crate::{
        memory::{mempool::MemoryMempool, reputation::MemoryReputation},
        reputation::Reputation,
    };

    use super::*;

    #[tokio::test]
    async fn user_operation_sanity_check() {
        let entry_point = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
            .parse::<Address>()
            .unwrap();
        let eth_provider =
            Arc::new(Provider::try_from("https://eth-goerli.public.blastapi.io").unwrap());
        let chain_id = eth_provider.get_chainid().await.unwrap();

        let mut reputation = Box::<MemoryReputation>::default();
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(0),
            U256::from(0),
        );

        let mut uo_pool = UoPool::<Provider<Http>>::new(
            EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
            Box::<MemoryMempool>::default(),
            reputation,
            eth_provider.clone(),
            U256::from(1500000),
            U256::from(2),
            chain_id,
        );

        let max_priority_fee_per_gas = U256::from(1500000000_u64);
        let max_fee_per_gas =
            max_priority_fee_per_gas + eth_provider.get_gas_price().await.unwrap();

        let user_operation_valid = UserOperation {
            sender: "0xeF5b78898D61b7020A6DB5a39608C4B02f95b50f".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a8000000000000000000000000000000000000000000000000000001861645d91d").unwrap(),
            call_data: Bytes::from_str("0xb61d27f6000000000000000000000000ef5b78898d61b7020a6db5a39608c4b02f95b50f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(22016),
            verification_gas_limit: U256::from(413910),
            pre_verification_gas: U256::from(48480),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };

        // valid user operation
        assert!(uo_pool
            .validate_user_operation(&user_operation_valid)
            .await
            .is_ok());

        // TODO: smart contract wallet already deployed

        // sender or init_code
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    init_code: Bytes::default(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::SenderOrInitCode { .. },
        ));
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    sender: "0x6f55C6b12CdF6D77A77bc3b8639Ac77468b3f5e9"
                        .parse()
                        .unwrap(),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::SenderOrInitCode { .. },
        ));

        // verification gas
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    verification_gas_limit: U256::from(2000000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::HighVerificationGasLimit { .. },
        ));
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    pre_verification_gas: U256::from(25000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::LowPreVerificationGas { .. },
        ));

        // paymaster verification
        let user_operation_pv = UserOperation {
            paymaster_and_data: Bytes::from_str("0x8bd4Ec4D7B6D7BC9fFA742A87AA134Bb6806e621")
                .unwrap(),
            ..user_operation_valid.clone()
        };
        assert!(uo_pool
            .validate_user_operation(&user_operation_pv)
            .await
            .is_ok());

        // call gas limit
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    call_gas_limit: U256::from(12000),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::LowCallGasLimit { .. },
        ));

        // max fee per gas and max priority fee per gas
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    max_priority_fee_per_gas: U256::from(max_fee_per_gas + 10),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::HighMaxPriorityFeePerGas { .. },
        ));
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    max_fee_per_gas: U256::from(1500000000_u64 + 10),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::LowMaxFeePerGas { .. },
        ));
        assert!(matches!(
            uo_pool
                .validate_user_operation(&UserOperation {
                    max_priority_fee_per_gas: U256::from(1),
                    ..user_operation_valid.clone()
                })
                .await
                .unwrap_err(),
            BadUserOperationError::LowMaxPriorityFeePerGas { .. },
        ));

        // sender verification
        let user_operation_sv = UserOperation {
            sender: "0x36503f2a3Fc1CF2ED2C6640eBc3632bCbb30EF1D"
                .parse()
                .unwrap(),
            ..user_operation_valid.clone()
        };
        assert_eq!(
            uo_pool
                .mempool
                .add(user_operation_sv.clone(), &entry_point, &chain_id)
                .unwrap(),
            user_operation_sv.hash(&entry_point, &chain_id)
        );
        // TODO: this test is valid if sender is staked
        // assert!(uo_pool
        //     .validate_user_operation(&UserOperation {
        //         nonce: U256::from(1),
        //         ..user_operation_sv.clone()
        //     })
        //     .await
        //     .is_ok());
    }
}
