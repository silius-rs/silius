use discv5::{enr::CombinedPublicKey, Enr};
use libp2p::{
    identity::{ed25519, secp256k1, KeyType, PublicKey},
    multiaddr::Protocol,
    Multiaddr, PeerId,
};
use tiny_keccak::{Hasher, Keccak};

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

pub fn peer_id_to_node_id(peer_id: &PeerId) -> Result<discv5::enr::NodeId, String> {
    let pk_bytes = &peer_id.to_bytes()[2..];

    let public_key = PublicKey::try_decode_protobuf(pk_bytes)
        .map_err(|e| format!(" Cannot parse libp2p public key public key from peer id: {e}"))?;

    match public_key.key_type() {
        KeyType::Secp256k1 => {
            let pk = public_key.clone().try_into_secp256k1().expect("right key type");
            let uncompressed_key_bytes = &pk.to_bytes_uncompressed()[1..];
            let mut output = [0_u8; 32];
            let mut hasher = Keccak::v256();
            hasher.update(uncompressed_key_bytes);
            hasher.finalize(&mut output);
            Ok(discv5::enr::NodeId::parse(&output).expect("Must be correct length"))
        }
        KeyType::Ed25519 => {
            let pk = public_key.clone().try_into_ed25519().expect("right key type");
            let uncompressed_key_bytes = pk.to_bytes();
            let mut output = [0_u8; 32];
            let mut hasher = Keccak::v256();
            hasher.update(&uncompressed_key_bytes);
            hasher.finalize(&mut output);
            Ok(discv5::enr::NodeId::parse(&output).expect("Must be correct length"))
        }

        _ => Err(format!("Unsupported public key from peer {peer_id}")),
    }
}
