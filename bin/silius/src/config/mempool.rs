use std::path::PathBuf;

use clap::{Parser, arg};

#[derive(Debug, Parser)]
pub struct MempoolConfig {
    #[arg(
        long,
        help = "The directory for storing application data. If used together with --ephemeral, new child directory will be created."
    )]
    pub data_dir: Option<PathBuf>,

    #[arg(
        long,
        short,
        help = "Use new data directory, located in OS temporary directory. If used together with --data-dir, new directory will be created there instead."
    )]
    pub ephemeral: bool,

    #[arg(long, help = "Purges the database.")]
    pub purge_db: bool,
}
