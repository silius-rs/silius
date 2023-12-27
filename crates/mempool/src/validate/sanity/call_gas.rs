use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    validate::{SanityCheck, SanityHelper},
    Reputation, SanityError,
};
use ethers::{providers::Middleware, types::U256};
use silius_primitives::UserOperation;

#[derive(Clone)]
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
    /// None if the sanity check passes, otherwise [SanityError].
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        _helper: &SanityHelper<M>,
    ) -> Result<(), SanityError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        // call gas limit is at least the cost of a CALL with non-zero value
        // https://github.com/wolflo/evm-opcodes/blob/main/gas.md#aa-1-call
        // gas_cost = 100 + 9000
        let call_gas_limit = U256::from(9100);

        if uo.call_gas_limit >= call_gas_limit {
            return Ok(());
        }

        Err(SanityError::CallGasLimitTooLow {
            call_gas_limit: uo.call_gas_limit,
            call_gas_limit_expected: call_gas_limit,
        })
    }
}
