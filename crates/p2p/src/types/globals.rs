use discv5::Enr;
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;

pub struct NetworkGlobals {
    /// The local ENR of the node.
    pub local_enr: RwLock<Enr>,
    /// The local peer id of the node.
    pub peer_id: RwLock<PeerId>,
    /// Listening multiaddrs.
    pub listen_multiaddrs: RwLock<Vec<Multiaddr>>,
}