use crate::{
    rpc::eth_api::{EstimateUserOperationGasResponse, EthApiServer},
    types::user_operation::{UserOperation, UserOperationHash, UserOperationReceipt},
    uopool::server::uopool::{uo_pool_client::UoPoolClient, AddRequest, AddResult},
};
use anyhow::format_err;
use async_trait::async_trait;
use ethers::types::{Address, U256, U64};
use jsonrpsee::{
    core::RpcResult,
    tracing::info,
    types::{error::CallError, ErrorObject},
};
use std::str::FromStr;

pub struct EthApiServerImpl {
    pub call_gas_limit: u64,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
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
        info!("{:?}", entry_point);
        info!("{:?}", user_operation);

        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(AddRequest {
            uo: Some(user_operation.into()),
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .add(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == AddResult::Added as i32 {
            let user_operation_hash = UserOperationHash::from_str(&response.data)
                .map_err(|err| format_err!("error parsing user operation hash: {}", err))?;
            return Ok(user_operation_hash);
        }

        Err(jsonrpsee::core::Error::Call(CallError::Custom(
            serde_json::from_str::<ErrorObject>(&response.data)
                .map_err(|err| format_err!("error parsing error object: {}", err))?,
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
