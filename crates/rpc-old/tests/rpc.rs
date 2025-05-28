mod common;

use crate::common::{test_port, ADDRESS};
use common::{
    build_http_client, build_ws_client, DummyEthApiClient, DummyEthApiServer, DummyEthApiServerImpl,
};
use ethers::types::U64;
use silius_rpc::{JsonRpcServer, JsonRpcServerType};
use std::net::IpAddr;
use tokio;

#[tokio::test]
async fn only_http_rpc_server() {
    let addr = IpAddr::from(ADDRESS);
    let port = test_port();
    let http: bool = true;
    let ws = false;

    let mut server = JsonRpcServer::new(http, addr.clone(), port, ws, addr.clone(), port);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_methods(DummyEthApiServerImpl { chain_id }.into_rpc(), JsonRpcServerType::Http)
        .unwrap();

    let (http_handle, _ws_handle) = server.start().await.unwrap();
    tokio::spawn(http_handle.unwrap().stopped());

    // http client return success response
    let http_client = build_http_client(addr.clone(), port).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await.unwrap();
    assert_eq!(http_response, chain_id);

    // ws client cannot connect to http server
    assert!(build_ws_client(addr, port).await.is_err());
}

#[tokio::test]
async fn only_ws_rpc_server() {
    let addr = IpAddr::from(ADDRESS);
    let port = test_port();
    let http = false;
    let ws = true;
    let mut server = JsonRpcServer::new(http, addr.clone(), port, ws, addr.clone(), port);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_methods(DummyEthApiServerImpl { chain_id }.into_rpc(), JsonRpcServerType::Ws)
        .unwrap();

    let (_http_handle, ws_handle) = server.start().await.unwrap();
    tokio::spawn(ws_handle.unwrap().stopped());

    // http client return error response
    let http_client = build_http_client(addr.clone(), port).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await;
    assert!(http_response.is_err());

    // ws client return success response
    let ws_client = build_ws_client(addr, port).await.unwrap();
    let ws_response = DummyEthApiClient::chain_id(&ws_client).await.unwrap();
    assert_eq!(ws_response, chain_id);
}

#[tokio::test]
async fn http_and_ws_rpc_server() {
    let addr = IpAddr::from(ADDRESS);
    let http_port = test_port();
    let ws_port = test_port();
    let http = true;
    let ws = true;
    let mut server = JsonRpcServer::new(http, addr.clone(), http_port, ws, addr.clone(), ws_port);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_methods(DummyEthApiServerImpl { chain_id }.into_rpc(), JsonRpcServerType::Both)
        .unwrap();

    let (http_handle, ws_handle) = server.start().await.unwrap();

    tokio::spawn(http_handle.unwrap().stopped());
    tokio::spawn(ws_handle.unwrap().stopped());

    // http client return success response
    let http_client = build_http_client(addr.clone(), http_port).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await.unwrap();
    assert_eq!(http_response, chain_id);

    // ws client return success response
    let ws_client = build_ws_client(addr, ws_port).await.unwrap();
    let ws_response = DummyEthApiClient::chain_id(&ws_client).await.unwrap();
    assert_eq!(ws_response, chain_id);
}
