use discv5::ListenConfig;
use libp2p::{multiaddr::Protocol, Multiaddr};
use ssz_rs::Bitvector;
use std::net::{Ipv4Addr, Ipv6Addr};

const DEFAULT_UDP_PORT: u16 = 9000;
const DEFAULT_TCP_PORT: u16 = 9000;

#[derive(ssz_rs_derive::Serializable, Clone, Debug, PartialEq, Default)]
pub struct Metadata {
    pub seq_number: u64,
    pub mempool_nets: Bitvector<64>,
}

/// The address to listen on for incoming connections.
/// Ip could be ipv4 or ipv6
#[derive(Clone, Debug)]
pub struct ListenAddr<Ip> {
    pub addr: Ip,
    pub udp_port: u16,
    pub tcp_port: u16,
}

/// Variant of ListenAddr that can be ipv4, ipv6 or dual.
#[derive(Clone, Debug)]
pub enum ListenAddress {
    Ipv4(ListenAddr<Ipv4Addr>),
    Ipv6(ListenAddr<Ipv6Addr>),
    Dual(ListenAddr<Ipv4Addr>, ListenAddr<Ipv6Addr>),
}

impl ListenAddress {
    pub fn to_multi_addr(&self) -> Vec<Multiaddr> {
        match self {
            ListenAddress::Ipv4(v) => vec![Multiaddr::from(v.addr).with(Protocol::Tcp(v.tcp_port))],
            ListenAddress::Ipv6(v) => vec![Multiaddr::from(v.addr).with(Protocol::Tcp(v.tcp_port))],
            ListenAddress::Dual(ipv4, ipv6) => {
                vec![
                    Multiaddr::from(ipv4.addr).with(Protocol::Tcp(ipv4.tcp_port)),
                    Multiaddr::from(ipv6.addr).with(Protocol::Tcp(ipv6.tcp_port)),
                ]
            }
        }
    }
}

impl Default for ListenAddress {
    fn default() -> Self {
        Self::Ipv4(ListenAddr {
            addr: Ipv4Addr::UNSPECIFIED,
            udp_port: DEFAULT_UDP_PORT,
            tcp_port: DEFAULT_TCP_PORT,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Config {
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: ListenAddress::Ipv4(ListenAddr {
                addr: Ipv4Addr::UNSPECIFIED,
                udp_port: DEFAULT_UDP_PORT,
                tcp_port: DEFAULT_TCP_PORT,
            }),
            ipv4_addr: Some(Ipv4Addr::UNSPECIFIED),
            ipv6_addr: None,
            enr_udp4_port: Some(DEFAULT_UDP_PORT),
            enr_tcp4_port: None,
            enr_udp6_port: None,
            enr_tcp6_port: None,
        }
    }
}

impl Config {
    pub fn to_listen_config(&self) -> ListenConfig {
        match &self.listen_addr {
            ListenAddress::Ipv4(v) => ListenConfig::Ipv4 { ip: v.addr, port: v.udp_port },
            ListenAddress::Ipv6(v) => ListenConfig::Ipv6 { ip: v.addr, port: v.udp_port },
            ListenAddress::Dual(ipv4, ipv6) => ListenConfig::DualStack {
                ipv4: ipv4.addr,
                ipv4_port: ipv4.udp_port,
                ipv6: ipv6.addr,
                ipv6_port: ipv6.udp_port,
            },
        }
    }
}
