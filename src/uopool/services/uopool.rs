use crate::{
    types::{reputation::ReputationEntry, user_operation::UserOperation},
    uopool::{
        server::uopool::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, ClearRequest,
            ClearResponse, ClearResult, GetAllRequest, GetAllResponse, GetAllResult, RemoveRequest,
            RemoveResponse,
        },
        MempoolBox, MempoolId, ReputationBox,
    },
};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use jsonrpsee::{tracing::info, types::ErrorObject};
use parking_lot::RwLock;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;

pub type UoPoolError = ErrorObject<'static>;

pub struct UoPoolService {
    pub mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
    pub reputation: Arc<RwLock<ReputationBox<Vec<ReputationEntry>>>>,
    pub chain_id: U256,
}

impl UoPoolService {
    pub fn new(
        mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
        reputation: Arc<RwLock<ReputationBox<Vec<ReputationEntry>>>>,
        chain_id: U256,
    ) -> Self {
        Self {
            mempools,
            reputation,
            chain_id,
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

            // TODO: make something with reputation

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

    #[cfg(debug_assertions)]
    async fn get_all(
        &self,
        request: tonic::Request<GetAllRequest>,
    ) -> Result<Response<GetAllResponse>, tonic::Status> {
        use crate::uopool::mempool_id;

        let req = request.into_inner();
        let mut res = GetAllResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            if let Some(mempool) = self
                .mempools
                .read()
                .get(&mempool_id(entry_point, self.chain_id))
            {
                res.result = GetAllResult::GotAll as i32;
                res.uos = mempool
                    .get_all()
                    .iter()
                    .map(|uo| uo.clone().into())
                    .collect();
            } else {
                res.result = GetAllResult::NotGotAll as i32;
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing entry point"))
    }

    #[cfg(debug_assertions)]
    async fn clear(
        &self,
        _request: tonic::Request<ClearRequest>,
    ) -> Result<Response<ClearResponse>, tonic::Status> {
        for mempool in self.mempools.write().values_mut() {
            mempool.clear();
        }

        self.reputation.write().clear();

        Ok(tonic::Response::new(ClearResponse {
            result: ClearResult::Cleared as i32,
        }))
    }
}
