use std::net::{IpAddr, Ipv4Addr};

use clap::{Parser, arg};

const DEFAULT_HTTP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_HTTP_PORT: u16 = 4337;
const DEFAULT_HTTP_API_MODULE: &str = "eth";

#[derive(Debug, Parser)]
pub struct RpcServerConfig {
    #[arg(long, help = "Enable HTTP RPC", default_value_t = true)]
    pub http: bool,

    #[arg(long, help = "Set HTTP address", default_value_t = DEFAULT_HTTP_ADDRESS)]
    pub http_address: IpAddr,

    #[arg(long, help = "Set HTTP Port", default_value_t = DEFAULT_HTTP_PORT)]
    pub http_port: u16,

    #[arg(long, help = "Set HTTP API modules", value_delimiter=',', default_value = DEFAULT_HTTP_API_MODULE, value_parser = ["eth", "debug", "web3"])]
    pub http_api: Vec<String>,
}
