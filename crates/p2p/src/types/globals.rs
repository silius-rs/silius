use crate::{discovery::enr_ext::EnrExt, peer_manager::peerdb::PeerDB, rpc::methods::MetaData};
use discv5::Enr;
use ethers::types::H256;
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use silius_primitives::chain::ChainSpec;

pub struct NetworkGlobals {
    /// The local ENR of the node.
    pub local_enr: RwLock<Enr>,
    /// The local peer id of the node.
    pub peer_id: RwLock<PeerId>,
    /// Listening multiaddrs.
    pub listen_multiaddrs: RwLock<Vec<Multiaddr>>,
    /// Peers of the node.
    pub peers: RwLock<PeerDB>,
    /// The local metadata of the node.
    pub local_metadata: RwLock<MetaData>,
    /// Chain information.
    pub chain_spec: RwLock<ChainSpec>,
    /// Latest block hash.
    pub latest_block_hash: RwLock<H256>,
    /// Latest block number.
    pub latest_block_number: RwLock<u64>,
}

impl NetworkGlobals {
    pub fn new(
        enr: Enr,
        local_metadata: MetaData,
        trusted_peers: Vec<PeerId>,
        chain_spec: ChainSpec,
        latest_block_hash: H256,
        latest_block_number: u64,
    ) -> Self {
        let peer_id = enr.peer_id();
        let multiaddrs = enr.multiaddr();

        Self {
            local_enr: RwLock::new(enr),
            peer_id: RwLock::new(peer_id),
            listen_multiaddrs: RwLock::new(multiaddrs),
            peers: RwLock::new(PeerDB::new(trusted_peers)),
            local_metadata: RwLock::new(local_metadata),
            chain_spec: RwLock::new(chain_spec),
            latest_block_hash: RwLock::new(latest_block_hash),
            latest_block_number: RwLock::new(latest_block_number),
        }
    }

    pub fn local_enr(&self) -> Enr {
        self.local_enr.read().clone()
    }

    pub fn peer_id(&self) -> PeerId {
        *self.peer_id.read()
    }

    pub fn listen_multiaddrs(&self) -> Vec<Multiaddr> {
        self.listen_multiaddrs.read().clone()
    }

    pub fn local_metadata(&self) -> MetaData {
        self.local_metadata.read().clone()
    }

    pub fn chain_spec(&self) -> ChainSpec {
        self.chain_spec.read().clone()
    }

    pub fn connected_or_dialing_peers(&self) -> usize {
        self.peers.read().connected_or_dialing_peers().count()
    }

    pub fn latest_block_hash(&self) -> H256 {
        *self.latest_block_hash.read()
    }

    pub fn latest_block_number(&self) -> u64 {
        *self.latest_block_number.read()
    }
}
