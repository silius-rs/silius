use async_trait::async_trait;
use ethers::types::U64;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use jsonrpsee::{
    core::{Error as RpcError, RpcResult},
    proc_macros::rpc,
};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU16, Ordering};

static PORT: AtomicU16 = AtomicU16::new(8000);
/// test_address returns a address on localhost with a increasing port number.
/// This is to prevent multiple tests from using the same port.
///
/// # Returns
/// * `String` - A localhost address with a increasing port number.
pub fn test_address() -> String {
    let port = PORT.fetch_add(1, Ordering::SeqCst);
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).to_string()
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

pub fn build_http_client(address: String) -> Result<HttpClient, RpcError> {
    HttpClientBuilder::default().build(format!("http://{}", address))
}

pub async fn build_ws_client(address: String) -> Result<WsClient, RpcError> {
    WsClientBuilder::default()
        .build(format!("ws://{}", address))
        .await
}
