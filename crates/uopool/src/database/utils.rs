use ethers::{
    abi::{AbiDecode, AbiEncode},
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, Bytes},
};
use reth_db::table::{Compress, Decode, Decompress, Encode};
use serde::{Deserialize, Serialize};
use silius_primitives::{
    reputation::ReputationEntry, simulation::CodeHash, UserOperation, UserOperationHash,
};

/// Cretaes a compression & decompression wrapper for a type(20 or 32 bytes) that is used in the database.
macro_rules! construct_wrap_hash {
    ($type:ty, $name:ident, $n_bytes:expr ) => {
        #[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
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
                self.encode().into()
            }
        }

        impl Decompress for $name {
            fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                Self::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
            }
        }
    };
}

/// Cretaes a compression & decompression wrapper for a type(struct) that is used in the database.
macro_rules! construct_wrap_struct {
    ($type:ty, $name:ident ) => {
        #[derive(
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Clone,
            Serialize,
            Deserialize,
            EthAbiCodec,
            EthAbiType,
        )]
        pub struct $name(pub $type);

        impl Compress for $name {
            type Compressed = Bytes;
            fn compress(self) -> Self::Compressed {
                self.encode().into()
            }
        }

        impl Decompress for $name {
            fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                Self::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
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
    };
}

construct_wrap_hash!(Address, WrapAddress, 20);
construct_wrap_hash!(UserOperationHash, WrapUserOperationHash, 32);

construct_wrap_struct!(CodeHash, WrapCodeHash);
construct_wrap_struct!(UserOperation, WrapUserOperation);
construct_wrap_struct!(ReputationEntry, WrapReputationEntry);
