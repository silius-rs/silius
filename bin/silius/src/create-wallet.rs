use anyhow::Result;
use clap::Parser;
use ethers::types::U256;
use expanded_pathbuf::ExpandedPathBuf;
use silius::utils::{parse_u256, unwrap_path_or_home};
use silius_primitives::Wallet;
use tracing::info;

#[derive(Parser)]
#[clap(
    name = "silius-create-wallet",
    about = "Bundler's wallet creation for ERC-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub output_path: Option<ExpandedPathBuf>,

    #[clap(long, value_parser=parse_u256, default_value="1")]
    pub chain_id: U256,

    #[clap(long)]
    pub build_fb_wallet: Option<bool>,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let path = unwrap_path_or_home(opt.output_path)?;

    if opt.build_fb_wallet == Some(true) {
        let wallet = Wallet::build_random(path, &opt.chain_id, true)?;
        info!("Wallet Signer {:?}", wallet.signer);
        info!("Flashbots Signer {:?}", wallet.fb_signer);
    } else {
        let wallet = Wallet::build_random(path, &opt.chain_id, false)?;
        info!("Wallet Signer {:?}", wallet.signer);
    }

    Ok(())
}
