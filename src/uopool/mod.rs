use crate::{
    contracts::EntryPoint,
    types::{
        reputation::{
            BadReputationError, ReputationEntry, ReputationStatus, StakeInfo, BAN_SLACK,
            MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
        },
        simulation::CodeHash,
        user_operation::{UserOperation, UserOperationHash},
    },
    uopool::{
        memory_mempool::MemoryMempool, memory_reputation::MemoryReputation,
        server::uopool::uo_pool_server::UoPoolServer, services::UoPoolService,
    },
    utils::parse_u256,
};
use anyhow::Result;
use clap::Parser;
use educe::Educe;
use ethers::{
    abi::AbiEncode,
    providers::{Http, Middleware, Provider},
    types::{Address, Bytes, H256, U256},
    utils::{keccak256, to_checksum},
};
use jsonrpsee::tracing::info;
use parking_lot::RwLock;
use std::{collections::HashMap, fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

pub mod database_mempool;
pub mod memory_mempool;
pub mod memory_reputation;
pub mod server;
pub mod services;
pub mod utils;

pub type MempoolId = H256;

pub type MempoolBox<T, U> =
    Box<dyn Mempool<UserOperations = T, CodeHashes = U, Error = anyhow::Error> + Send + Sync>;
pub type ReputationBox<T> = Box<dyn Reputation<ReputationEntries = T> + Send + Sync>;

pub fn mempool_id(entry_point: &Address, chain_id: &U256) -> MempoolId {
    H256::from_slice(
        keccak256([to_checksum(entry_point, None).encode(), chain_id.encode()].concat()).as_slice(),
    )
}

pub trait Mempool: Debug {
    type UserOperations: IntoIterator<Item = UserOperation>;
    type CodeHashes: IntoIterator<Item = CodeHash>;
    type Error;
    fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, Self::Error>;
    fn get(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, Self::Error>;
    fn get_all_by_sender(&self, sender: &Address) -> Self::UserOperations;
    fn get_number_by_sender(&self, sender: &Address) -> usize;
    fn has_code_hashes(&self, user_operation_hash: &UserOperationHash)
        -> Result<bool, Self::Error>;
    fn set_code_hashes(
        &mut self,
        user_operation_hash: &UserOperationHash,
        code_hashes: &Self::CodeHashes,
    ) -> Result<(), Self::Error>;
    fn get_code_hashes(&self, user_operation_hash: &UserOperationHash) -> Self::CodeHashes;
    fn remove(&mut self, user_operation_hash: &UserOperationHash) -> Result<(), Self::Error>;
    // Get UserOperations sorted by max_priority_fee_per_gas without dup sender
    fn get_sorted(&self) -> Result<Self::UserOperations, Self::Error>;
    fn get_all(&self) -> Self::UserOperations;
    fn clear(&mut self);
}

pub trait Reputation: Debug {
    type ReputationEntries: IntoIterator<Item = ReputationEntry>;

    fn init(
        &mut self,
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: U256,
    );
    fn get(&mut self, address: &Address) -> ReputationEntry;
    fn increment_seen(&mut self, address: &Address);
    fn increment_included(&mut self, address: &Address);
    fn update_hourly(&mut self);
    fn add_whitelist(&mut self, address: &Address) -> bool;
    fn remove_whitelist(&mut self, address: &Address) -> bool;
    fn is_whitelist(&self, address: &Address) -> bool;
    fn add_blacklist(&mut self, address: &Address) -> bool;
    fn remove_blacklist(&mut self, address: &Address) -> bool;
    fn is_blacklist(&self, address: &Address) -> bool;
    fn get_status(&self, address: &Address) -> ReputationStatus;
    fn update_handle_ops_reverted(&mut self, address: &Address);
    fn verify_stake(
        &self,
        title: &str,
        stake_info: Option<StakeInfo>,
    ) -> Result<(), BadReputationError>;

    // Try to get the reputation status from a sequence of bytes which the first 20 bytes should be the address
    // This is useful in getting the reputation directly from paymaster_and_data field and init_code field in user operation.
    // If the address is not found in the first 20 bytes, it would return ReputationStatus::OK directly.
    fn get_status_from_bytes(&self, bytes: &Bytes) -> ReputationStatus {
        let address_opt = utils::get_addr(bytes);
        if let Some(address) = address_opt {
            self.get_status(&address)
        } else {
            ReputationStatus::OK
        }
    }

    fn set(&mut self, reputation_entries: Self::ReputationEntries);
    fn get_all(&self) -> Self::ReputationEntries;
    fn clear(&mut self);
}

#[derive(Educe)]
#[educe(Debug)]
pub struct UserOperationPool<M: Mempool> {
    pub pool: Arc<M>,
}

#[derive(Clone, Copy, Debug, Parser, PartialEq)]
pub struct UoPoolOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,

    #[clap(long, value_parser=parse_u256, default_value = "1")]
    pub min_stake: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_unstake_delay: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_priority_fee_per_gas: U256,
}

pub async fn run(
    opts: UoPoolOpts,
    entry_points: Vec<Address>,
    eth_provider: Arc<Provider<Http>>,
    max_verification_gas: U256,
) -> Result<()> {
    let chain_id = eth_provider.get_chainid().await?;

    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let mut entry_points_map = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
        let mut mempools =
            HashMap::<MempoolId, MempoolBox<Vec<UserOperation>, Vec<CodeHash>>>::new();
        let mut reputations = HashMap::<MempoolId, ReputationBox<Vec<ReputationEntry>>>::new();

        for entry_point in entry_points {
            let id = mempool_id(&entry_point, &chain_id);
            mempools.insert(id, Box::<MemoryMempool>::default());

            reputations.insert(id, Box::<MemoryReputation>::default());
            if let Some(reputation) = reputations.get_mut(&id) {
                reputation.init(
                    MIN_INCLUSION_RATE_DENOMINATOR,
                    THROTTLING_SLACK,
                    BAN_SLACK,
                    opts.min_stake,
                    opts.min_unstake_delay,
                );
            }
            entry_points_map.insert(
                id,
                EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
            );
        }

        let reputations = Arc::new(RwLock::new(reputations));

        let svc = UoPoolServer::new(UoPoolService::new(
            Arc::new(entry_points_map),
            Arc::new(RwLock::new(mempools)),
            reputations.clone(),
            eth_provider,
            max_verification_gas,
            opts.min_priority_fee_per_gas,
            chain_id,
        ));

        tokio::spawn(async move {
            loop {
                for reputation in reputations.write().values_mut() {
                    reputation.update_hourly();
                }
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });

        info!(
            "UoPool gRPC server starting on {}",
            opts.uopool_grpc_listen_address
        );

        builder
            .add_service(svc)
            .serve(opts.uopool_grpc_listen_address)
            .await
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
