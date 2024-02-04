use libp2p::gossipsub::{IdentTopic, TopicHash, WhitelistSubscriptionFilter};
use silius_primitives::constants::p2p::{SSZ_SNAPPY_ENCODING, TOPIC_PREFIX, USER_OPERATIONS_TOPIC};
use std::collections::HashSet;

pub fn topic(mempool_id: &str) -> IdentTopic {
    IdentTopic::new(format!(
        "/{TOPIC_PREFIX:}/{mempool_id}/{USER_OPERATIONS_TOPIC:}/{SSZ_SNAPPY_ENCODING:}"
    ))
}

pub fn create_whitelist_filter(mempool_ids: Vec<String>) -> WhitelistSubscriptionFilter {
    let mut possible_hashes: HashSet<TopicHash> = HashSet::new();
    for mempool_id in mempool_ids {
        let topic = topic(&mempool_id);
        possible_hashes.insert(topic.into());
    }
    WhitelistSubscriptionFilter(possible_hashes)
}
