use std::sync::Arc;

use alloy_consensus::{Signed, TxEip1559, TypedTransaction};
use alloy_primitives::{TxHash, U256};
use alloy_provider::Provider;
use silius_chain::Chain;
use silius_primitives::{network_spec::network_spec, user_operation::UserOperation};
use silius_wallet::Wallet;

pub mod noop;
pub mod transaction;

#[async_trait::async_trait]
pub trait BundleSubmitter: Send + Sync {
    async fn submit_bundle(&self, bundle: Signed<TypedTransaction>) -> anyhow::Result<TxHash>;
}

#[derive(Clone)]
pub struct Builder<P: Provider + 'static> {
    pub chain: Arc<Chain<P>>,
    pub wallet: Wallet,
    pub is_active: bool,
    pub submitter: Arc<dyn BundleSubmitter>,
}

impl<P: Provider + 'static> Builder<P> {
    pub fn new(chain: Arc<Chain<P>>, wallet: Wallet, submitter: Arc<dyn BundleSubmitter>) -> Self {
        Self {
            chain,
            wallet,
            is_active: false,
            submitter,
        }
    }

    pub fn start(&mut self) {
        self.is_active = true;
    }

    pub fn stop(&mut self) {
        self.is_active = false;
    }

    pub async fn create_bundle(
        &self,
        user_operations: &[UserOperation],
    ) -> anyhow::Result<Signed<TypedTransaction>> {
        let address = self.wallet.signer().address();

        let transaction = self
            .chain
            .create_handle_ops_transaction_request(
                user_operations,
                address,
            )
            .await;

        let chain_id = network_spec().chain_id();
        let nonce = self.chain.provider().get_transaction_count(address).await?;
        let gas_limit = self
            .chain
            .provider()
            .estimate_gas(transaction.clone())
            .await?;
        let max_fee_per_gas = user_operations
            .iter()
            .map(|u| u.max_fee_per_gas)
            .sum::<U256>();
        let max_priority_fee_per_gas = user_operations
            .iter()
            .map(|u| u.max_priority_fee_per_gas)
            .sum::<U256>();
        let input = transaction.clone().input.input.unwrap_or_default();
        let access_list = self
            .chain
            .provider()
            .create_access_list(&transaction)
            .await?;

        let transaction = TypedTransaction::Eip1559(TxEip1559 {
            chain_id,
            nonce,
            gas_limit,
            max_fee_per_gas: max_fee_per_gas.to::<u128>() / user_operations.len() as u128,
            max_priority_fee_per_gas: max_priority_fee_per_gas.to::<u128>()
                / user_operations.len() as u128,
            to: network_spec().entry_point_address.into(),
            value: U256::ZERO,
            access_list: access_list.access_list,
            input,
        });

        self.wallet.sign_transaction(transaction).await
    }

    pub async fn submit_bundle(&self, bundle: Signed<TypedTransaction>) -> anyhow::Result<TxHash> {
        self.submitter.submit_bundle(bundle).await
    }
}
