use crate::{debug_api::DebugApiServer, error::JsonRpcError};
use async_trait::async_trait;
use ethers::types::{Address, H256};
use jsonrpsee::{
    core::RpcResult,
    types::{error::INTERNAL_ERROR_CODE, ErrorObjectOwned},
};
use silius_grpc::{
    bundler_client::BundlerClient, uo_pool_client::UoPoolClient, GetAllReputationRequest,
    GetAllRequest, Mode as GrpcMode, SetModeRequest, SetReputationRequest, SetReputationResult,
};
use silius_primitives::{
    bundler::DEFAULT_BUNDLE_INTERVAL, reputation::ReputationEntry, BundlerMode, UserOperation,
};
use tonic::Request;

pub struct DebugApiServerImpl {
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    pub bundler_grpc_client: BundlerClient<tonic::transport::Channel>,
}

#[async_trait]
impl DebugApiServer for DebugApiServerImpl {
    async fn clear_state(&self) -> RpcResult<()> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        uopool_grpc_client
            .clear(Request::new(()))
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        Ok(())
    }

    async fn dump_mempool(&self, ep: Address) -> RpcResult<Vec<UserOperation>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let req = Request::new(GetAllRequest {
            ep: Some(ep.into()),
        });

        let res = uopool_grpc_client
            .get_all(req)
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        let mut uos: Vec<UserOperation> = res.uos.iter().map(|uo| uo.clone().into()).collect();
        uos.sort_by(|a, b| a.nonce.cmp(&b.nonce));
        Ok(uos)
    }

    async fn set_reputation(&self, entries: Vec<ReputationEntry>, ep: Address) -> RpcResult<()> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let req = Request::new(SetReputationRequest {
            rep: entries.iter().map(|re| (*re).into()).collect(),
            ep: Some(ep.into()),
        });

        let res = uopool_grpc_client
            .set_reputation(req)
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        if res.res == SetReputationResult::SetReputation as i32 {
            return Ok(());
        }

        Err(ErrorObjectOwned::owned(
            INTERNAL_ERROR_CODE,
            "Error setting reputation".to_string(),
            None::<bool>,
        ))
    }

    async fn dump_reputation(&self, ep: Address) -> RpcResult<Vec<ReputationEntry>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = Request::new(GetAllReputationRequest {
            ep: Some(ep.into()),
        });

        let res = uopool_grpc_client
            .get_all_reputation(request)
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        Ok(res.rep.iter().map(|re| re.clone().into()).collect())
    }

    async fn set_bundling_mode(&self, mode: BundlerMode) -> RpcResult<()> {
        let mut bundler_grpc_client = self.bundler_grpc_client.clone();

        let req = Request::new(SetModeRequest {
            mode: Into::<GrpcMode>::into(mode).into(),
            interval: DEFAULT_BUNDLE_INTERVAL,
        });

        match bundler_grpc_client.set_bundler_mode(req).await {
            Ok(_) => Ok(()),
            Err(s) => Err(JsonRpcError::from(s).into()),
        }
    }

    async fn send_bundle_now(&self) -> RpcResult<H256> {
        let mut bundler_grpc_client = self.bundler_grpc_client.clone();

        let req = Request::new(());

        match bundler_grpc_client.send_bundle_now(req).await {
            Ok(res) => Ok(res
                .into_inner()
                .res
                .expect("Must return send bundle tx data")
                .into()),
            Err(s) => Err(JsonRpcError::from(s).into()),
        }
    }
}
