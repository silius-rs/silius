use super::topics::{create_whitelist_filter, topic};
use crate::{config::gossipsub_config, service::behaviour::Gossipsub};
use libp2p::gossipsub::{DataTransform, Message, MessageAuthenticity, RawMessage, TopicHash};
use silius_primitives::{constants::p2p::GOSSIP_MAX_SIZE, VerifiedUserOperation};
use snap::raw::{decompress_len, Decoder, Encoder};
use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
pub enum PubsubMessage {
    UserOperation(VerifiedUserOperation),
}

/// Implements the `DataTransform` trait of gossipsub to employ snappy compression
pub struct SnappyTransform {
    /// Sets the maximum size we allow gossipsub messages to decompress to.
    max_size_per_message: usize,
}

impl SnappyTransform {
    pub fn new(max_size_per_message: usize) -> Self {
        SnappyTransform { max_size_per_message }
    }
}

impl Default for SnappyTransform {
    fn default() -> Self {
        SnappyTransform { max_size_per_message: GOSSIP_MAX_SIZE }
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

/// Creates a gossipsub instance with the given mempool ids
pub fn create_gossipsub(mempool_ids: Vec<String>) -> Result<Gossipsub, String> {
    let filter = create_whitelist_filter(mempool_ids.clone());
    let config = gossipsub_config();
    let snappy_transform = SnappyTransform::new(GOSSIP_MAX_SIZE);
    let mut gossipsub = Gossipsub::new_with_subscription_filter_and_transform(
        MessageAuthenticity::Anonymous,
        config,
        None,
        filter,
        snappy_transform,
    )?;

    for mempool_id in mempool_ids {
        let _ = gossipsub.subscribe(&topic(&mempool_id)).map_err(|_| "subscribe error")?;
    }

    Ok(gossipsub)
}
