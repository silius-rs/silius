use discv5::ListenConfig;
use libp2p::{multiaddr::Protocol, Multiaddr};
use std::net::{Ipv4Addr, Ipv6Addr};

/// The listening address for p2p.
#[derive(Clone, Debug)]
pub struct ListenAddr<Ip> {
    /// IP address.
    pub addr: Ip,
    /// The UDC port for discovery.
    pub udp_port: u16,
    /// The TCP port for libp2p services.
    pub tcp_port: u16,
}

/// All variants of listen address.
#[derive(Clone, Debug)]
pub enum ListenAddress {
    V4(ListenAddr<Ipv4Addr>),
    V6(ListenAddr<Ipv6Addr>),
    Dual(ListenAddr<Ipv4Addr>, ListenAddr<Ipv6Addr>),
}

impl ListenAddress {
    pub fn to_multi_addr(&self) -> Vec<Multiaddr> {
        match self {
            ListenAddress::V4(v) => vec![Multiaddr::from(v.addr).with(Protocol::Tcp(v.tcp_port))],
            ListenAddress::V6(v) => vec![Multiaddr::from(v.addr).with(Protocol::Tcp(v.tcp_port))],
            ListenAddress::Dual(ipv4, ipv6) => {
                vec![
                    Multiaddr::from(ipv4.addr).with(Protocol::Tcp(ipv4.tcp_port)),
                    Multiaddr::from(ipv6.addr).with(Protocol::Tcp(ipv6.tcp_port)),
                ]
            }
        }
    }

    pub fn to_listen_config(&self) -> ListenConfig {
        match self {
            ListenAddress::V4(v) => ListenConfig::Ipv4 { ip: v.addr, port: v.udp_port },
            ListenAddress::V6(v) => ListenConfig::Ipv6 { ip: v.addr, port: v.udp_port },
            ListenAddress::Dual(ipv4, ipv6) => ListenConfig::DualStack {
                ipv4: ipv4.addr,
                ipv4_port: ipv4.udp_port,
                ipv6: ipv6.addr,
                ipv6_port: ipv6.udp_port,
            },
        }
    }
}
