use super::sanity_check::UserOperationSanityCheckError;
use crate::{
    contracts::EntryPoint,
    types::user_operation::UserOperation,
    uopool::{
        server::uopool::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, AllRequest, AllResponse,
            RemoveRequest, RemoveResponse,
        },
        services::sanity_check::SANITY_CHECK_ERROR_CODE,
        MempoolBox, MempoolId,
    },
};
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use jsonrpsee::{
    tracing::info,
    types::{error::ErrorCode, ErrorObject},
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

pub type UoPoolError = ErrorObject<'static>;

impl<M: Middleware> From<UserOperationSanityCheckError<M>> for UoPoolError {
    fn from(_: UserOperationSanityCheckError<M>) -> Self {
        UoPoolError::from(ErrorCode::ServerError(-32602))
    }
}

pub struct UoPoolService<M: Middleware> {
    pub mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
    pub entry_points: Arc<HashMap<MempoolId, EntryPoint<M>>>,
    pub eth_provider: Arc<M>,
    pub max_verification_gas: U256,
    pub chain_id: U256,
}

impl<M: Middleware + 'static> UoPoolService<M> {
    pub fn new(
        mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
        entry_points: Arc<HashMap<MempoolId, EntryPoint<M>>>,
        eth_provider: Arc<M>,
        max_verification_gas: U256,
        chain_id: U256,
    ) -> Self {
        Self {
            mempools,
            entry_points,
            eth_provider,
            max_verification_gas,
            chain_id,
        }
    }
}

#[async_trait]
impl<M: Middleware + 'static> UoPool for UoPoolService<M> {
    async fn add(
        &self,
        request: tonic::Request<AddRequest>,
    ) -> Result<Response<AddResponse>, tonic::Status> {
        let req = request.into_inner();
        let mut res = AddResponse::default();

        if let AddRequest {
            uo: Some(user_operation),
            ep: Some(entry_point),
        } = req
        {
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid user operation"))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            info!("{:?}", user_operation);
            info!("{:?}", entry_point);

            //  sanity check
            match self.validate_user_operation(&user_operation).await {
                Ok(_) => {
                    // simulation

                    // add to mempool

                    res.set_result(AddResult::Added);
                    res.data =
                        serde_json::to_string(&user_operation.hash(&entry_point, &self.chain_id))
                            .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                }
                Err(error) => match error {
                    UserOperationSanityCheckError::SanityCheck(user_operation_error) => {
                        res.set_result(AddResult::NotAdded);
                        res.data = serde_json::to_string(&UoPoolError::owned::<String>(
                            SANITY_CHECK_ERROR_CODE,
                            user_operation_error.to_string(),
                            None,
                        ))
                        .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                    }
                    _ => {
                        return Err(tonic::Status::internal("error adding user operation"));
                    }
                },
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing user operation"))
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
