use crate::{
    network::NetworkEvent,
    request_response::{GetMetadata, Request, Response, Status},
    tests::build_connnected_p2p_pair,
};
use std::time::Duration;

async fn reqrep_case(request_case: Request, response_case: Response) -> eyre::Result<()> {
    let (mut peer1, mut peer2) = build_connnected_p2p_pair().await?;
    let peer1_id = peer1.local_peer_id().clone();
    let peer2_id = peer2.local_peer_id().clone();

    let sender_fut = async {
        loop {
            match peer1.next_event().await {
                NetworkEvent::PeerConnected(_) => {
                    println!("Send request ");
                    peer1.send_request(&peer2_id, request_case.clone());
                }
                NetworkEvent::RequestMessage { .. } => {
                    panic!("Unexpected request")
                }
                NetworkEvent::ResponseMessage { peer_id, response } => {
                    println!("receive response");
                    assert_eq!(peer2_id, peer_id);
                    assert_eq!(response, response_case.clone());
                    return;
                }

                _ => {}
            }
        }
    };

    let receiver_fut = async {
        loop {
            match peer2.next_event().await {
                NetworkEvent::RequestMessage {
                    request,
                    response_sender,
                    peer_id,
                } => {
                    println!("received request");
                    assert_eq!(request, request_case.clone());
                    assert_eq!(peer1_id, peer_id);
                    peer2
                        .send_response(response_sender, response_case.clone())
                        .unwrap();
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
async fn reqrep_status() -> eyre::Result<()> {
    reqrep_case(
        Request::Status(Status::default()),
        Response::Status(Status::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn reqrep_goodbye() -> eyre::Result<()> {
    reqrep_case(
        Request::GoodbyeReason(Default::default()),
        Response::GoodbyeReason(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn reqrep_ping_pong() -> eyre::Result<()> {
    reqrep_case(
        Request::Ping(Default::default()),
        Response::Pong(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn reqrep_metadata() -> eyre::Result<()> {
    reqrep_case(
        Request::GetMetadata(GetMetadata),
        Response::Metadata(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn reqrep_pooled_userops() -> eyre::Result<()> {
    reqrep_case(
        Request::PooledUserOpHashesReq(Default::default()),
        Response::PooledUserOpHashes(Default::default()),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn reqrep_pooled_userops_by_hash() -> eyre::Result<()> {
    reqrep_case(
        Request::PooledUserOpsByHashReq(Default::default()),
        Response::PooledUserOpsByHash(Default::default()),
    )
    .await?;
    Ok(())
}
