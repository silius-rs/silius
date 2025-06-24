use std::path::PathBuf;

use clap::Parser;

const DEFAULT_BUILDER_INTERVAL: u64 = 5000;
const DEFAULT_BUILDER_TYPE: &str = "transaction";
const DEFAULT_DISABLE_BUILDER: bool = false;

#[derive(Debug, Parser)]
pub struct BuilderConfig {
    #[arg(long, help = "Disable the builder", default_value_t = DEFAULT_DISABLE_BUILDER)]
    pub disable_builder: bool,

    #[arg(long, help = "Type of builder to use", default_value = DEFAULT_BUILDER_TYPE, value_parser = ["transaction"])]
    pub builder_type: String,

    #[arg(
        long,
        help = "Interval between bundle submissions in milliseconds",
        default_value_t = DEFAULT_BUILDER_INTERVAL
    )]
    pub builder_interval: u64,

    #[arg(
        long,
        help = "Path to the builder's wallet file (can be a mnemonic or private key)"
    )]
    pub wallet_path: Option<PathBuf>,

    #[arg(long, help = "Mnemonic or private key of the builder's wallet")]
    pub wallet: Option<String>,
}
