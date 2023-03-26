use ethers::{
    types::{Address, Bytes},
    utils::to_checksum,
};
use reth_db::table::{Compress, Decode, Decompress, Encode};
use serde::{Deserialize, Serialize};

pub fn as_checksum<S>(val: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&to_checksum(val, None))
}

macro_rules! construct_wrap_hash {
    ($type:ty, $name:ident, $n_bytes:expr ) => {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
        pub struct $name($type);

        impl Decode for $name {
            fn decode<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                Ok(<$type>::from_slice(value.into().as_ref()).into())
            }
        }

        impl Encode for $name {
            type Encoded = [u8; $n_bytes];
            fn encode(self) -> Self::Encoded {
                *self.0.as_fixed_bytes()
            }
        }

        impl From<$type> for $name {
            fn from(value: $type) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $type {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl Compress for $name {
            type Compressed = Bytes;
            fn compress(self) -> Self::Compressed {
                Bytes::from(self.encode())
            }
        }

        impl Decompress for $name {
            fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                Self::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
            }
        }
    };
}

construct_wrap_hash!(Address, WrapAddress, 20);
