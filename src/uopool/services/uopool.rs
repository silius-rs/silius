use crate::{uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    }, UserOperationPool,
}, contracts::EntryPoint};
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use std::sync::Arc;
use tonic::Response;

pub struct UoPoolService<M: Middleware> {
    pub uo_pool: Arc<UserOperationPool>,
    pub eth_provider: Arc<M>,
    pub entry_point: EntryPoint<M>,
    pub max_verification_gas: U256,
}

impl<M: Middleware + 'static> UoPoolService<M> {
    pub fn new(
        uo_pool: Arc<UserOperationPool>,
        eth_provider: Arc<M>,
        entry_point: Address,
        max_verification_gas: U256,
    ) -> Self {
        Self {
            uo_pool,
            entry_point: EntryPoint::new(Arc::clone(&eth_provider), entry_point),
            eth_provider,
            max_verification_gas,
        }
    }
}

#[async_trait]
impl<M: Middleware + 'static> UoPool for UoPoolService<M> {
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
