use libp2p::gossipsub::{
    Behaviour, ConfigBuilder, DataTransform, IdentTopic, Message, MessageAuthenticity, RawMessage,
    TopicHash, ValidationMode, WhitelistSubscriptionFilter,
};
use silius_primitives::consts::p2p::{
    MAX_GOSSIP_SNAP_SIZE, SSZ_SNAPPY_ENCODING, TOPIC_PREFIX, USER_OPS_WITH_ENTRY_POINT_TOPIC,
};
use snap::raw::{decompress_len, Decoder, Encoder};
use std::{
    collections::HashSet,
    io::{Error, ErrorKind},
};

pub type Gossipsub = Behaviour<SnappyTransform, WhitelistSubscriptionFilter>;

// Highly inspired by https://github.com/sigp/lighthouse/blob/stable/beacon_node/lighthouse_network/src/types/pubsub.rs#L45-L103
// Implements the `DataTransform` trait of gossipsub to employ snappy compression
pub struct SnappyTransform {
    /// Sets the maximum size we allow gossipsub messages to decompress to.
    max_size_per_message: usize,
}

impl SnappyTransform {
    pub fn new(max_size_per_message: usize) -> Self {
        SnappyTransform {
            max_size_per_message,
        }
    }
}
impl Default for SnappyTransform {
    fn default() -> Self {
        SnappyTransform {
            max_size_per_message: MAX_GOSSIP_SNAP_SIZE,
        }
    }
}

impl DataTransform for SnappyTransform {
    // Provides the snappy decompression from RawGossipsubMessages
    fn inbound_transform(&self, raw_message: RawMessage) -> Result<Message, std::io::Error> {
        // check the length of the raw bytes
        let len = decompress_len(&raw_message.data)?;
        if len > self.max_size_per_message {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "ssz_snappy decoded data > GOSSIP_MAX_SIZE",
            ));
        }

        let mut decoder = Decoder::new();
        let decompressed_data = decoder.decompress_vec(&raw_message.data)?;

        // Build the GossipsubMessage struct
        Ok(Message {
            source: raw_message.source,
            data: decompressed_data,
            sequence_number: raw_message.sequence_number,
            topic: raw_message.topic,
        })
    }

    /// Provides the snappy compression logic to gossipsub.
    fn outbound_transform(
        &self,
        _topic: &TopicHash,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, std::io::Error> {
        // Currently we are not employing topic-based compression. Everything is expected to be
        // snappy compressed.
        if data.len() > self.max_size_per_message {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "ssz_snappy Encoded data > GOSSIP_MAX_SIZE",
            ));
        }
        let mut encoder = Encoder::new();
        encoder.compress_vec(&data).map_err(Into::into)
    }
}

pub fn create_whitelist_filter(mempool_ids: Vec<String>) -> WhitelistSubscriptionFilter {
    let mut possible_hashes: HashSet<TopicHash> = HashSet::new();
    for mempool_id in mempool_ids {
        let topic = topic(&mempool_id);
        possible_hashes.insert(topic.into());
    }
    WhitelistSubscriptionFilter(possible_hashes)
}

pub fn topic(mempool_id: &str) -> IdentTopic {
    IdentTopic::new(format!(
        "/{TOPIC_PREFIX:}/{mempool_id}/{USER_OPS_WITH_ENTRY_POINT_TOPIC:}/{SSZ_SNAPPY_ENCODING:}"
    ))
}

/// Creates a gossipsub instance with the given mempool ids
pub fn create_gossisub(mempool_ids: Vec<String>) -> Result<Gossipsub, &'static str> {
    let filter = create_whitelist_filter(mempool_ids.clone());
    let gs_config = ConfigBuilder::default()
        .validation_mode(ValidationMode::Anonymous)
        .build()?;
    let snappy_transform = SnappyTransform::new(MAX_GOSSIP_SNAP_SIZE);
    let mut gossipsub = Gossipsub::new_with_subscription_filter_and_transform(
        MessageAuthenticity::Anonymous,
        gs_config,
        None,
        filter,
        snappy_transform,
    )?;
    for mempool_id in mempool_ids {
        let _ = gossipsub
            .subscribe(&topic(&mempool_id))
            .map_err(|_| "subscribe error")?;
    }
    Ok(gossipsub)
}
