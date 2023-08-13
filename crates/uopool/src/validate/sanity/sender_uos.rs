use crate::{
    utils::calculate_valid_gas,
    validate::{SanityCheck, SanityHelper},
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{
    consts::entities::ACCOUNT, reputation::StakeInfo, sanity::SanityCheckError, UserOperation,
};

pub struct SenderUos {
    pub max_uos_per_unstaked_sender: usize,
    pub gas_increase_perc: U256,
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for SenderUos {
    /// The [check_user_operation] method implementation that performs the sanity check on the [UserOperation](UserOperation) sender.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// Nothing if the sanity check is successful, otherwise a [SanityCheckError](SanityCheckError) is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError> {
        if helper.mempool.get_number_by_sender(&uo.sender) == 0 {
            return Ok(());
        }

        let uo_prev = helper
            .mempool
            .get_all_by_sender(&uo.sender)
            .iter()
            .find(|uo_prev| uo_prev.nonce == uo.nonce)
            .cloned();

        match uo_prev {
            Some(uo_prev) => {
                if uo.max_fee_per_gas
                    >= calculate_valid_gas(uo_prev.max_fee_per_gas, self.gas_increase_perc)
                    && uo.max_priority_fee_per_gas
                        >= calculate_valid_gas(
                            uo_prev.max_priority_fee_per_gas,
                            self.gas_increase_perc,
                        )
                {
                    return Ok(());
                } else {
                    Err(SanityCheckError::SenderVerification {
                        sender: uo.sender,
                        message: "couldn't replace user operation (gas increase too low)".into(),
                    })
                }
            }
            None => {
                if helper.mempool.get_number_by_sender(&uo.sender)
                    >= self.max_uos_per_unstaked_sender
                {
                    let info = helper
                        .entry_point
                        .get_deposit_info(&uo.sender)
                        .await
                        .map_err(|_| SanityCheckError::UnknownError {
                            message: "Couldn't retrieve deposit info from entry point".to_string(),
                        })?;
                    match helper.reputation.verify_stake(
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

                Ok(())
            }
        }
    }
}
