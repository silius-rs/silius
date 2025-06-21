use alloy_provider::Provider;
use alloy_sol_types::sol;
use silius_primitives::network_spec::network_spec;
use tracing::info;

use crate::IEntryPoint::IEntryPointInstance;

sol!(
    #[sol(rpc)]
    IEntryPoint,
    "resources/entry_point_v08.json"
);

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
}
