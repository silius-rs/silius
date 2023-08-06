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

/// EthApiServer implements the ERC-4337 `eth` namespace RPC methods trait [EthApiServer](EthApiServer).
pub struct EthApiServerImpl {
    /// The [UoPool gRPC client](UoPoolClient).
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

#[async_trait]
impl EthApiServer for EthApiServerImpl {
    /// Retrieve the current [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    ///
    /// # Returns
    /// * `RpcResult<U64>` - The chain ID as a U64.
    async fn chain_id(&self) -> RpcResult<U64> {
        let mut uopool_grpc_client = self.uopool_grpc_client.clone();

        let res = uopool_grpc_client
            .get_chain_id(Request::new(()))
            .await
            .map_err(JsonRpcError::from)?
            .into_inner();

        return Ok(res.chain_id.into());
    }

    /// Get the supported entry points for [UserOperations](UserOperation).
    ///
    /// # Returns
    /// * `RpcResult<Vec<String>>` - A array of the entry point addresses as strings.
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

    /// Send a user operation via the [AddRequest](AddRequest).
    ///
    /// # Arguments
    /// * `user_operation: UserOperation` - The user operation to be sent.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<UserOperationHash>` - The hash of the sent user operation.
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

    /// Estimate the gas required for a [UserOperation](UserOperation) via the [EstimateUserOperationGasRequest](EstimateUserOperationGasRequest).
    /// This allows you to gauge the computational cost of the operation.
    /// See [How ERC-4337 Gas Estimation Works](https://www.alchemy.com/blog/erc-4337-gas-estimation).
    ///
    /// # Arguments
    /// * `user_operation: [UserOperationPartial](UserOperationPartial)` - The partial user operation for which to estimate the gas.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<UserOperationGasEstimation>` - The [UserOperationGasEstimation](UserOperationGasEstimation) for the user operation.
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

    /// Retrieve the receipt of a [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// * `user_operation_hash: String` - The hash of the user operation.
    ///
    /// # Returns
    /// * `RpcResult<Option<UserOperationReceipt>>` - The [UserOperationReceipt] of the user operation.    
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

    /// Retrieve a [UserOperation](UserOperation) by its hash via [UserOperationHashRequest](UserOperationHashRequest).
    /// The hash serves as a unique identifier for the [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// * `user_operation_hash: String` - The hash of a [UserOperation](UserOperation).
    ///
    /// # Returns
    /// * `RpcResult<Option<UserOperationByHash>>` - The [UserOperation](UserOperation) associated with the hash, or None if it does not exist.
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
