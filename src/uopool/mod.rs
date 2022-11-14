use crate::uopool::{server::server::uo_pool_server::UoPoolServer, services::UoPoolService};
use anyhow::Result;
use clap::Parser;
use educe::Educe;
use jsonrpsee::tracing::info;
use std::{net::SocketAddr, time::Duration};

pub mod server;
pub mod services;

#[derive(Educe, Parser)]
#[educe(Debug)]
pub struct Opts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub grpc_listen_address: SocketAddr,
}

pub async fn run(opts: Opts) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = UoPoolServer::new(UoPoolService::new());

        info!(
            "UoPool gRPC server starting on {}",
            opts.grpc_listen_address
        );

        builder
            .add_service(svc)
            .serve(opts.grpc_listen_address)
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
