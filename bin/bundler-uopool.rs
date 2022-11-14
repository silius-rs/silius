use anyhow::Result;
use clap::Parser;
use educe::Educe;
use std::future::pending;

#[derive(Educe, Parser)]
#[clap(
    name = "aa-bundler-uopool",
    about = "User operation pool for EIP-4337 Account Abstraction Bundler"
)]
#[educe(Debug)]
pub struct Opt {
    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::Opts,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    aa_bundler::uopool::run(opt.uopool_opts).await?;

    pending().await
}
