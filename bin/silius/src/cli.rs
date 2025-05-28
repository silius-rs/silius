use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::Arc,
};

use clap::{Parser, Subcommand};
use silius_node_version::FULL_VERSION;
use silius_types::{network_spec::NetworkSpec, utils::network_parser};

const DEFAULT_DISABLE_DISCOVERY: bool = false;
const DEFAULT_DISCOVERY_PORT: u16 = 9000;
const DEFAULT_NETWORK: &str = "mainnet";
const DEFAULT_SOCKET_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_SOCKET_PORT: u16 = 4000;

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

    

    #[arg(long, help = "Set P2P socket address", default_value_t = DEFAULT_SOCKET_ADDRESS)]
    pub socket_address: IpAddr,

    #[arg(long, help = "Set P2P socket port (TCP)", default_value_t = DEFAULT_SOCKET_PORT)]
    pub socket_port: u16,

    #[arg(long, help = "Discovery 5 listening port (UDP)", default_value_t = DEFAULT_DISCOVERY_PORT)]
    pub discovery_port: u16,

    #[arg(long, help = "Disable Discv5", default_value_t = DEFAULT_DISABLE_DISCOVERY)]
    pub disable_discovery: bool,

    #[arg(
        long,
        help = "The directory for storing application data. If used together with --ephemeral, new child directory will be created."
    )]
    pub data_dir: Option<PathBuf>,

    #[arg(
        long,
        short,
        help = "Use new data directory, located in OS temporary directory. If used together with --data-dir, new directory will be created there instead."
    )]
    pub ephemeral: bool,

    #[arg(long, help = "Purges the database.")]
    pub purge_db: bool,


}
