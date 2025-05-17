use super::args::{
    BundlerAndUoPoolArgs, BundlerArgs, CreateWalletArgs, MetricsArgs, RpcArgs, UoPoolArgs,
};
use crate::bundler::{create_wallet, launch_bundler, launch_bundling, launch_rpc, launch_uopool};
use clap::{Parser, Subcommand};
use ethers::types::Address;
use silius_mempool::{
    init_env, DatabaseTable, UserOperationAddrOp, UserOperationOp, UserOperations,
    UserOperationsByEntity, UserOperationsBySender, WriteMap,
};
use silius_metrics::ethers::MetricsMiddleware;
use silius_primitives::provider::{
    create_http_block_streams, create_http_provider, create_ws_block_streams, create_ws_provider,
};
use std::{future::pending, path::PathBuf, sync::Arc};

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
            let http_client =
                create_http_provider(&self.common.eth_client_address, self.common.poll_interval)
                    .await?;
            let eth_client = Arc::new(MetricsMiddleware::new(http_client));

            let eth_bundle_client = if let Some(eth_client_bundle_address) =
                self.bundler.eth_client_bundle_address.clone()
            {
                let http_client_bundle =
                    create_http_provider(&eth_client_bundle_address, self.common.poll_interval)
                        .await?;
                Arc::new(MetricsMiddleware::new(http_client_bundle))
            } else {
                eth_client.clone()
            };

            let block_streams =
                create_http_block_streams(eth_client.clone(), self.common.entry_points.len()).await;

            launch_bundler(
                self.bundler,
                self.uopool,
                self.common.clone(),
                self.rpc,
                self.common.metrics,
                eth_client,
                eth_bundle_client,
                block_streams,
            )
            .await?;
        } else {
            let ws_client = create_ws_provider(&self.common.eth_client_address).await?;
            let eth_client = Arc::new(MetricsMiddleware::new(ws_client));

            let block_streams =
                create_ws_block_streams(eth_client.clone(), self.common.entry_points.len()).await;

            if let Some(eth_client_bundle_address) = self.bundler.eth_client_bundle_address.clone()
            {
                let http_client_bundle =
                    create_http_provider(&eth_client_bundle_address, self.common.poll_interval)
                        .await?;
                let eth_client_bundle = Arc::new(MetricsMiddleware::new(http_client_bundle));

                launch_bundler(
                    self.bundler,
                    self.uopool,
                    self.common.clone(),
                    self.rpc,
                    self.common.metrics,
                    eth_client,
                    eth_client_bundle,
                    block_streams,
                )
                .await?;
            } else {
                launch_bundler(
                    self.bundler,
                    self.uopool,
                    self.common.clone(),
                    self.rpc,
                    self.common.metrics,
                    eth_client.clone(),
                    eth_client,
                    block_streams,
                )
                .await?;
            }
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
        let eth_client_address = if let Some(eth_client_bundle_address) =
            self.bundler.eth_client_bundle_address.clone()
        {
            eth_client_bundle_address
        } else {
            self.common.eth_client_address.clone()
        };

        if eth_client_address.clone().starts_with("http") {
            let eth_client = Arc::new(
                create_http_provider(&eth_client_address, self.common.poll_interval).await?,
            );
            launch_bundling(
                self.bundler,
                eth_client,
                self.common.chain,
                self.common.entry_points,
                self.uopool_grpc_listen_address,
                self.common.metrics,
            )
            .await?;
        } else {
            let eth_client = Arc::new(create_ws_provider(&eth_client_address).await?);
            launch_bundling(
                self.bundler,
                eth_client,
                self.common.chain,
                self.common.entry_points,
                self.uopool_grpc_listen_address,
                self.common.metrics,
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
            let eth_client = Arc::new(
                create_http_provider(&self.common.eth_client_address, self.common.poll_interval)
                    .await?,
            );
            let block_streams =
                create_http_block_streams(eth_client.clone(), self.common.entry_points.len()).await;
            launch_uopool(
                self.uopool,
                eth_client,
                block_streams,
                self.common.chain,
                self.common.entry_points,
                self.common.metrics,
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
                self.common.metrics,
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

    /// All metrics args
    #[clap(flatten)]
    metrics: MetricsArgs,
}

impl RpcCommand {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<()> {
        launch_rpc(
            self.rpc,
            self.uopool_grpc_listen_address,
            self.bundler_grpc_listen_address,
            self.metrics,
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

/// Dump the database
#[derive(Debug, Subcommand)]
/// Represents the `Dump` command.
pub enum DebugCommand {
    #[command(name = "dump-userops")]
    DumpUserops(DumpUserOperations),

    #[command(name = "dump-uo-by-sender")]
    DumpUoBySender(DumpUserOperationsBySender),
}

impl DebugCommand {
    /// Execute the command
    pub fn execute(self) -> eyre::Result<()> {
        match self {
            DebugCommand::DumpUserops(command) => command.execute(),
            DebugCommand::DumpUoBySender(command) => command.execute(),
        }
    }
}
#[derive(Debug, Parser)]
pub struct DumpUserOperations {
    /// The directory where the data will be dumped.
    #[clap(long, short)]
    data_dir: PathBuf,
}

impl DumpUserOperations {
    pub fn execute(self) -> eyre::Result<()> {
        let env = Arc::new(init_env::<WriteMap>(self.data_dir).expect("Init mdbx failed"));
        let table = DatabaseTable::<WriteMap, UserOperations>::new(env.clone());
        let uo = table.get_all()?;
        serde_json::to_writer(std::io::stdout(), &uo)?;
        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct DumpUserOperationsBySender {
    /// The directory where the data will be dumped.
    #[clap(long, short)]
    data_dir: PathBuf,

    #[clap(long)]
    address: Address,
}
impl DumpUserOperationsBySender {
    pub fn execute(self) -> eyre::Result<()> {
        let env = Arc::new(init_env::<WriteMap>(self.data_dir).expect("Init mdbx failed"));
        let table = DatabaseTable::<WriteMap, UserOperationsBySender>::new(env.clone());
        let mut uo = table.get_all_by_address(&self.address);

        let table = DatabaseTable::<WriteMap, UserOperationsByEntity>::new(env.clone());
        let mut uo2 = table.get_all_by_address(&self.address);
        uo.append(&mut uo2);
        serde_json::to_writer(std::io::stdout(), &uo)?;
        Ok(())
    }
}
