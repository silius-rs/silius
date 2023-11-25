use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    validate::{SanityCheck, SanityHelper},
    Reputation,
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{get_address, sanity::SanityCheckError, UserOperation};

#[derive(Clone)]
pub struct Paymaster;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Paymaster {
    /// The [check_user_operation] method implementation that performs the sanity check on the paymaster.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityCheckError] is returned.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
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
