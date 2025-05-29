pub mod code_hash;
pub mod entity;
pub mod reputation;
pub mod sender;
pub mod user_operation;

use std::{any::type_name, cmp::Ordering, fmt::Debug};

use bincode::serde::{decode_from_slice, encode_to_vec};
use redb::{Key, TypeName, Value};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::DatabaseError;

pub trait Table {
    type Key;

    type Value;

    fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, DatabaseError>;

    fn insert(&self, key: Self::Key, value: Self::Value) -> Result<(), DatabaseError>;

    fn remove(&self, key: Self::Key) -> Result<(), DatabaseError>;
}

#[allow(clippy::result_large_err)]
pub trait MultimapTable {
    type Key;

    type GetValue;

    type InsertValue;

    type RemoveValue;

    fn get(&self, key: Self::Key) -> Result<Option<Self::GetValue>, DatabaseError>;

    fn insert(&self, key: Self::Key, value: Self::InsertValue) -> Result<(), DatabaseError>;

    fn remove(&self, key: Self::Key, value: Self::RemoveValue) -> Result<(), DatabaseError>;
}

#[derive(Debug)]
pub struct Bincode<T>(pub T);

impl<T> Value for Bincode<T>
where
    T: Debug + Serialize + DeserializeOwned,
{
    type SelfType<'a>
        = T
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        decode_from_slice(data, bincode::config::standard())
            .unwrap()
            .0
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        encode_to_vec(value, bincode::config::standard()).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new(&format!("Bincode<{}>", type_name::<T>()))
    }
}

impl<T> Key for Bincode<T>
where
    T: Debug + Ord + Serialize + DeserializeOwned,
{
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        Self::from_bytes(data1).cmp(&Self::from_bytes(data2))
    }
}
