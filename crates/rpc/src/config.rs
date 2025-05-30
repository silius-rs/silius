use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Clone)]
pub struct HttpRpcServerConfig {
    pub http_socket_address: SocketAddr,
    pub http_allow_origins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WsRpcServerConfig {
    pub ws_socket_address: SocketAddr,
    pub ws_allow_origins: Vec<String>,
}

impl HttpRpcServerConfig {
    pub fn new(http_address: IpAddr, http_port: u16, http_allow_origins: Vec<String>) -> Self {
        Self {
            http_socket_address: SocketAddr::new(http_address, http_port),
            http_allow_origins,
        }
    }
}

impl WsRpcServerConfig {
    pub fn new(ws_address: IpAddr, ws_port: u16, ws_allow_origins: Vec<String>) -> Self {
        Self {
            ws_socket_address: SocketAddr::new(ws_address, ws_port),
            ws_allow_origins,
        }
    }
}
