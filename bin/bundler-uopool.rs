use aa_bundler::utils::{parse_address, parse_u256};
use anyhow::Result;
use clap::Parser;
use ethers::types::{Address, U256};
use std::future::pending;

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

    #[clap(long, value_parser=parse_u256)]
    pub chain_id: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    aa_bundler::uopool::run(opt.uopool_opts, opt.entry_points, opt.chain_id).await?;

    pending().await
}
