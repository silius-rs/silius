use crate::listen_addr::{ListenAddr, ListenAddress};
use discv5::{Enr, ListenConfig};
use libp2p::gossipsub;
use sha2::{Digest, Sha256};
use silius_primitives::{
    chain::ChainSpec,
    constants::p2p::{
        IPV4_ADDRESS, MESSAGE_DOMAIN_VALID_SNAPPY, NODE_ENR_FILE_NAME, NODE_KEY_FILE_NAME,
        TARGET_PEERS, TCP_PORT, UDP_PORT,
    },
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::PathBuf,
};

#[derive(Clone, Debug)]
pub struct Config {
    /// File to store the node's key.
    pub node_key_file: PathBuf,

    /// File to store the node's enr.
    pub node_enr_file: PathBuf,

    /// The listening address for p2p.
    pub listen_addr: ListenAddress,

    /// The ipv4 address to broadcast to peers about which address we are listening on.
    pub ipv4_addr: Option<Ipv4Addr>,

    /// The ipv6 address to broadcast to peers about which address we are listening on.
    pub ipv6_addr: Option<Ipv6Addr>,

    /// The udp4 port to broadcast to peers in order to reach back for discovery.
    pub enr_udp4_port: Option<u16>,

    /// The tcp4 port to broadcast to peers in order to reach back for libp2p services.
    pub enr_tcp4_port: Option<u16>,

    /// The udp6 port to broadcast to peers in order to reach back for discovery.
    pub enr_udp6_port: Option<u16>,

    /// The tcp6 port to broadcast to peers in order to reach back for libp2p services.
    pub enr_tcp6_port: Option<u16>,

    /// Gossipsub configuration.
    pub gs_config: gossipsub::Config,

    /// Discv5 configuration.
    pub discv5_config: discv5::Config,

    /// Chain specification the p2p network is connected on.
    pub chain_spec: ChainSpec,

    /// Target number of peers.
    pub target_peers: usize,

    /// List of bootnodes.
    pub bootnodes: Vec<Enr>,

    /// List of whitelisted peer ENRs
    pub peers_whitelist: Vec<Enr>,

    /// List of whitelisted IP addresses
    pub ips_whitelist: Vec<IpAddr>,
}

impl Default for Config {
    fn default() -> Self {
        let gs_config = gossipsub::ConfigBuilder::default().build().expect("valid config");
        let discv5_config =
            discv5::ConfigBuilder::new(ListenConfig::Ipv4 { ip: IPV4_ADDRESS, port: UDP_PORT })
                .build();

        Self {
            node_key_file: PathBuf::from(NODE_KEY_FILE_NAME),
            node_enr_file: PathBuf::from(NODE_ENR_FILE_NAME),
            listen_addr: ListenAddress::V4(ListenAddr {
                addr: IPV4_ADDRESS,
                udp_port: UDP_PORT,
                tcp_port: TCP_PORT,
            }),
            ipv4_addr: Some(IPV4_ADDRESS),
            ipv6_addr: None,
            enr_udp4_port: Some(UDP_PORT),
            enr_tcp4_port: Some(TCP_PORT),
            enr_udp6_port: None,
            enr_tcp6_port: None,
            gs_config,
            discv5_config,
            chain_spec: ChainSpec::dev(),
            target_peers: TARGET_PEERS,
            bootnodes: vec![],
            peers_whitelist: vec![],
            ips_whitelist: vec![],
        }
    }
}

/// Builder for constructing p2p config
pub struct ConfigBuilder {
    config: Config,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    /// Create a new config builder.
    pub fn new() -> Self {
        Self { config: Config::default() }
    }

    /// Builds the config.
    pub fn build(self) -> Config {
        self.config
    }

    /// Set the node key file.
    pub fn node_key_file(mut self, node_key_file: PathBuf) -> Self {
        self.config.node_key_file = node_key_file;
        self
    }

    /// Set the node enr file.
    pub fn node_enr_file(mut self, node_enr_file: PathBuf) -> Self {
        self.config.node_enr_file = node_enr_file;
        self
    }

    /// Set the listen address.
    pub fn listen_addr(mut self, listen_addr: ListenAddress) -> Self {
        self.config.listen_addr = listen_addr;
        self
    }

    /// Set the ipv4 address.
    pub fn ipv4_addr(mut self, ipv4_addr: Option<Ipv4Addr>) -> Self {
        self.config.ipv4_addr = ipv4_addr;
        self
    }

    /// Set the ipv6 address.
    pub fn ipv6_addr(mut self, ipv6_addr: Option<Ipv6Addr>) -> Self {
        self.config.ipv6_addr = ipv6_addr;
        self
    }

    /// Set the udp4 port.
    pub fn enr_udp4_port(mut self, enr_udp4_port: Option<u16>) -> Self {
        self.config.enr_udp4_port = enr_udp4_port;
        self
    }

    /// Set the tcp4 port.
    pub fn enr_tcp4_port(mut self, enr_tcp4_port: Option<u16>) -> Self {
        self.config.enr_tcp4_port = enr_tcp4_port;
        self
    }

    /// Set the udp6 port.
    pub fn enr_udp6_port(mut self, enr_udp6_port: Option<u16>) -> Self {
        self.config.enr_udp6_port = enr_udp6_port;
        self
    }

    /// Set the tcp6 port.
    pub fn enr_tcp6_port(mut self, enr_tcp6_port: Option<u16>) -> Self {
        self.config.enr_tcp6_port = enr_tcp6_port;
        self
    }

    /// Set the gossipsub configuration.
    pub fn gs_config(mut self, gs_config: gossipsub::Config) -> Self {
        self.config.gs_config = gs_config;
        self
    }

    /// Set the discv5 configuration.
    pub fn discv5_config(mut self, discv5_config: discv5::Config) -> Self {
        self.config.discv5_config = discv5_config;
        self
    }

    /// Set the chain spec.
    pub fn chain_spec(mut self, chain_spec: ChainSpec) -> Self {
        self.config.chain_spec = chain_spec;
        self
    }

    /// Set the target number of peers.
    pub fn target_peers(mut self, target_peers: usize) -> Self {
        self.config.target_peers = target_peers;
        self
    }

    /// Set the bootnodes.
    pub fn bootnodes(mut self, bootnodes: Vec<Enr>) -> Self {
        self.config.bootnodes = bootnodes;
        self
    }

    /// Set the peers whitelist.
    pub fn peers_whitelist(mut self, peers_whitelist: Vec<Enr>) -> Self {
        self.config.peers_whitelist = peers_whitelist;
        self
    }

    /// Set the IPs whitelist.
    pub fn ips_whitelist(mut self, ips_whitelist: Vec<IpAddr>) -> Self {
        self.config.ips_whitelist = ips_whitelist;
        self
    }
}

/// Create a `GossipsubConfig`.
pub fn gossipsub_config() -> gossipsub::Config {
    let message_id_fn = |message: &gossipsub::Message| {
        let topic_bytes = message.topic.as_str().as_bytes();
        let topic_len_bytes = topic_bytes.len().to_le_bytes();

        let mut vec: Vec<u8> = Vec::with_capacity(
            MESSAGE_DOMAIN_VALID_SNAPPY.len() +
                topic_len_bytes.len() +
                topic_bytes.len() +
                message.data.len(),
        );
        vec.extend_from_slice(&MESSAGE_DOMAIN_VALID_SNAPPY);
        vec.extend_from_slice(&topic_len_bytes);
        vec.extend_from_slice(topic_bytes);
        vec.extend_from_slice(&message.data);

        Sha256::digest(vec)[..20].into()
    };

    gossipsub::ConfigBuilder::default()
        .validate_messages()
        .validation_mode(gossipsub::ValidationMode::Anonymous)
        .message_id_fn(message_id_fn)
        .build()
        .expect("valid config")
}
