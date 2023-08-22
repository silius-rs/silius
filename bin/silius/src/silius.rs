use anyhow::{format_err, Result};
use clap::Parser;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use silius::{
    cli::{BundlerServiceOpts, RpcServiceOpts, UoPoolServiceOpts},
    utils::{parse_address, parse_u256, run_until_ctrl_c},
};
use silius_bundler::SendBundleMode;
use silius_grpc::{
    bundler_client::BundlerClient, bundler_service_run, uo_pool_client::UoPoolClient,
    uopool_service_run,
};
use silius_primitives::{
    chain::SUPPORTED_CHAINS, consts::flashbots_relay_endpoints, Chain, Wallet,
};
use silius_rpc::{
    debug_api::{DebugApiServer, DebugApiServerImpl},
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer,
};
use std::{collections::HashSet, future::pending, panic, sync::Arc};
use tracing::info;

#[derive(Parser)]
#[clap(name = "silius", about = "Bundler for ERC-4337 Account Abstraction")]
pub struct Opt {
    #[clap(long)]
    pub mnemonic_file: ExpandedPathBuf,

    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    #[clap(long)]
    pub no_uopool: bool,

    #[clap(flatten)]
    pub uopool_opts: UoPoolServiceOpts,

    #[clap(long, default_value="3000000", value_parser=parse_u256)]
    pub max_verification_gas: U256,

    #[clap(flatten)]
    pub rpc_opts: RpcServiceOpts,

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

    std::env::set_var("RUST_LOG", "info");
    tracing_subscriber::fmt::init();

    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(128 * 1024 * 1024)
                .build()?;

            let task = async move {
                info!("Starting ERC-4337 AA Bundler");

                let eth_client =
                    Arc::new(Provider::<Http>::try_from(opt.eth_client_address.clone())?);
                info!(
                    "Connected to the Ethereum execution client at {}: {}",
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

                let wallet: Wallet;
                if opt.rpc_opts.build_fb_signer == Some(true) {
                    wallet = Wallet::from_file(
                        opt.mnemonic_file.clone(),
                        &chain_id,
                        true,
                    )
                    .map_err(|error| format_err!("Could not load mnemonic file: {}", error))?;
                    info!("Wallet Signer {:?}", wallet.signer);
                    info!("Flashbots Signer {:?}", wallet.fb_signer);
                } else {
                    wallet = Wallet::from_file(opt.mnemonic_file.clone(), &chain_id, false)
                        .map_err(|error| format_err!("Could not load mnemonic file: {}", error))?;
                    info!("{:?}", wallet.signer);
                }


                if !opt.no_uopool {
                    info!("Starting uopool gRPC service...");
                    uopool_service_run(
                        opt.uopool_opts.uopool_grpc_listen_address,
                        opt.entry_points.clone(),
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
                    opt.bundler_opts.bundler_grpc_listen_address,
                    wallet,
                    opt.entry_points,
                    opt.eth_client_address.clone(),
                    chain,
                    opt.bundler_opts.beneficiary,
                    opt.bundler_opts.min_balance,
                    opt.bundler_opts.bundle_interval,
                    uopool_grpc_client.clone(),
                    match opt.rpc_opts.send_bundle_mode.as_deref() {
                        Some(mode) => match mode {
                            "eth-client" => SendBundleMode::EthClient,
                            "flashbots" => SendBundleMode::Flashbots,
                            _ => SendBundleMode::EthClient,
                        },
                        None => SendBundleMode::EthClient,
                    },
                    match opt.rpc_opts.clone().send_bundle_mode {
                        Some(mode) => match mode.clone().as_str() {
                            "eth-client" => None,
                            "flashbots" => Some(vec![
                                flashbots_relay_endpoints::FLASHBOTS.to_string(),
                            ]),
                            _ => None,
                        },
                        None => None,
                    },
                );
                info!(
                    "Started bundler gRPC service at {:}",
                    opt.bundler_opts.bundler_grpc_listen_address
                );

                if opt.rpc_opts.is_enabled() {

                    info!("Starting bundler JSON-RPC server...");
                    tokio::spawn({
                        async move {
                            let api: HashSet<String> =
                                HashSet::from_iter(opt.rpc_opts.rpc_api.iter().cloned());

                            let mut server = JsonRpcServer::new(opt.rpc_opts.rpc_listen_address.clone(), opt.rpc_opts.http, opt.rpc_opts.ws)
                            .with_proxy(opt.eth_client_address)
                            .with_cors(opt.rpc_opts.cors_domain);

                            if api.contains("web3") {
                                server.add_method(Web3ApiServerImpl{}.into_rpc())?;
                            }

                            if api.contains("eth") {
                                server.add_method(
                                    EthApiServerImpl {
                                        uopool_grpc_client: uopool_grpc_client.clone(),
                                    }
                                    .into_rpc(),
                                )?;
                            }

                            if api.contains("debug") {
                                let bundler_grpc_client = BundlerClient::connect(format!(
                                    "http://{}",
                                    opt.bundler_opts.bundler_grpc_listen_address
                                ))
                                .await?;
                                server.add_method(
                                    DebugApiServerImpl {
                                        uopool_grpc_client,
                                        bundler_grpc_client,
                                    }
                                    .into_rpc(),
                                )?;
                            }

                            let _handle = server.start().await?;
                            info!(
                                "Started bundler JSON-RPC server at {:} with http: {:?} ws: {:?}",
                                opt.rpc_opts.rpc_listen_address,
                                opt.rpc_opts.http,
                                opt.rpc_opts.ws
                            );

                            pending::<Result<()>>().await
                        }
                    });
                }

                pending().await
            };
            rt.block_on(run_until_ctrl_c(task))?;
            Ok(())

        })?
        .join()
        .unwrap_or_else(|e| panic::resume_unwind(e))
}
