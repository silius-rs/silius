use crate::{
    network::{NetworkEvent, PubsubMessage},
    tests::build_connnected_p2p_pair,
};
use alloy_chains::Chain;
use silius_primitives::{chain::ChainExt, UserOperationsWithEntryPoint};
use std::time::Duration;

#[tokio::test]
async fn pubsub_msg() -> eyre::Result<()> {
    let chain: Chain = 5.into();
    let (mut peer1, mut peer2) = build_connnected_p2p_pair().await?;
    let mempool_id = chain.canonical_mempool_id();
    let _ = peer1.subscribe(&mempool_id)?;
    let res = peer2.subscribe(&mempool_id)?;
    println!("{mempool_id}, {res:?}");

    let peer1_id = peer1.local_peer_id().clone();
    let uo_entrypoint =
        UserOperationsWithEntryPoint::new(Default::default(), Default::default(), 5.into(), vec![]);
    let sender_fut = async {
        loop {
            match peer1.next_event().await {
                NetworkEvent::Subscribe { .. } => {
                    peer1.publish(uo_entrypoint.clone()).unwrap();
                }
                _ => {}
            }
        }
    };

    let receiver_fut = async {
        loop {
            match peer2.next_event().await {
                NetworkEvent::PubsubMessage {
                    source_peer,
                    message,
                    ..
                } => {
                    assert_eq!(source_peer, peer1_id);
                    assert_eq!(message, PubsubMessage::UserOps(uo_entrypoint.clone()));
                    return;
                }
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = sender_fut => {}
        _ = receiver_fut => {}
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            panic!("Future timed out");
        }
    }
    Ok(())
}
