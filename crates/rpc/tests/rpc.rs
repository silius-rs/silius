mod common;

use common::{
    build_http_client, build_ws_client, test_address, DummyEthApiClient, DummyEthApiServer,
    DummyEthApiServerImpl,
};
use ethers::types::U64;
use silius_rpc::JsonRpcServer;
use tokio;

#[tokio::test]
async fn test_only_http_rpc_server() {
    let address = test_address();
    let http_enabled = true;
    let ws_disabled = false;
    let mut server = JsonRpcServer::new(address.clone(), http_enabled, ws_disabled);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_method(DummyEthApiServerImpl { chain_id }.into_rpc())
        .unwrap();

    let handle = server.start().await.unwrap();
    tokio::spawn(handle.stopped());

    // http client return success response
    let http_client = build_http_client(address.clone()).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await.unwrap();
    assert_eq!(http_response, chain_id);

    // ws client cannot connect to http server
    assert!(build_ws_client(address.clone()).await.is_err());
}

#[tokio::test]
async fn test_only_ws_rpc_server() {
    let address = test_address();
    let http_disabled = false;
    let ws_enabled = true;
    let mut server = JsonRpcServer::new(address.clone(), http_disabled, ws_enabled);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_method(DummyEthApiServerImpl { chain_id }.into_rpc())
        .unwrap();

    let handle = server.start().await.unwrap();
    tokio::spawn(handle.stopped());

    // http client return error response
    let http_client = build_http_client(address.clone()).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await;
    assert!(http_response.is_err());

    // ws client return success response
    let ws_client = build_ws_client(address.clone()).await.unwrap();
    let ws_response = DummyEthApiClient::chain_id(&ws_client).await.unwrap();
    assert_eq!(ws_response, chain_id);
}

#[tokio::test]
async fn test_http_and_ws_rpc_server() {
    let address = test_address();
    let http_enabled = true;
    let ws_enabled = true;
    let mut server = JsonRpcServer::new(address.clone(), http_enabled, ws_enabled);

    let chain_id: U64 = U64::from(0x7a69);
    server
        .add_method(DummyEthApiServerImpl { chain_id }.into_rpc())
        .unwrap();

    let handle = server.start().await.unwrap();
    tokio::spawn(handle.stopped());

    // http client return success response
    let http_client = build_http_client(address.clone()).unwrap();
    let http_response = DummyEthApiClient::chain_id(&http_client).await.unwrap();
    assert_eq!(http_response, chain_id);

    // ws client return success response
    let ws_client = build_ws_client(address.clone()).await.unwrap();
    let ws_response = DummyEthApiClient::chain_id(&ws_client).await.unwrap();
    assert_eq!(ws_response, chain_id);
}
