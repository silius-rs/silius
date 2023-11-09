use crate::{
    cli::args::{BundlerAndUoPoolArgs, BundlerArgs, CreateWalletArgs, RpcArgs, UoPoolArgs},
    utils::unwrap_path_or_home,
};
use ethers::{providers::Middleware, types::Address};
use silius_grpc::{
    bundler_client::BundlerClient, bundler_service_run, uo_pool_client::UoPoolClient,
    uopool_service_run,
};
use silius_primitives::{
    bundler::SendBundleMode,
    consts::{flashbots_relay_endpoints, p2p::DISCOVERY_SECRET_FILE_NAME},
    provider::BlockStream,
    Chain, Wallet,
};
use silius_rpc::{
    debug_api::{DebugApiServer, DebugApiServerImpl},
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer, JsonRpcServerType,
};
use std::{collections::HashSet, future::pending, net::SocketAddr, sync::Arc};
use tracing::info;

pub async fn launch_bundler<M>(
    bundler_args: BundlerArgs,
    uopool_args: UoPoolArgs,
    common_args: BundlerAndUoPoolArgs,
    rpc_args: RpcArgs,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    launch_uopool(
        uopool_args.clone(),
        eth_client.clone(),
        block_streams,
        common_args.chain.clone(),
        common_args.entry_points.clone(),
    )
    .await?;

    launch_bundling(
        bundler_args.clone(),
        eth_client.clone(),
        common_args.chain,
        common_args.entry_points,
        format!(
            "http://{:?}:{:?}",
            uopool_args.uopool_addr, uopool_args.uopool_port
        ),
    )
    .await?;

    launch_rpc(
        rpc_args,
        format!(
            "http://{:?}:{:?}",
            uopool_args.uopool_addr, uopool_args.uopool_port
        ),
        format!(
            "http://{:?}:{:?}",
            bundler_args.bundler_addr, bundler_args.bundler_port
        ),
    )
    .await?;

    Ok(())
}

pub async fn launch_bundling<M>(
    args: BundlerArgs,
    eth_client: Arc<M>,
    chain: Option<String>,
    entry_points: Vec<Address>,
    uopool_grpc_listen_address: String,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    info!("Starting bundling gRPC service...");

    let eth_client_version = check_connected_chain(eth_client.clone(), chain).await?;
    info!(
        "Bundling component connected to Ethereum execution client with version {}",
        eth_client_version,
    );

    let chain_id = eth_client.get_chainid().await?;
    let chain_conn = Chain::from(chain_id);

    let wallet: Wallet;
    if args.send_bundle_mode == SendBundleMode::Flashbots {
        wallet = Wallet::from_file(args.mnemonic_file.into(), &chain_id, true)
            .map_err(|error| eyre::format_err!("Could not load mnemonic file: {}", error))?;
        info!("Wallet Signer {:?}", wallet.signer);
        info!("Flashbots Signer {:?}", wallet.flashbots_signer);
    } else {
        wallet = Wallet::from_file(args.mnemonic_file.into(), &chain_id, false)
            .map_err(|error| eyre::format_err!("Could not load mnemonic file: {}", error))?;
        info!("{:?}", wallet.signer);
    }

    info!("Connecting to uopool gRPC service...");
    let uopool_grpc_client = UoPoolClient::connect(uopool_grpc_listen_address).await?;
    info!("Connected to uopool gRPC service");

    bundler_service_run(
        SocketAddr::new(args.bundler_addr, args.bundler_port),
        wallet,
        entry_points,
        eth_client,
        chain_conn,
        args.beneficiary,
        args.min_balance,
        args.bundle_interval,
        uopool_grpc_client,
        args.send_bundle_mode,
        match args.send_bundle_mode {
            SendBundleMode::EthClient => None,
            SendBundleMode::Flashbots => {
                Some(vec![flashbots_relay_endpoints::FLASHBOTS.to_string()])
            }
        },
    );
    info!(
        "Started bundler gRPC service at {:?}:{:?}",
        args.bundler_addr, args.bundler_port
    );

    Ok(())
}

pub async fn launch_uopool<M>(
    args: UoPoolArgs,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
    chain: Option<String>,
    entry_points: Vec<Address>,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    info!("Starting uopool gRPC service...");

    let eth_client_version = check_connected_chain(eth_client.clone(), chain).await?;
    info!(
        "UoPool component connected to Ethereum execution client with version {}",
        eth_client_version
    );

    let chain_id = Chain::from(eth_client.get_chainid().await?);

    let datadir = unwrap_path_or_home(args.datadir)?;

    let node_key_file = match args.p2p_opts.node_key.clone() {
        Some(key_file) => key_file,
        None => datadir.join(DISCOVERY_SECRET_FILE_NAME),
    };

    uopool_service_run(
        SocketAddr::new(args.uopool_addr, args.uopool_port),
        datadir,
        entry_points,
        eth_client,
        block_streams,
        chain_id,
        args.max_verification_gas,
        args.min_stake,
        args.min_priority_fee_per_gas,
        args.whitelist,
        args.uopool_mode,
        args.p2p_opts.enable_p2p,
        node_key_file,
        args.p2p_opts.to_config(),
        args.p2p_opts.bootnodes,
    )
    .await?;

    info!(
        "Started uopool gRPC service at {:?}:{:?}",
        args.uopool_addr, args.uopool_port
    );

    Ok(())
}

pub async fn launch_rpc(
    args: RpcArgs,
    uopool_grpc_listen_address: String,
    bundler_grpc_listen_address: String,
) -> eyre::Result<()> {
    if !args.is_enabled() {
        return Err(eyre::eyre!("No RPC protocol is enabled"));
    }

    info!("Starting bundler JSON-RPC server...");

    let mut server = JsonRpcServer::new(
        args.http,
        args.http_addr,
        args.http_port,
        args.ws,
        args.ws_addr,
        args.ws_port,
    )
    .with_cors(&args.http_corsdomain, JsonRpcServerType::Http)
    .with_cors(&args.ws_origins, JsonRpcServerType::Ws);

    if let Some(eth_client_proxy_address) = args.eth_client_proxy_address.clone() {
        server = server.with_proxy(eth_client_proxy_address);
    }

    let http_api: HashSet<String> = HashSet::from_iter(args.http_api.iter().cloned());
    let ws_api: HashSet<String> = HashSet::from_iter(args.ws_api.iter().cloned());

    if http_api.contains("web3") {
        server.add_methods(Web3ApiServerImpl {}.into_rpc(), JsonRpcServerType::Http)?;
    }
    if ws_api.contains("web3") {
        server.add_methods(Web3ApiServerImpl {}.into_rpc(), JsonRpcServerType::Ws)?;
    }

    info!("Connecting to uopool gRPC service...");
    let uopool_grpc_client = UoPoolClient::connect(uopool_grpc_listen_address).await?;
    info!("Connected to uopool gRPC service...");

    if args.is_api_method_enabled("eth") {
        if http_api.contains("eth") {
            server.add_methods(
                EthApiServerImpl {
                    uopool_grpc_client: uopool_grpc_client.clone(),
                }
                .into_rpc(),
                JsonRpcServerType::Http,
            )?;
        }
        if ws_api.contains("eth") {
            server.add_methods(
                EthApiServerImpl {
                    uopool_grpc_client: uopool_grpc_client.clone(),
                }
                .into_rpc(),
                JsonRpcServerType::Ws,
            )?;
        }
    }

    if args.is_api_method_enabled("debug") {
        info!("Connecting to bundling gRPC service...");
        let bundler_grpc_client = BundlerClient::connect(bundler_grpc_listen_address).await?;
        info!("Connected to bundling gRPC service...");

        if http_api.contains("debug") {
            server.add_methods(
                DebugApiServerImpl {
                    uopool_grpc_client: uopool_grpc_client.clone(),
                    bundler_grpc_client: bundler_grpc_client.clone(),
                }
                .into_rpc(),
                JsonRpcServerType::Http,
            )?;
        }

        if ws_api.contains("debug") {
            server.add_methods(
                DebugApiServerImpl {
                    uopool_grpc_client,
                    bundler_grpc_client,
                }
                .into_rpc(),
                JsonRpcServerType::Ws,
            )?;
        }
    }

    tokio::spawn(async move {
        let (_http_handle, _ws_handle) = server.start().await?;

        info!(
            "Started bundler JSON-RPC server with http: {:?}:{:?}, ws: {:?}:{:?}",
            args.http_addr, args.http_port, args.ws_addr, args.ws_port,
        );
        pending::<eyre::Result<()>>().await
    });

    Ok(())
}

pub fn create_wallet(args: CreateWalletArgs) -> eyre::Result<()> {
    info!(
        "Creating bundler wallet... Storing to: {:?}",
        args.output_path
    );

    let path = unwrap_path_or_home(args.output_path)?;

    if args.flashbots_key {
        let wallet = Wallet::build_random(path, &args.chain_id, true)?;
        info!("Wallet signer {:?}", wallet.signer);
        info!("Flashbots signer {:?}", wallet.flashbots_signer);
    } else {
        let wallet = Wallet::build_random(path, &args.chain_id, false)?;
        info!("Wallet signer {:?}", wallet.signer);
    }

    Ok(())
}

async fn check_connected_chain<M>(eth_client: Arc<M>, chain: Option<String>) -> eyre::Result<String>
where
    M: Middleware + Clone + 'static,
{
    let chain_id = eth_client.get_chainid().await?;
    let chain_conn = Chain::from(chain_id);

    if let Some(chain_opt) = chain {
        if chain_conn.name() != chain_opt {
            return Err(eyre::format_err!(
                "Tried to connect to the execution client of different chain: {} != {}",
                chain_opt,
                chain_conn.name()
            ));
        }
    }

    Ok(eth_client.client_version().await?)
}
