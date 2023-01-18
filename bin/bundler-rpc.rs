use anyhow::Result;
use clap::Parser;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::future::pending;

use aa_bundler::{
    rpc::{eth::EthApiServerImpl, eth_api::EthApiServer},
    uopool::server::uopool::uo_pool_client::UoPoolClient,
};

#[derive(Parser)]
#[clap(
    name = "aa-bundler-rpc",
    about = "JSON-RPC server for EIP-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(long, default_value = "127.0.0.1:4337")]
    pub rpc_listen_address: String,

    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let jsonrpc_server = ServerBuilder::default()
        .build(&opt.rpc_listen_address)
        .await?;

    let mut api = Methods::new();
    let uopool_grpc_client =
        UoPoolClient::connect(format!("http://{}", opt.uopool_grpc_listen_address)).await?;
    api.merge(
        EthApiServerImpl {
            call_gas_limit: 100_000_000,
            uopool_grpc_client,
        }
        .into_rpc(),
    )?;

    let _jsonrpc_server_handle = jsonrpc_server.start(api.clone())?;
    info!("JSON-RPC server listening on {}", opt.rpc_listen_address);

    pending().await
}
