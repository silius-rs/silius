use super::debug_api::DebugApiServer;
use crate::{
    bundler::Mode,
    types::{reputation::ReputationEntry, user_operation::UserOperation},
    uopool::server::{
        bundler::{bundler_client::BundlerClient, Mode as GrpcMode, SetModeRequest},
        uopool::{
            uo_pool_client::UoPoolClient, ClearResult, GetAllReputationRequest,
            GetAllReputationResult, GetAllRequest, GetAllResult, SetReputationRequest,
            SetReputationResult,
        },
    },
};
use anyhow::format_err;
use async_trait::async_trait;
use ethers::types::{Address, H256};
use jsonrpsee::core::RpcResult;
use tracing::{debug, trace};

pub struct DebugApiServerImpl {
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    pub bundler_grpc_client: BundlerClient<tonic::transport::Channel>,
}

#[async_trait]
impl DebugApiServer for DebugApiServerImpl {
    async fn clear_state(&self) -> RpcResult<()> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let response = uopool_grpc_client
            .clear(tonic::Request::new(()))
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
        debug!("Sending getAll request to mempool");
        let response = uopool_grpc_client
            .get_all(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();
        trace!("Getall from mempool: {response:?}");
        if response.result == GetAllResult::GotAll as i32 {
            return Ok(response.uos.iter().map(|uo| uo.clone().into()).collect());
        }

        Err(jsonrpsee::core::Error::Custom(
            "error getting mempool".to_string(),
        ))
    }

    async fn set_reputation(
        &self,
        reputation_entries: Vec<ReputationEntry>,
        entry_point: Address,
    ) -> RpcResult<()> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(SetReputationRequest {
            res: reputation_entries.iter().map(|re| (*re).into()).collect(),
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .set_reputation(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == SetReputationResult::SetReputation as i32 {
            return Ok(());
        }

        Err(jsonrpsee::core::Error::Custom(
            "error setting reputation".to_string(),
        ))
    }

    async fn dump_reputation(&self, entry_point: Address) -> RpcResult<Vec<ReputationEntry>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(GetAllReputationRequest {
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .get_all_reputation(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == GetAllReputationResult::GotAllReputation as i32 {
            return Ok(response.res.iter().map(|re| re.clone().into()).collect());
        }

        Err(jsonrpsee::core::Error::Custom(
            "error getting reputation".to_string(),
        ))
    }

    async fn set_bundling_mode(&self, mode: Mode) -> RpcResult<()> {
        let mut bundler_grpc_client = self.bundler_grpc_client.clone();

        let request = tonic::Request::new(SetModeRequest {
            mode: Into::<GrpcMode>::into(mode).into(),
        });

        match bundler_grpc_client.set_bundler_mode(request).await {
            Ok(_) => Ok(()),
            Err(status) => Err(jsonrpsee::core::Error::Custom(format!(
                "GRPC error (bundler): {}",
                status.message()
            ))),
        }
    }

    async fn send_bundle_now(&self) -> RpcResult<H256> {
        let mut bundler_grpc_client = self.bundler_grpc_client.clone();
        let request = tonic::Request::new(());
        match bundler_grpc_client.send_bundle_now(request).await {
            Ok(response) => Ok(response
                .into_inner()
                .result
                .expect("Must return send bundle now tx data")
                .into()),
            Err(status) => Err(jsonrpsee::core::Error::Custom(format!(
                "GRPC error (bundler): {}",
                status.message()
            ))),
        }
    }
}
