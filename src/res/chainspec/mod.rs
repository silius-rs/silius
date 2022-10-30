use crate::models::ChainSpec;
use once_cell::sync::Lazy;

pub static MAINNET: Lazy<ChainSpec> =
    Lazy::new(|| ron::from_str(include_str!("ethereum.ron")).unwrap());
pub static GOERLI: Lazy<ChainSpec> =
    Lazy::new(|| ron::from_str(include_str!("goerli.ron")).unwrap());
