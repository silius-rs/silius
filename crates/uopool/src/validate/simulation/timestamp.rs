use crate::validate::{utils::extract_timestamps, SimulationCheck, SimulationHelper};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{
    simulation::{SimulationCheckError, EXPIRATION_TIMESTAMP_DIFF},
    UserOperation,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Timestamp;

#[async_trait::async_trait]
impl<M: Middleware> SimulationCheck<M> for Timestamp {
    /// The [check_user_operation] method implementation that checks the timestamp of the [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationHelper](crate::validate::SimulationHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationHelper<M>,
    ) -> Result<(), SimulationCheckError> {
        let (valid_after, valid_until) = extract_timestamps(helper.simulate_validation_result);

        let now = U256::from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| SimulationCheckError::UnknownError {
                    message: "Failed to get current timestamp".to_string(),
                })?
                .as_secs(),
        );

        if valid_until <= now + EXPIRATION_TIMESTAMP_DIFF {
            return Err(SimulationCheckError::Expiration {
                valid_after,
                valid_until,
                paymaster: None, // TODO: fill with paymaster address this error was triggered by the paymaster
            });
        }

        if valid_after > now {
            helper.valid_after = Some(valid_after);
        }

        Ok(())
    }
}
