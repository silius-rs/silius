use aa_bundler::models::wallet::Wallet;
use anyhow::Result;
use clap::Parser;
use dirs::home_dir;
use expanded_pathbuf::ExpandedPathBuf;
use jsonrpsee::tracing::info;
use std::str::FromStr;

#[derive(Parser)]
#[clap(
    name = "aa-bundler-create-wallet",
    about = "Bundler's wallet creation for EIP-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub output_path: Option<ExpandedPathBuf>,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let path = if let Some(output_path) = opt.output_path {
        output_path
    } else {
        ExpandedPathBuf::from_str(home_dir().unwrap().join(".aa-bundler").to_str().unwrap())
            .unwrap()
    };

    let wallet = Wallet::new(path);
    info!("{:?}", wallet.signer);

    Ok(())
}
