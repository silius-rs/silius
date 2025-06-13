use bin_layout::{Decoder, Encoder};
use ethers::{
    abi::{AbiDecode, AbiEncode},
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, Bytes},
};
use reth_db::table::{Compress, Decode, Decompress, Encode};
use serde::{Deserialize, Serialize};
use silius_primitives::{
    reputation::ReputationEntry, simulation::CodeHash, UserOperationHash, UserOperationSigned,
};
use std::{collections::HashSet, fmt::Debug};

/// Creates a compression & decompression wrapper for a type(20 or 32 bytes) that is used in the
/// database.
macro_rules! construct_wrap_hash {
    ($type:ty, $name:ident, $n_bytes:expr ) => {
        #[derive(
            Default, Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize,
        )]
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
                <Self as Encode>::encode(self).into()
            }
        }

        impl Decompress for $name {
            fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                <Self as Decode>::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
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
                <Self as AbiEncode>::encode(self).into()
            }
        }

        impl Decompress for $name {
            fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
                <Self as AbiDecode>::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
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
construct_wrap_struct!(UserOperationSigned, WrapUserOperationSigned);
construct_wrap_struct!(ReputationEntry, WrapReputationEntry);

impl<'de> Decoder<'de> for WrapUserOperationHash {
    fn decoder(data: &mut &'de [u8]) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let data: [u8; 32] = <[u8; 32]>::decoder(data)?;
        Ok(WrapUserOperationHash(UserOperationHash::from_slice(&data)))
    }
}

impl Encoder for WrapUserOperationHash {
    fn encoder(&self, write: &mut impl std::io::prelude::Write) -> std::io::Result<()> {
        self.0.as_fixed_bytes().encoder(write)
    }
}
impl<'de> Decoder<'de> for WrapCodeHash {
    fn decoder(data: &mut &'de [u8]) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let address = <[u8; 20]>::decoder(data)?.into();
        let hash = <[u8; 32]>::decoder(data)?.into();
        Ok(WrapCodeHash(CodeHash { address, hash }))
    }
}

impl Encoder for WrapCodeHash {
    fn encoder(&self, write: &mut impl std::io::prelude::Write) -> std::io::Result<()> {
        self.0.address.as_fixed_bytes().encoder(write)?;
        self.0.hash.as_fixed_bytes().encoder(write)
    }
}

#[derive(Decoder, Encoder, Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WrapUserOpSet(HashSet<WrapUserOperationHash>);

impl WrapUserOpSet {
    pub fn insert(&mut self, value: WrapUserOperationHash) -> bool {
        self.0.insert(value)
    }

    pub fn remove(&mut self, value: &WrapUserOperationHash) -> bool {
        self.0.remove(value)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_vec(&self) -> Vec<UserOperationHash> {
        self.0.iter().cloned().map(Into::into).collect()
    }
}

impl From<HashSet<WrapUserOperationHash>> for WrapUserOpSet {
    fn from(value: HashSet<WrapUserOperationHash>) -> Self {
        Self(value)
    }
}

impl From<WrapUserOpSet> for HashSet<WrapUserOperationHash> {
    fn from(value: WrapUserOpSet) -> Self {
        value.0
    }
}

impl Compress for WrapUserOpSet {
    type Compressed = Vec<u8>;
    fn compress(self) -> Self::Compressed {
        self.encode()
    }
}

impl Decompress for WrapUserOpSet {
    fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        Self::decode(value.into().as_ref()).map_err(|_| reth_db::Error::DecodeError)
    }
}

#[derive(Decoder, Encoder, Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WrapCodeHashVec(Vec<WrapCodeHash>);

impl From<Vec<WrapCodeHash>> for WrapCodeHashVec {
    fn from(value: Vec<WrapCodeHash>) -> Self {
        Self(value)
    }
}

impl From<WrapCodeHashVec> for Vec<WrapCodeHash> {
    fn from(value: WrapCodeHashVec) -> Self {
        value.0
    }
}
impl Compress for WrapCodeHashVec {
    type Compressed = Vec<u8>;
    fn compress(self) -> Self::Compressed {
        <Vec<WrapCodeHash> as Encoder>::encode(&self.0)
    }
}

impl Decompress for WrapCodeHashVec {
    fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        let v = value.into();
        let decoded = <Vec<WrapCodeHash> as Decoder>::decode(v.as_ref())
            .map_err(|_| reth_db::Error::DecodeError)?;
        Ok(decoded.into())
    }
}
