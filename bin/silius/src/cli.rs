use std::sync::Arc;

use clap::{Parser, Subcommand};
use silius_node_version::FULL_VERSION;
use silius_primitives::{network_spec::NetworkSpec, utils::network_parser};

use crate::config::{
    builder::BuilderConfig, mempool::MempoolConfig, metrics::MetricsConfig, network::NetworkConfig,
    rpc_server::RpcServerConfig,
};

const DEFAULT_NETWORK: &str = "mainnet";
const DEFAULT_PROVIDER_URL: &str = "http://localhost:8545";

#[derive(Debug, Parser)]
#[command(author, version = FULL_VERSION, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Starts the bundler
    #[command(name = "node")]
    Node(NodeConfig),
}

#[derive(Debug, Parser)]
pub struct NodeConfig {
    /// Verbosity level
    #[arg(short, long, default_value_t = 3)]
    pub verbosity: u8,

    #[arg(
        long,
        help = "Choose mainnet, dev, or provide a path to a YAML config file",
        default_value = DEFAULT_NETWORK,
        value_parser = network_parser
    )]
    pub network: Arc<NetworkSpec>,

    #[arg(long, help = "Set the URL to the chain provider", default_value = DEFAULT_PROVIDER_URL)]
    pub provider_url: String,

    #[clap(flatten)]
    pub builder_config: BuilderConfig,

    #[clap(flatten)]
    pub mempool_config: MempoolConfig,

    #[clap(flatten)]
    pub metrics_config: MetricsConfig,

    #[clap(flatten)]
    pub network_config: NetworkConfig,

    #[clap(flatten)]
    pub rpc_server_config: RpcServerConfig,
}
