use aa_bundler::{
    bundler::bundler::Bundler,
    models::wallet::Wallet,
    rpc::{eth::EthApiServerImpl, eth_api::EthApiServer},
};
use anyhow::Result;
use clap::Parser;
use expanded_pathbuf::ExpandedPathBuf;
use jsonrpsee::{core::server::rpc_module::Methods, server::ServerBuilder, tracing::info};
use std::{future::pending, panic};

#[derive(Parser)]
#[clap(
    name = "aa-bundler",
    about = "Bundler for EIP-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub mnemonic_file: ExpandedPathBuf,

    // #[clap(long, default_value = "127.0.0.1:3000")]
    // pub grpc_listen_address: String,
    #[clap(long)]
    pub no_uopool: bool,

    #[clap(flatten)]
    pub uopool_opts: aa_bundler::uopool::Opts,

    #[clap(long)]
    pub no_rpc: bool,

    #[clap(long, default_value = "127.0.0.1:4337")]
    pub rpc_listen_address: String,
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
                println!("{:?}", wallet.signer);

                let bundler = Bundler::new(wallet);

                if !opt.no_uopool {
                    aa_bundler::uopool::run(opt.uopool_opts).await?;
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
