use std::sync::Arc;

use crate::uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    },
    Mempool, MempoolIdGenerator,
};
use async_trait::async_trait;
use tonic::Response;

pub struct UoPoolService<M: MempoolIdGenerator + Mempool> {
    _pool: Arc<M>,
}

impl<M: MempoolIdGenerator + Mempool> UoPoolService<M> {
    pub fn new(pool: Arc<M>) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl<M: MempoolIdGenerator + Mempool> UoPool for UoPoolService<M> {
    async fn add(
        &self,
        _request: tonic::Request<AddRequest>,
    ) -> Result<Response<AddResponse>, tonic::Status> {
        // let req = request.into_inner();
        // TODO: sanity checks
        // TODO: simulation
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
