use anyhow::Result;
use clap::Parser;
use educe::Educe;
use ethers::providers::{Http, Provider};
use std::{future::pending, sync::Arc};

#[derive(Educe, Parser)]
#[clap(
    name = "aa-bundler-uopool",
    about = "User operation pool for EIP-4337 Account Abstraction Bundler"
)]
#[educe(Debug)]
pub struct Opt {
    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::UoPoolOpts,

    // execution client rpc endpoint
    #[clap(long, default_value = "127.0.0.1:8545")]
    pub eth_client_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let eth_provider = Arc::new(Provider::<Http>::try_from(opt.eth_client_address)?);

    aa_bundler::uopool::run(opt.uopool_opts, eth_provider).await?;

    pending().await
}
