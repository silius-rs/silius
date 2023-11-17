use crate::config::Config;
use discv5::{
    enr::{
        k256::{ecdsa::VerifyingKey, CompressedPoint},
        CombinedKey, CombinedPublicKey, EnrBuilder,
    },
    Enr,
};
use libp2p::{
    identity::{secp256k1, Keypair, PublicKey},
    multiaddr::Protocol,
    Multiaddr, PeerId,
};

/// Convert a libp2p Keypair into a discv5 CombinedKey
pub fn keypair_to_combine(keypair: Keypair) -> eyre::Result<CombinedKey> {
    match keypair.try_into_secp256k1() {
        Ok(key) => {
            let secret =
                discv5::enr::k256::ecdsa::SigningKey::from_bytes(&key.secret().to_bytes().into())
                    .expect("libp2p key must be valid");
            Ok(CombinedKey::Secp256k1(secret))
        }
        Err(_) => eyre::bail!("libp2p key must be either secp256k1"),
    }
}

/// Build an ENR from a libp2p Keypair and config
pub fn build_enr(enr_key: &CombinedKey, config: &Config) -> eyre::Result<Enr> {
    let mut enr_builder = EnrBuilder::new("v4");
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

    let enr = enr_builder.build(enr_key)?;
    Ok(enr)
}

pub trait EnrExt {
    /// PeerId of the ENR
    fn peer_id(&self) -> eyre::Result<PeerId>;

    /// Multiaddr used for dialing
    fn multiaddr(&self) -> eyre::Result<Vec<Multiaddr>>;
}

impl EnrExt for Enr {
    fn peer_id(&self) -> eyre::Result<PeerId> {
        self.public_key().to_peer_id()
    }

    fn multiaddr(&self) -> eyre::Result<Vec<Multiaddr>> {
        let mut multiaddrs: Vec<Multiaddr> = Vec::new();
        if let Some(ipv4) = self.ip4() {
            let mut addr: Multiaddr = ipv4.into();

            if let Some(tcp4) = self.tcp4() {
                addr.push(Protocol::Tcp(tcp4));
            }
            multiaddrs.push(addr);
        }

        Ok(multiaddrs)
    }
}

pub trait CombineKeyPubExt {
    /// PeerId of the CombinedPublicKey
    fn to_peer_id(&self) -> eyre::Result<PeerId>;
}

impl CombineKeyPubExt for CombinedPublicKey {
    fn to_peer_id(&self) -> eyre::Result<PeerId> {
        let pub_key: PublicKey = match self {
            CombinedPublicKey::Secp256k1(pk) => {
                PublicKey::from(secp256k1::PublicKey::try_from_bytes(
                    <&VerifyingKey as Into<CompressedPoint>>::into(pk).as_slice(),
                )?)
            }
            _ => eyre::bail!("Only secp256k1 is supported"),
        };
        Ok(PeerId::from_public_key(&pub_key))
    }
}
