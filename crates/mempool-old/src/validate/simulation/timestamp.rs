use crate::{
    validate::{utils::extract_timestamps, SimulationCheck, SimulationHelper},
    SimulationError,
};
use ethers::types::U256;
use silius_primitives::{simulation::EXPIRATION_TIMESTAMP_DIFF, UserOperation};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Timestamp;

impl SimulationCheck for Timestamp {
    /// The method implementation that checks the timestamp of the user operation.
    ///
    /// # Arguments
    /// `_uo` - Not used in this check
    /// `helper` - The [SimulationHelper]
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    fn check_user_operation(
        &self,
        _uo: &UserOperation,
        helper: &mut SimulationHelper,
    ) -> Result<(), SimulationError> {
        let (valid_after, valid_until) = extract_timestamps(helper.simulate_validation_result);

        let now = U256::from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| SimulationError::Other { inner: err.to_string() })?
                .as_secs(),
        );

        if valid_until < now {
            return Err(SimulationError::Timestamp { inner: "already expired".into() });
        }

        if valid_until <= now + EXPIRATION_TIMESTAMP_DIFF {
            return Err(SimulationError::Timestamp { inner: "expires too soon".into() });
        }

        if valid_after > now {
            helper.valid_after = Some(valid_after);
        }

        Ok(())
    }
}
