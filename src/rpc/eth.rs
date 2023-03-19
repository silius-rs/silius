use crate::{
    rpc::eth_api::EthApiServer,
    types::user_operation::{
        UserOperation, UserOperationByHash, UserOperationGasEstimation, UserOperationHash,
        UserOperationPartial, UserOperationReceipt,
    },
    uopool::server::uopool::{
        uo_pool_client::UoPoolClient, AddRequest, AddResult, EstimateUserOperationGasRequest,
        EstimateUserOperationGasResult,
    },
};
use anyhow::format_err;
use async_trait::async_trait;
use ethers::{
    types::{Address, U64},
    utils::to_checksum,
};
use jsonrpsee::{
    core::RpcResult,
    tracing::info,
    types::{error::CallError, ErrorObject},
};

pub struct EthApiServerImpl {
    pub call_gas_limit: u64,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

#[async_trait]
impl EthApiServer for EthApiServerImpl {
    async fn chain_id(&self) -> RpcResult<U64> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let response = uopool_grpc_client
            .get_chain_id(tonic::Request::new(()))
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        return Ok(response.chain_id.into());
    }

    async fn supported_entry_points(&self) -> RpcResult<Vec<String>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let response = uopool_grpc_client
            .get_supported_entry_points(tonic::Request::new(()))
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        return Ok(response
            .eps
            .into_iter()
            .map(|entry_point| to_checksum(&entry_point.into(), None))
            .collect());
    }

    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash> {
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
            let user_operation_hash = serde_json::from_str::<UserOperationHash>(&response.data)
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
        user_operation: UserOperationPartial,
        entry_point: Address,
    ) -> RpcResult<UserOperationGasEstimation> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let request = tonic::Request::new(EstimateUserOperationGasRequest {
            uo: Some(UserOperation::from(user_operation).into()),
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .estimate_user_operation_gas(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();

        if response.result == EstimateUserOperationGasResult::Estimated as i32 {
            let user_operation_gas_estimation = serde_json::from_str::<UserOperationGasEstimation>(
                &response.data,
            )
            .map_err(|err| format_err!("error parsing user operation gas estimation: {}", err))?;
            return Ok(user_operation_gas_estimation);
        }

        Err(jsonrpsee::core::Error::Call(CallError::Custom(
            serde_json::from_str::<ErrorObject>(&response.data)
                .map_err(|err| format_err!("error parsing error object: {}", err))?,
        )))
    }

    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>> {
        info!("{:?}", user_operation_hash);
        Ok(None)
    }

    async fn get_user_operation_by_hash(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationByHash>> {
        info!("{:?}", user_operation_hash);
        Ok(None)
    }
}
