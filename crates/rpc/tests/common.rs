use async_trait::async_trait;
use ethers::types::U64;
use jsonrpsee::{
    core::{ClientError as RpcError, RpcResult},
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
    ws_client::{WsClient, WsClientBuilder},
};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicU16, Ordering},
};

pub static ADDRESS: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
static PORT: AtomicU16 = AtomicU16::new(8000);

/// test_port returns a port with a increasing port number.
/// This is to prevent multiple tests from using the same port.
///
/// # Returns
/// * `u16` - A port with a increasing port number.
pub fn test_port() -> u16 {
    let port = PORT.fetch_add(1, Ordering::SeqCst);
    port
}

#[rpc(client, server, namespace = "eth")]
pub trait DummyEthApi {
    #[method(name = "chainId")]
    async fn chain_id(&self) -> RpcResult<U64>;
}

pub struct DummyEthApiServerImpl {
    pub chain_id: U64,
}

#[async_trait]
impl DummyEthApiServer for DummyEthApiServerImpl {
    async fn chain_id(&self) -> RpcResult<U64> {
        let chain_id = self.chain_id;
        return Ok(chain_id);
    }
}

pub fn build_http_client(addr: IpAddr, port: u16) -> Result<HttpClient, RpcError> {
    HttpClientBuilder::default().build(format!("http://{addr}:{port}"))
}

pub async fn build_ws_client(addr: IpAddr, port: u16) -> Result<WsClient, RpcError> {
    WsClientBuilder::default().build(format!("ws://{addr}:{port}")).await
}
