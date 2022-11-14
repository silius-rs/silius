use crate::uopool::server::server::{
    uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
    RemoveResponse,
};
use async_trait::async_trait;
use tonic::Response;

pub struct UoPoolService {}

impl UoPoolService {
    pub fn new() -> Self {
        Self {}
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
