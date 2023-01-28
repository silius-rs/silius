use super::debug_api::DebugApiServer;
use crate::{
    types::{reputation::ReputationEntry, user_operation::UserOperation},
    uopool::server::uopool::uo_pool_client::UoPoolClient,
};
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
        todo!()
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
