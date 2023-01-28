use super::debug_api::DebugApiServer;
use crate::{
    types::{reputation::ReputationEntry, user_operation::UserOperation},
    uopool::server::uopool::{uo_pool_client::UoPoolClient, ClearRequest, ClearResult},
};
use anyhow::format_err;
use async_trait::async_trait;
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

    async fn dump_mempool(&self) -> RpcResult<Vec<UserOperation>> {
        todo!()
    }

    async fn set_reputation(&self, _reputation_entries: Vec<ReputationEntry>) -> RpcResult<()> {
        todo!()
    }

    async fn dump_reputation(&self) -> RpcResult<Vec<ReputationEntry>> {
        todo!()
    }
}
