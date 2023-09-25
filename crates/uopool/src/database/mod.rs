//! The database implementation of the [Mempool](crate::mempool::Mempool) trait. Primarily used for storing mempool information in a local database.
pub use self::env::DBError;
use self::env::Env;
use reth_libmdbx::EnvironmentKind;
pub use reth_libmdbx::WriteMap;
use std::path::PathBuf;

mod env;
pub mod mempool;
pub mod reputation;
mod tables;
mod utils;

pub fn init_env<E: EnvironmentKind>(path: PathBuf) -> eyre::Result<Env<E>> {
    let env = Env::open(path)?;
    env.create_tables()?;
    Ok(env)
}
