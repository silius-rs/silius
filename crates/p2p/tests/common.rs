use discv5::Enr;
use futures::channel::mpsc::unbounded;
use silius_p2p::{
    config::{gossipsub_config, Config},
    listen_addr::{ListenAddr, ListenAddress},
    service::{Network, NetworkEvent},
};
use silius_primitives::{chain::ChainSpec, constants::p2p::TARGET_PEERS};
use std::{
    net::{Ipv4Addr, TcpListener},
    time::Duration,
};
use tempfile::TempDir;

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

async fn build_p2p_instance(bootnode: Option<Enr>) -> eyre::Result<Network> {
    let dir = TempDir::new().unwrap();
    let node_key_file = dir.path().join("node_key");
    let node_enr_file = dir.path().join("node_enr");

    let available_port = get_available_port().unwrap();
    let listen_addr = ListenAddress::V4(ListenAddr {
        addr: Ipv4Addr::LOCALHOST,
        udp_port: available_port,
        tcp_port: available_port,
    });

    let chain_spec = ChainSpec::dev();

    let config = Config {
        node_key_file,
        node_enr_file,
        listen_addr: listen_addr.clone(),
        ipv4_addr: Some(Ipv4Addr::LOCALHOST),
        ipv6_addr: None,
        enr_udp4_port: Some(available_port),
        enr_tcp4_port: Some(available_port),
        enr_udp6_port: None,
        enr_tcp6_port: None,
        gs_config: gossipsub_config(),
        discv5_config: discv5::ConfigBuilder::new(listen_addr.to_listen_config()).build(),
        chain_spec: chain_spec.clone(),
        target_peers: TARGET_PEERS,
        bootnodes: if let Some(bootnode) = bootnode { vec![bootnode] } else { vec![] },
        peers_whitelist: vec![],
        ips_whitelist: vec![],
    };

    let (_, receiver) = unbounded();
    let (sender, _) = unbounded();

    let network = Network::new(
        config,
        (Default::default(), Default::default()),
        vec![(Default::default(), sender, receiver)],
    )
    .await?;

    Ok(network)
}

pub async fn build_connnected_p2p_pair() -> eyre::Result<(Network, Network)> {
    let mut peer1 = build_p2p_instance(None).await?;
    let mut peer2 = build_p2p_instance(Some(peer1.local_enr())).await?;

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
        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
        _ = joined => {}
    }

    Ok((peer1, peer2))
}
