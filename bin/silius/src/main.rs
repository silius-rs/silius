use std::env;

use clap::Parser;
use silius::{
    cli::{Cli, Commands},
    utils::print_ascii_logo,
};
use silius_primitives::network_spec::set_network_spec;
use silius_rpc::{config::HttpRpcServerConfig, start_http_server};
use silius_storage::{
    db::{SiliusDB, reset_db},
    dir::setup_data_dir,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

pub const APP_NAME: &str = "silius";

#[tokio::main]
async fn main() {
    print_ascii_logo();

    let rust_log = env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default();
    let env_filter = match rust_log.is_empty() {
        true => EnvFilter::builder().parse_lossy("info"),
        false => EnvFilter::builder().parse_lossy(rust_log),
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();

    // let async_executor = SiliusExecutor::new().expect("Failed to create executor");

    // let main_executor = SiliusExecutor::new().expect("Failed to create executor");

    match cli.command {
        Commands::Node(config) => {
            info!("Silius starting up...");

            set_network_spec(config.network.clone());

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

            let http_server_config = HttpRpcServerConfig::new(
                config.rpc_server_config.http_address,
                config.rpc_server_config.http_port,
                config.rpc_server_config.http_allow_origins,
            );

            // let ws_server_config = WsRpcServerConfig::new(
            //     config.rpc_server_config.ws_address,
            //     config.rpc_server_config.ws_port,
            //     config.rpc_server_config.ws_allow_origins,
            // );

            let http_future = start_http_server(http_server_config, silius_db.clone());

            tokio::select! {
                _ = http_future => {
                    info!("HTTP server stopped");
                }
            }
        }
    }
}
