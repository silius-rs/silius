use crate::Web3ApiServer;
use aa_bundler_primitives::SUPPORT_ENTRYPOINT_VERSION;
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;

pub struct Web3ApiServerImpl {}

#[async_trait]
impl Web3ApiServer for Web3ApiServerImpl {
    async fn client_version(&self) -> RpcResult<String> {
        let git_version = git_version::git_version!(args = ["--tags"], fallback = "unknown");
        return Ok(format!(
            "rust-aa-bundler/{SUPPORT_ENTRYPOINT_VERSION}/{git_version}"
        ));
    }
}
