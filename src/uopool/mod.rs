use crate::{
    types::user_operation::{UserOperation, UserOperationHash},
    uopool::{server::uopool_server::uo_pool_server::UoPoolServer, services::UoPoolService},
};
use anyhow::Result;
use clap::Parser;
use educe::Educe;
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
};
use jsonrpsee::tracing::info;
use parking_lot::RwLock;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

pub mod server;
pub mod services;

#[derive(Educe)]
#[educe(Debug)]
pub struct UserOperationPool {
    pub pool: Arc<RwLock<HashMap<UserOperationHash, UserOperation>>>,
}

impl UserOperationPool {
    pub fn new() -> Self {
        Self {
            pool: Default::default(),
        }
    }
}

impl Default for UserOperationPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Educe, Parser)]
#[educe(Debug)]
pub struct UoPoolOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,
}

pub async fn run(
    opts: UoPoolOpts,
    eth_provider: Arc<Provider<Http>>,
    entry_point: Address,
    max_verification_gas: U256,
) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = UoPoolServer::new(UoPoolService::new(
            Arc::new(UserOperationPool::new()),
            eth_provider,
            entry_point,
            max_verification_gas,
        ));

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
