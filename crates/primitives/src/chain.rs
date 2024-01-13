//! Chain extensions
use alloy_chains::{Chain, NamedChain};

pub trait ChainExt {
    fn canonical_mempool_id(&self) -> &str;
}

impl ChainExt for Chain {
    fn canonical_mempool_id(&self) -> &str {
        match self.named().expect("Canonical mempool on chain {self:?} is not supported!") {
            NamedChain::Dev => "Qmf7P3CuhzSbpJa8LqXPwRzfPqsvoQ6RG7aXvthYTzGxb2",
            NamedChain::Goerli => "QmTmj4cizhWpEFCCqk5dP67yws7R2PPgCtb2bd2RgVPCbF",
            NamedChain::Sepolia => "QmdDwVFoEEcgv5qnaTB8ncnXGMnqrhnA5nYpRr4ouWe4AT",
            NamedChain::PolygonMumbai => "QmQfRyE9iVTBqZ17hPSP4tuMzaez83Y5wD874ymyRtj9VE",
            _ => panic!("Canonical mempool on chain {self:?} is not supported!"),
        }
    }
}
