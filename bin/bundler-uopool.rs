use aa_bundler::{parse_address, parse_u256};
use anyhow::Result;
use clap::Parser;
use ethers::{
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

    #[clap(long, value_parser=parse_address)]
    pub entry_point: Address,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,

    // execution client rpc endpoint
    #[clap(long, default_value = "127.0.0.1:8545")]
    pub eth_client_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let eth_provider = Arc::new(Provider::<Http>::try_from(opt.eth_client_address)?);

    aa_bundler::uopool::run(
        opt.uopool_opts,
        eth_provider,
        opt.entry_point,
        opt.max_verification_gas,
    )
    .await?;

    pending().await
}
