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
        methods::{MetaData, MetaDataRequest, Ping, RPCResponse, RequestId},
        outbound::OutboundRequest,
        protocol::InboundRequest,
        RPCEvent, RPC,
    },
    service::{behaviour::BehaviourEvent, utils::save_private_key_to_file},
    types::{
        globals::NetworkGlobals,
        pubsub::{create_gossipsub, PubsubMessage},
        topics::topic,
    },
};
use discv5::Enr;
use ethers::types::{Address, U256};
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
    UserOperation, VerifiedUserOperation,
};
use ssz_rs::{Deserialize, List, Serialize, Vector};
use std::{
    env,
    sync::Arc,
    task::{Context, Poll},
};
use tracing::{debug, error, info, warn};

pub type MempoolChannels =
    Vec<(Address, UnboundedReceiver<(UserOperation, U256)>, UnboundedSender<UserOperation>)>; // entrypoint ...

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
    mempool_channels: MempoolChannels,
}

impl From<Network> for Swarm<Behaviour> {
    fn from(value: Network) -> Self {
        value.swarm
    }
}

impl Network {
    pub async fn new(config: Config, mempool_channels: MempoolChannels) -> eyre::Result<Self> {
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
        let enr = if let Some(enr) = load_enr_from_file(&config.node_enr_file) {
            enr
        } else {
            let enr = build_enr(&combined_key, &config).expect("enr building failed");

            save_enr_to_file(&enr, &config.node_enr_file);

            enr
        };

        info!("Enr: {}", enr);

        let canonical_mempools = config.chain_spec.canonical_mempools.clone();
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
            ))
        };

        let mut gossipsub = create_gossipsub(canonical_mempools).map_err(|e| eyre::anyhow!(e))?;
        for bootnode in &config.bootnodes {
            gossipsub.add_explicit_peer(&bootnode.peer_id());
        }

        let rpc = RPC::new();

        let peer_manager = PeerManager::new(network_globals.clone());

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

        let mut network = Network { swarm, network_globals, mempool_channels };

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

    /// handle gossipsub event
    fn handle_gossipsub_event(&self, event: Box<gossipsub::Event>) -> Option<NetworkEvent> {
        match *event {
            gossipsub::Event::Message { propagation_source, message_id, message } => {
                let user_op = match VerifiedUserOperation::deserialize(message.data.as_ref()) {
                    Ok(user_op) => user_op,
                    Err(e) => {
                        debug!("Failed to deserialize user operations: {:?}", e);
                        return None;
                    }
                };
                self.mempool_channels.iter().find_map(|(ep, _, new_coming_uos_ch)| {
                    if *ep == user_op.entry_point() {
                        let uo = user_op.clone().user_operation();
                        new_coming_uos_ch
                            .unbounded_send(UserOperation::from_user_operation_signed(
                                uo.hash(ep, self.network_globals.chain_spec().chain.id()),
                                uo,
                            ))
                            .expect("new user operation channel should be open all the time");
                        Some(())
                    } else {
                        warn!("Received unsupported entrypoint user operations {ep:?} from p2p");
                        None
                    }
                });
                let message = PubsubMessage::UserOperation(user_op);

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
        let mut uos_publish: Vec<VerifiedUserOperation> = Vec::new();

        for (ep, waiting_to_publish_ch, _) in self.mempool_channels.iter_mut() {
            while let Ok(Some((user_op, verified_at_block_hash))) = waiting_to_publish_ch.try_next()
            {
                info!("Got user operation {user_op:?} from ep {ep:?} verified in {verified_at_block_hash:?} to publish to p2p network!");
                let user_op =
                    VerifiedUserOperation::new(user_op.user_operation, *ep, verified_at_block_hash);
                uos_publish.push(user_op);
            }
        }

        for uo in uos_publish.into_iter() {
            for canonical_mempool in self.network_globals.chain_spec().canonical_mempools.iter() {
                let topic_hash: TopicHash = topic(canonical_mempool).into();
                match self.publish(uo.clone(), topic_hash) {
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
