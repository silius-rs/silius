use crate::{
    mempool::Mempool,
    validate::{SanityCheck, SanityHelper},
    Reputation, SanityError,
};
use ethers::providers::Middleware;
use silius_primitives::UserOperation;

#[derive(Clone)]
pub struct Paymaster;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Paymaster {
    /// The method implementation that performs the sanity check on the paymaster.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to
    /// perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityError] is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        if !uo.paymaster.is_zero() {
            let code = helper
                .entry_point
                .eth_client()
                .get_code(uo.paymaster, None)
                .await
                .map_err(|e| SanityError::Provider { inner: e.to_string() })?;

            if !code.is_empty() {
                let deposit_info = helper.entry_point.get_deposit_info(&uo.paymaster).await?;

                if deposit_info.deposit >= uo.max_fee_per_gas {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}
