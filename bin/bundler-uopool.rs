use aa_bundler::utils::{parse_address, parse_u256};
use anyhow::Result;
use clap::Parser;
use ethers::{
    prelude::gas_oracle::ProviderOracle,
    providers::{Http, Provider},
    types::{Address, U256},
};
use std::{future::pending, sync::Arc};

#[derive(Parser)]
#[clap(
    name = "aa-bundler-uopool",
    about = "User operation pool for EIP-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::UoPoolOpts,

    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    // execution client rpc endpoint
    #[clap(long, default_value = "127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,

    #[clap(long, value_parser=parse_u256)]
    pub chain_id: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let eth_provider = Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
    let gas_oracle = Arc::new(ProviderOracle::new(Provider::<Http>::try_from(
        opt.eth_client_address,
    )?));

    aa_bundler::uopool::run(
        opt.uopool_opts,
        opt.entry_points,
        eth_provider,
        gas_oracle,
        opt.max_verification_gas,
        opt.chain_id,
    )
    .await?;

    pending().await
}
