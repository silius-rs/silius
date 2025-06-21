use std::net::{IpAddr, Ipv4Addr};

use clap::{Parser, arg};

const DEFAULT_HTTP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_HTTP_ALLOW_ORIGINS: &str = "*";
const DEFAULT_HTTP_API_MODULE: &str = "eth,web3,debug";
const DEFAULT_HTTP_ENABLED: bool = true;
const DEFAULT_HTTP_PORT: u16 = 3000;
const DEFAULT_WS_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_WS_ALLOW_ORIGINS: &str = "*";
const DEFAULT_WS_API_MODULE: &str = "eth,web3,debug";
const DEFAULT_WS_ENABLED: bool = true;
const DEFAULT_WS_PORT: u16 = 3001;

#[derive(Debug, Parser)]
pub struct RpcServerConfig {
    #[arg(long, help = "Enable HTTP RPC", default_value_t = DEFAULT_HTTP_ENABLED)]
    pub http: bool,

    #[arg(long, help = "Set HTTP address", default_value_t = DEFAULT_HTTP_ADDRESS)]
    pub http_address: IpAddr,

    #[arg(long, help = "Set HTTP port", default_value_t = DEFAULT_HTTP_PORT)]
    pub http_port: u16,

    #[arg(long, help = "Set HTTP API modules", value_delimiter=',', default_value = DEFAULT_HTTP_API_MODULE, value_parser = ["eth", "debug", "web3"])]
    pub http_api: Vec<String>,

    #[arg(long, help = "Set HTTP allow origins", default_value = DEFAULT_HTTP_ALLOW_ORIGINS)]
    pub http_allow_origins: Vec<String>,

    #[arg(long, help = "Enable WebSocket RPC", default_value_t = DEFAULT_WS_ENABLED)]
    pub ws: bool,

    #[arg(long, help = "Set WebSocket address", default_value_t = DEFAULT_WS_ADDRESS)]
    pub ws_address: IpAddr,

    #[arg(long, help = "Set WebSocket port", default_value_t = DEFAULT_WS_PORT)]
    pub ws_port: u16,

    #[arg(long, help = "Set WebSocket API modules", value_delimiter=',', default_value = DEFAULT_WS_API_MODULE, value_parser = ["eth", "debug", "web3"])]
    pub ws_api: Vec<String>,

    #[arg(long, help = "Set WebSocket allow origins", default_value = DEFAULT_WS_ALLOW_ORIGINS)]
    pub ws_allow_origins: Vec<String>,
}
