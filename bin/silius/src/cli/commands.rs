use super::args::{BundlerAndUoPoolArgs, BundlerArgs, CreateWalletArgs, RpcArgs, UoPoolArgs};
use crate::bundler::{create_wallet, launch_bundler, launch_bundling, launch_rpc, launch_uopool};
use clap::Parser;
use silius_primitives::provider::{
    create_http_block_streams, create_http_provider, create_ws_block_streams, create_ws_provider,
};
use std::{future::pending, sync::Arc};

/// Start the bundler with all components (bundling component, user operation mempool, RPC server)
#[derive(Debug, Parser)]
pub struct NodeCommand {
    /// All Bundler specific args
    #[clap(flatten)]
    bundler: BundlerArgs,

    /// All UoPool specific args
    #[clap(flatten)]
    uopool: UoPoolArgs,

    /// Common Bundler and UoPool args
    #[clap(flatten)]
    common: BundlerAndUoPoolArgs,

    /// All RPC args
    #[clap(flatten)]
    rpc: RpcArgs,
}

impl NodeCommand {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<()> {
        if self.common.eth_client_address.clone().starts_with("http") {
            let eth_client = Arc::new(create_http_provider(&self.common.eth_client_address).await?);
            let block_streams =
                create_http_block_streams(eth_client.clone(), self.common.entry_points.len()).await;
            launch_bundler(
                self.bundler,
                self.uopool,
                self.common,
                self.rpc,
                eth_client,
                block_streams,
            )
            .await?;
        } else {
            let eth_client = Arc::new(create_ws_provider(&self.common.eth_client_address).await?);
            let block_streams =
                create_ws_block_streams(eth_client.clone(), self.common.entry_points.len()).await;
            launch_bundler(
                self.bundler,
                self.uopool,
                self.common,
                self.rpc,
                eth_client,
                block_streams,
            )
            .await?;
        }

        pending().await
    }
}

/// Start the bundling component
#[derive(Debug, Parser)]
pub struct BundlerCommand {
    /// All Bundler specific args
    #[clap(flatten)]
    bundler: BundlerArgs,

    /// Common Bundler and UoPool args
    #[clap(flatten)]
    common: BundlerAndUoPoolArgs,

    /// UoPool gRPC listen address
    #[clap(long, default_value = "http://127.0.0.1:3002")]
    pub uopool_grpc_listen_address: String,
}

impl BundlerCommand {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<()> {
        if self.common.eth_client_address.clone().starts_with("http") {
            let eth_client = Arc::new(create_http_provider(&self.common.eth_client_address).await?);
            launch_bundling(
                self.bundler,
                eth_client,
                self.common.chain,
                self.common.entry_points,
                self.uopool_grpc_listen_address,
            )
            .await?;
        } else {
            let eth_client = Arc::new(create_ws_provider(&self.common.eth_client_address).await?);
            launch_bundling(
                self.bundler,
                eth_client,
                self.common.chain,
                self.common.entry_points,
                self.uopool_grpc_listen_address,
            )
            .await?;
        }

        pending().await
    }
}

/// Start the user operation mempool
#[derive(Debug, Parser)]
pub struct UoPoolCommand {
    /// All UoPool specific args
    #[clap(flatten)]
    uopool: UoPoolArgs,

    /// Common Bundler and UoPool args
    #[clap(flatten)]
    common: BundlerAndUoPoolArgs,
}

impl UoPoolCommand {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<()> {
        if self.common.eth_client_address.clone().starts_with("http") {
            let eth_client = Arc::new(create_http_provider(&self.common.eth_client_address).await?);
            let block_streams =
                create_http_block_streams(eth_client.clone(), self.common.entry_points.len()).await;
            launch_uopool(
                self.uopool,
                eth_client,
                block_streams,
                self.common.chain,
                self.common.entry_points,
            )
            .await?;
        } else {
            let eth_client = Arc::new(create_ws_provider(&self.common.eth_client_address).await?);
            let block_streams =
                create_ws_block_streams(eth_client.clone(), self.common.entry_points.len()).await;
            launch_uopool(
                self.uopool,
                eth_client,
                block_streams,
                self.common.chain,
                self.common.entry_points,
            )
            .await?;
        }

        pending().await
    }
}

/// Start the RPC server
#[derive(Debug, Parser)]
pub struct RpcCommand {
    /// All RPC args
    #[clap(flatten)]
    rpc: RpcArgs,

    /// UoPool gRPC listen address
    #[clap(long, default_value = "http://127.0.0.1:3002")]
    pub uopool_grpc_listen_address: String,

    /// Bundler gRPC listen address
    #[clap(long, default_value = "http://127.0.0.1:3003")]
    pub bundler_grpc_listen_address: String,
}

impl RpcCommand {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<()> {
        launch_rpc(
            self.rpc,
            self.uopool_grpc_listen_address,
            self.bundler_grpc_listen_address,
        )
        .await?;
        pending().await
    }
}

/// Create wallet for bundling component
#[derive(Debug, Parser)]
pub struct CreateWalletCommand {
    /// All create wallet args
    #[clap(flatten)]
    create_wallet: CreateWalletArgs,
}

impl CreateWalletCommand {
    /// Execute the command
    pub fn execute(self) -> eyre::Result<()> {
        create_wallet(self.create_wallet)
    }
}
