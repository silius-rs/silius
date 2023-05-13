use aa_bundler_grpc::{uopool_service_run, UoPoolServiceOpts};
use aa_bundler_primitives::{parse_address, parse_u256, Chain, SUPPORTED_CHAINS};
use anyhow::{format_err, Result};
use clap::Parser;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use jsonrpsee::tracing::info;
use std::{future::pending, sync::Arc};

#[derive(Parser)]
#[clap(
    name = "aa-bundler-uopool",
    about = "User operation pool for ERC-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(flatten)]
    pub uopool_opts: UoPoolServiceOpts,

    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    #[clap(long, default_value= "dev", value_parser = SUPPORTED_CHAINS)]
    pub chain: Option<String>,

    // execution client rpc endpoint
    #[clap(long, default_value = "127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let eth_provider = Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
    info!(
        "Connected to Ethereum execution client at {}: {}",
        opt.eth_client_address,
        eth_provider.client_version().await?
    );

    let chain_id = eth_provider.get_chainid().await?;
    let chain = Chain::from(chain_id);

    if let Some(chain_opt) = opt.chain {
        if chain.name() != chain_opt {
            return Err(format_err!(
                "Bundler tries to connect to the execution client of different chain: {} != {}",
                chain_opt,
                chain.name()
            ));
        }
    }

    info!("Starting uopool gRPC service...");

    uopool_service_run(
        opt.uopool_opts,
        opt.entry_points,
        eth_provider,
        chain,
        opt.max_verification_gas,
    )
    .await?;

    info!(
        "Started uopool gRPC service at {:}",
        opt.uopool_opts.uopool_grpc_listen_address
    );

    pending().await
}
