use aa_bundler::{
    bundler::Bundler,
    models::wallet::Wallet,
    parse_address, parse_u256,
    rpc::{eth::EthApiServerImpl, eth_api::EthApiServer},
};
use anyhow::Result;
use clap::Parser;
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::{future::pending, panic, sync::Arc};

#[derive(Parser)]
#[clap(
    name = "aa-bundler",
    about = "Bundler for EIP-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub mnemonic_file: ExpandedPathBuf,

    #[clap(long, value_parser=parse_address)]
    pub entry_point: Address,

    #[clap(long)]
    pub no_uopool: bool,

    #[clap(long, value_parser=parse_u256)]
    pub max_verification_gas: U256,

    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::UoPoolOpts,

    #[clap(long)]
    pub no_rpc: bool,

    #[clap(long, default_value = "127.0.0.1:4337")]
    pub rpc_listen_address: String,

    // execution client rpc endpoint
    #[clap(long, default_value = "127.0.0.1:8545")]
    pub eth_client_address: String,

    #[clap(flatten)]
    pub bundler_opts: aa_bundler::bundler::BundlerOpts,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(128 * 1024 * 1024)
                .build()?;

            rt.block_on(async move {
                info!("Starting AA - Bundler");

                let wallet = Wallet::from_file(opt.mnemonic_file);
                info!("{:?}", wallet.signer);

                let eth_provider = Arc::new(Provider::<Http>::try_from(opt.eth_client_address)?);

                let _bundler = Bundler::new(wallet);

                if !opt.no_uopool {
                    aa_bundler::uopool::run(
                        opt.uopool_opts,
                        eth_provider,
                        opt.entry_point,
                        opt.max_verification_gas,
                    )
                    .await?;
                }

                if !opt.no_rpc {
                    tokio::spawn({
                        async move {
                            let jsonrpc_server = ServerBuilder::default()
                                .build(&opt.rpc_listen_address)
                                .await
                                .unwrap();

                            let mut api = Methods::new();
                            api.merge(
                                EthApiServerImpl {
                                    call_gas_limit: 100_000_000,
                                }
                                .into_rpc(),
                            )
                            .unwrap();

                            let _jsonrpc_server_handle = jsonrpc_server.start(api.clone()).unwrap();
                            info!("JSON-RPC server listening on {}", opt.rpc_listen_address);

                            pending::<()>().await
                        }
                    });
                }

                pending().await
            })
        })?
        .join()
        .unwrap_or_else(|e| panic::resume_unwind(e))
}
