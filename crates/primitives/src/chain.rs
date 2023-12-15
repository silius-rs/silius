//! Chain extensions

use crate::constants::supported_chains::{DEV, GOERLI, MUMBAI, SEPOLIA};
use alloy_chains::Chain;

pub trait ChainExt {
    fn canonical_mempool_id(&self) -> &str;
}

impl ChainExt for Chain {
    fn canonical_mempool_id(&self) -> &str {
        match self.id() {
            DEV => "Qmf7P3CuhzSbpJa8LqXPwRzfPqsvoQ6RG7aXvthYTzGxb2",
            GOERLI => "QmTmj4cizhWpEFCCqk5dP67yws7R2PPgCtb2bd2RgVPCbF",
            SEPOLIA => "QmdDwVFoEEcgv5qnaTB8ncnXGMnqrhnA5nYpRr4ouWe4AT",
            MUMBAI => "QmQfRyE9iVTBqZ17hPSP4tuMzaez83Y5wD874ymyRtj9VE",
            _ => panic!("Canonical mempool on chain {self:?} is not supported!"),
        }
    }
}
