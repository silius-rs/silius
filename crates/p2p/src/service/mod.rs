pub mod api_types;
pub mod behaviour;
pub mod utils;

use self::{
    behaviour::Behaviour,
    utils::{load_enr_from_file, load_private_key_from_file, save_enr_to_file},
};
use crate::{
    config::Config,
    discovery::{
        enr::{build_enr, keypair_to_combined},
        enr_ext::{CombinedPublicKeyExt, EnrExt},
        DiscoveredPeers, Discovery,
    },
    peer_manager::{PeerManager, PeerManagerEvent},
    rpc::{
        methods::{MetaData, MetaDataRequest, Ping, RPCResponse, RequestId, Status},
        outbound::OutboundRequest,
        protocol::InboundRequest,
        RPCEvent, RPC,
    },
    service::{
        behaviour::BehaviourEvent,
        utils::{fetch_mempool_config, save_private_key_to_file},
    },
    types::{
        globals::NetworkGlobals,
        pubsub::{create_gossipsub, PubsubMessage},
        topics::topic,
    },
};
use alloy_chains::Chain;
use discv5::Enr;
use ethers::types::{Address, H256};
use futures::channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::Sender,
};
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub::{self, MessageId, PublishError, SubscriptionError, TopicHash},
    identity::{secp256k1, Keypair},
    noise,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use libp2p_mplex::{MaxBufferBehaviour, MplexConfig};
use silius_primitives::{
    constants::p2p::{FIND_NODE_QUERY_CLOSEST_PEERS, MAX_IPFS_CID_LENGTH, MAX_SUPPORTED_MEMPOOLS},
    p2p::NetworkMessage,
    simulation::ValidationConfig,
    MempoolConfig, UserOperation, VerifiedUserOperation,
};
use ssz_rs::{Deserialize, List, Serialize, Vector};
use std::{
    env,
    sync::Arc,
    task::{Context, Poll},
};
use tracing::{debug, error, info, warn};

/// Channel for sending and receiving messages between p2p network and mempool (entry point
/// address).
pub type MempoolChannel =
    (Address, UnboundedSender<NetworkMessage>, UnboundedReceiver<NetworkMessage>);

#[derive(Debug)]
pub enum NetworkEvent {
    /// We successfully connected to a peer.
    PeerConnectedOutgoing(PeerId),
    /// A peer successfully connected to us.
    PeerConnectedIncoming(PeerId),
    /// A peer was disconnected.
    PeerDisconnected(PeerId),
    /// A peer successfully connected with us.
    PeerConnected(PeerId),
    /// Gossipsub message from the network
    PubsubMessage {
        /// The peer that forwarded us this message.
        source_peer: PeerId,
        /// The gossipsub message id. Used when propagating blocks after validation.
        id: MessageId,
        /// The message itself.
        message: PubsubMessage,
    },
    /// Request message to the network
    RequestMessage {
        /// The peer that sent the request.
        peer_id: PeerId,
        /// Request the peer sent.
        request: InboundRequest,
        /// response sender
        sender: Sender<RPCResponse>,
    },
    /// Response message from the network
    ResponseMessage {
        /// The peer that sent the request.
        peer_id: PeerId,
        /// Request the peer sent.
        response: RPCResponse,
    },
    /// Describe a peer is subscribe on something
    Subscribe {
        /// The peer that subscribe
        peer_id: PeerId,
        /// The topic which peer is subscribing
        topic: TopicHash,
    },
    /// Network listens to address successfully
    NewListenAddr(Multiaddr),
}

/// P2P network struct that holds the libp2p Swarm
/// Other components should interact with Network directly instead of behaviour
pub struct Network {
    swarm: Swarm<Behaviour>,
    network_globals: Arc<NetworkGlobals>,
    // Each entry point address has its own mempool channel.
    mempool_channels: Vec<MempoolChannel>,
    mempool_configs: Vec<(TopicHash, MempoolConfig)>,
}

impl From<Network> for Swarm<Behaviour> {
    fn from(value: Network) -> Self {
        value.swarm
    }
}

impl Network {
    pub async fn new(
        config: Config,
        latest_block: (H256, u64),
        mempool_channels: Vec<MempoolChannel>,
    ) -> eyre::Result<Self> {
        // Handle private key
        let key = if let Some(key) = load_private_key_from_file(&config.node_key_file) {
            key
        } else if let Ok(seed) = env::var("P2P_SEED") {
            // For test purposes
            let bytes = seed.as_bytes().to_vec();
            let key: secp256k1::Keypair = secp256k1::SecretKey::try_from_bytes(bytes)
                .expect("Env P2P_SEED is not valid bytes")
                .into();
            key.into()
        } else {
            info!("The p2p private key doesn't exist. Creating one now!");

            let key = Keypair::generate_secp256k1();
            save_private_key_to_file(&key, &config.node_key_file);

            key
        };
        let combined_key = keypair_to_combined(&key).expect("keypair to combined key failed");

        // Handle ENR
        let enr = load_enr_from_file(&config.node_enr_file)
            .filter(|enr| enr.ip4() == config.ipv4_addr)
            .unwrap_or_else(|| {
                let enr = build_enr(&combined_key, &config).expect("enr building failed");
                save_enr_to_file(&enr, &config.node_enr_file);
                enr
            });

        info!("Enr: {}", enr);

        let canonical_mempools = config.chain_spec.canonical_mempools.clone();
        let mempool_configs = {
            let mut m: Vec<(TopicHash, MempoolConfig)> = Vec::new();

            for canonical_mempool in canonical_mempools.iter() {
                let mempool_config = if config.chain_spec.chain == Chain::dev() {
                    MempoolConfig::dev()
                } else {
                    fetch_mempool_config(canonical_mempool.clone()).await?
                };
                m.push((
                    topic(canonical_mempool).into(),
                    mempool_config.with_id(canonical_mempool.clone()),
                ));
            }

            m
        };
        let trusted_peers = config.bootnodes.iter().map(|x| x.public_key().as_peer_id()).collect();

        let network_globals = {
            let mut supported_mempools: List<
                Vector<u8, MAX_IPFS_CID_LENGTH>,
                MAX_SUPPORTED_MEMPOOLS,
            > = List::default();

            for canonical_mempool in canonical_mempools.iter() {
                let mut mempool_id = canonical_mempool.as_bytes().to_vec();
                mempool_id.resize_with(MAX_IPFS_CID_LENGTH, Default::default);

                supported_mempools.push(
                    Vector::try_from(mempool_id)
                        .expect("canonical mempool id should be equal to 256 bytes"),
                );
            }

            // metadata
            let metadata = MetaData { seq_number: 0, supported_mempools };

            Arc::new(NetworkGlobals::new(
                enr.clone(),
                metadata,
                trusted_peers,
                config.chain_spec.clone(),
                latest_block.0,
                latest_block.1,
            ))
        };

        let mut gossipsub = create_gossipsub(canonical_mempools).map_err(|e| eyre::anyhow!(e))?;
        for bootnode in &config.bootnodes {
            gossipsub.add_explicit_peer(&bootnode.peer_id());
        }

        let rpc = RPC::new();

        let peer_manager = PeerManager::new(
            network_globals.clone(),
            config.clone().peers_whitelist,
            config.clone().ips_whitelist,
        );

        let mut discovery =
            Discovery::new(combined_key, config.clone(), network_globals.clone()).await?;
        discovery.discover_peers(FIND_NODE_QUERY_CLOSEST_PEERS);

        let behaviour = Behaviour { peer_manager, rpc, discovery, gossipsub };

        // mplex config
        let mut mplex_config = MplexConfig::new();
        mplex_config.set_max_buffer_size(256);
        mplex_config.set_max_buffer_behaviour(MaxBufferBehaviour::Block);

        // yamux config
        let yamux_config = libp2p::yamux::Config::default();
        let swarm = SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(libp2p::tcp::Config::default().nodelay(true), noise::Config::new, || {
                upgrade::SelectUpgrade::new(yamux_config, mplex_config)
            })
            .expect("building p2p transport failed")
            .with_behaviour(|_| behaviour)
            .expect("building p2p behaviour failed")
            .build();

        let mut network = Network { swarm, network_globals, mempool_channels, mempool_configs };

        network.start(&config).await?;

        Ok(network)
    }

    async fn start(&mut self, config: &Config) -> eyre::Result<()> {
        let listen_addrs = config.listen_addr.to_multi_addr();

        for listen_addr in listen_addrs {
            let _ = self.swarm.listen_on(listen_addr);
        }

        for bootnode_enr in &config.bootnodes {
            for multiaddr in &bootnode_enr.multiaddr() {
                if !self
                    .network_globals
                    .peers
                    .read()
                    .is_connected_or_dialing(&bootnode_enr.peer_id())
                {
                    let _ = self.swarm.dial(multiaddr.clone());
                }
            }
        }

        Ok(())
    }

    pub fn metadata(&self) -> MetaData {
        self.network_globals.local_metadata()
    }

    pub fn status(&self) -> Status {
        Status {
            chain_id: self.network_globals.chain_spec().chain.id(),
            block_hash: *self.network_globals.latest_block_hash().as_fixed_bytes(),
            block_number: self.network_globals.latest_block_number(),
        }
    }

    /// handle gossipsub event
    fn handle_gossipsub_event(&self, event: Box<gossipsub::Event>) -> Option<NetworkEvent> {
        match *event {
            gossipsub::Event::Message { propagation_source, message_id, message } => {
                let uo = match VerifiedUserOperation::deserialize(message.data.as_ref()) {
                    Ok(uo) => uo,
                    Err(e) => {
                        debug!("Failed to deserialize user operations: {:?}", e);
                        return None;
                    }
                };

                self.mempool_channels.iter().find_map(|(ep, mempool_sender, _)| {
                    if *ep == uo.entry_point() {
                        self.mempool_configs.iter().find_map(|(topic, canonical_mempool_config)| {
                            if topic == &message.topic {
                                let uo = uo.clone().user_operation();

                                mempool_sender
                                    .unbounded_send(NetworkMessage::Validate {
                                        user_operation: UserOperation::from_user_operation_signed(
                                            uo.hash(ep, self.network_globals.chain_spec().chain.id()),
                                            uo,
                                        ),
                                        validation_config: ValidationConfig {
                                            min_stake: Some(canonical_mempool_config.min_stake),
                                            min_unstake_delay: None,
                                            topic: Some(message.topic.to_string()),
                                            ignore_prev: false,
                                        }
                                    })
                                    .expect("mempool channel should be open all the time");

                                Some(())
                            } else {
                                warn!("User operation from p2p is using unsupported canonical mempool {}" , message.topic);
                                None
                            }
                        });

                        Some(())
                    } else {
                        warn!("User operation from p2p is using unsupported entry point {ep:?}");
                        None
                    }
                });

                let message = PubsubMessage::UserOperation(uo);

                Some(NetworkEvent::PubsubMessage {
                    source_peer: propagation_source,
                    id: message_id,
                    message,
                })
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                debug!("Peer {:?} subscribed to {:?}", peer_id, topic);
                Some(NetworkEvent::Subscribe { peer_id, topic })
            }
            msg => {
                debug!("{msg:?}");
                None
            }
        }
    }

    /// handle reqrep event
    fn handle_rpc_event(&mut self, event: RPCEvent) -> Option<NetworkEvent> {
        match event {
            RPCEvent::Request { peer_id, request, sender, .. } => match request {
                InboundRequest::Ping(ping) => {
                    self.swarm.behaviour_mut().peer_manager.ping_request(&peer_id, ping.data);
                    sender
                        .send(RPCResponse::Pong(Ping::new(self.metadata().seq_number)))
                        .expect("channel should exist");
                    None
                }
                InboundRequest::MetaData(_) => {
                    sender
                        .send(RPCResponse::MetaData(self.metadata()))
                        .expect("channel should exist");
                    None
                }
                InboundRequest::Status(_status) => {
                    // TODO: verify status message
                    sender.send(RPCResponse::Status(self.status())).expect("channel should exist");
                    None
                }
                InboundRequest::Goodbye(_) => None,
                _ => Some(NetworkEvent::RequestMessage { peer_id, request, sender }),
            },
            RPCEvent::Response { peer_id, response, .. } => match response {
                RPCResponse::Pong(ping) => {
                    self.swarm.behaviour_mut().peer_manager.pong_response(&peer_id, ping.data);
                    None
                }
                RPCResponse::MetaData(metadata) => {
                    self.swarm.behaviour_mut().peer_manager.metadata_response(&peer_id, metadata);
                    None
                }
                _ => Some(NetworkEvent::ResponseMessage { peer_id, response }),
            },
            _ => None,
        }
    }

    // handle discovery event
    fn handle_discovery_event(&mut self, event: DiscoveredPeers) -> Option<NetworkEvent> {
        self.swarm.behaviour_mut().peer_manager.peers_discovered(event.peers);
        None
    }

    /// handle peer manager event
    fn handler_peer_manager_event(&mut self, event: PeerManagerEvent) -> Option<NetworkEvent> {
        match event {
            PeerManagerEvent::PeerConnectedIncoming(peer_id) => {
                Some(NetworkEvent::PeerConnectedIncoming(peer_id))
            }
            PeerManagerEvent::PeerConnectedOutgoing(peer_id) => {
                Some(NetworkEvent::PeerConnectedOutgoing(peer_id))
            }
            PeerManagerEvent::PeerDisconnected(peer_id) => {
                Some(NetworkEvent::PeerDisconnected(peer_id))
            }
            PeerManagerEvent::DiscoverPeers(peers_to_find) => {
                self.swarm.behaviour_mut().discovery.discover_peers(peers_to_find);
                None
            }
            PeerManagerEvent::Ping(peer_id) => {
                self.send_request(
                    &peer_id,
                    OutboundRequest::Ping(Ping::new(self.metadata().seq_number)),
                );
                None
            }
            PeerManagerEvent::MetaData(peer_id) => {
                self.send_request(&peer_id, OutboundRequest::MetaData(MetaDataRequest));
                None
            }
            _ => None,
        }
    }

    pub fn poll_network(&mut self, cx: &mut Context) -> Poll<NetworkEvent> {
        let mut uos_received: Vec<(VerifiedUserOperation, TopicHash)> = Vec::new();

        for (ep, mempool_sender, mempool_receiver) in self.mempool_channels.iter_mut() {
            while let Ok(Some(message)) = mempool_receiver.try_next() {
                match message {
                    NetworkMessage::Publish {
                        user_operation,
                        verified_at_block_hash,
                        validation_config,
                    } => {
                        info!("Received user operation (verified at {verified_at_block_hash:?}) to gossip over p2p: {user_operation:?}");

                        let user_op = VerifiedUserOperation::new(
                            user_operation.user_operation.clone(),
                            *ep,
                            verified_at_block_hash,
                        );

                        if let Some(topic) = validation_config.topic {
                            uos_received.push((user_op, TopicHash::from_raw(topic)));
                        } else if let Some((first_mempool_topic, first_mempool_config)) =
                            self.mempool_configs.first()
                        {
                            mempool_sender
                                .unbounded_send(NetworkMessage::Validate {
                                    user_operation,
                                    validation_config: ValidationConfig {
                                        min_stake: Some(first_mempool_config.min_stake),
                                        min_unstake_delay: None,
                                        topic: Some(first_mempool_topic.to_string()),
                                        ignore_prev: true,
                                    },
                                })
                                .expect("mempool channel should be open all the time");
                        }
                    }
                    NetworkMessage::FindNewMempool { user_operation, topic } => {
                        let mut next = false;

                        for (canonical_mempool_topic, canonical_mempool_config) in
                            self.mempool_configs.iter()
                        {
                            if next {
                                mempool_sender
                                    .unbounded_send(NetworkMessage::Validate {
                                        user_operation,
                                        validation_config: ValidationConfig {
                                            min_stake: Some(canonical_mempool_config.min_stake),
                                            min_unstake_delay: None,
                                            topic: Some(canonical_mempool_topic.to_string()),
                                            ignore_prev: true,
                                        },
                                    })
                                    .expect("mempool channel should be open all the time");
                                break;
                            }

                            if topic == canonical_mempool_topic.to_string() {
                                next = true;
                            }
                        }
                    }
                    NetworkMessage::NewBlock { block_hash, block_number } => {
                        let mut latest_block_hash = self.network_globals.latest_block_hash.write();
                        *latest_block_hash = block_hash;
                        let mut latest_block_number =
                            self.network_globals.latest_block_number.write();
                        *latest_block_number = block_number;
                    }
                    _ => {}
                }
            }
        }

        for (uo, topic) in uos_received {
            match self.publish(uo.clone(), topic) {
                Ok(_) => {}
                Err(err) => match err {
                    PublishError::InsufficientPeers => {
                        warn!("Currently no peers to publish message");
                        self.swarm
                            .behaviour_mut()
                            .discovery
                            .discover_peers(FIND_NODE_QUERY_CLOSEST_PEERS);
                    }
                    e => error!("Error in publish message {e:?}"),
                },
            }
        }

        while let Poll::Ready(Some(swarm_event)) = self.swarm.poll_next_unpin(cx) {
            info!("Swarm event {swarm_event:?}");
            let event_opt = match swarm_event {
                SwarmEvent::Behaviour(event) => match event {
                    BehaviourEvent::GossipSub(event) => self.handle_gossipsub_event(event),
                    BehaviourEvent::RPC(event) => self.handle_rpc_event(event),
                    BehaviourEvent::Discovery(event) => self.handle_discovery_event(event),
                    BehaviourEvent::PeerManager(event) => self.handler_peer_manager_event(event),
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    Some(NetworkEvent::NewListenAddr(address))
                }
                event => {
                    {
                        debug!("Unhandled swarm event: {event:?}");
                    }
                    None
                }
            };
            if let Some(event) = event_opt {
                return Poll::Ready(event);
            }
        }

        Poll::Pending
    }

    pub async fn next_event(&mut self) -> NetworkEvent {
        futures::future::poll_fn(|cx| self.poll_network(cx)).await
    }

    pub fn local_peer_id(&self) -> &PeerId {
        self.swarm.local_peer_id()
    }

    /// Publish a gossipsub message.
    pub fn publish(
        &mut self,
        user_op: VerifiedUserOperation,
        topic_hash: TopicHash,
    ) -> Result<MessageId, PublishError> {
        let mut buf = Vec::new();
        let _ = user_op.serialize(&mut buf).expect("ssz of user ops serialization failed");
        self.swarm.behaviour_mut().gossipsub.publish(topic_hash, buf)
    }

    /// Subscribe to a topic.
    pub fn subscribe(&mut self, mempool_id: &str) -> Result<bool, SubscriptionError> {
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic(mempool_id))
    }

    /// Return the nodes local ENR.
    pub fn listened_addrs(&self) -> Vec<&Multiaddr> {
        self.swarm.listeners().collect()
    }

    /// Return the nodes local ENR.
    pub fn local_enr(&self) -> Enr {
        self.swarm.behaviour().discovery.local_enr()
    }

    /// Send a request to a peer.
    pub fn send_request(&mut self, peer: &PeerId, request: OutboundRequest) -> RequestId {
        self.swarm.behaviour_mut().rpc.send_request(peer, request)
    }

    /// Send a response to a peer.
    pub fn send_response(
        &mut self,
        response_channel: Sender<RPCResponse>,
        response: RPCResponse,
    ) -> Result<(), RPCResponse> {
        self.swarm.behaviour_mut().rpc.send_response(response_channel, response)
    }
}
