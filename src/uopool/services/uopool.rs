use std::{collections::HashMap, sync::Arc};

use crate::uopool::{
    server::uopool_server::{
        uo_pool_server::UoPool, AddRequest, AddResponse, AllRequest, AllResponse, RemoveRequest,
        RemoveResponse,
    },
    Mempool, MempoolId,
};
use async_trait::async_trait;
use parking_lot::RwLock;
use tonic::Response;

pub struct UoPoolService {
    _mempools: Arc<RwLock<HashMap<MempoolId, Box<dyn Mempool>>>>,
}

impl UoPoolService {
    pub fn new(mempools: Arc<RwLock<HashMap<MempoolId, Box<dyn Mempool>>>>) -> Self {
        Self {
            _mempools: mempools,
        }
    }
}

#[async_trait]
impl UoPool for UoPoolService {
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
