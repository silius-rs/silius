use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    utils::calculate_valid_gas,
    validate::{SanityCheck, SanityHelper},
    Reputation,
};
use ethers::providers::Middleware;
use silius_primitives::{
    constants::mempool::GAS_INCREASE_PERC, sanity::SanityCheckError, UserOperation,
};

#[derive(Clone)]
pub struct Sender;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Sender {
    /// The [check_user_operation] method implementation that performs the check for the sender of
    /// the [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to
    /// perform the sanity check.
    ///
    /// # Returns
    /// Nothing if the sanity check is successful, otherwise a [SanityCheckError](SanityCheckError)
    /// is returned.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
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
        let code = helper.entry_point.eth_client().get_code(uo.sender, None).await?;

        // check if sender or init code
        if (code.is_empty() && uo.init_code.is_empty()) ||
            (!code.is_empty() && !uo.init_code.is_empty())
        {
            return Err(SanityCheckError::SenderOrInitCode {
                sender: uo.sender,
                init_code: uo.init_code.clone(),
            });
        }

        // check if prev user operation exists
        if mempool.get_number_by_sender(&uo.sender) == 0 {
            return Ok(());
        }

        let uo_prev = mempool
            .get_all_by_sender(&uo.sender)
            .iter()
            .find(|uo_prev| uo_prev.nonce == uo.nonce)
            .cloned();

        if let Some(uo_prev) = uo_prev {
            if uo.max_fee_per_gas <
                calculate_valid_gas(uo_prev.max_fee_per_gas, GAS_INCREASE_PERC.into()) ||
                uo.max_priority_fee_per_gas <
                    calculate_valid_gas(
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
