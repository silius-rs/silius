mod common;

use crate::common::build_connnected_p2p_pair;
use ethers::types::H160;
use silius_p2p::{
    service::NetworkEvent,
    types::{pubsub::PubsubMessage, topics::topic},
};
use silius_primitives::{chain::ChainSpec, constants::entry_point::ADDRESS, VerifiedUserOperation};
use std::{str::FromStr, time::Duration};

#[tokio::test]
async fn pubsub_msg() -> eyre::Result<()> {
    let chain_spec = ChainSpec::dev();
    let (mut peer1, mut peer2) = build_connnected_p2p_pair().await?;

    let mempool_id = chain_spec.canonical_mempools.first().unwrap();
    let res1 = peer1.subscribe(&mempool_id)?;
    let res2 = peer2.subscribe(&mempool_id)?;
    println!("{mempool_id}, {res1:?}, {res2:?}");

    let peer1_id = peer1.local_peer_id().clone();
    let user_op = VerifiedUserOperation::new(
        Default::default(),
        H160::from_str(ADDRESS)?,
        Default::default(),
    );

    let sender_fut = async {
        loop {
            match peer1.next_event().await {
                NetworkEvent::Subscribe { .. } => {
                    let topic_hash = topic(&mempool_id).into();
                    peer1.publish(user_op.clone(), topic_hash).unwrap();
                }
                _ => {}
            }
        }
    };

    let receiver_fut = async {
        loop {
            match peer2.next_event().await {
                NetworkEvent::PubsubMessage { source_peer, message, .. } => {
                    assert_eq!(source_peer, peer1_id);
                    assert_eq!(message, PubsubMessage::UserOperation(user_op.clone()));
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
