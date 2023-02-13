use aa_bundler::{
    bundler::Bundler,
    models::wallet::Wallet,
    rpc::{
        debug::DebugApiServerImpl, debug_api::DebugApiServer, eth::EthApiServerImpl,
        eth_api::EthApiServer,
    },
    uopool::server::uopool::uo_pool_client::UoPoolClient,
    utils::{parse_address, parse_u256},
};
use anyhow::Result;
use clap::Parser;
use ethers::{
    providers::{Http, Middleware, Provider},
    {
    prelude::gas_oracle::ProviderOracle,
    types::{Address, U256},
},
};
use expanded_pathbuf::ExpandedPathBuf;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::{future::pending, panic, sync::Arc};

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

    #[clap(long, value_parser=parse_u256)]
    pub chain_id: U256,

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

                let wallet = Wallet::from_file(opt.mnemonic_file, opt.chain_id)?;
                info!("{:?}", wallet.signer);

                let eth_provider =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
                let gas_oracle = Arc::new(ProviderOracle::new(Provider::<Http>::try_from(
                    opt.eth_client_address,
                )?));

                let _bundler = Bundler::new(wallet);

                if !opt.no_uopool {
                    aa_bundler::uopool::run(
                        opt.uopool_opts,
                        opt.entry_points,
                        eth_provider,
                        gas_oracle,
                        opt.max_verification_gas,
                        opt.chain_id,
                    )
                    .await?;
                }

                if !opt.no_rpc {
                    tokio::spawn({
                        async move {
                            let jsonrpc_server = ServerBuilder::default()
                                .build(&opt.rpc_listen_address)
                                .await?;

                            let mut api = Methods::new();
                            let uopool_grpc_client = UoPoolClient::connect(format!(
                                "http://{}",
                                opt.uopool_opts.uopool_grpc_listen_address
                            ))
                            .await?;

                            #[cfg(debug_assertions)]
                            api.merge(
                                DebugApiServerImpl {
                                    uopool_grpc_client: uopool_grpc_client.clone(),
                                }
                                .into_rpc(),
                            )?;
                            api.merge(
                                EthApiServerImpl {
                                    call_gas_limit: 100_000_000,
                                    uopool_grpc_client,
                                }
                                .into_rpc(),
                            )?;

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
