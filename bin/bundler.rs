use aa_bundler::rpc::{eth::EthApiServerImpl, eth_api::EthApiServer};
use anyhow::Result;
use clap::Parser;
use ethers::{
    prelude::rand,
    signers::{coins_bip39::English, MnemonicBuilder},
};
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
    pub mnemonic_file: Option<ExpandedPathBuf>,

    #[clap(long, default_value = "./src/res/bundler")]
    pub mnemonic_folder: ExpandedPathBuf,

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

                // TODO: move this to bundler package
                let wallet = if let Some(mnemonic_file) = opt.mnemonic_file {
                    MnemonicBuilder::<English>::default()
                        .phrase(mnemonic_file.to_path_buf())
                        .build()?
                } else {
                    let mut rng = rand::thread_rng();
                    MnemonicBuilder::<English>::default()
                        .write_to(opt.mnemonic_folder.to_path_buf())
                        .build_random(&mut rng)?
                };

                println!("{:?}", wallet);

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
                            info!("JSONRPC server listening on {}", opt.rpc_listen_address);

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
