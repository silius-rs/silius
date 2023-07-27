use silius::{
    cli::UoPoolServiceOpts,
    utils::{parse_address, parse_u256},
};
use silius_grpc::uopool_service_run;
use silius_primitives::{chain::SUPPORTED_CHAINS, Chain};
use anyhow::{format_err, Result};
use clap::Parser;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use std::{future::pending, sync::Arc};
use tracing::info;

#[derive(Parser)]
#[clap(
    name = "silius-uopool",
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

    let eth_client = Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
    info!(
        "Connected to Ethereum execution client at {}: {}",
        opt.eth_client_address,
        eth_client.client_version().await?
    );

    let chain_id = eth_client.get_chainid().await?;
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
        opt.uopool_opts.uopool_grpc_listen_address,
        opt.entry_points,
        eth_client,
        chain,
        opt.max_verification_gas,
        opt.uopool_opts.min_stake,
        opt.uopool_opts.min_unstake_delay,
        opt.uopool_opts.min_priority_fee_per_gas,
        opt.uopool_opts.whitelist,
        opt.uopool_opts.uo_pool_mode,
    )
    .await?;

    info!(
        "Started uopool gRPC service at {:}",
        opt.uopool_opts.uopool_grpc_listen_address
    );

    pending().await
}
