use std::net::SocketAddr;

use async_trait::async_trait;
use tonic::Response;
use tracing::info;

use crate::uopool::server::{
    bundler::{
        bundler_server::{Bundler, BundlerServer},
        Mode, SendBundleNowResponse, SetModeRequest, SetModeResponse, SetModeResult,
    },
    types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
};

use super::BundlerService;

#[async_trait]
impl Bundler for BundlerService {
    async fn chain_id(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetChainIdResponse>, tonic::Status> {
        todo!()
    }

    async fn supported_entry_points(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, tonic::Status> {
        todo!()
    }

    async fn set_bundler_mode(
        &self,
        request: tonic::Request<SetModeRequest>,
    ) -> Result<Response<SetModeResponse>, tonic::Status> {
        let req = request.into_inner();
        match req.mode() {
            Mode::Manual => {
                info!("Stopping auto bundling");
                self.stop_bundling();
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
            Mode::Auto => {
                let interval = req.interval;
                self.start_bundling(interval);
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
        }
    }

    async fn send_bundle_now(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<SendBundleNowResponse>, tonic::Status> {
        let res = self
            .send_bundles_now()
            .await
            .map_err(|e| tonic::Status::internal(format!("Send bundle now with error: {e:?}")))?;
        Ok(Response::new(SendBundleNowResponse {
            result: Some(res.into()),
        }))
    }
}

pub fn run_server(bundler_manager: BundlerService, listen_address: SocketAddr) {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = BundlerServer::new(bundler_manager);
        builder.add_service(svc).serve(listen_address).await
    });
}
