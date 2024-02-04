use crate::rpc::methods::MetaData;

/// Information about a peer.
pub struct PeerInfo {
    /// Connection status of the peer.
    pub connection_status: ConnectionStatus,
    /// Metadata of the peer.
    pub _metadata: Option<MetaData>, // TODO: need to handle metadata updates
}

pub enum ConnectionStatus {
    Connected,
    Disconnected,
}
