use std::path::PathBuf;

use clap::Parser;

const DEFAULT_DISABLE_BUILDER: bool = false;

#[derive(Debug, Parser)]
pub struct BuilderConfig {
    #[arg(long, help = "Disable the builder", default_value_t = DEFAULT_DISABLE_BUILDER)]
    pub disable_builder: bool,

    #[arg(
        long,
        help = "Path to the builder's wallet file (can be a mnemonic or private key)"
    )]
    pub wallet_path: Option<PathBuf>,

    #[arg(long, help = "Mnemonic or private key of the builder's wallet")]
    pub wallet: Option<String>,
}
