use anyhow::Result;
use clap::Parser;
use jsonrpsee::tracing::info;
use std::future::pending;

#[derive(Parser)]
#[clap(
    name = "AA - Bundler UoPool",
    about = "User operation pool for EIP-4337 Account Abstraction Bundler"
)]
pub struct Opt {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub grpc_listen_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let mut builder = tonic::transport::Server::builder();

    #[cfg(feature = "grpc-reflection")]
    builder.add_service(
        tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(ethereum_interfaces::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap(),
    );

    println!("{:?}", builder);

    info!("gRPC server listening on {}", opt.grpc_listen_address);
    pending().await
}
