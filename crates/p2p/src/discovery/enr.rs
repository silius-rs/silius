use crate::config::Config;
use discv5::{
    enr::{CombinedKey, Enr as EnrBuilder},
    Enr,
};
use libp2p::identity::Keypair;

/// Convert a libp2p Keypair into a discv5 CombinedKey
pub fn keypair_to_combined(keypair: &Keypair) -> eyre::Result<CombinedKey> {
    match keypair.clone().try_into_secp256k1() {
        Ok(key) => {
            let secret =
                discv5::enr::k256::ecdsa::SigningKey::from_bytes(&key.secret().to_bytes().into())
                    .expect("libp2p key must be valid");
            Ok(CombinedKey::Secp256k1(secret))
        }
        Err(_) => eyre::bail!("libp2p key must be secp256k1"),
    }
}

/// Build an ENR from a libp2p Keypair and config
pub fn build_enr(key: &CombinedKey, config: &Config) -> eyre::Result<Enr> {
    let mut enr_builder = EnrBuilder::builder();

    if let Some(ip) = config.ipv4_addr {
        enr_builder.ip4(ip);
    }
    if let Some(ip) = config.ipv6_addr {
        enr_builder.ip6(ip);
    }
    if let Some(port) = config.enr_tcp4_port {
        enr_builder.tcp4(port);
    }
    if let Some(port) = config.enr_tcp6_port {
        enr_builder.tcp6(port);
    }
    if let Some(port) = config.enr_udp4_port {
        enr_builder.udp4(port);
    }
    if let Some(port) = config.enr_udp6_port {
        enr_builder.udp6(port);
    }

    enr_builder.add_value("chain_id", &ssz_rs::serialize(&config.chain_spec.chain.id())?);

    let enr = enr_builder.build(key)?;

    Ok(enr)
}
