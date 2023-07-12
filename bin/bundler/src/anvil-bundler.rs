use aa_bundler_grpc::{
    bundler_service_run, uo_pool_client::UoPoolClient,
    uopool_service_run,
};
use aa_bundler_primitives::{Chain, Wallet, UoPoolMode};
use aa_bundler_rpc::{
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer,
};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use anyhow::{format_err, Result};
use ethers::{
	core::utils::Anvil,
	providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use std::{collections::HashSet, panic, sync::Arc};
use log;
use std::env;
use tracing::info;
use dotenv::dotenv;

fn main() -> Result<()> {

    tracing_subscriber::fmt::init();

    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(128 * 1024 * 1024)
                .build()?;

            let _ = rt.block_on(async {

                let task = async move {
    
                    dotenv().ok();
                    let mainnet_http_url = env::var("HTTP_RPC").unwrap_or_else(|e| {
                        log::error!("Error: {}", e);
                        return e.to_string();
                    });
    
                    let temp_provider = Provider::<Http>::try_from(mainnet_http_url.clone()).unwrap();
                    let latest_block = temp_provider.get_block_number().await.unwrap();
                    drop(temp_provider);
    
                    let port = 8545u16;
                    let url = format!("http://localhost:{}", port).to_string();
    
                    let _anvil = Anvil::new()
                        .port(port)
                        .fork(mainnet_http_url.clone())
                        .fork_block_number(latest_block.as_u64())
                        .spawn();
    
                    println!("Connecting to anvil instance at {}", url);
                    let provider = Arc::new(
                        Provider::<Http>::try_from(url.clone())
                            .ok()
                            .unwrap(),
                    );
                    let block = provider.get_block_number().await?;
                    println!("Provider address: {}", block);
                    log::info!("Connected to anvil instance at {}", url);
                    info!("Starting ERC-4337 AA Bundler");
    
                    let eth_client = provider.clone();
    
                    info!(
                        "Connected to the Ethereum execution client at {}: {}",
                        "http://localhost:8545",
                        eth_client.client_version().await?
                    );
    
                    let chain_id = eth_client.get_chainid().await?;
                    let chain = Chain::from(chain_id);
    
                    let wallet = Wallet::from_key(
                        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80", 
                        &chain_id
                    ).map_err(|error| format_err!("Could not load mnemonic file: {}", error))?;
    
                    uopool_service_run(
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3001),
                        vec!["0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse::<Address>().unwrap()],
                        eth_client,
                        chain,
                        U256::from(1500000),
                        U256::from(1),
                        U256::from(0),
                        U256::from(0),
                        vec!["0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse::<Address>().unwrap()],
                        UoPoolMode::Standard,
                    )
                    .await?;
                    info!(
                        "Started uopool gRPC service at {:}",
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3001),
                    );
    
                    info!("Connecting to uopool gRPC service");
                    let uopool_grpc_client = UoPoolClient::connect(format!(
                        "http://{}",
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3001),
                    ))
                    .await?;
                    info!("Connected to uopool gRPC service");
    
                    info!("Starting bundler gRPC service...");
                    bundler_service_run(
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3001),
                        wallet,
                        vec!["0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse::<Address>().unwrap()],
                        "http://localhost:8545".to_string(),
                        chain,
                        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 ".parse::<Address>().unwrap(),
                        U256::from(600),
                        U256::from(1),
                        10,
                        uopool_grpc_client.clone(),
                    );
                    info!(
                        "Started bundler gRPC service at {:}",
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3001),
                    );
    
    
                    info!("Starting bundler JSON-RPC server...");
                    tokio::spawn({
                        async move {
                            let api: HashSet<String> =
                                HashSet::from_iter(vec!["eth".to_string()]);
    
                            let mut server = JsonRpcServer::new("127.0.0.1:3000".to_string())
                                .with_proxy("http://localhost:8545".to_string())
                                .with_cors(vec!["*".to_string()]);
    
                            server.add_method(Web3ApiServerImpl{}.into_rpc())?;
    
                            if api.contains("eth") {
                                server.add_method(
                                    EthApiServerImpl {
                                        uopool_grpc_client: uopool_grpc_client.clone(),
                                    }
                                    .into_rpc(),
                                )?;
                            }
    
                            let _handle = server.start().await?;
                            info!(
                                "Started bundler JSON-RPC server at {:}",
                                "127.0.0.1:3000".to_string()
                            );

                            loop {
                                let stopped = _handle.is_stopped();
                                log::info!("The server is running: {}", !stopped);
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            }
                            Ok::<(), anyhow::Error>(())
                        }
                    });
                    drop(_anvil);
                    Ok::<(), anyhow::Error>(())

                };
                let _ = task.await;
            });

            rt.block_on(async {
                let ctrl_c = tokio::signal::ctrl_c();
                tokio::select! {
                    _ = ctrl_c => {
                        println!("Ctrl+C received, shutting down");
                    }
                    else => {
                        println!("Server stopped unexpectedly");
                    }
                }
            });
            Ok(())

        })?
        .join()
        .unwrap_or_else(|e| panic::resume_unwind(e))
}
