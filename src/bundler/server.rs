use async_trait::async_trait;
use tonic::Response;

use crate::uopool::server::{
    bundler::{
        bundler_server::Bundler as BundlerServer, Mode, SetModeRequest, SetModeResponse,
        SetModeResult,
    },
    types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
};

use super::BundlerManager;

#[async_trait]
impl BundlerServer for BundlerManager {
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
                self.stop();
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
            Mode::Auto => {
                self.start();
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
        }
    }
}
