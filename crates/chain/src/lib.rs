use alloy_primitives::{Address, B256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::sol;
use silius_primitives::{network_spec::network_spec, user_operation::PackedUserOperation};
use tracing::info;

use crate::IEntryPoint::IEntryPointInstance;

sol!(
    #[sol(rpc)]
    IEntryPoint,
    "resources/entry_point_v08.json"
);

impl From<PackedUserOperation> for IEntryPoint::PackedUserOperation {
    fn from(packed_user_operation: PackedUserOperation) -> Self {
        Self {
            sender: packed_user_operation.sender,
            nonce: packed_user_operation.nonce,
            initCode: packed_user_operation.init_code,
            callData: packed_user_operation.call_data,
            accountGasLimits: packed_user_operation.account_gas_limit,
            preVerificationGas: packed_user_operation.pre_verification_gas,
            gasFees: packed_user_operation.gas_fees,
            paymasterAndData: packed_user_operation.paymaster_and_data,
            signature: packed_user_operation.signature,
        }
    }
}

#[derive(Clone)]
pub struct Chain<P: Provider + 'static> {
    entry_point: IEntryPointInstance<P>,
}

impl<P: Provider + 'static> Chain<P> {
    pub async fn new(provider: P) -> anyhow::Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        if chain_id != network_spec().chain_id() {
            anyhow::bail!(
                "Chain id mismatch: expected {}, got {}",
                network_spec().chain_id(),
                chain_id
            );
        }
        info!(
            "Connected to chain with chain id: {}",
            provider.get_chain_id().await?
        );

        let code = provider
            .get_code_at(network_spec().entry_point_address)
            .await?;
        if code.is_empty() {
            anyhow::bail!(
                "Entry point contract is not deployed at address: {}",
                network_spec().entry_point_address
            );
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
    ) -> anyhow::Result<B256> {
        self.entry_point
            .getUserOpHash(packed_user_operation.into())
            .call()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user operation hash: {}", e))
    }
}
