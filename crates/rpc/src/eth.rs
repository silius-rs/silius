use crate::{error::JsonRpcError, eth_api::EthApiServer};
use async_trait::async_trait;
use ethers::{
    types::{Address, U64},
    utils::to_checksum,
};
use jsonrpsee::{core::RpcResult, types::ErrorObjectOwned};
use silius_grpc::{
    uo_pool_client::UoPoolClient, AddRequest, AddResult, EstimateUserOperationGasRequest,
    EstimateUserOperationGasResult, UserOperationHashRequest,
};
use silius_primitives::{
    consts::rpc_error_codes::USER_OPERATION_HASH, simulation::SimulationCheckError,
    uopool::ValidationError, UserOperation, UserOperationByHash, UserOperationGasEstimation,
    UserOperationHash, UserOperationPartial, UserOperationReceipt,
};
use std::str::FromStr;
use tonic::Request;

pub struct EthApiServerImpl {
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

#[async_trait]
impl EthApiServer for EthApiServerImpl {
    async fn chain_id(&self) -> RpcResult<U64> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let res = uopool_grpc_client
            .get_chain_id(Request::new(()))
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        return Ok(res.chain_id.into());
    }

    async fn supported_entry_points(&self) -> RpcResult<Vec<String>> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let res = uopool_grpc_client
            .get_supported_entry_points(Request::new(()))
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        return Ok(res
            .eps
            .into_iter()
            .map(|ep| to_checksum(&ep.into(), None))
            .collect());
    }

    async fn send_user_operation(
        &self,
        uo: UserOperation,
        ep: Address,
    ) -> RpcResult<UserOperationHash> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let req = Request::new(AddRequest {
            uo: Some(uo.into()),
            ep: Some(ep.into()),
        });

        let res = uopool_grpc_client
            .add(req)
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        if res.res == AddResult::Added as i32 {
            let uo_hash =
                serde_json::from_str::<UserOperationHash>(&res.data).map_err(JsonRpcError::from)?;
            return Ok(uo_hash);
        }

        Err(JsonRpcError::from(
            serde_json::from_str::<ValidationError>(&res.data).map_err(JsonRpcError::from)?,
        )
        .0)
    }

    async fn estimate_user_operation_gas(
        &self,
        uo: UserOperationPartial,
        ep: Address,
    ) -> RpcResult<UserOperationGasEstimation> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let req = Request::new(EstimateUserOperationGasRequest {
            uo: Some(UserOperation::from(uo).into()),
            ep: Some(ep.into()),
        });

        let res = uopool_grpc_client
            .estimate_user_operation_gas(req)
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        if res.res == EstimateUserOperationGasResult::Estimated as i32 {
            let gas_est = serde_json::from_str::<UserOperationGasEstimation>(&res.data)
                .map_err(JsonRpcError::from)?;
            return Ok(gas_est);
        }

        Err(JsonRpcError::from(
            serde_json::from_str::<SimulationCheckError>(&res.data).map_err(JsonRpcError::from)?,
        )
        .0)
    }

    async fn get_user_operation_receipt(
        &self,
        uo_hash: String,
    ) -> RpcResult<Option<UserOperationReceipt>> {
        match UserOperationHash::from_str(&uo_hash) {
            Ok(uo_hash) => {
                let req = Request::new(UserOperationHashRequest {
                    hash: Some(uo_hash.into()),
                });

                match self
                    .uopool_grpc_client
                    .clone()
                    .get_user_operation_receipt(req)
                    .await
                {
                    Ok(res) => {
                        let res = res.into_inner();

                        let receipt = res.user_operation_hash.and_then(|uo_hash| {
                            Some(UserOperationReceipt {
                                user_operation_hash: uo_hash.into(),
                                sender: res.sender?.into(),
                                nonce: res.nonce?.into(),
                                paymaster: res.paymaster.map(|p| p.into()),
                                actual_gas_cost: res.actual_gas_cost?.into(),
                                actual_gas_used: res.actual_gas_used?.into(),
                                success: res.success,
                                reason: String::new(),
                                logs: res.logs.into_iter().map(|l| l.into()).collect(),
                                tx_receipt: res.tx_receipt?.into(),
                            })
                        });
                        Ok(receipt)
                    }
                    Err(s) => match s.code() {
                        tonic::Code::NotFound => Ok(None),
                        _ => Err(ErrorObjectOwned::owned(
                            USER_OPERATION_HASH,
                            "Missing/invalid userOpHash".to_string(),
                            None::<bool>,
                        )),
                    },
                }
            }
            Err(_) => Err(ErrorObjectOwned::owned(
                USER_OPERATION_HASH,
                "Missing/invalid userOpHash".to_string(),
                None::<bool>,
            )),
        }
    }

    async fn get_user_operation_by_hash(
        &self,
        uo_hash: String,
    ) -> RpcResult<Option<UserOperationByHash>> {
        match UserOperationHash::from_str(&uo_hash) {
            Ok(uo_hash) => {
                let req = Request::new(UserOperationHashRequest {
                    hash: Some(uo_hash.into()),
                });

                match self
                    .uopool_grpc_client
                    .clone()
                    .get_user_operation_by_hash(req)
                    .await
                {
                    Ok(res) => {
                        let res = res.into_inner();

                        let uo: Option<UserOperationByHash> = res.user_operation.and_then(|uo| {
                            let entry_point = res.entry_point?.into();
                            let block_hash = res.block_hash?.into();
                            let transaction_hash = res.transaction_hash?.into();
                            Some(UserOperationByHash {
                                user_operation: uo.into(),
                                entry_point,
                                block_number: res.block_number.into(),
                                block_hash,
                                transaction_hash,
                            })
                        });
                        Ok(uo)
                    }
                    Err(s) => match s.code() {
                        tonic::Code::NotFound => Ok(None),
                        _ => Err(ErrorObjectOwned::owned(
                            USER_OPERATION_HASH,
                            "Missing/invalid userOpHash".to_string(),
                            None::<bool>,
                        )),
                    },
                }
            }
            Err(_) => Err(ErrorObjectOwned::owned(
                USER_OPERATION_HASH,
                "Missing/invalid userOpHash".to_string(),
                None::<bool>,
            )),
        }
    }
}
