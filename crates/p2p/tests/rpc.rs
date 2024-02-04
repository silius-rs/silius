mod common;

use crate::common::build_connnected_p2p_pair;
use silius_p2p::{
    rpc::{
        methods::{RPCResponse, StatusMessage},
        outbound::OutboundRequest,
    },
    service::NetworkEvent,
};
use std::time::Duration;

async fn rpc_case(request_case: OutboundRequest, response_case: RPCResponse) -> eyre::Result<()> {
    let (mut peer1, mut peer2) = build_connnected_p2p_pair().await?;
    let peer1_id = peer1.local_peer_id().clone();
    let peer2_id = peer2.local_peer_id().clone();
    let _peer1_metadata = peer1.metadata();
    let peer2_metadata = peer2.metadata();

    let sender_fut = async {
        loop {
            match peer1.next_event().await {
                NetworkEvent::PeerConnected(_) => {
                    println!("Send request");
                    peer1.send_request(&peer2_id, request_case.clone());
                }
                NetworkEvent::RequestMessage { .. } => {
                    panic!("Unexpected request")
                }
                NetworkEvent::ResponseMessage { peer_id, response } => {
                    println!("Received response");
                    assert_eq!(peer2_id, peer_id);
                    match response {
                        RPCResponse::MetaData(metadata) => {
                            assert_eq!(metadata, peer2_metadata)
                        }
                        _ => assert_eq!(response, response_case.clone()),
                    }
                    return;
                }

                _ => {}
            }
        }
    };

    let receiver_fut = async {
        loop {
            match peer2.next_event().await {
                NetworkEvent::RequestMessage { peer_id, request, sender } => {
                    println!("Received request");
                    assert_eq!(request, request_case.clone());
                    assert_eq!(peer1_id, peer_id);
                    peer2.send_response(sender, response_case.clone()).unwrap();
                }
                NetworkEvent::ResponseMessage { .. } => {
                    panic!("Unexpected response")
                }
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = sender_fut => {}
        _ = receiver_fut => {}
        _ = tokio::time::sleep(Duration::from_secs(20)) => {
            panic!("Future timed out");
        }
    }
    Ok(())
}

#[tokio::test]
async fn rpc_status() -> eyre::Result<()> {
    rpc_case(
        OutboundRequest::Status(StatusMessage::default()),
        RPCResponse::Status(StatusMessage::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn rpc_goodbye() -> eyre::Result<()> {
    rpc_case(
        OutboundRequest::Goodbye(Default::default()),
        RPCResponse::Goodbye(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn rpc_ping_pong() -> eyre::Result<()> {
    rpc_case(OutboundRequest::Ping(Default::default()), RPCResponse::Pong(Default::default()))
        .await?;
    Ok(())
}

#[tokio::test]
async fn rpc_metadata() -> eyre::Result<()> {
    rpc_case(
        OutboundRequest::MetaData(Default::default()),
        RPCResponse::MetaData(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn rpc_pooled_userops() -> eyre::Result<()> {
    rpc_case(
        OutboundRequest::PooledUserOpHashes(Default::default()),
        RPCResponse::PooledUserOpHashes(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn rpc_pooled_userops_by_hash() -> eyre::Result<()> {
    rpc_case(
        OutboundRequest::PooledUserOpsByHash(Default::default()),
        RPCResponse::PooledUserOpsByHash(Default::default()),
    )
    .await?;
    Ok(())
}
