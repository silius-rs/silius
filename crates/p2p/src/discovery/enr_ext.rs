use discv5::{enr::CombinedPublicKey, Enr};
use libp2p::{
    identity::{ed25519, secp256k1, PublicKey},
    multiaddr::Protocol,
    Multiaddr, PeerId,
};

pub trait EnrExt {
    /// PeerId of the ENR
    fn peer_id(&self) -> PeerId;

    /// Multiaddr used for dialing
    fn multiaddr(&self) -> Vec<Multiaddr>;
}

impl EnrExt for Enr {
    fn peer_id(&self) -> PeerId {
        self.public_key().as_peer_id()
    }

    fn multiaddr(&self) -> Vec<Multiaddr> {
        let mut multiaddrs: Vec<Multiaddr> = Vec::new();

        if let Some(ipv4) = self.ip4() {
            let mut addr: Multiaddr = ipv4.into();

            if let Some(tcp4) = self.tcp4() {
                addr.push(Protocol::Tcp(tcp4));
            }
            multiaddrs.push(addr);
        }

        multiaddrs
    }
}

pub trait CombinedPublicKeyExt {
    /// PeerId of the CombinedPublicKey
    fn as_peer_id(&self) -> PeerId;
}

impl CombinedPublicKeyExt for CombinedPublicKey {
    fn as_peer_id(&self) -> PeerId {
        match self {
            Self::Secp256k1(pk) => {
                let pk_bytes = pk.to_sec1_bytes();
                let libp2p_pk: PublicKey = secp256k1::PublicKey::try_from_bytes(&pk_bytes)
                    .expect("valid public key")
                    .into();
                PeerId::from_public_key(&libp2p_pk)
            }
            Self::Ed25519(pk) => {
                let pk_bytes = pk.to_bytes();
                let libp2p_pk: PublicKey =
                    ed25519::PublicKey::try_from_bytes(&pk_bytes).expect("valid public key").into();
                PeerId::from_public_key(&libp2p_pk)
            }
        }
    }
}
