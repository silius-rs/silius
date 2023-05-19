use aa_bundler_grpc::{bundler_client::BundlerClient, uo_pool_client::UoPoolClient};
use aa_bundler_rpc::{DebugApiServer, DebugApiServerImpl, EthApiServer, EthApiServerImpl};
use anyhow::Result;
use clap::Parser;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::{collections::HashSet, future::pending};

#[derive(Parser)]
#[clap(
    name = "aa-bundler-rpc",
    about = "JSON-RPC server for ERC-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(long, default_value = "127.0.0.1:3000")]
    pub rpc_listen_address: String,

    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: String,

    #[clap(long, default_value = "127.0.0.1:3002")]
    pub bundler_grpc_listen_address: String,

    #[clap(long, value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug"])]
    pub rpc_api: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    info!("Starting bundler JSON-RPC server...");

    let jsonrpc_server = ServerBuilder::default()
        .build(&opt.rpc_listen_address)
        .await?;

    let mut api = Methods::new();
    let uopool_grpc_client =
        UoPoolClient::connect(format!("http://{}", opt.uopool_grpc_listen_address)).await?;

    let rpc_api: HashSet<String> = HashSet::from_iter(opt.rpc_api.iter().cloned());

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
        let bundler_grpc_client =
            BundlerClient::connect(format!("http://{}", opt.bundler_grpc_listen_address)).await?;
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

    pending().await
}
