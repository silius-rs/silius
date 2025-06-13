use crate::{
    mempool::Mempool,
    utils::calculate_valid_gas,
    validate::{SanityCheck, SanityHelper},
    Reputation, SanityError,
};
use ethers::providers::Middleware;
use silius_primitives::{constants::mempool::GAS_INCREASE_PERC, UserOperation};

#[derive(Clone)]
pub struct Sender;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Sender {
    /// The method implementation that performs the check for the sender of
    /// the [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to
    /// perform the sanity check.
    ///
    /// # Returns
    /// Nothing if the sanity check is successful, otherwise a [SanityError](SanityError)
    /// is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        _reputation: &Reputation,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        let code = helper
            .entry_point
            .eth_client()
            .get_code(uo.sender, None)
            .await
            .map_err(|e| SanityError::Provider { inner: e.to_string() })?;

        // check if sender or init code
        if (code.is_empty() && uo.init_code.is_empty()) ||
            (!code.is_empty() && !uo.init_code.is_empty())
        {
            return Err(SanityError::Sender {
                inner: format!("sender {0:?} is an existing contract, or the initCode {1} is not empty (but not both)", uo.sender, uo.init_code),
            });
        }

        // check if prev user operation exists
        if mempool.get_number_by_sender(&uo.sender) == 0 {
            return Ok(());
        }

        let mut uo_prev: Option<UserOperation> = None;

        if !helper.val_config.ignore_prev {
            uo_prev = mempool
                .get_all_by_sender(&uo.sender)
                .iter()
                .find(|uo_prev| uo_prev.nonce == uo.nonce)
                .cloned();
        }

        if let Some(uo_prev) = uo_prev {
            if uo.max_fee_per_gas <
                calculate_valid_gas(uo_prev.max_fee_per_gas, GAS_INCREASE_PERC.into()) ||
                uo.max_priority_fee_per_gas <
                    calculate_valid_gas(
                        uo_prev.max_priority_fee_per_gas,
                        GAS_INCREASE_PERC.into(),
                    )
            {
                return Err(SanityError::Sender {
                    inner: format!(
                        "{0} couldn't replace user operation (gas increase too low)",
                        uo.sender
                    ),
                });
            }
        }

        Ok(())
    }
}
