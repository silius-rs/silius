use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_sol_types::sol;

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
    pub fn new(provider: P, entry_point_address: Address) -> Self {
        Self {
            entry_point: IEntryPoint::new(entry_point_address, provider),
        }
    }

    pub fn provider(&self) -> &P {
        self.entry_point.provider()
    }

    pub fn entry_point(&self) -> &IEntryPointInstance<P> {
        &self.entry_point
    }
}
