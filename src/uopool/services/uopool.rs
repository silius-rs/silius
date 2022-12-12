use crate::{uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    }, UserOperationPool,
}, types::user_operation::UserOperation, chain::gas::{Overhead, self}};
use async_trait::async_trait;
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
};
use std::sync::Arc;
use tonic::Response;

pub struct UoPoolService {
    pub uo_pool: Arc<UserOperationPool>,
    pub eth_provider: Arc<Provider<Http>>,
    pub entry_point: Address,
    pub max_verification_gas: U256,
}

impl UoPoolService {
    pub fn new(
        uo_pool: Arc<UserOperationPool>,
        eth_provider: Arc<Provider<Http>>,
        entry_point: Address,
        max_verification_gas: U256,
    ) -> Self {
        Self {
            uo_pool,
            eth_provider,
            entry_point,
            max_verification_gas,
        }
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
