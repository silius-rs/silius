//! The database implementation of the [Mempool](crate::mempool::Mempool) trait. Primarily used for
//! storing mempool information in a local database.

pub use reth_db::{
    init_db, mdbx::DatabaseArguments, DatabaseEnv, DatabaseError as RethDatabaseError,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use thiserror::Error;

pub mod mempool;
pub mod reputation;
pub mod tables;
mod utils;

/// The database-based implementation of the [Mempool](crate::mempool::Mempool) trait.
#[derive(Debug)]
pub struct DatabaseTable<Table> {
    pub env: Arc<DatabaseEnv>,
    _table: std::marker::PhantomData<Table>,
}

impl<Table> Clone for DatabaseTable<Table> {
    fn clone(&self) -> Self {
        Self { env: self.env.clone(), _table: std::marker::PhantomData }
    }
}

impl<Table> DatabaseTable<Table> {
    pub fn new(env: Arc<DatabaseEnv>) -> Self {
        Self { env, _table: std::marker::PhantomData }
    }
}

/// Database error
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Internal database error
    #[error(transparent)]
    Internal(RethDatabaseError),
    /// Databse not found
    #[error("Database not found")]
    NotFound,
}

impl From<RethDatabaseError> for DatabaseError {
    fn from(value: RethDatabaseError) -> Self {
        DatabaseError::Internal(value)
    }
}

impl Serialize for DatabaseError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{self:?}"))
    }
}

// TODO: implement correct deserialization
impl<'de> Deserialize<'de> for DatabaseError {
    fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Ok(DatabaseError::NotFound)
    }
}
