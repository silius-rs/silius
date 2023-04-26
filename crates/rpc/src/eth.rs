use std::str::FromStr;

use aa_bundler_grpc::{
    uo_pool_client::UoPoolClient, AddRequest, AddResult, EstimateUserOperationGasRequest,
    EstimateUserOperationGasResult, UserOperationHashRequest,
};
use aa_bundler_primitives::{
    UserOperation, UserOperationByHash, UserOperationGasEstimation, UserOperationHash,
    UserOperationPartial, UserOperationReceipt, USER_OPERATION_HASH_ERROR_CODE,
};
use anyhow::format_err;
use async_trait::async_trait;
use ethers::{
    types::{Address, U64},
    utils::to_checksum,
};
use jsonrpsee::{
    core::RpcResult,
    types::{error::CallError, ErrorObject},
};
use tracing::{debug, trace};

use crate::eth_api::EthApiServer;

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
        trace!("Receive user operation {user_operation:?} from {entry_point:x?}");

        let request = tonic::Request::new(AddRequest {
            uo: Some(user_operation.into()),
            ep: Some(entry_point.into()),
        });

        let response = uopool_grpc_client
            .add(request)
            .await
            .map_err(|status| format_err!("GRPC error (uopool): {}", status.message()))?
            .into_inner();
        trace!("Send user operation response: {response:?}");
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
        user_operation_hash: String,
    ) -> RpcResult<Option<UserOperationReceipt>> {
        trace!("Receive getUserOperationReceipt request {user_operation_hash:?}");
        match UserOperationHash::from_str(&user_operation_hash) {
            Ok(user_operation_hash) => {
                let request = tonic::Request::new(UserOperationHashRequest {
                    hash: Some(user_operation_hash.into()),
                });
                match self
                    .uopool_grpc_client
                    .clone()
                    .get_user_operation_receipt(request)
                    .await
                {
                    Ok(res) => {
                        let result = res.into_inner();
                        trace!("Got grpc result from getUserOperationReceipt endpoint {result:?}");
                        let receipt = result.user_operation_hash.and_then(|user_op_hash| {
                            Some(UserOperationReceipt {
                                user_op_hash: user_op_hash.into(),
                                sender: result.sender?.into(),
                                nonce: result.nonce?.into(),
                                paymaster: result.paymaster.map(|p| p.into()),
                                actual_gas_cost: result.actual_gas_cost?.into(),
                                actual_gas_used: result.actual_gas_used?.into(),
                                success: result.success,
                                reason: String::new(),
                                logs: result.logs.into_iter().map(|l| l.into()).collect(),
                                receipt: result.transaction_receipt?.into(),
                            })
                        });
                        Ok(receipt)
                    }
                    Err(e) => match e.code() {
                        tonic::Code::NotFound => Ok(None),
                        _ => {
                            debug!("getUserOperationByHash with GRPC error {e:?}");
                            Err(jsonrpsee::core::Error::Call(CallError::Custom(
                                ErrorObject::owned(
                                    USER_OPERATION_HASH_ERROR_CODE,
                                    "Missing/invalid userOpHash".to_string(),
                                    None::<bool>,
                                ),
                            )))
                        }
                    },
                }
            }
            Err(_) => Err(jsonrpsee::core::Error::Call(CallError::Custom(
                ErrorObject::owned(
                    USER_OPERATION_HASH_ERROR_CODE,
                    "Missing/invalid userOpHash".to_string(),
                    None::<bool>,
                ),
            ))),
        }
    }

    async fn get_user_operation_by_hash(
        &self,
        user_operation_hash: String,
    ) -> RpcResult<Option<UserOperationByHash>> {
        trace!("Receive getUserOperationByHash request {user_operation_hash:?}");
        match UserOperationHash::from_str(&user_operation_hash) {
            Ok(user_operation_hash) => {
                let request = tonic::Request::new(UserOperationHashRequest {
                    hash: Some(user_operation_hash.into()),
                });
                match self
                    .uopool_grpc_client
                    .clone()
                    .get_user_operation_by_hash(request)
                    .await
                {
                    Ok(res) => {
                        let result = res.into_inner();
                        trace!("Got grpc result from getUserOperationByHash endpoint {result:?}");
                        let uo: Option<UserOperationByHash> =
                            result.user_operation.and_then(|user_operation| {
                                let entry_point = result.entry_point?.into();
                                let block_hash = result.block_hash?.into();
                                let transaction_hash = result.transaction_hash?.into();
                                Some(UserOperationByHash {
                                    user_operation: user_operation.into(),
                                    entry_point,
                                    block_number: result.block_number.into(),
                                    block_hash,
                                    transaction_hash,
                                })
                            });
                        Ok(uo)
                    }
                    Err(e) => match e.code() {
                        tonic::Code::NotFound => Ok(None),
                        _ => {
                            debug!("getUserOperationByHash with GRPC error {e:?}");
                            Err(jsonrpsee::core::Error::Call(CallError::Custom(
                                ErrorObject::owned(
                                    USER_OPERATION_HASH_ERROR_CODE,
                                    "Missing/invalid userOpHash".to_string(),
                                    None::<bool>,
                                ),
                            )))
                        }
                    },
                }
            }
            Err(_) => Err(jsonrpsee::core::Error::Call(CallError::Custom(
                ErrorObject::owned(
                    USER_OPERATION_HASH_ERROR_CODE,
                    "Missing/invalid userOpHash".to_string(),
                    None::<bool>,
                ),
            ))),
        }
    }
}
