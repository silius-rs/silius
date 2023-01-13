use crate::{
    contracts::EntryPoint,
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
use ethers::{providers::Middleware, types::U256};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

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
