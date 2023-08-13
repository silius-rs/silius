use crate::validate::{SanityCheck, SanityHelper};
use ethers::providers::Middleware;
use silius_primitives::{sanity::SanityCheckError, UserOperation};

pub struct SenderOrInitCode;

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for SenderOrInitCode {
    /// The [check_user_operation] method implementation that performs the check whether the [UserOperation](UserOperation) is a deployment or a transaction.
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
        helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError> {
        let code = helper.eth_client.get_code(uo.sender, None).await?;
        if (code.is_empty() && uo.init_code.is_empty())
            || (!code.is_empty() && !uo.init_code.is_empty())
        {
            return Err(SanityCheckError::SenderOrInitCode {
                sender: uo.sender,
                init_code: uo.init_code.clone(),
            });
        }
        Ok(())
    }
}
