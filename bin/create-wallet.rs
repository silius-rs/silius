use aa_bundler::models::wallet::Wallet;
use anyhow::Result;
use clap::Parser;
use expanded_pathbuf::ExpandedPathBuf;

#[derive(Parser)]
#[clap(
    name = "aa-bundler-create-wallet",
    about = "Bundler's wallet creation for EIP-4337 Account Abstraction"
)]
pub struct Opt {
    #[clap(long, default_value = "./src/res/bundler")]
    pub output_folder: ExpandedPathBuf,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    tracing_subscriber::fmt::init();

    let wallet = Wallet::new(opt.output_folder);
    println!("{:?}", wallet.signer);

    Ok(())
}
