use crate::{
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
use jsonrpsee::tracing::info;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

pub struct UoPoolService {
    _mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
}

impl UoPoolService {
    pub fn new(mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>) -> Self {
        Self {
            _mempools: mempools,
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
                .map_err(|_| tonic::Status::invalid_argument("user operation is not valid"))?;

            info!("{:?}", user_operation);

            // TODO: validate user operation
            // TODO: sanity checks
            // TODO: simulation

            res.set_result(AddResult::NotAdded);
            res.data = String::from("\"{\"code\": -32602, \"message\": \"user operation was not added\", \"data\": {\"reason\": \"this is error\"}}\"");

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("user operation is missing"))
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
