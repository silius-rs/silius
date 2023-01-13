use std::{collections::HashMap, sync::Arc};

use crate::{
    types::user_operation::UserOperation,
    uopool::{
        server::uopool_server::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse,
            RemoveRequest, RemoveResponse,
        },
        MempoolBox, MempoolId,
    },
};
use async_trait::async_trait;
use parking_lot::RwLock;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use std::sync::Arc;
use tonic::Response;

pub struct UoPoolService<M: Middleware> {
    pub _mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
    pub eth_provider: Arc<M>,
    pub entry_point: EntryPoint<M>,
    pub max_verification_gas: U256,
}

impl<M: Middleware + 'static> UoPoolService<M> {
    pub fn new(mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
        eth_provider: Arc<M>,
        entry_point: Address,
        max_verification_gas: U256,) -> Self {
        Self { _uo_pool: uo_pool }
    }
}

#[async_trait]
impl<M: Middleware + 'static> UoPool for UoPoolService<M> {
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
