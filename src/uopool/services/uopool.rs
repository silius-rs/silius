use crate::{
    contracts::EntryPoint,
    types::user_operation::UserOperation,
    uopool::{
        server::uopool::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, AllRequest, AllResponse,
            RemoveRequest, RemoveResponse,
        },
        MempoolBox, MempoolId,
    },
};
use async_trait::async_trait;
use ethers::{providers::Middleware, types::U256};
use jsonrpsee::{tracing::info, types::ErrorObject};
use parking_lot::RwLock;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

pub type UoPoolError = ErrorObject<'static>;

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

        if let Some(user_operation) = req.uo {
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid user operation"))?;

            info!("{:?}", user_operation);

            // TODO: validate user operation
            // TODO: sanity checks
            // TODO: simulation

            let uo_pool_error = UoPoolError::owned(
                -32602,
                "user operation was not added",
                Some(json!({
                    "reason": "this is error",
                })),
            );

            res.set_result(AddResult::NotAdded);
            res.data = serde_json::to_string(&uo_pool_error)
                .map_err(|_| tonic::Status::internal("error adding user operation"))?;

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
