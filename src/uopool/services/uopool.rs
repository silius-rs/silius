use std::sync::Arc;

use crate::uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    },
    UserOperationPool,
};
use async_trait::async_trait;
use tonic::Response;

pub struct UoPoolService {
    _uo_pool: Arc<UserOperationPool>,
}

impl UoPoolService {
    pub fn new(uo_pool: Arc<UserOperationPool>) -> Self {
        Self { _uo_pool: uo_pool }
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
