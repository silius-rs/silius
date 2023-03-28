use aa_bundler::{
    bundler::BundlerManager,
    models::wallet::Wallet,
    rpc::{
        debug::DebugApiServerImpl, debug_api::DebugApiServer, eth::EthApiServerImpl,
        eth_api::EthApiServer,
    },
    uopool::server::{
        bundler::bundler_client::BundlerClient, uopool::uo_pool_client::UoPoolClient,
    },
    utils::{parse_address, parse_u256},
};
use anyhow::Result;
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
    about = "Bundler for EIP-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub mnemonic_file: ExpandedPathBuf,

    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    #[clap(long)]
    pub no_uopool: bool,

    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::UoPoolOpts,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,

    #[clap(long)]
    pub no_rpc: bool,

    #[clap(long, default_value = "127.0.0.1:3000")]
    pub rpc_listen_address: String,

    #[clap(long, value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug"])]
    pub rpc_api: Vec<String>,

    // execution client rpc endpoint
    #[clap(long, default_value = "http://127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(flatten)]
    pub bundler_opts: aa_bundler::bundler::BundlerOpts,
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
                info!("Starting AA - Bundler");

                let eth_provider =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
                info!(
                    "Connected to Ethereum execution client at {}: {}",
                    opt.eth_client_address,
                    eth_provider.client_version().await?
                );

                let chain_id = eth_provider.get_chainid().await?;

                let wallet = Wallet::from_file(opt.mnemonic_file.clone(), chain_id)?;
                info!("{:?}", wallet.signer);

                let eth_provider =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);

                if !opt.no_uopool {
                    info!("Starting op pool with bundler");
                    aa_bundler::uopool::run(
                        opt.uopool_opts,
                        opt.entry_points.clone(),
                        eth_provider,
                        opt.max_verification_gas,
                    )
                    .await?;
                }

                info!("Connecting to uopool grpc");
                let uopool_grpc_client = UoPoolClient::connect(format!(
                    "http://{}",
                    opt.uopool_opts.uopool_grpc_listen_address
                ))
                .await?;
                info!("Connected to uopool grpc");

                let bundler_manager = BundlerManager::new(
                    wallet,
                    opt.bundler_opts.beneficiary,
                    uopool_grpc_client.clone(),
                    opt.entry_points,
                    opt.eth_client_address.clone(),
                    opt.bundler_opts.bundle_interval,
                );
                info!("Starting bundler manager");
                bundler_manager.start();

                if !opt.no_rpc {
                    info!("Starting rpc server with bundler");
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
                            info!("JSON-RPC server listening on {}", opt.rpc_listen_address);

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
