use discv5::{
    enr::{
        k256::{ecdsa::VerifyingKey, CompressedPoint},
        CombinedKey, CombinedPublicKey,
    },
    Enr,
};
use libp2p::{
    identity::{secp256k1, Keypair, PublicKey},
    multiaddr::Protocol,
    Multiaddr, PeerId,
};

pub trait EnrExt {
    /// PeerId of the ENR
    fn peer_id(&self) -> eyre::Result<PeerId>;

    /// Multiaddr used for dialing
    fn multiaddr(&self) -> eyre::Result<Vec<Multiaddr>>;
}

impl EnrExt for Enr {
    fn peer_id(&self) -> eyre::Result<PeerId> {
        self.public_key().as_peer_id()
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

pub trait CombinedKeyExt {
    /// Convert a libp2p Keypair into a discv5 CombinedKey
    fn from_libp2p_keypair(keypair: Keypair) -> eyre::Result<CombinedKey>;
}

pub trait CombinedPublicKeyExt {
    /// PeerId of the CombinedPublicKey
    fn as_peer_id(&self) -> eyre::Result<PeerId>;
}

impl CombinedKeyExt for CombinedKey {
    fn from_libp2p_keypair(keypair: Keypair) -> eyre::Result<CombinedKey> {
        match keypair.try_into_secp256k1() {
            Ok(key) => {
                let secret = discv5::enr::k256::ecdsa::SigningKey::from_bytes(
                    &key.secret().to_bytes().into(),
                )
                .expect("libp2p key must be valid");
                Ok(CombinedKey::Secp256k1(secret))
            }
            Err(_) => eyre::bail!("libp2p key must be either secp256k1"),
        }
    }
}

impl CombinedPublicKeyExt for CombinedPublicKey {
    fn as_peer_id(&self) -> eyre::Result<PeerId> {
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
