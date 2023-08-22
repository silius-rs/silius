use ethers::types::U256;

/// Currently supported chains
pub const SUPPORTED_CHAINS: [&str; 5] = [
    "mainnet", // Ethereum mainnet
    "goerli",  // Ethereum goerli testnet
    "sepolia", // Ethereum goerli testnet
    "dev",     // Development chain
    "mumbai",  // Polygon PoS testnet
];

/// Chain information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chain {
    Named(ethers::types::Chain),
    Custom(u64),
}

impl Chain {
    pub fn id(&self) -> u64 {
        match self {
            Chain::Named(chain) => *chain as u64,
            Chain::Custom(id) => *id,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Chain::Named(chain) => chain.to_string(),
            Chain::Custom(_) => "custom".to_string(),
        }
    }
}

impl From<u64> for Chain {
    fn from(id: u64) -> Self {
        ethers::types::Chain::try_from(id)
            .map(Chain::Named)
            .unwrap_or_else(|_| Chain::Custom(id))
    }
}

impl From<U256> for Chain {
    fn from(id: U256) -> Self {
        id.as_u64().into()
    }
}
