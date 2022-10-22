use ethereum_types::{Address, U64};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

use crate::types::user_operation::UserOperation;

#[rpc(server, namespace = "eth")]
pub trait EthApi {
    #[method(name = "chainId")]
    async fn chain_id(&self) -> RpcResult<U64>;
    #[method(name = "supportedEntryPoints")]
    async fn supported_entry_points(&self) -> RpcResult<Vec<Address>>;
    #[method(name = "sendUserOperation")]
    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<bool>;
}
