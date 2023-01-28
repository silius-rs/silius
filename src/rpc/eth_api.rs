use ethers::types::{Address, U256, U64};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};

use crate::types::user_operation::{UserOperation, UserOperationHash, UserOperationReceipt};

#[derive(Serialize, Deserialize)]
pub struct EstimateUserOperationGasResponse {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

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
    ) -> RpcResult<UserOperationHash>;

    #[method(name = "estimateUserOperationGas")]
    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<EstimateUserOperationGasResponse>;

    #[method(name = "getUserOperationReceipt")]
    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>>;
}
