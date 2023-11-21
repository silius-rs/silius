use crate::{
    mempool::Mempool,
    utils::calculate_valid_gas,
    validate::{SanityCheck, SanityHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    consts::uopool::GAS_INCREASE_PERC, sanity::SanityCheckError, UserOperation,
};
use std::fmt::Debug;

pub struct Sender;

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SanityCheck<M, P, R, E> for Sender
where
    P: Mempool<Error = E> + Send + Sync,
    R: Reputation<Error = E> + Send + Sync,
    E: Debug,
{
    /// The [check_user_operation] method implementation that performs the check for the sender of the [UserOperation](UserOperation).
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
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<(), SanityCheckError> {
        let code = helper
            .entry_point
            .eth_client()
            .get_code(uo.sender, None)
            .await?;

        // check if sender or init code
        if (code.is_empty() && uo.init_code.is_empty())
            || (!code.is_empty() && !uo.init_code.is_empty())
        {
            return Err(SanityCheckError::SenderOrInitCode {
                sender: uo.sender,
                init_code: uo.init_code.clone(),
            });
        }

        // check if prev user operation exists
        if helper.mempool.get_number_by_sender(&uo.sender) == 0 {
            return Ok(());
        }

        let uo_prev = helper
            .mempool
            .get_all_by_sender(&uo.sender)
            .iter()
            .find(|uo_prev| uo_prev.nonce == uo.nonce)
            .cloned();

        if let Some(uo_prev) = uo_prev {
            if uo.max_fee_per_gas
                < calculate_valid_gas(uo_prev.max_fee_per_gas, GAS_INCREASE_PERC.into())
                || uo.max_priority_fee_per_gas
                    < calculate_valid_gas(
                        uo_prev.max_priority_fee_per_gas,
                        GAS_INCREASE_PERC.into(),
                    )
            {
                return Err(SanityCheckError::SenderVerification {
                    sender: uo.sender,
                    message: "couldn't replace user operation (gas increase too low)".into(),
                });
            }
        }

        Ok(())
    }
}
