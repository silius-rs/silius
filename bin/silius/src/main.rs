use std::{env, process, sync::Arc};

use alloy_provider::{ProviderBuilder, WsConnect};
use clap::Parser;
use silius::{
    cli::{Cli, Commands, NodeConfig},
    utils::print_ascii_logo,
};
use silius_builder::{Builder, noop::NoopSubmitter, transaction::TransactionSubmitter};
use silius_chain::Chain;
use silius_executor::SiliusExecutor;
use silius_manager::SiliusManager;
use silius_mempool::Mempool;
use silius_primitives::network_spec::{network_spec, set_network_spec};
use silius_rpc::{
    http::{HttpRpcServerConfig, start_http_server},
    ws::{WsRpcServerConfig, start_ws_server},
};
use silius_storage::{
    db::{SiliusDB, reset_db},
    dir::setup_data_dir,
};
use silius_validator::validator::Validator;
use silius_wallet::{KeySource, Wallet};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub const APP_NAME: &str = "silius";

fn main() {
    print_ascii_logo();

    let rust_log = env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default();
    let env_filter = match rust_log.is_empty() {
        true => EnvFilter::builder().parse_lossy("info"),
        false => EnvFilter::builder().parse_lossy(rust_log),
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();

    let executor = SiliusExecutor::new().expect("Failed to create executor");
    let executor_clone = executor.clone();

    match cli.command {
        Commands::Node(config) => {
            executor_clone.spawn(async move { run_silius_node(config, executor).await });
        }
    }

    executor_clone.runtime().block_on(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to pause until Ctrl-c");
        info!("Ctrl-c received, shutting down...");
        executor_clone.shutdown_signal();
    });

    executor_clone.shutdown_runtime();

    process::exit(0);
}

pub async fn run_silius_node(config: NodeConfig, executor: SiliusExecutor) {
    info!("Silius starting up...");

    set_network_spec(config.network.clone());
    info!("Network spec: {}", network_spec());

    let silius_dir = setup_data_dir(
        APP_NAME,
        config.mempool_config.data_dir.clone(),
        config.mempool_config.ephemeral,
    )
    .expect("Unable to initialize database directory");

    if config.mempool_config.purge_db {
        reset_db(silius_dir.clone()).expect("Unable to delete database");
    }

    let silius_db = SiliusDB::new(silius_dir).expect("Unable to init Silius database");

    info!("Silius database initialized!");

    let chain = if config.provider_url.starts_with("http") {
        let provider = ProviderBuilder::new().connect_http(
            config
                .provider_url
                .parse()
                .expect("Cannot parse HTTP provider URL"),
        );
        Arc::new(
            Chain::new(provider)
                .await
                .expect("Failed to connect to chain"),
        )
    } else if config.provider_url.starts_with("ws") {
        let provider = ProviderBuilder::new()
            .connect_ws(WsConnect::new(config.provider_url))
            .await
            .expect("Failed to connect to WebSocket provider");
        Arc::new(
            Chain::new(provider)
                .await
                .expect("Failed to connect to chain"),
        )
    } else {
        panic!("Transport not supported");
    };

    let wallet = {
        if let Some(wallet_path) = config.builder_config.wallet_path {
            Wallet::from_key_source(&KeySource::File(wallet_path))
        } else if let Some(wallet) = config.builder_config.wallet {
            Wallet::from_key_source(&KeySource::Argument(wallet))
        } else {
            Wallet::new()
        }
    }
    .expect("Unable to create wallet");

    let builder = if config.builder_config.disable_builder {
        info!("Builder is disabled");

        Builder::new(chain.clone(), wallet, Arc::new(NoopSubmitter {}))
    } else {
        info!("Bundler's wallet address: {:?}", wallet.signer().address());

        Builder::new(
            chain.clone(),
            wallet,
            Arc::new(TransactionSubmitter {
                chain: chain.clone(),
            }),
        )
    };

    let mempool = Arc::new(Mempool::new(silius_db.clone(), Validator::new()));

    let (network_sender, network_receiver) = mpsc::unbounded_channel();

    let manager = Arc::new(
        SiliusManager::new(builder, mempool, chain, network_sender)
            .await
            .expect("Failed to create manager"),
    );

    let http_future = if config.rpc_server_config.http {
        let http_server_config = HttpRpcServerConfig::new(
            config.rpc_server_config.http_address,
            config.rpc_server_config.http_port,
            config.rpc_server_config.http_allow_origins,
            config.rpc_server_config.http_api,
        );

        let manager_http = manager.clone();
        executor.spawn(async move { start_http_server(http_server_config, manager_http).await })
    } else {
        executor.spawn(async move {
            info!("HTTP server is disabled");
            Ok(())
        })
    };

    let ws_future = if config.rpc_server_config.ws {
        let ws_server_config = WsRpcServerConfig::new(
            config.rpc_server_config.ws_address,
            config.rpc_server_config.ws_port,
            config.rpc_server_config.ws_allow_origins,
            config.rpc_server_config.ws_api,
        );

        let manager_ws = manager.clone();
        executor.spawn(async move { start_ws_server(ws_server_config, manager_ws).await })
    } else {
        executor.spawn(async move {
            info!("WS server is disabled");
            Ok(())
        })
    };

    let manager_future = executor.spawn(async move {
        manager
            .start(config.builder_config.builder_interval, network_receiver)
            .await
    });

    tokio::select! {
        _ = http_future => {
            info!("HTTP server stopped");
        }
        _ = ws_future => {
            info!("WS server stopped");
        }
        _ = manager_future => {
            info!("Manager stopped");
        }
    }
}
