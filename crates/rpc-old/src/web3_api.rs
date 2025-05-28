pub use crate::web3::Web3ApiServerImpl;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(server, namespace = "web3")]
pub trait Web3Api {
    #[method(name = "clientVersion")]
    async fn client_version(&self) -> RpcResult<String>;
}
