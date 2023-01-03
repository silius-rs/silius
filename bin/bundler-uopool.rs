use aa_bundler::utils::parse_address;
use anyhow::Result;
use clap::Parser;
use ethers::types::Address;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    aa_bundler::uopool::run(opt.uopool_opts, opt.entry_points).await?;

    pending().await
}
