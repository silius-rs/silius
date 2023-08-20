use crate::{
    validate::{SanityCheck, SanityHelper},
    Overhead,
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::{sanity::SanityCheckError, UserOperation};

pub struct VerificationGas {
    pub max_verification_gas: U256,
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for VerificationGas {
    /// The [check_user_operation] method implementation that performs the check on verification gas.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// Nothing if the sanity check is successful, otherwise a [SanityCheckError](SanityCheckError) is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError> {
        if uo.verification_gas_limit > self.max_verification_gas {
            return Err(SanityCheckError::HighVerificationGasLimit {
                verification_gas_limit: uo.verification_gas_limit,
                max_verification_gas: self.max_verification_gas,
            });
        }

        let pre_gas = Overhead::default().calculate_pre_verification_gas(uo);
        if uo.pre_verification_gas < pre_gas {
            return Err(SanityCheckError::LowPreVerificationGas {
                pre_verification_gas: uo.pre_verification_gas,
                pre_verification_gas_expected: pre_gas,
            });
        }

        Ok(())
    }
}
