use std::{
    io,
    task::{Context, Poll},
    time::Duration,
};

use discv5::Enr;
use ethers::types::{Address, U256};
use futures::channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use libp2p::{
    core::{transport::ListenerId, upgrade},
    futures::StreamExt,
    gossipsub::{self, MessageId, PublishError, SubscriptionError, TopicHash},
    identity::Keypair,
    noise,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm, SwarmBuilder, TransportError,
};
use libp2p_mplex::{MaxBufferBehaviour, MplexConfig};
use silius_primitives::{Chain, UserOperation, UserOperationsWithEntryPoint};
use ssz_rs::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{
    behaviour::Behaviour,
    config::Config,
    discovery,
    enr::{keypair_to_combine, EnrExt},
    gossipsub::topic,
    peer_manager::PeerManagerEvent,
    request_response::{self, Ping, Request, RequestId, Response},
};

struct TokioExecutor;
impl libp2p::swarm::Executor for TokioExecutor {
    fn exec(&self, future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }
}

#[derive(Debug, PartialEq)]
pub enum PubsubMessage {
    UserOps(UserOperationsWithEntryPoint),
}

#[derive(Debug)]
pub enum NetworkEvent {
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
    /// Request or response message from the network
    RequestMessage {
        /// The peer that sent the request.
        peer_id: PeerId,
        /// Request the peer sent.
        request: Request,
        /// response sender
        response_sender: oneshot::Sender<Response>,
    },
    ResponseMessage {
        /// The peer that sent the request.
        peer_id: PeerId,
        /// Request the peer sent.
        response: Response,
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

pub type EntrypointChannels = Vec<(
    Chain,
    Address,
    UnboundedReceiver<(UserOperation, U256)>,
    UnboundedSender<UserOperation>,
)>;
/// P2P network struct that holds the libp2p Swarm
/// Other components should interact with Network directly instead of behaviour
pub struct Network {
    swarm: Swarm<Behaviour>,
    entrypoint_channels: EntrypointChannels,
}

impl From<Network> for Swarm<Behaviour> {
    fn from(value: Network) -> Self {
        value.swarm
    }
}

impl Network {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: Keypair,
        config: Config,
        entrypoint_channels: EntrypointChannels,
        ping_interval: Duration,
        target_peers: usize,
    ) -> eyre::Result<Self> {
        let combine_key = keypair_to_combine(key.clone())?;
        let behaviour = Behaviour::new(
            combine_key,
            config,
            entrypoint_channels
                .iter()
                .map(|(c, _, _, _)| c.p2p_mempool_id())
                .collect(),
            ping_interval,
            target_peers,
        )?;

        // mplex config
        let mut mplex_config = MplexConfig::new();
        mplex_config.set_max_buffer_size(256);
        mplex_config.set_max_buffer_behaviour(MaxBufferBehaviour::Block);

        // yamux config
        let mut yamux_config = libp2p::yamux::Config::default();
        yamux_config.set_window_update_mode(libp2p::yamux::WindowUpdateMode::on_read());
        let swarm = SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                noise::Config::new,
                || upgrade::SelectUpgrade::new(yamux_config, mplex_config),
            )
            .expect("building p2p transport failed")
            .with_behaviour(|_| behaviour)
            .expect("building p2p behaviour failed")
            .build();
        Ok(Self {
            swarm,
            entrypoint_channels,
        })
    }

    fn handle_gossipsub_event(&self, event: Box<gossipsub::Event>) -> Option<NetworkEvent> {
        match *event {
            gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            } => {
                let userops = match UserOperationsWithEntryPoint::deserialize(message.data.as_ref())
                {
                    Ok(userops) => userops,
                    Err(e) => {
                        debug!("Failed to deserialize userops: {:?}", e);
                        return None;
                    }
                };
                self.entrypoint_channels
                    .iter()
                    .find_map(|(_, ep, _, new_coming_uos_ch)| {
                        if *ep == userops.entrypoint_address() {
                            for user_op in userops.clone().user_ops().into_iter() {
                                new_coming_uos_ch
                                    .unbounded_send(user_op)
                                    .expect("new useop channel should be open all the time");
                            }
                            Some(())
                        } else {
                            warn!("Received unsupported entrypoint userops {ep:?} from p2p");
                            None
                        }
                    });
                let message = PubsubMessage::UserOps(userops);

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

    fn handle_reqrep_event(&self, event: request_response::Event) -> Option<NetworkEvent> {
        match event {
            request_response::Event::Request {
                peer_id,
                request,
                response_sender,
                ..
            } => Some(NetworkEvent::RequestMessage {
                peer_id,
                request,
                response_sender,
            }),
            request_response::Event::Response {
                peer_id, response, ..
            } => Some(NetworkEvent::ResponseMessage { peer_id, response }),
            _ => None,
        }
    }

    // TODO: discovery peer connect
    fn handle_discovery_event(&self, _event: discovery::DiscoverPeers) -> Option<NetworkEvent> {
        None
    }

    /// handle peer manager event
    fn handler_peer_manager_event(&mut self, event: PeerManagerEvent) -> Option<NetworkEvent> {
        match event {
            PeerManagerEvent::Ping(peer) => {
                // FIXME: seq number should be a counter
                self.send_request(&peer, Request::Ping(Ping::new(1)));
                None
            }
            PeerManagerEvent::PeerConnectedIncoming(peer)
            | PeerManagerEvent::PeerConnectedOutgoing(peer) => {
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer);

                Some(NetworkEvent::PeerConnected(peer))
            }
            PeerManagerEvent::PeerDisconnected(_) => None,
            PeerManagerEvent::DiscoverPeers(_) => None,
        }
    }

    pub fn poll_network(&mut self, cx: &mut Context) -> Poll<NetworkEvent> {
        let mut msg_to_publich = Vec::new();
        for (chain, ep, waiting_to_publish_ch, _) in self.entrypoint_channels.iter_mut() {
            while let Ok(Some((pub_userop, verified_block))) = waiting_to_publish_ch.try_next() {
                info!("Got userop {pub_userop:?} from ep {ep:} verified in {verified_block:?} to publish to p2p network!");
                let pub_msg = UserOperationsWithEntryPoint::new(
                    *ep,
                    verified_block,
                    chain.id().into(),
                    vec![pub_userop],
                );
                msg_to_publich.push(pub_msg);
            }
        }

        for pub_msg in msg_to_publich.into_iter() {
            match self.publish(pub_msg) {
                Ok(_) => {}
                Err(err) => match err {
                    PublishError::InsufficientPeers => {
                        warn!("Currently no peers to publish message");
                        self.swarm.behaviour_mut().discv5.discover_peers(16usize);
                    }
                    e => error!("Error in publish message {e:?}"),
                },
            }
        }

        while let Poll::Ready(Some(swarm_event)) = self.swarm.poll_next_unpin(cx) {
            info!("Swarm get event {swarm_event:?}");
            let event_opt = match swarm_event {
                SwarmEvent::Behaviour(e) => match e {
                    crate::behaviour::Event::GossipSub(event) => self.handle_gossipsub_event(event),
                    crate::behaviour::Event::Reqrep(event) => self.handle_reqrep_event(event),
                    crate::behaviour::Event::Discovery(event) => self.handle_discovery_event(event),
                    crate::behaviour::Event::PeerManager(event) => {
                        self.handler_peer_manager_event(event)
                    }
                },

                SwarmEvent::NewListenAddr { address, .. } => {
                    Some(NetworkEvent::NewListenAddr(address))
                }
                event => {
                    {
                        debug!("unhandled swarn event:  {event:?}");
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

    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<ListenerId, TransportError<io::Error>> {
        self.swarm.listen_on(addr)
    }

    pub async fn next_event(&mut self) -> NetworkEvent {
        futures::future::poll_fn(|cx| self.poll_network(cx)).await
    }

    pub fn local_peer_id(&self) -> &PeerId {
        self.swarm.local_peer_id()
    }

    pub fn publish(
        &mut self,
        user_ops: UserOperationsWithEntryPoint,
    ) -> Result<MessageId, PublishError> {
        let mut buf = Vec::new();
        let _ = user_ops
            .serialize(&mut buf)
            .expect("ssz of user ops serialization failed");
        let topic_hash: TopicHash = topic(user_ops.chain().p2p_mempool_id().as_str()).into();
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic_hash, buf)
    }

    pub fn dial(&mut self, enr: Enr) -> eyre::Result<()> {
        let addrs = enr.multiaddr()?;
        for addr in addrs {
            self.swarm.dial(addr)?;
        }
        self.swarm
            .behaviour_mut()
            .discv5
            .discovery
            .add_enr(enr)
            .map_err(|e| eyre::eyre!(e.to_string()))?;
        Ok(())
    }

    pub fn subscribe(&mut self, mempool_id: &str) -> Result<bool, SubscriptionError> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic(mempool_id))
    }

    pub fn listened_addrs(&self) -> Vec<&Multiaddr> {
        self.swarm.listeners().collect()
    }

    pub fn local_enr(&self) -> Enr {
        self.swarm.behaviour().discv5.local_enr()
    }

    pub fn send_request(&mut self, peer: &PeerId, request: Request) -> RequestId {
        self.swarm
            .behaviour_mut()
            .reqrep
            .send_request(peer, request)
    }

    pub fn send_response(
        &mut self,
        response_channel: oneshot::Sender<Response>,
        response: Response,
    ) -> Result<(), Response> {
        self.swarm
            .behaviour_mut()
            .reqrep
            .send_response(response_channel, response)
    }
}
