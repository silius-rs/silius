use ethers::types::{Address, U64};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

use crate::types::user_operation::{
    UserOperation, UserOperationGasEstimation, UserOperationHash, UserOperationPartial,
    UserOperationReceipt,
};

#[rpc(server, namespace = "eth")]
pub trait EthApi {
    #[method(name = "chainId")]
    async fn chain_id(&self) -> RpcResult<U64>;

    #[method(name = "supportedEntryPoints")]
    async fn supported_entry_points(&self) -> RpcResult<Vec<String>>;

    #[method(name = "sendUserOperation")]
    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash>;

    #[method(name = "estimateUserOperationGas")]
    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperationPartial,
        entry_point: Address,
    ) -> RpcResult<UserOperationGasEstimation>;

    #[method(name = "getUserOperationReceipt")]
    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>>;
}
