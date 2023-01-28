use crate::types::{reputation::ReputationEntry, user_operation::UserOperation};
use ethers::types::Address;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[cfg(debug_assertions)]
#[rpc(server, namespace = "debug_bundler")]
pub trait DebugApi {
    #[method(name = "clearState")]
    async fn clear_state(&self) -> RpcResult<()>;

    #[method(name = "dumpMempool")]
    async fn dump_mempool(&self, entry_point: Address) -> RpcResult<Vec<UserOperation>>;

    #[method(name = "setReputation")]
    async fn set_reputation(&self, reputation_entries: Vec<ReputationEntry>) -> RpcResult<()>;

    #[method(name = "dumpReputation")]
    async fn dump_reputation(&self) -> RpcResult<Vec<ReputationEntry>>;
}
