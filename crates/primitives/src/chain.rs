//! Chain information

use alloy_chains::{Chain, NamedChain};
use std::{fmt::Debug, time::Duration};

/// Chain specification structure
#[derive(PartialEq, Debug, Clone)]
pub struct ChainSpec {
    /// Chain
    pub chain: Chain,
    /// Block time/interval in milliseconds
    pub block_time: Duration,
    /// List of canonicam mempools
    pub canonical_mempools: Vec<String>,
}

impl ChainSpec {
    /// Constructs a 'ChainSpec' from chain id
    pub fn from_chain_id(chain_id: u64) -> Self {
        match chain_id {
            1 => Self::mainnet(),
            1337 => Self::dev(),
            11155111 => Self::sepolia(),
            137 => Self::polygon(),
            80001 => Self::polygon_mumbai(),
            421614 => Self::arbitrum_sepolia(),
            _ => Self::default(chain_id),
        }
    }

    /// 'ChainSpec' for mainnet
    pub fn mainnet() -> Self {
        Self {
            chain: Chain::from(NamedChain::Mainnet),
            block_time: Duration::from_secs(12),
            canonical_mempools: vec![],
        }
    }

    /// 'ChainSpec' for dev
    pub fn dev() -> Self {
        Self {
            chain: Chain::from(NamedChain::Dev),
            block_time: Duration::from_secs(1),
            canonical_mempools: vec!["Qmf7P3CuhzSbpJa8LqXPwRzfPqsvoQ6RG7aXvthYTzGxb2".into()],
        }
    }

    /// 'ChainSpec' for sepolia
    pub fn sepolia() -> Self {
        Self {
            chain: Chain::from(NamedChain::Sepolia),
            block_time: Duration::from_secs(12),
            canonical_mempools: vec!["QmdDwVFoEEcgv5qnaTB8ncnXGMnqrhnA5nYpRr4ouWe4AT".into()],
        }
    }

    /// 'ChainSpec' for polygon
    pub fn polygon() -> Self {
        Self {
            chain: Chain::from(NamedChain::Polygon),
            block_time: Duration::from_secs(2),
            canonical_mempools: vec![
                "QmRJ1EPhmRDb8SKrPLRXcUBi2weUN8VJ8X9zUtXByC7eJg".into(),
                "QmaHG3xiRYhxTth7vSTyZCyodBDrtj5hmEMz5DuzaJVKHH".into(),
            ],
        }
    }

    /// 'ChainSpec' for polygon mumbai
    pub fn polygon_mumbai() -> Self {
        Self {
            chain: Chain::from(NamedChain::PolygonMumbai),
            block_time: Duration::from_secs(2),
            canonical_mempools: vec!["QmQfRyE9iVTBqZ17hPSP4tuMzaez83Y5wD874ymyRtj9VE".into()],
        }
    }

    /// 'ChainSpec' for arbitrum sepolia
    pub fn arbitrum_sepolia() -> Self {
        Self {
            chain: Chain::from(NamedChain::ArbitrumSepolia),
            block_time: Duration::from_millis(250),
            canonical_mempools: vec!["QmVwhF77aVNzRUkMJNLDkeF9BtQMHLnfDY5ePpZ81uKLzA".into()],
        }
    }

    /// Default 'ChainSpec'
    pub fn default(chain_id: u64) -> Self {
        Self {
            chain: Chain::from_id(chain_id),
            block_time: Duration::from_secs(2), // Use default block time
            canonical_mempools: vec![],
        }
    }
}
