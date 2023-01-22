use crate::{
    types::user_operation::UserOperation,
    uopool::{
        server::uopool::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, AllRequest, AllResponse,
            RemoveRequest, RemoveResponse,
        },
        MempoolBox, MempoolId, memory_reputation::MemoryReputation,
    },
};
use async_trait::async_trait;
use jsonrpsee::{tracing::info, types::ErrorObject};
use parking_lot::RwLock;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

pub type UoPoolError = ErrorObject<'static>;

pub struct UoPoolService {
    _mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
    _reputation: Arc<RwLock<MemoryReputation>>,
}

impl UoPoolService {
    pub fn new(mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>, reputation: Arc<RwLock<MemoryReputation>>) -> Self {
        Self {
            _mempools: mempools,
            _reputation: reputation,
        }
    }
}

#[async_trait]
impl UoPool for UoPoolService {
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
