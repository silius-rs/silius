use aa_bundler::utils::parse_u256;
use aa_bundler_primitives::Wallet;
use anyhow::Result;
use clap::Parser;
use dirs::home_dir;
use ethers::types::U256;
use expanded_pathbuf::ExpandedPathBuf;
use tracing::info;

#[derive(Parser)]
#[clap(
    name = "aa-bundler-create-wallet",
    about = "Bundler's wallet creation for ERC-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long)]
    pub output_path: Option<ExpandedPathBuf>,

    #[clap(long, value_parser=parse_u256, default_value="1")]
    pub chain_id: U256,

    #[clap(long)]
    pub fb_bundler_path: Option<ExpandedPathBuf>,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let path = if let Some(output_path) = opt.output_path {
        output_path
    } else {
        home_dir()
            .map(|h| h.join(".aa-bundler/bundler.json"))
            .ok_or_else(|| anyhow::anyhow!("Get Home directory error"))
            .map(ExpandedPathBuf)?
    };

    let fb_path = if let Some(fb_bundler_path) = opt.fb_bundler_path.clone() {
        fb_bundler_path
    } else {
        home_dir()
            .map(|h| h.join(".aa-bundler/fb-bundler.json"))
            .ok_or_else(|| anyhow::anyhow!("Get Home directory error"))
            .map(ExpandedPathBuf)?
    };

    if opt.fb_bundler_path.is_some() {
        let wallet = Wallet::build_random(path.clone(), &opt.chain_id, None)?;
        info!("Wallet's Signer {:?}", wallet.signer);
        let fb_wallet = Wallet::build_random(path, &opt.chain_id, Some(fb_path))?;
        info!("Flashbots' Bundle Signer {:?}", fb_wallet.signer);
    } else {
        let wallet = Wallet::build_random(path, &opt.chain_id, None)?;
        info!("Wallet's Signer {:?}", wallet.signer);
    }

    Ok(())
}
