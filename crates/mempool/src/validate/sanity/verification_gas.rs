use crate::{
    mempool::Mempool,
    utils::div_ceil,
    validate::{SanityCheck, SanityHelper},
    Overhead, Reputation, SanityError,
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::UserOperation;

#[derive(Clone)]
pub struct VerificationGas {
    pub max_verification_gas: U256,
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for VerificationGas {
    /// The method implementation that performs the check on verification gas.
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
        _mempool: &Mempool,
        _reputation: &Reputation,
        _helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        if uo.verification_gas_limit > self.max_verification_gas {
            return Err(SanityError::VerificationGasLimitTooHigh {
                verification_gas_limit: uo.verification_gas_limit,
                verification_gas_limit_expected: self.max_verification_gas,
            });
        }

        // calculate the pvg and allow 10 % deviation
        let pre_gas = div_ceil(
            Overhead::default().calculate_pre_verification_gas(uo).saturating_mul(U256::from(90)),
            U256::from(100),
        );
        if uo.pre_verification_gas < pre_gas {
            return Err(SanityError::PreVerificationGasTooLow {
                pre_verification_gas: uo.pre_verification_gas,
                pre_verification_gas_expected: pre_gas,
            });
        }

        Ok(())
    }
}
