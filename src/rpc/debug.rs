use super::debug_api::DebugApiServer;
use crate::{
    types::{reputation::ReputationEntry, user_operation::UserOperation},
    uopool::server::uopool::{
        uo_pool_client::UoPoolClient, ClearRequest, ClearResult, GetAllRequest,
        GetAllResult,
    },
};
use anyhow::format_err;
use async_trait::async_trait;
use ethers::types::Address;
use jsonrpsee::core::RpcResult;

#[cfg(debug_assertions)]
pub struct DebugApiServerImpl {
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

#[cfg(debug_assertions)]
#[async_trait]
impl DebugApiServer for DebugApiServerImpl {
    async fn clear_state(&self) -> RpcResult<()> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(ClearRequest {});

        let response = uopool_grpc_client
            .clear(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == ClearResult::Cleared as i32 {
            return Ok(());
        }

        Err(jsonrpsee::core::Error::Custom(
            "error clearing state".to_string(),
        ))
    }

    async fn dump_mempool(&self, entry_point: Address) -> RpcResult<Vec<UserOperation>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(GetAllRequest {
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .get_all(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == GetAllResult::GotAll as i32 {
            return Ok(response.uos.iter().map(|uo| uo.clone().into()).collect());
        }

        Err(jsonrpsee::core::Error::Custom(
            "error getting mempool".to_string(),
        ))
    }

    async fn set_reputation(&self, _reputation_entries: Vec<ReputationEntry>) -> RpcResult<()> {
        todo!()
    }

    async fn dump_reputation(&self) -> RpcResult<Vec<ReputationEntry>> {
        todo!()
    }
}
