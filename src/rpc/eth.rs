use crate::{
    rpc::eth_api::{EstimateUserOperationGasResponse, EthApiServer},
    types::user_operation::{UserOperation, UserOperationHash, UserOperationReceipt},
};
use async_trait::async_trait;
use ethers::types::{Address, U256, U64};
use jsonrpsee::{
    core::RpcResult,
    tracing::info,
    types::{
        error::{CallError, ErrorCode},
        ErrorObject,
    },
};

pub struct EthApiServerImpl {
    pub call_gas_limit: u64,
}

#[async_trait]
impl EthApiServer for EthApiServerImpl {
    async fn chain_id(&self) -> RpcResult<U64> {
        Ok(U64::default())
    }

    async fn supported_entry_points(&self) -> RpcResult<Vec<Address>> {
        Ok(vec![Address::default()])
    }

    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash> {
        info!("{:?}", user_operation);
        info!("{:?}", entry_point);
        // Ok(SendUserOperationResponse::Success(H256::default()))
        let data = serde_json::value::to_raw_value(&"{\"a\": 100, \"b\": 200}").unwrap();
        Err(jsonrpsee::core::Error::Call(CallError::Custom(
            ErrorObject::owned(
                ErrorCode::ServerError(-32000).code(),
                "Not implemented",
                Some(data),
            ),
        )))
    }

    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<EstimateUserOperationGasResponse> {
        info!("{:?}", user_operation);
        info!("{:?}", entry_point);
        Ok(EstimateUserOperationGasResponse {
            pre_verification_gas: U256::from(0),
            verification_gas_limit: U256::from(0),
            call_gas_limit: U256::from(self.call_gas_limit),
        })
    }

    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>> {
        info!("{:?}", user_operation_hash);
        Ok(None)
    }
}
