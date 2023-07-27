use crate::web3_api::Web3ApiServer;
use silius_primitives::consts::entry_point::VERSION;
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;

pub struct Web3ApiServerImpl {}

#[async_trait]
impl Web3ApiServer for Web3ApiServerImpl {
    async fn client_version(&self) -> RpcResult<String> {
        let git_version = git_version::git_version!(args = ["--tags"], fallback = "unknown");
        return Ok(format!("silius/{VERSION}/{git_version}"));
    }
}
