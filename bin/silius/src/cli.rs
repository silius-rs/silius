use std::sync::Arc;

use clap::{Parser, Subcommand};
use silius_node_version::FULL_VERSION;
use silius_types::{network_spec::NetworkSpec, utils::network_parser};

const DEFAULT_HTTP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_NETWORK: &str = "mainnet";

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
        help = "Choose mainnet, or provide a path to a YAML config file",
        default_value = DEFAULT_NETWORK,
        value_parser = network_parser
    )]
    pub network: Arc<NetworkSpec>,

    #[arg(long, help = "Enable HTTP RPC", default_value_t = true)]
    pub http: bool,

    #[arg(long, help = "Set HTTP address", default_value_t = DEFAULT_HTTP_ADDRESS)]
    pub http_address: IpAddr,

    #[arg(long, help = "Set HTTP Port", default_value_t = DEFAULT_HTTP_PORT)]
    pub http_port: u16,

    #[arg(long, default_value_t = DEFAULT_HTTP_ALLOW_ORIGIN)]
    pub http_allow_origin: bool,
}
