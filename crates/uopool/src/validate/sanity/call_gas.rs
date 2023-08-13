use crate::{
    utils::calculate_call_gas_limit,
    validate::{SanityCheck, SanityHelper},
};
use ethers::{providers::Middleware, types::BlockNumber};
use silius_contracts::entry_point::EntryPointErr;
use silius_primitives::{sanity::SanityCheckError, UserOperation};

pub struct CallGas;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for CallGas {
    /// The `check_user_operation` method implementation for the `CallGas` sanity check.
    ///
    /// # Arguments
    /// `uo` - The user operation to check.
    /// `helper` - The helper struct that contains the entry point and the Ethereum client.
    ///
    /// # Returns
    /// None if the sanity check passes, otherwise [SanityCheckError].
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError> {
        let exec_res = match helper.entry_point.simulate_handle_op(uo.clone()).await {
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

        let block = helper
            .eth_client
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|err| SanityCheckError::UnknownError {
                message: err.to_string(),
            })?
            .ok_or(SanityCheckError::UnknownError {
                message: "No block found".to_string(),
            })?;
        let base_fee_per_gas = block
            .base_fee_per_gas
            .ok_or(SanityCheckError::UnknownError {
                message: "No base fee".to_string(),
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
}
