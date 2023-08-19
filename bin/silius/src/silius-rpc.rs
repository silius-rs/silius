use anyhow::Result;
use clap::Parser;
use silius::cli::RpcServiceOpts;
use silius_grpc::{bundler_client::BundlerClient, uo_pool_client::UoPoolClient};
use silius_rpc::{
    debug_api::{DebugApiServer, DebugApiServerImpl},
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer,
};
use std::{collections::HashSet, future::pending};
use tracing::info;

#[derive(Parser)]
#[clap(
    name = "silius-rpc",
    about = "JSON-RPC server for ERC-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(flatten)]
    pub rpc_opts: RpcServiceOpts,

    // execution client rpc endpoint
    #[clap(long, default_value = "http://127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: String,

    #[clap(long, default_value = "127.0.0.1:3002")]
    pub bundler_grpc_listen_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    if !opt.rpc_opts.is_enabled() {
        return Err(anyhow::anyhow!("No RPC protocol is enabled"));
    }

    tracing_subscriber::fmt::init();

    info!("Starting bundler JSON-RPC server...");

    let api: HashSet<String> = HashSet::from_iter(opt.rpc_opts.rpc_api.iter().cloned());

    let mut server = JsonRpcServer::new(
        opt.rpc_opts.rpc_listen_address.clone(),
        opt.rpc_opts.http,
        opt.rpc_opts.ws,
    )
    .with_proxy(opt.eth_client_address)
    .with_cors(opt.rpc_opts.cors_domain);

    if api.contains("web3") {
        server.add_method(Web3ApiServerImpl {}.into_rpc())?;
    }

    let uopool_grpc_client =
        UoPoolClient::connect(format!("http://{}", opt.uopool_grpc_listen_address)).await?;

    if api.contains("eth") {
        server.add_method(
            EthApiServerImpl {
                uopool_grpc_client: uopool_grpc_client.clone(),
            }
            .into_rpc(),
        )?;
    }

    if api.contains("debug") {
        let bundler_grpc_client =
            BundlerClient::connect(format!("http://{}", opt.bundler_grpc_listen_address)).await?;
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
        opt.rpc_opts.rpc_listen_address, opt.rpc_opts.http, opt.rpc_opts.ws
    );

    pending::<Result<()>>().await
}
