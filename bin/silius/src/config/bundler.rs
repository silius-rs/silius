use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct BundlerConfig {
    #[arg(
        long,
        help = "Path to the bundler's wallet file (can be a mnemonic or private key)"
    )]
    pub wallet_path: Option<PathBuf>,

    #[arg(long, help = "Mnemonic or private key of the bundler's wallet")]
    pub wallet: Option<String>,
}
