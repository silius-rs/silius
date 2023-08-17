use self::env::Env;
use reth_libmdbx::EnvironmentKind;
use std::path::PathBuf;

mod env;
pub mod mempool;
pub mod reputation;
mod tables;
mod utils;

pub fn init_env<E: EnvironmentKind>(path: PathBuf) -> anyhow::Result<Env<E>> {
    let env = Env::open(path)?;
    env.create_tables()?;
    Ok(env)
}
