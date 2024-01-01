//! The database implementation of the [Mempool](crate::mempool::Mempool) trait. Primarily used for
//! storing mempool information in a local database.

pub use self::env::DatabaseError;
use self::env::Env;
use reth_libmdbx::EnvironmentKind;
pub use reth_libmdbx::WriteMap;
use std::{path::PathBuf, sync::Arc};

mod env;
pub mod mempool;
pub mod reputation;
pub mod tables;
mod utils;

pub fn init_env<E: EnvironmentKind>(path: PathBuf) -> eyre::Result<Env<E>> {
    let env = Env::open(path)?;
    env.create_tables()?;
    Ok(env)
}
/// The database-based implementation of the [Mempool](crate::mempool::Mempool) trait.
#[derive(Debug)]
pub struct DatabaseTable<E: EnvironmentKind, Table> {
    pub env: Arc<Env<E>>,
    _table: std::marker::PhantomData<Table>,
}

impl<E: EnvironmentKind, Table> Clone for DatabaseTable<E, Table> {
    fn clone(&self) -> Self {
        Self { env: self.env.clone(), _table: std::marker::PhantomData }
    }
}

impl<E: EnvironmentKind, Table: Clone> DatabaseTable<E, Table> {
    pub fn new(env: Arc<Env<E>>) -> Self {
        Self { env, _table: std::marker::PhantomData }
    }
}
