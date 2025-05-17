use std::sync::{Arc, LazyLock, OnceLock};

use alloy_primitives::Address;

#[derive(Debug)]
pub enum Network {
    Mainnet,
}

static NETWORK_SPEC: OnceLock<Arc<NetworkSpec>> = OnceLock::new();

pub fn set_network_spec(network_spec: Arc<NetworkSpec>) {
    NETWORK_SPEC
        .set(network_spec)
        .expect("network spec already set");
}

pub fn network_spec() -> Arc<NetworkSpec> {
    NETWORK_SPEC.get().expect("network spec not set").clone()
}

#[derive(Debug)]
pub struct NetworkSpec {
    pub network: Network,
    pub entry_point_address: Address,
}

impl NetworkSpec {
    pub fn chain_id(&self) -> u64 {
        match self.network {
            Network::Mainnet => 1,
        }
    }
}

pub static MAINNET: LazyLock<Arc<NetworkSpec>> = LazyLock::new(|| {
    Arc::new(NetworkSpec {
        network: Network::Mainnet,
        entry_point_address: "0x4337084d9e255ff0702461cf8895ce9e3b5ff108"
            .parse()
            .unwrap(),
    })
});
