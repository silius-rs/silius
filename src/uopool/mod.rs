use crate::{
    types::user_operation::{UserOperation, UserOperationHash},
    uopool::{server::uopool_server::uo_pool_server::UoPoolServer, services::UoPoolService},
};
use anyhow::Result;
use clap::Parser;
use educe::Educe;
use ethers::types::Address;
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

pub async fn run(opts: UoPoolOpts, _entry_points: Vec<Address>) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = UoPoolServer::new(UoPoolService::new(Arc::new(UserOperationPool::new())));

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
