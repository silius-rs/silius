use alloy_chains::Chain;
use ethers::{providers::Middleware, types::Address};
use expanded_pathbuf::ExpandedPathBuf;
use parking_lot::RwLock;
use silius_bundler::{Bundler, EthereumClient};
use silius_contracts::EntryPoint;
use silius_mempool::{
    init_env, metrics::MetricsHandler, DatabaseTable, EntitiesReputation, Mempool, Reputation,
    UserOperations, UserOperationsByEntity, UserOperationsBySender, WriteMap,
};
use silius_primitives::{
    constants::{
        entry_point::ADDRESS,
        validation::reputation::{BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK},
    },
    provider::{create_http_block_streams, create_http_provider},
    Wallet,
};
use silius_rpc::{
    debug_api::{DebugApiServer, DebugApiServerImpl},
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer, JsonRpcServerType,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Ethereum client setup
    let eth_client = Arc::new(create_http_provider(
        "http://localhost:8545",
        Duration::from_millis(500),
    ).await?);

    let chain_id = eth_client.get_chainid().await?.as_u64();
    let chain = Chain::from(chain_id);
    let block_streams = create_http_block_streams(eth_client.clone(), 1).await;

    // Initialize mempool database
    let datadir = ExpandedPathBuf::from(".local/db");
    let db_env = Arc::new(init_env::<WriteMap>(datadir)?);
    db_env.create_tables()?;

    // Create mempool
    let mempool = Mempool::new(
        Box::new(MetricsHandler::new(DatabaseTable::<WriteMap, UserOperations>::new(
            db_env.clone(),
        ))),
        Box::new(DatabaseTable::<WriteMap, UserOperationsBySender>::new(db_env.clone())),
        Box::new(DatabaseTable::<WriteMap, UserOperationsByEntity>::new(db_env.clone())),
        Box::new(DatabaseTable::<WriteMap, UserOperations>::new(db_env.clone())),
    );

    // Initialize reputation system
    let reputation = Reputation::new(
        MIN_INCLUSION_RATE_DENOMINATOR,
        THROTTLING_SLACK,
        BAN_SLACK,
        1_000_000u64.into(), // min_stake
        100u64.into(),       // min_unstake_delay
        Arc::new(RwLock::new(HashSet::<Address>::default())),
        Arc::new(RwLock::new(HashSet::<Address>::default())),
        Box::new(MetricsHandler::new(DatabaseTable::<WriteMap, EntitiesReputation>::new(
            db_env,
        ))),
    );

    // Create bundler
    let entry_point = EntryPoint::new(eth_client.clone(), ADDRESS.parse()?);
    let wallet = Wallet::from_file(".silius/bundler-wallet".into(), chain_id, false)?;
    let client = Arc::new(EthereumClient::new(eth_client.clone(), wallet.clone()));

    let bundler = Bundler::new(
        wallet.clone(),
        wallet.signer.address(),
        entry_point.address(),
        chain,
        100_000_000_000_000_000u64.into(), // min_balance
        eth_client.clone(),
        client,
        false, // enable_access_list
    );

    // Set up RPC server
    let mut server = JsonRpcServer::new(
        true,                   // http enabled
        "127.0.0.1".parse()?,  // http addr
        3000,                  // http port
        true,                   // ws enabled
        "127.0.0.1".parse()?,  // ws addr
        3001,                  // ws port
    )
    .with_cors(&["*".into()], JsonRpcServerType::Http)
    .with_cors(&["*".into()], JsonRpcServerType::Ws);

    // Add API implementations with direct component access
    server.add_methods(Web3ApiServerImpl{}.into_rpc(), JsonRpcServerType::Http)?;
    server.add_methods(Web3ApiServerImpl{}.into_rpc(), JsonRpcServerType::Ws)?;

    let eth_api = EthApiServerImpl::new(mempool.clone(), bundler.clone());
    server.add_methods(eth_api.clone().into_rpc(), JsonRpcServerType::Http)?;
    server.add_methods(eth_api.into_rpc(), JsonRpcServerType::Ws)?;

    let debug_api = DebugApiServerImpl::new(mempool.clone(), bundler.clone());
    server.add_methods(debug_api.clone().into_rpc(), JsonRpcServerType::Http)?;
    server.add_methods(debug_api.into_rpc(), JsonRpcServerType::Ws)?;

    // Start server
    let (_http_handle, _ws_handle) = server.start().await?;

    info!(
        "Started JSON-RPC server at http://127.0.0.1:3000 and ws://127.0.0.1:3001"
    );

    // Handle Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}