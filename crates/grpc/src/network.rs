use ethers::providers::Middleware;
use silius_p2p::network::Network;
use silius_primitives::reputation::ReputationEntry;
use silius_uopool::{Mempool, Reputation, VecCh, VecUo};

use crate::builder::UoPoolBuilder;

/// The Integrator is for the integrations between p2p network and the uopool
pub struct NetworkIntegrator<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    network: Network,
    uopool_builder: UoPoolBuilder<M, P, R, E>,
}

impl<M, P, R, E> NetworkIntegrator<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    pub fn new(network: Network, uopool_builder: UoPoolBuilder<M, P, R, E>) -> Self {
        Self {
            network,
            uopool_builder,
        }
    }
}
