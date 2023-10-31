use std::{
    net::{Ipv4Addr, TcpListener},
    time::Duration,
};

use futures::channel::mpsc::unbounded;
use libp2p::identity::Keypair;
use silius_primitives::Chain;

use crate::{
    config::{Config, ListenAddr},
    network::{Network, NetworkEvent},
};
mod enr;
mod pubsub;
mod req_rep;

pub fn get_available_port() -> Option<u16> {
    let unused_port: u16;
    loop {
        let socket_addr = std::net::SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), 0);
        match TcpListener::bind(socket_addr) {
            Ok(listener) => match listener.local_addr().map(|s| s.port()) {
                Ok(p) => {
                    unused_port = p;
                    break;
                }
                Err(_) => {}
            },
            Err(_) => {}
        }
    }
    Some(unused_port)
}

fn build_p2p_instance() -> eyre::Result<Network> {
    let key = Keypair::generate_secp256k1();
    let available_port = get_available_port().unwrap();
    let config = Config {
        listen_addr: crate::config::ListenAddress::Ipv4(ListenAddr {
            addr: Ipv4Addr::LOCALHOST,
            udp_port: available_port,
            tcp_port: available_port,
        }),
        ipv4_addr: Some(Ipv4Addr::LOCALHOST),
        ipv6_addr: None,
        enr_udp4_port: Some(available_port),
        enr_tcp4_port: Some(available_port),
        enr_udp6_port: None,
        enr_tcp6_port: None,
    };
    let listen_addrs = config.listen_addr.to_multi_addr();
    let (_, rv) = unbounded();
    let (sd, _) = unbounded();
    let mut network = Network::new(
        key,
        config,
        vec![(Chain::from(5), Default::default(), rv, sd)],
        Duration::from_secs(10),
        30,
    )?;
    for listen_addr in listen_addrs {
        println!("listen on {listen_addr:?}");
        network.listen_on(listen_addr)?;
    }
    Ok(network)
}

pub async fn build_connnected_p2p_pair() -> eyre::Result<(Network, Network)> {
    let mut peer1 = build_p2p_instance()?;
    let mut peer2 = build_p2p_instance()?;
    // let the two nodes set up listeners
    let peer1_fut = async {
        loop {
            if let NetworkEvent::NewListenAddr(_) = peer1.next_event().await {
                return;
            }
        }
    };
    let peer2_fut = async {
        loop {
            if let NetworkEvent::NewListenAddr(_) = peer2.next_event().await {
                return;
            }
        }
    };

    let joined = futures::future::join(peer1_fut, peer2_fut);

    // wait for either both nodes to listen or a timeout
    tokio::select! {
        _  = tokio::time::sleep(Duration::from_millis(500)) => {}
        _ = joined => {}
    }
    let peer2_enr = peer2.local_enr();
    println!("peer1 dial peer2");
    peer1.dial(peer2_enr)?;
    Ok((peer1, peer2))
}
