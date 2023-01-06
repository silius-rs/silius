use crate::{
    types::user_operation::{UserOperation, UserOperationHash},
    uopool::{
        memory::MemoryMempool, server::uopool_server::uo_pool_server::UoPoolServer,
        services::UoPoolService,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use educe::Educe;
use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::keccak256,
};
use jsonrpsee::tracing::info;
use parking_lot::RwLock;
use std::{collections::HashMap, fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

pub mod memory;
pub mod server;
pub mod services;

pub type MempoolId = H256;

pub fn mempool_id(entry_point: Address, chain_id: U256) -> MempoolId {
    H256::from_slice(keccak256([entry_point.encode(), chain_id.encode()].concat()).as_slice())
}

#[async_trait]
pub trait Mempool: Debug + Send + Sync + 'static {
    async fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<UserOperationHash>;
    async fn get(&self, user_operation_hash: UserOperationHash) -> anyhow::Result<UserOperation>;
    async fn get_all(&self) -> anyhow::Result<Vec<UserOperation>>;
    async fn get_all_by_sender(&self, sender: Address) -> anyhow::Result<Vec<UserOperation>>;
    async fn remove(&mut self, user_operation_hash: UserOperationHash) -> anyhow::Result<()>;
    async fn clear(&mut self) -> anyhow::Result<()>;
}

#[derive(Educe)]
#[educe(Debug)]
pub struct UserOperationPool<M: Mempool> {
    pub pool: Arc<M>,
}

#[derive(Educe, Parser)]
#[educe(Debug)]
pub struct UoPoolOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,
}

pub async fn run(opts: UoPoolOpts, entry_points: Vec<Address>, chain_id: U256) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let mut mempools = HashMap::<MempoolId, Box<dyn Mempool>>::new();
        for entry_point in entry_points {
            let id = mempool_id(entry_point, chain_id);
            mempools.insert(id, Box::<MemoryMempool>::default());
        }

        let svc = UoPoolServer::new(UoPoolService::new(Arc::new(RwLock::new(mempools))));

        info!(
            "UoPool gRPC server starting on {}",
            opts.uopool_grpc_listen_address
        );

        builder
            .add_service(svc)
            .serve(opts.uopool_grpc_listen_address)
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
