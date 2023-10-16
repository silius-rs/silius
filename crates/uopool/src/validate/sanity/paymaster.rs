use crate::{
    mempool::Mempool,
    uopool::{VecCh, VecUo},
    validate::{SanityCheck, SanityHelper},
    Reputation,
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{
    get_address, reputation::ReputationEntry, sanity::SanityCheckError, UserOperation,
};
use std::fmt::Debug;

pub struct Paymaster;

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SanityCheck<M, P, R, E> for Paymaster
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
    E: Debug,
{
    /// The [check_user_operation] method implementation that performs the sanity check on the paymaster.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityCheckError] is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<(), SanityCheckError> {
        if !uo.paymaster_and_data.is_empty() {
            if let Some(addr) = get_address(&uo.paymaster_and_data) {
                let code = helper.entry_point.eth_client().get_code(addr, None).await?;

                if !code.is_empty() {
                    let deposit_info =
                        helper
                            .entry_point
                            .get_deposit_info(&addr)
                            .await
                            .map_err(|_| SanityCheckError::UnknownError {
                                message: "Couldn't retrieve deposit info from entry point".into(),
                            })?;

                    if U256::from(deposit_info.deposit) >= uo.max_fee_per_gas {
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
}
