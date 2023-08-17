use crate::validate::{SanityCheck, SanityHelper};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{get_address, reputation::Status, sanity::SanityCheckError, UserOperation};

pub struct Paymaster;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Paymaster {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError> {
        if !uo.paymaster_and_data.is_empty() {
            if let Some(addr) = get_address(&uo.paymaster_and_data) {
                let code = helper.eth_client.get_code(addr, None).await?;

                if !code.is_empty() {
                    let deposit_info =
                        helper
                            .entry_point
                            .get_deposit_info(&addr)
                            .await
                            .map_err(|_| SanityCheckError::UnknownError {
                                message: "Couldn't retrieve deposit info from entry point".into(),
                            })?;

                    if U256::from(deposit_info.deposit) >= uo.max_fee_per_gas
                        && Status::from(helper.reputation.get_status(&addr).map_err(|_| {
                            SanityCheckError::UnknownError {
                                message: "Failed to retrieve reputation status for paymaster"
                                    .into(),
                            }
                        })?) != Status::BANNED
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
}
