use crate::{
    mempool::Mempool,
    validate::{SanityCheck, SanityHelper},
    Reputation, SanityError,
};
use ethers::{
    providers::Middleware,
    types::{BlockNumber, U256},
};
use silius_primitives::UserOperation;

#[derive(Clone)]
pub struct MaxFee {
    pub min_priority_fee_per_gas: U256,
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for MaxFee {
    /// The method implementation that checks the max fee.
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The helper struct that contains the middleware
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SanityError]
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        if uo.max_priority_fee_per_gas > uo.max_fee_per_gas {
            return Err(SanityError::MaxPriorityFeePerGasTooHigh {
                max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
                max_fee_per_gas: uo.max_fee_per_gas,
            });
        }

        let block = helper
            .entry_point
            .eth_client()
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|err| SanityError::Provider { inner: err.to_string() })?
            .ok_or(SanityError::Other { inner: "No block found".into() })?;
        let base_fee_per_gas =
            block.base_fee_per_gas.ok_or(SanityError::Other { inner: "No base fee".into() })?;

        if base_fee_per_gas > uo.max_fee_per_gas {
            return Err(SanityError::MaxFeePerGasTooLow {
                max_fee_per_gas: uo.max_fee_per_gas,
                base_fee_per_gas,
            });
        }

        if uo.max_priority_fee_per_gas < self.min_priority_fee_per_gas {
            return Err(SanityError::MaxPriorityFeePerGasTooLow {
                max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
                max_priority_fee_per_gas_expected: self.min_priority_fee_per_gas,
            });
        }

        Ok(())
    }
}
