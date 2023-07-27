use crate::{
    utils::{calculate_call_gas_limit, calculate_valid_gas, Overhead},
    UoPool,
};
use silius_contracts::entry_point::EntryPointErr;
use silius_primitives::{
    consts::entities::ACCOUNT,
    get_address,
    reputation::{ReputationStatus, StakeInfo},
    sanity_check::SanityCheckError,
    UserOperation, UserOperationHash,
};
use ethers::{providers::Middleware, types::U256};
use serde::{Deserialize, Serialize};

const MAX_UOS_PER_UNSTAKED_SENDER: usize = 4;
const GAS_INCREASE_PERC: u64 = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct SanityCheckResult {
    pub user_operation_hash: Option<UserOperationHash>,
}

impl<M: Middleware + 'static> UoPool<M> {
    async fn sender_or_init_code(&self, uo: &UserOperation) -> Result<(), SanityCheckError> {
        let code = self.eth_client.get_code(uo.sender, None).await?;
        if (code.is_empty() && uo.init_code.is_empty())
            || (!code.is_empty() && !uo.init_code.is_empty())
        {
            return Err(SanityCheckError::SenderOrInitCode {
                sender: uo.sender,
                init_code: uo.init_code.clone(),
            });
        }
        Ok(())
    }

    fn verification_gas(&self, uo: &UserOperation) -> Result<(), SanityCheckError> {
        if uo.verification_gas_limit > self.max_verification_gas {
            return Err(SanityCheckError::HighVerificationGasLimit {
                verification_gas_limit: uo.verification_gas_limit,
                max_verification_gas: self.max_verification_gas,
            });
        }

        let pre_gas = Overhead::default().calculate_pre_verification_gas(uo);
        if uo.pre_verification_gas < pre_gas {
            return Err(SanityCheckError::LowPreVerificationGas {
                pre_verification_gas: uo.pre_verification_gas,
                pre_verification_gas_expected: pre_gas,
            });
        }

        Ok(())
    }

    async fn verify_paymaster(&self, uo: &UserOperation) -> Result<(), SanityCheckError> {
        if !uo.paymaster_and_data.is_empty() {
            if let Some(addr) = get_address(&uo.paymaster_and_data) {
                let code = self.eth_client.get_code(addr, None).await?;

                if !code.is_empty() {
                    let deposit_info =
                        self.entry_point
                            .get_deposit_info(&addr)
                            .await
                            .map_err(|_| SanityCheckError::UnknownError {
                                message: "Couldn't retrieve deposit info from entry point"
                                    .to_string(),
                            })?;

                    if U256::from(deposit_info.deposit) >= uo.max_fee_per_gas
                        && self.reputation.get_status(&addr) != ReputationStatus::BANNED
                    {
                        return Ok(());
                    }
                }
            }

            return Err(SanityCheckError::PaymasterVerification {
                paymaster_and_data: uo.paymaster_and_data.clone(),
            });
        }

        Ok(())
    }

    async fn call_gas_limit(&self, uo: &UserOperation) -> Result<(), SanityCheckError> {
        let exec_res = match self.entry_point.simulate_handle_op(uo.clone()).await {
            Ok(res) => res,
            Err(err) => {
                return Err(match err {
                    EntryPointErr::FailedOp(f) => {
                        SanityCheckError::Validation { message: f.reason }
                    }
                    _ => SanityCheckError::UnknownError {
                        message: format!("{err:?}"),
                    },
                })
            }
        };

        let base_fee_per_gas =
            self.base_fee_per_gas()
                .await
                .map_err(|err| SanityCheckError::UnknownError {
                    message: err.to_string(),
                })?;
        let call_gas_limit = calculate_call_gas_limit(
            exec_res.paid,
            exec_res.pre_op_gas,
            uo.max_fee_per_gas
                .min(uo.max_priority_fee_per_gas + base_fee_per_gas),
        );

        if uo.call_gas_limit >= call_gas_limit {
            return Ok(());
        }

        Err(SanityCheckError::LowCallGasLimit {
            call_gas_limit: uo.call_gas_limit,
            call_gas_limit_expected: call_gas_limit,
        })
    }

    async fn max_fee_per_gas(&self, uo: &UserOperation) -> Result<(), SanityCheckError> {
        if uo.max_priority_fee_per_gas > uo.max_fee_per_gas {
            return Err(SanityCheckError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
                max_fee_per_gas: uo.max_fee_per_gas,
            });
        }

        let base_fee =
            self.base_fee_per_gas()
                .await
                .map_err(|err| SanityCheckError::UnknownError {
                    message: err.to_string(),
                })?;

        if base_fee > uo.max_fee_per_gas {
            return Err(SanityCheckError::LowMaxFeePerGas {
                max_fee_per_gas: uo.max_fee_per_gas,
                base_fee,
            });
        }

        if uo.max_priority_fee_per_gas < self.min_priority_fee_per_gas {
            return Err(SanityCheckError::LowMaxPriorityFeePerGas {
                max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
                min_priority_fee_per_gas: self.min_priority_fee_per_gas,
            });
        }

        Ok(())
    }

    async fn verify_sender_user_operations(
        &self,
        uo: &UserOperation,
    ) -> Result<Option<UserOperationHash>, SanityCheckError> {
        if self.mempool.get_number_by_sender(&uo.sender) == 0 {
            return Ok(None);
        }

        let uo_prev = self
            .mempool
            .get_all_by_sender(&uo.sender)
            .iter()
            .find(|uo_prev| uo_prev.nonce == uo.nonce)
            .cloned();

        match uo_prev {
            Some(uo_prev) => {
                if uo.max_fee_per_gas
                    >= calculate_valid_gas(uo_prev.max_fee_per_gas, GAS_INCREASE_PERC.into())
                    && uo.max_priority_fee_per_gas
                        >= calculate_valid_gas(
                            uo_prev.max_priority_fee_per_gas,
                            GAS_INCREASE_PERC.into(),
                        )
                {
                    Ok(Some(uo_prev.hash(
                        &self.entry_point.address(),
                        &self.chain.id().into(),
                    )))
                } else {
                    Err(SanityCheckError::SenderVerification {
                        sender: uo.sender,
                        message: "couldn't replace user operation (gas increase too low)".into(),
                    })
                }
            }
            None => {
                if self.mempool.get_number_by_sender(&uo.sender) >= MAX_UOS_PER_UNSTAKED_SENDER {
                    let info = self
                        .entry_point
                        .get_deposit_info(&uo.sender)
                        .await
                        .map_err(|_| SanityCheckError::UnknownError {
                            message: "Couldn't retrieve deposit info from entry point".to_string(),
                        })?;
                    match self.reputation.verify_stake(
                        ACCOUNT,
                        Some(StakeInfo {
                            address: uo.sender,
                            stake: U256::from(info.stake),
                            unstake_delay: U256::from(info.unstake_delay_sec),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(_) => {
                            return Err(SanityCheckError::SenderVerification {
                                sender: uo.sender,
                                message: "has too many user operations in the mempool".into(),
                            });
                        }
                    }
                }

                Ok(None)
            }
        }
    }

    pub async fn check_user_operation(
        &self,
        uo: &UserOperation,
    ) -> Result<SanityCheckResult, SanityCheckError> {
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        self.sender_or_init_code(uo).await?;

        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)
        self.verification_gas(uo)?;

        // The paymasterAndData is either empty, or start with the paymaster address, which is a contract that (i) currently has nonempty code on chain, (ii) has a sufficient deposit to pay for the UserOperation, and (iii) is not currently banned. During simulation, the paymaster's stake is also checked, depending on its storage usage - see reputation, throttling and banning section for details.
        self.verify_paymaster(uo).await?;

        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.
        self.max_fee_per_gas(uo).await?;

        // The callgas is at least the cost of a CALL with non-zero value.
        self.call_gas_limit(uo).await?;

        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch. A sender is exempt from this rule and may have multiple UserOperations in the pool and in a batch if it is staked (see reputation, throttling and banning section below), but this exception is of limited use to normal accounts.
        let user_operation_prev_hash = self.verify_sender_user_operations(uo).await?;

        Ok(SanityCheckResult {
            user_operation_hash: user_operation_prev_hash,
        })
    }
}
