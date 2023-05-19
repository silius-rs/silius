use aa_bundler_grpc::{
    bundler_client::BundlerClient, bundler_service_run, uo_pool_client::UoPoolClient,
    uopool_service_run, BundlerServiceOpts, UoPoolServiceOpts,
};
use aa_bundler_primitives::{parse_address, parse_u256, Chain, Wallet, SUPPORTED_CHAINS};
use aa_bundler_rpc::{DebugApiServer, DebugApiServerImpl, EthApiServer, EthApiServerImpl};
use anyhow::{format_err, Result};
use clap::Parser;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::{collections::HashSet, future::pending, panic, sync::Arc};

#[derive(Parser)]
#[clap(
    name = "aa-bundler",
    about = "Bundler for ERC-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub mnemonic_file: ExpandedPathBuf,

    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    #[clap(long)]
    pub no_uopool: bool,

    #[clap(flatten)]
    pub uopool_opts: UoPoolServiceOpts,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,

    #[clap(long)]
    pub no_rpc: bool,

    #[clap(long, default_value = "127.0.0.1:3000")]
    pub rpc_listen_address: String,

    #[clap(long, value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug"])]
    pub rpc_api: Vec<String>,

    #[clap(long, default_value=None, value_parser = SUPPORTED_CHAINS)]
    pub chain: Option<String>,

    // execution client rpc endpoint
    #[clap(long, default_value = "http://127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(flatten)]
    pub bundler_opts: BundlerServiceOpts,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(128 * 1024 * 1024)
                .build()?;

            rt.block_on(async move {
                info!("Starting ERC-4337 AA Bundler");

                let eth_provider =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
                info!(
                    "Connected to the Ethereum execution client at {}: {}",
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

                let wallet = Wallet::from_file(opt.mnemonic_file.clone(), chain_id)
                    .map_err(|error| format_err!("Could not load mnemonic file: {}", error))?;
                info!("{:?}", wallet.signer);

                let eth_provider =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);

                if !opt.no_uopool {
                    info!("Starting uopool gRPC service...");
                    uopool_service_run(
                        opt.uopool_opts,
                        opt.entry_points.clone(),
                        eth_provider,
                        chain,
                        opt.max_verification_gas,
                    )
                    .await?;
                    info!(
                        "Started uopool gRPC service at {:}",
                        opt.uopool_opts.uopool_grpc_listen_address
                    );
                }

                info!("Connecting to uopool gRPC service");
                let uopool_grpc_client = UoPoolClient::connect(format!(
                    "http://{}",
                    opt.uopool_opts.uopool_grpc_listen_address
                ))
                .await?;
                info!("Connected to uopool gRPC service");

                info!("Starting bundler gRPC service...");
                bundler_service_run(
                    opt.bundler_opts,
                    wallet,
                    opt.entry_points,
                    chain,
                    opt.eth_client_address,
                    uopool_grpc_client.clone(),
                );
                info!(
                    "Started bundler gRPC service at {:}",
                    opt.bundler_opts.bundler_grpc_listen_address
                );

                if !opt.no_rpc {
                    info!("Starting bundler JSON-RPC server...");
                    tokio::spawn({
                        async move {
                            let jsonrpc_server = ServerBuilder::default()
                                .build(&opt.rpc_listen_address)
                                .await?;

                            let mut api = Methods::new();

                            let rpc_api: HashSet<String> =
                                HashSet::from_iter(opt.rpc_api.iter().cloned());

                            if rpc_api.contains("eth") {
                                api.merge(
                                    EthApiServerImpl {
                                        call_gas_limit: 100_000_000,
                                        uopool_grpc_client: uopool_grpc_client.clone(),
                                    }
                                    .into_rpc(),
                                )?;
                            }

                            if rpc_api.contains("debug") {
                                let bundler_grpc_client = BundlerClient::connect(format!(
                                    "http://{}",
                                    opt.bundler_opts.bundler_grpc_listen_address
                                ))
                                .await?;
                                api.merge(
                                    DebugApiServerImpl {
                                        uopool_grpc_client,
                                        bundler_grpc_client,
                                    }
                                    .into_rpc(),
                                )?;
                            }

                            let _jsonrpc_server_handle = jsonrpc_server.start(api.clone())?;
                            info!(
                                "Started bundler JSON-RPC server at {:}",
                                opt.rpc_listen_address
                            );

                            pending::<Result<()>>().await
                        }
                    });
                }

                pending().await
            })
        })?
        .join()
        .unwrap_or_else(|e| panic::resume_unwind(e))
}
