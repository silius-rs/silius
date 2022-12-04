use crate::{uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    }, UserOperationPool,
}, types::user_operation::UserOperation};
use async_trait::async_trait;
use ethers::{
    providers::{Http, Middleware, Provider, ProviderError},
    types::{Address, Bytes},
};
use std::sync::Arc;
use tonic::Response;

#[derive(Debug)]
pub enum BadUserOperationError {
    SenderOrInitCode { sender: Address, init_code: Bytes },
}

#[derive(Debug)]
pub enum UserOperationValidationError {
    Validation(BadUserOperationError),
    Internal(anyhow::Error),
    Provider(ProviderError),
}

impl From<anyhow::Error> for UserOperationValidationError {
    fn from(e: anyhow::Error) -> Self {
        UserOperationValidationError::Internal(e)
    }
}

impl From<ProviderError> for UserOperationValidationError {
    fn from(e: ProviderError) -> Self {
        UserOperationValidationError::Provider(e)
    }
}

pub struct UoPoolService {
    uo_pool: Arc<UserOperationPool>,
    eth_provider: Arc<Provider<Http>>,
}

impl UoPoolService {
    pub fn new(uo_pool: Arc<UserOperationPool>, eth_provider: Arc<Provider<Http>>) -> Self {
        Self {
            uo_pool,
            eth_provider,
        }
    }

    async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationValidationError> {
        // Either the sender is an existing contract, or the initCode is not empty (but not both)
        let code = self
            .eth_provider
            .get_code(user_operation.sender, None)
            .await?;
        if (code.is_empty() && user_operation.init_code.is_empty())
            || (!code.is_empty() && !user_operation.init_code.is_empty())
        {
            return Err(UserOperationValidationError::Validation(
                BadUserOperationError::SenderOrInitCode {
                    sender: user_operation.sender,
                    init_code: user_operation.init_code.clone(),
                },
            ));
        }

        // The verificationGasLimit is sufficiently low (<= MAX_VERIFICATION_GAS) and the preVerificationGas is sufficiently high (enough to pay for the calldata gas cost of serializing the UserOperation plus PRE_VERIFICATION_OVERHEAD_GAS)

        // The paymaster is either the zero address or is a contract which (i) currently has nonempty code on chain, (ii) has registered with sufficient stake value, (iii) has a sufficient deposit to pay for the UserOperation, and (v) is not currently banned.

        // The callgas is at least the cost of a CALL with non-zero value.

        // The maxFeePerGas and maxPriorityFeePerGas are above a configurable minimum value that the client is willing to accept. At the minimum, they are sufficiently high to be included with the current block.basefee.

        // The sender doesn't have another UserOperation already present in the pool (or it replaces an existing entry with the same sender and nonce, with a higher maxPriorityFeePerGas and an equally increased maxFeePerGas). Only one UserOperation per sender may be included in a single batch.
        Ok(())
    }
}

#[async_trait]
impl UoPool for UoPoolService {
    async fn add(
        &self,
        _request: tonic::Request<AddRequest>,
    ) -> Result<Response<AddResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("todo"))
    }

    async fn remove(
        &self,
        _request: tonic::Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("todo"))
    }

    async fn all(
        &self,
        _request: tonic::Request<AllRequest>,
    ) -> Result<Response<AllResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("todo"))
    }
}
