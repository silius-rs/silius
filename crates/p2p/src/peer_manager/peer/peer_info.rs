use crate::rpc::methods::MetaData;
use discv5::Enr;
use eyre::Result;
use libp2p::Multiaddr;

/// Information about a peer.
#[derive(Default, Debug, Clone)]
pub struct PeerInfo {
    /// Connection status of the peer.
    connection_status: PeerConnectionStatus,
    /// ENR of the peer.
    enr: Option<Enr>,
    /// Metadata of the peer.
    metadata: Option<MetaData>,
    /// Connection direction (ingoing or outgoing).
    connection_direction: Option<ConnectionDirection>,
}

impl PeerInfo {
    pub fn connection_status(&self) -> &PeerConnectionStatus {
        &self.connection_status
    }

    pub fn enr(&self) -> &Option<Enr> {
        &self.enr
    }

    pub fn metadata(&self) -> &Option<MetaData> {
        &self.metadata
    }

    pub fn set_metadata(&mut self, metadata: MetaData) {
        self.metadata = Some(metadata);
    }

    pub fn connection_direction(&self) -> &Option<ConnectionDirection> {
        &self.connection_direction
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, PeerConnectionStatus::Connected)
    }

    pub fn is_disconnected(&self) -> bool {
        matches!(self.connection_status, PeerConnectionStatus::Disconnected)
    }

    pub fn is_dialing(&self) -> bool {
        matches!(self.connection_status, PeerConnectionStatus::Dialing)
    }

    pub fn is_connected_or_dialing(&self) -> bool {
        self.is_connected() || self.is_dialing()
    }

    pub fn set_enr(&mut self, enr: Option<Enr>) {
        self.enr = enr;
    }

    pub fn set_connection_status(&mut self, connection_status: PeerConnectionStatus) {
        self.connection_status = connection_status;
    }

    pub fn set_dialing_peer(&mut self) -> Result<()> {
        match &mut self.connection_status {
            PeerConnectionStatus::Connected => {
                return Err(eyre::eyre!("Dialing peer is already connected"));
            }
            PeerConnectionStatus::Disconnecting => {
                return Err(eyre::eyre!("Dialing peer is disconnecting"));
            }
            PeerConnectionStatus::Dialing => {
                return Err(eyre::eyre!("Dialing peer is already dialing"));
            }
            PeerConnectionStatus::Disconnected | PeerConnectionStatus::Unknown => {
                self.connection_status = PeerConnectionStatus::Dialing;
            }
        }
        Ok(())
    }

    pub fn connect_ingoing(&mut self, _multiaddr: Multiaddr) {
        match &mut self.connection_status {
            PeerConnectionStatus::Connected |
            PeerConnectionStatus::Disconnected |
            PeerConnectionStatus::Disconnecting |
            PeerConnectionStatus::Dialing |
            PeerConnectionStatus::Unknown => {
                self.connection_status = PeerConnectionStatus::Connected;
                self.connection_direction = Some(ConnectionDirection::Incoming);
            }
        }
    }

    pub fn connect_outgoing(&mut self, _multiaddr: Multiaddr) {
        match &mut self.connection_status {
            PeerConnectionStatus::Connected |
            PeerConnectionStatus::Disconnected |
            PeerConnectionStatus::Disconnecting |
            PeerConnectionStatus::Dialing |
            PeerConnectionStatus::Unknown => {
                self.connection_status = PeerConnectionStatus::Connected;
                self.connection_direction = Some(ConnectionDirection::Outgoing);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Default)]
pub enum PeerConnectionStatus {
    Connected,
    Disconnected,
    Disconnecting,
    Dialing,
    #[default]
    Unknown,
}
