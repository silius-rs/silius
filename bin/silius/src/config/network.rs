use std::net::{IpAddr, Ipv4Addr};

use clap::Parser;

const DEFAULT_DISABLE_DISCOVERY: bool = false;
const DEFAULT_DISCOVERY_PORT: u16 = 9000;
const DEFAULT_SOCKET_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_SOCKET_PORT: u16 = 4000;

#[derive(Debug, Parser)]
pub struct NetworkConfig {
    #[arg(long, help = "Set P2P socket address", default_value_t = DEFAULT_SOCKET_ADDRESS)]
    pub socket_address: IpAddr,

    #[arg(long, help = "Set P2P socket port (TCP)", default_value_t = DEFAULT_SOCKET_PORT)]
    pub socket_port: u16,

    #[arg(long, help = "Discovery 5 listening port (UDP)", default_value_t = DEFAULT_DISCOVERY_PORT)]
    pub discovery_port: u16,

    #[arg(long, help = "Disable Discv5", default_value_t = DEFAULT_DISABLE_DISCOVERY)]
    pub disable_discovery: bool,
}
