use std::env;

use clap::Parser;
use silius::cli::Cli;
use tracing_subscriber::EnvFilter;

fn print_ascii_logo() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║               SILIUS - ERC-4337 BUNDLER                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}

#[tokio::main]
async fn main() {
    print_ascii_logo();

    let rust_log = env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default();
    let env_filter = match rust_log.is_empty() {
        true => EnvFilter::builder().parse_lossy("info"),
        false => EnvFilter::builder().parse_lossy(rust_log),
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();
}
