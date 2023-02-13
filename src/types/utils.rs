use ethers::types::Address;
use reth_db::table::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct WrapAddress(Address);

impl Decode for WrapAddress {
    fn decode<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        Ok(Address::from_slice(value.into().as_ref()).into())
    }
}

impl Encode for WrapAddress {
    type Encoded = [u8; 20];
    fn encode(self) -> Self::Encoded {
        *self.0.as_fixed_bytes()
    }
}

impl From<Address> for WrapAddress {
    fn from(value: Address) -> Self {
        Self(value)
    }
}

impl From<WrapAddress> for Address {
    fn from(value: WrapAddress) -> Self {
        value.0
    }
}
