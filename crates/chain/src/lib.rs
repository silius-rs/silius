use alloy_primitives::{Address, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use silius_primitives::{
    network_spec::network_spec, reputation::DepositInfo, user_operation::PackedUserOperation,
};
use tracing::info;

use crate::{
    entry_point::IEntryPoint::{self, IEntryPointInstance},
    error::ChainError,
};

pub mod entry_point;
pub mod error;

#[derive(Clone)]
pub struct Chain<P: Provider + 'static> {
    entry_point: IEntryPointInstance<P>,
}

impl<P: Provider + 'static> Chain<P> {
    pub async fn new(provider: P) -> Result<Self, ChainError> {
        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|e| ChainError::Provider(e.to_string()))?;
        if chain_id != network_spec().chain_id() {
            return Err(ChainError::ChainIdMismatch {
                expected: network_spec().chain_id(),
                got: chain_id,
            });
        }
        info!("Connected to chain with chain id: {}", chain_id);

        let code = provider
            .get_code_at(network_spec().entry_point_address)
            .await
            .map_err(|e| ChainError::Provider(e.to_string()))?;
        if code.is_empty() {
            return Err(ChainError::Provider(format!(
                "Entry point contract is not deployed at address: {}",
                network_spec().entry_point_address
            )));
        }
        info!(
            "Entry point contract is deployed at address: {}",
            network_spec().entry_point_address
        );

        Ok(Self {
            entry_point: IEntryPoint::new(network_spec().entry_point_address, provider),
        })
    }

    pub fn provider(&self) -> &P {
        self.entry_point.provider()
    }

    pub fn entry_point(&self) -> &IEntryPointInstance<P> {
        &self.entry_point
    }

    pub async fn get_base_fee_per_gas(&self) -> Result<U256, ChainError> {
        Ok(U256::from(
            self.provider()
                .get_gas_price()
                .await
                .map_err(|e| ChainError::Provider(e.to_string()))?,
        ))
    }

    pub async fn create_handle_ops_transaction(
        &self,
        packed_user_operations: Vec<PackedUserOperation>,
        beneficiary: Address,
    ) -> TransactionRequest {
        self.entry_point
            .handleOps(
                packed_user_operations
                    .into_iter()
                    .map(|p| p.into())
                    .collect(),
                beneficiary,
            )
            .into_transaction_request()
    }

    pub async fn get_user_operation_hash(
        &self,
        packed_user_operation: PackedUserOperation,
    ) -> Result<B256, ChainError> {
        self.entry_point
            .getUserOpHash(packed_user_operation.into())
            .call()
            .await
            .map_err(|e| ChainError::Provider(e.to_string()))
    }

    pub async fn get_deposit_info(&self, address: Address) -> Result<DepositInfo, ChainError> {
        self.entry_point
            .getDepositInfo(address)
            .call()
            .await
            .map(|deposit_info| deposit_info.into())
            .map_err(|e| ChainError::Provider(e.to_string()))
    }
}
