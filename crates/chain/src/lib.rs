use alloy_primitives::{Address, B256, U256};
use alloy_provider::{Provider, ext::DebugApi};
use alloy_rpc_types_eth::{BlockId, TransactionRequest};
use alloy_rpc_types_trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
    erc7562::Erc7562Frame,
};
use silius_primitives::{
    network_spec::network_spec,
    reputation::DepositInfo,
    user_operation::{PackedUserOperation, UserOperation},
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

    pub async fn create_handle_ops_transaction_request(
        &self,
        user_operations: &[UserOperation],
        beneficiary: Address,
    ) -> TransactionRequest {
        let mut authorization_list = Vec::new();

        for user_operation in user_operations.iter() {
            if let Some(signed_authorization) = &user_operation.signed_authorization {
                authorization_list.push(signed_authorization.clone());
            }
        }

        let mut transaction_request = self
            .entry_point
            .handleOps(
                user_operations
                    .into_iter()
                    .map(|p| p.to_packed_user_operation().into())
                    .collect(),
                beneficiary,
            )
            .into_transaction_request();

        if !authorization_list.is_empty() {
            transaction_request.authorization_list = Some(authorization_list);
        }

        transaction_request
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

    pub async fn trace_handle_ops(
        &self,
        user_operation: &UserOperation,
    ) -> Result<Erc7562Frame, ChainError> {
        let gas_limit = user_operation.pre_verification_gas
            + user_operation.verification_gas_limit
            + user_operation
                .paymaster_verification_gas_limit
                .unwrap_or_default();

        let mut transaction_request = self
            .create_handle_ops_transaction_request(&[user_operation.clone()], Address::ZERO)
            .await;

        transaction_request.gas = Some(gas_limit.to::<u64>());

        self.provider()
            .debug_trace_call(
                transaction_request,
                BlockId::latest(),
                GethDebugTracingCallOptions::new(GethDebugTracingOptions::new_tracer(
                    GethDebugBuiltInTracerType::Erc7562Tracer,
                )),
            )
            .await
            .map_err(|e| ChainError::Provider(e.to_string()))?
            .try_into_erc7562_frame()
            .map_err(|e| ChainError::Provider(e.to_string()))
    }
}
